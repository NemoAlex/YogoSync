#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod codex_connection;
mod commands;
mod hooks_setup;
mod i18n;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{Emitter, Manager};
use yogo_core::{config_store::data_dir, service_runtime::ServiceRuntime};
#[derive(Default)]
struct UnsavedWindows(std::sync::Mutex<std::collections::HashSet<String>>);
static QUITTING: AtomicBool = AtomicBool::new(false);
static QUIT_READY: AtomicBool = AtomicBool::new(false);
fn show_main(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}
fn quit(app: tauri::AppHandle) {
    if QUITTING.swap(true, Ordering::SeqCst) {
        return;
    }
    std::thread::spawn(move || {
        let has_changes = !app.state::<UnsavedWindows>().0.lock().unwrap().is_empty();
        if has_changes {
            let result = rfd::MessageDialog::new()
                .set_title(crate::i18n::t("退出 YogoSync？"))
                .set_description(crate::i18n::t("还有未保存的修改。退出后将丢失这些修改。"))
                .set_buttons(rfd::MessageButtons::OkCancelCustom(
                    crate::i18n::t("放弃修改并退出").into(),
                    crate::i18n::t("继续编辑").into(),
                ))
                .show();
            if result != rfd::MessageDialogResult::Ok
                && result
                    != rfd::MessageDialogResult::Custom(crate::i18n::t("放弃修改并退出").into())
            {
                QUITTING.store(false, Ordering::SeqCst);
                return;
            }
        }
        match app.state::<ServiceRuntime>().shutdown() {
            Ok(_) => {
                QUIT_READY.store(true, Ordering::SeqCst);
                app.exit(0)
            }
            Err(_) => {
                QUITTING.store(false, Ordering::SeqCst);
                show_main(&app);
            }
        }
    });
}
fn application_menu(app: &tauri::AppHandle) -> tauri::Result<tauri::menu::Menu<tauri::Wry>> {
    use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
    // Cocoa's predefined Quit directly terminates the app. Route Cmd+Q through
    // our async restore flow instead; keep a synchronous Exit fallback for OS termination.
    let quit_item = MenuItem::with_id(
        app,
        "app-quit",
        crate::i18n::t("退出 YogoSync"),
        true,
        Some("CmdOrCtrl+Q"),
    )?;
    let settings_item = MenuItem::with_id(
        app,
        "app-settings",
        crate::i18n::t("设置…"),
        true,
        Some("CmdOrCtrl+,"),
    )?;
    let application_menu = Submenu::with_items(
        app,
        "YogoSync",
        true,
        &[
            &settings_item,
            &PredefinedMenuItem::separator(app)?,
            &quit_item,
        ],
    )?;
    let close_item = MenuItem::with_id(
        app,
        "app-close",
        crate::i18n::t("关闭窗口"),
        true,
        Some("CmdOrCtrl+W"),
    )?;
    let file_menu = Submenu::with_items(app, crate::i18n::t("文件"), true, &[&close_item])?;
    let edit_menu = Submenu::with_items(
        app,
        crate::i18n::t("编辑"),
        true,
        &[
            &PredefinedMenuItem::undo(app, Some(crate::i18n::t("撤销")))?,
            &PredefinedMenuItem::redo(app, Some(crate::i18n::t("重做")))?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::cut(app, Some(crate::i18n::t("剪切")))?,
            &PredefinedMenuItem::copy(app, Some(crate::i18n::t("拷贝")))?,
            &PredefinedMenuItem::paste(app, Some(crate::i18n::t("粘贴")))?,
            &PredefinedMenuItem::select_all(app, Some(crate::i18n::t("全选")))?,
        ],
    )?;
    Ok(Menu::with_items(
        app,
        &[&application_menu, &file_menu, &edit_menu],
    )?)
}
fn tray_menu(app: &tauri::AppHandle) -> tauri::Result<tauri::menu::Menu<tauri::Wry>> {
    use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
    let show = MenuItem::with_id(
        app,
        "show",
        crate::i18n::t("打开 YogoSync"),
        true,
        None::<&str>,
    )?;
    let settings = MenuItem::with_id(app, "settings", crate::i18n::t("设置…"), true, None::<&str>)?;
    let stop = MenuItem::with_id(
        app,
        "stop",
        crate::i18n::t("停止并恢复原灯效"),
        true,
        None::<&str>,
    )?;
    let exit = MenuItem::with_id(
        app,
        "quit",
        crate::i18n::t("退出 YogoSync"),
        true,
        None::<&str>,
    )?;
    let sep = PredefinedMenuItem::separator(app)?;
    Menu::with_items(app, &[&show, &settings, &stop, &sep, &exit])
}
fn refresh_language(app: &tauri::AppHandle) -> Result<(), String> {
    app.set_menu(application_menu(app).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    if let Some(tray) = app.tray_by_id("main-tray") {
        tray.set_menu(Some(tray_menu(app).map_err(|e| e.to_string())?))
            .map_err(|e| e.to_string())?;
        tray.set_tooltip(Some(i18n::t("YogoSync · Codex 点阵伙伴")))
            .map_err(|e| e.to_string())?;
    }
    for (label, title) in [("settings", "YogoSync 设置"), ("themes", "YogoSync · 主题")] {
        if let Some(window) = app.get_webview_window(label) {
            window
                .set_title(i18n::t(title))
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn main() {
    let app = tauri::Builder::default()
        .manage(UnsavedWindows::default())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            show_main(app)
        }))
        .invoke_handler(tauri::generate_handler![
            i18n::get_locale,
            i18n::get_language_preference,
            i18n::set_language,
            commands::get_snapshot,
            commands::close_current_window,
            commands::set_unsaved_changes,
            commands::confirm_discard_changes,
            commands::open_themes,
            commands::get_themes,
            commands::save_theme,
            commands::activate_theme,
            commands::import_theme,
            commands::export_theme,
            hooks_setup::configure_codex_hooks,
            codex_connection::get_codex_connection,
            commands::start_service,
            commands::stop_service,
            commands::preview_state,
            commands::automatic,
            commands::save_preferences,
            commands::get_autostart,
            commands::set_autostart,
            commands::set_preview_cover,
            commands::clear_logs,
            commands::open_settings,
            commands::open_atk_driver,
            commands::quit_app
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            let runtime = ServiceRuntime::spawn(data_dir(), move |snapshot| {
                let _ = handle.emit("snapshot-changed", snapshot);
            })
            .map_err(std::io::Error::other)?;
            let startup_runtime = runtime.clone();
            app.manage(runtime);
            tauri::async_runtime::spawn_blocking(move || {
                let _ = startup_runtime.start();
            });
            app.manage(std::sync::Arc::new(
                codex_connection::ConnectionProbe::default(),
            ));
            app.set_menu(application_menu(app.handle())?)?;
            app.on_menu_event(|app, event| match event.id.as_ref() {
                "app-quit" => quit(app.clone()),
                "app-close" => {
                    if let Some(window) = app
                        .webview_windows()
                        .values()
                        .find(|w| w.is_focused().unwrap_or(false))
                    {
                        let _ = window.close();
                    }
                }
                "app-settings" => {
                    let _ = commands::open_settings(app.clone());
                }
                _ => {}
            });
            let menu = tray_menu(app.handle())?;
            tauri::tray::TrayIconBuilder::with_id("main-tray")
                // A monochrome Retina template lets macOS choose the menu-bar tint.
                .icon(tauri::image::Image::new_owned(
                    include_bytes!("../icons/tray.rgba").to_vec(),
                    36,
                    36,
                ))
                .icon_as_template(true)
                .tooltip(crate::i18n::t("YogoSync · Codex 点阵伙伴"))
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => show_main(app),
                    "settings" => {
                        let _ = commands::open_settings(app.clone());
                    }
                    "stop" => {
                        let runtime = app.state::<ServiceRuntime>().inner().clone();
                        std::thread::spawn(move || {
                            let _ = runtime.stop();
                        });
                    }
                    "quit" => quit(app.clone()),
                    _ => {}
                })
                .build(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if matches!(event, tauri::WindowEvent::Destroyed) {
                window
                    .app_handle()
                    .state::<UnsavedWindows>()
                    .0
                    .lock()
                    .unwrap()
                    .remove(window.label());
            }
            if window.label() == "main" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .build(tauri::generate_context!())
        .expect(crate::i18n::t("YogoSync 启动失败"));
    app.run(|app, event| match event {
        tauri::RunEvent::ExitRequested { api, .. } if !QUIT_READY.load(Ordering::SeqCst) => {
            api.prevent_exit();
            quit(app.clone());
        }
        tauri::RunEvent::Exit if !QUIT_READY.load(Ordering::SeqCst) => {
            let _ = app.state::<ServiceRuntime>().shutdown();
        }
        #[cfg(target_os = "macos")]
        tauri::RunEvent::Reopen { .. } => show_main(app),
        _ => {}
    });
}
