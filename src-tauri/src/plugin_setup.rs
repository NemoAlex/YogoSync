//! Prepare the personal-marketplace package without replacing unrelated plugins.
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use tauri::Manager;
static SETUP_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn encode(s: &str) -> String {
    s.as_bytes()
        .iter()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-._~".contains(b) {
                (*b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}
fn home() -> Result<PathBuf, String> {
    std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
        .map(PathBuf::from)
        .ok_or("无法定位用户目录".into())
}
fn prepare(home: &Path, resource: &Path) -> Result<PathBuf, String> {
    let marketplace = home.join(".agents/plugins/marketplace.json");
    let original = match std::fs::read(&marketplace) {
        Ok(v) => Some(v),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(format!("无法读取个人插件市场：{e}")),
    };
    let mut config: Value = match &original {
        Some(bytes) => {
            serde_json::from_slice(bytes).map_err(|e| format!("个人插件市场格式有误：{e}"))?
        }
        None => json!({"name":"personal","interface":{"displayName":"Personal"},"plugins":[]}),
    };
    let name = config["name"].as_str().ok_or("个人插件市场缺少名称")?;
    if name.is_empty()
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
    {
        return Err("个人插件市场名称无效".into());
    }
    let plugins = config["plugins"]
        .as_array()
        .ok_or("个人插件市场缺少插件列表")?;
    // Existing installations may be owned by the user or come from another source.
    // Reuse their marketplace entry; never replace it or change trust.
    if plugins.iter().any(|p| p["name"] == "yogo-pet") {
        return Ok(marketplace);
    }
    let source = home.join("plugins/yogo-pet");
    if source.exists() {
        return Err("发现已有 ~/plugins/yogo-pet，请先在 Codex 中导入现有插件，避免覆盖。".into());
    }
    let stage = home.join(format!("plugins/.yogo-pet-setup-{}", std::process::id()));
    if stage.exists() {
        return Err("插件安装准备目录已存在，请稍后重试。".into());
    }
    let result = (|| {
        for dir in [".codex-plugin", "hooks", "bin"] {
            yogo_core::events::private_dir(&stage.join(dir))?;
        }
        let binary = if cfg!(windows) {
            "yogo-pet-hook.exe"
        } else {
            "yogo-pet-hook"
        };
        std::fs::copy(
            resource.join("resources").join(binary),
            stage.join("bin").join(binary),
        )
        .map_err(|e| format!("无法准备插件助手：{e}"))?;
        std::fs::write(
            stage.join(".codex-plugin/plugin.json"),
            include_str!("../resources/plugin.json"),
        )
        .map_err(|e| e.to_string())?;
        std::fs::write(
            stage.join("hooks/hooks.json"),
            include_str!("../resources/hooks.json"),
        )
        .map_err(|e| e.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(
                stage.join("bin").join(binary),
                std::fs::Permissions::from_mode(0o755),
            )
            .map_err(|e| e.to_string())?;
        }
        config["plugins"].as_array_mut().unwrap().push(json!({"name":"yogo-pet","source":{"source":"local","path":"./plugins/yogo-pet"},"policy":{"installation":"AVAILABLE","authentication":"ON_INSTALL"},"category":"Productivity"}));
        yogo_core::events::private_dir(marketplace.parent().unwrap())?;
        // Abort if another app changed the market while the package was prepared.
        let current = std::fs::read(&marketplace).ok();
        if current != original {
            return Err("个人插件市场已变更，请重试。".into());
        }
        let temporary = marketplace.with_extension(format!("yogo-{}.tmp", std::process::id()));
        let bytes = serde_json::to_vec_pretty(&config).map_err(|e| e.to_string())?;
        std::fs::write(&temporary, bytes).map_err(|e| e.to_string())?;
        std::fs::rename(&stage, &source).map_err(|e| e.to_string())?;
        if let Err(e) = std::fs::rename(&temporary, &marketplace) {
            let _ = std::fs::remove_file(&temporary);
            let _ = std::fs::remove_dir_all(&source);
            return Err(format!("无法保存插件市场：{e}"));
        }
        Ok(marketplace.clone())
    })();
    if result.is_err() {
        let _ = std::fs::remove_dir_all(&stage);
    }
    result
}

#[tauri::command]
pub async fn install_codex_plugin(app: tauri::AppHandle) -> Result<(), String> {
    let resource = app.path().resource_dir().map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = SETUP_LOCK
            .lock()
            .map_err(|_| "插件准备暂不可用，请重启应用")?;
        let market = prepare(&home()?, &resource)?;
        super::codex_connection::open_link(&format!(
            "codex://plugins/yogo-pet?marketplacePath={}",
            encode(&market.to_string_lossy())
        ))
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn open_codex_plugin() -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(|| {
        super::codex_connection::open_link("codex://plugins/yogo-pet")
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn setup_preserves_existing_entries_and_is_repeatable() {
        let root = std::env::temp_dir().join(format!("yogo-onboarding-{}", std::process::id()));
        let home = root.join("home");
        let resources = root.join("app/resources");
        std::fs::create_dir_all(home.join(".agents/plugins")).unwrap();
        std::fs::create_dir_all(&resources).unwrap();
        let binary = if cfg!(windows) {
            "yogo-pet-hook.exe"
        } else {
            "yogo-pet-hook"
        };
        std::fs::write(resources.join(binary), "fixture").unwrap();
        let path = home.join(".agents/plugins/marketplace.json");
        std::fs::write(&path, r#"{"name":"personal","interface":{"displayName":"Mine"},"plugins":[{"name":"keep-me"}]}"#).unwrap();
        prepare(&home, &root.join("app")).unwrap();
        let first = std::fs::read(&path).unwrap();
        let market: Value = serde_json::from_slice(&first).unwrap();
        assert_eq!(market["interface"]["displayName"], "Mine");
        assert_eq!(market["plugins"][0]["name"], "keep-me");
        assert_eq!(market["plugins"].as_array().unwrap().len(), 2);
        assert!(home.join("plugins/yogo-pet/hooks/hooks.json").is_file());
        prepare(&home, &root.join("app")).unwrap();
        assert_eq!(first, std::fs::read(&path).unwrap());
        std::fs::remove_dir_all(root).unwrap();
    }
}
