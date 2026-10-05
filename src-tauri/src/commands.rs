use tauri::{Manager, State};
use yogo_core::{
    config_store::Preferences,
    service_runtime::{DesktopSnapshot, ServiceRuntime},
    task_states::PetState,
};
#[tauri::command]
pub fn get_snapshot(runtime: State<ServiceRuntime>) -> DesktopSnapshot {
    runtime.snapshot()
}
async fn run(
    runtime: ServiceRuntime,
    action: impl FnOnce(ServiceRuntime) -> Result<(), String> + Send + 'static,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || action(runtime))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn start_service(runtime: State<'_, ServiceRuntime>) -> Result<(), String> {
    run(runtime.inner().clone(), |r| r.start()).await
}
#[tauri::command]
pub async fn stop_service(runtime: State<'_, ServiceRuntime>) -> Result<(), String> {
    run(runtime.inner().clone(), |r| r.stop()).await
}
#[tauri::command]
pub async fn preview_state(
    runtime: State<'_, ServiceRuntime>,
    state: PetState,
) -> Result<(), String> {
    run(runtime.inner().clone(), move |r| r.demo(state)).await
}
#[tauri::command]
pub async fn automatic(runtime: State<'_, ServiceRuntime>) -> Result<(), String> {
    run(runtime.inner().clone(), |r| r.automatic()).await
}
#[tauri::command]
pub async fn save_preferences(
    runtime: State<'_, ServiceRuntime>,
    preferences: Preferences,
) -> Result<(), String> {
    run(runtime.inner().clone(), move |r| r.save(preferences)).await
}
#[tauri::command]
pub async fn clear_logs(runtime: State<'_, ServiceRuntime>) -> Result<(), String> {
    run(runtime.inner().clone(), |r| r.clear_logs()).await
}
#[tauri::command]
pub fn open_settings(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("settings") {
        w.show().map_err(|e| e.to_string())?;
        return w.set_focus().map_err(|e| e.to_string());
    }
    tauri::WebviewWindowBuilder::new(
        &app,
        "settings",
        tauri::WebviewUrl::App("settings.html".into()),
    )
    .title(crate::i18n::t("YogoSync 设置"))
    .inner_size(460.0, 370.0)
    .min_inner_size(420.0, 350.0)
    .build()
    .map_err(|e| e.to_string())?;
    Ok(())
}
#[tauri::command]
pub fn quit_app(app: tauri::AppHandle) {
    super::quit(app)
}
#[tauri::command]
pub fn open_themes(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("themes") {
        w.show().map_err(|e| e.to_string())?;
        return w.set_focus().map_err(|e| e.to_string());
    }
    tauri::WebviewWindowBuilder::new(&app, "themes", tauri::WebviewUrl::App("themes.html".into()))
        .title(crate::i18n::t("YogoSync · 主题"))
        .inner_size(920.0, 760.0)
        .min_inner_size(800.0, 650.0)
        .build()
        .map_err(|e| e.to_string())?;
    Ok(())
}
#[tauri::command]
pub fn get_themes() -> Result<yogo_core::themes::ThemeLibrary, String> {
    yogo_core::themes::ThemeLibrary::load(&yogo_core::config_store::data_dir())
}
#[tauri::command]
pub async fn save_theme(
    runtime: State<'_, ServiceRuntime>,
    theme: yogo_core::themes::Theme,
) -> Result<(), String> {
    run(runtime.inner().clone(), move |r| r.save_theme(theme)).await
}
#[tauri::command]
pub async fn activate_theme(runtime: State<'_, ServiceRuntime>, id: String) -> Result<(), String> {
    run(runtime.inner().clone(), move |r| r.activate_theme(id)).await
}
#[tauri::command]
pub async fn import_theme() -> Result<Option<yogo_core::themes::Theme>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let Some(path) = rfd::FileDialog::new()
            .add_filter(crate::i18n::t("YOGO 主题"), &["json"])
            .pick_file()
        else {
            return Ok(None);
        };
        use std::io::Read;
        let mut bytes = Vec::new();
        std::fs::File::open(path)
            .map_err(|e| e.to_string())?
            .take(2_000_001)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() > 2_000_000 {
            return Err(crate::i18n::t("主题文件不能超过 2 MB").into());
        }
        let mut theme: yogo_core::themes::Theme =
            serde_json::from_slice(&bytes).map_err(|e| format!("主题文件格式无效：{e}"))?;
        theme.validate()?;
        theme.id = uuid::Uuid::new_v4().to_string();
        Ok(Some(theme))
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn export_theme(theme: yogo_core::themes::Theme) -> Result<bool, String> {
    theme.validate()?;
    tauri::async_runtime::spawn_blocking(move || {
        let name: String = theme
            .name
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
            .collect();
        let Some(path) = rfd::FileDialog::new()
            .add_filter(crate::i18n::t("YOGO 主题"), &["json"])
            .set_file_name(format!("{name}.yogo.json"))
            .save_file()
        else {
            return Ok(false);
        };
        let bytes = serde_json::to_vec_pretty(&theme).map_err(|e| e.to_string())?;
        yogo_core::events::atomic_write(&path, &bytes)?;
        Ok(true)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn close_current_window(window: tauri::WebviewWindow) -> Result<(), String> {
    window.close().map_err(|e| e.to_string())
}
#[tauri::command]
pub fn set_unsaved_changes(window: tauri::WebviewWindow, app: tauri::AppHandle, dirty: bool) {
    if !matches!(window.label(), "themes" | "settings") {
        return;
    }
    let state = app.state::<super::UnsavedWindows>();
    let mut labels = state.0.lock().unwrap();
    if dirty {
        labels.insert(window.label().into());
    } else {
        labels.remove(window.label());
    }
}
#[tauri::command]
pub async fn confirm_discard_changes() -> Result<bool, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let result = rfd::MessageDialog::new()
            .set_title(crate::i18n::t("放弃未保存的修改？"))
            .set_description(crate::i18n::t("关闭窗口后将丢失这些修改。"))
            .set_buttons(rfd::MessageButtons::OkCancelCustom(
                crate::i18n::t("放弃修改").into(),
                crate::i18n::t("继续编辑").into(),
            ))
            .show();
        result == rfd::MessageDialogResult::Ok
            || result == rfd::MessageDialogResult::Custom(crate::i18n::t("放弃修改").into())
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_preview_cover(
    runtime: State<'_, ServiceRuntime>,
    color: String,
) -> Result<(), String> {
    run(runtime.inner().clone(), move |r| r.set_preview_cover(color)).await
}

#[tauri::command]
pub async fn get_autostart(app: tauri::AppHandle) -> Result<bool, String> {
    tauri::async_runtime::spawn_blocking(move || {
        use tauri_plugin_autostart::ManagerExt;
        app.autolaunch().is_enabled().map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn set_autostart(app: tauri::AppHandle, enabled: bool) -> Result<bool, String> {
    tauri::async_runtime::spawn_blocking(move || {
        use tauri_plugin_autostart::ManagerExt;
        let manager = app.autolaunch();
        if enabled {
            manager.enable()
        } else {
            manager.disable()
        }
        .map_err(|e| e.to_string())?;
        let actual = manager.is_enabled().map_err(|e| e.to_string())?;
        if actual != enabled {
            return Err(crate::i18n::t("系统自动启动设置未生效，请重试").into());
        }
        Ok(actual)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn open_atk_driver() -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(|| {
        const URL: &str = "https://hub.atk.pro/";
        #[cfg(target_os = "macos")]
        let result = std::process::Command::new("open").arg(URL).status();
        #[cfg(windows)]
        let result = std::process::Command::new("explorer.exe").arg(URL).status();
        #[cfg(not(any(target_os = "macos", windows)))]
        let result = std::process::Command::new("xdg-open").arg(URL).status();
        match result {
            Ok(status) if status.success() => Ok(()),
            _ => Err("无法打开浏览器：https://hub.atk.pro/".into()),
        }
    })
    .await
    .map_err(|e| e.to_string())?
}
