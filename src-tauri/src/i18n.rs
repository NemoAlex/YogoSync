use std::collections::HashMap;
use std::path::Path;
use std::sync::{
    atomic::{AtomicU8, Ordering},
    OnceLock,
};
use tauri::Emitter;

fn resolve_locale(language: &str) -> &'static str {
    let primary = language.split(['-', '_', '.']).next().unwrap_or("");
    if primary.eq_ignore_ascii_case("zh") {
        "zh-CN"
    } else {
        "en"
    }
}

#[cfg(target_os = "macos")]
fn apple_language(preferences: &str) -> Option<&str> {
    preferences
        .split(['(', ')', ',', '\n'])
        .map(|value| value.trim().trim_matches('"'))
        .find(|value| !value.is_empty())
}

fn system_locale() -> &'static str {
    // GUI apps do not reliably inherit LANG. Read macOS's ordered UI language
    // preferences, not its regional number/date locale or the shell locale.
    #[cfg(target_os = "macos")]
    if let Ok(output) = std::process::Command::new("/usr/bin/defaults")
        .args(["read", "-g", "AppleLanguages"])
        .output()
    {
        if output.status.success() {
            let value = String::from_utf8_lossy(&output.stdout);
            if let Some(language) = apple_language(&value) {
                return resolve_locale(language);
            }
        }
    }
    for name in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(value) = std::env::var(name) {
            if !value.is_empty() {
                return resolve_locale(&value);
            }
        }
    }
    "en"
}

fn language_code(language: &str) -> Result<u8, String> {
    match language {
        "system" => Ok(0),
        "zh-CN" => Ok(1),
        "en" => Ok(2),
        _ => Err("Invalid language".into()),
    }
}
fn load_language(dir: &Path) -> u8 {
    std::fs::read(dir.join("language.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<String>(&bytes).ok())
        .and_then(|value| language_code(&value).ok())
        .unwrap_or(0)
}
fn save_language(dir: &Path, language: &str) -> Result<(), String> {
    language_code(language)?;
    yogo_core::events::atomic_write(
        &dir.join("language.json"),
        &serde_json::to_vec(language).map_err(|e| e.to_string())?,
    )
}
fn preference() -> &'static AtomicU8 {
    static VALUE: OnceLock<AtomicU8> = OnceLock::new();
    VALUE.get_or_init(|| AtomicU8::new(load_language(&yogo_core::config_store::data_dir())))
}
#[tauri::command]
pub fn get_language_preference() -> &'static str {
    match preference().load(Ordering::SeqCst) {
        1 => "zh-CN",
        2 => "en",
        _ => "system",
    }
}
#[tauri::command]
pub fn get_locale() -> &'static str {
    match preference().load(Ordering::SeqCst) {
        1 => "zh-CN",
        2 => "en",
        _ => {
            static SYSTEM: OnceLock<&'static str> = OnceLock::new();
            SYSTEM.get_or_init(system_locale)
        }
    }
}
#[tauri::command]
pub fn set_language(app: tauri::AppHandle, language: String) -> Result<(), String> {
    let code = language_code(&language)?;
    let previous = get_language_preference();
    let dir = yogo_core::config_store::data_dir();
    save_language(&dir, &language)?;
    preference().store(code, Ordering::SeqCst);
    if let Err(error) = super::refresh_language(&app).and_then(|_| {
        app.emit("language-changed", get_locale())
            .map_err(|e| e.to_string())
    }) {
        preference().store(language_code(previous)?, Ordering::SeqCst);
        let _ = save_language(&dir, previous);
        let _ = super::refresh_language(&app);
        return Err(error);
    }
    Ok(())
}

pub fn t(key: &str) -> &str {
    if get_locale() != "en" {
        return key;
    }
    static CATALOG: OnceLock<HashMap<String, String>> = OnceLock::new();
    CATALOG
        .get_or_init(|| {
            serde_json::from_str(include_str!("../../desktop/locales/en.json"))
                .expect("Invalid bundled language catalog")
        })
        .get(key)
        .map(String::as_str)
        .unwrap_or(key)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn language_preference_round_trips_and_rejects_invalid_values() {
        let dir = std::env::temp_dir().join(format!("yogo-language-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(load_language(&dir), 0);
        for (value, code) in [("zh-CN", 1), ("en", 2), ("system", 0)] {
            save_language(&dir, value).unwrap();
            assert_eq!(load_language(&dir), code);
        }
        assert!(save_language(&dir, "invalid").is_err());
        assert_eq!(load_language(&dir), 0);
        std::fs::write(dir.join("language.json"), b"broken").unwrap();
        assert_eq!(load_language(&dir), 0);
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn chinese_variants_and_english_fallback() {
        for language in ["zh", "zh-CN", "zh_TW.UTF-8", "zh-Hant-HK", "ZH-sg"] {
            assert_eq!(resolve_locale(language), "zh-CN");
        }
        for language in ["en-GB", "ja-JP", "de", "", "C", "zho"] {
            assert_eq!(resolve_locale(language), "en");
        }
    }
    #[cfg(target_os = "macos")]
    #[test]
    fn uses_first_preferred_language_not_any_chinese_entry() {
        assert_eq!(
            apple_language("(\n    \"en-US\",\n    \"zh-Hans-CN\"\n)"),
            Some("en-US")
        );
        assert_eq!(
            apple_language("(\n    \"zh-Hant\",\n    en\n)"),
            Some("zh-Hant")
        );
        assert_eq!(apple_language("()"), None);
    }
}
