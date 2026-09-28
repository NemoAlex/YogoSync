#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod codex_connection;
mod commands;
mod plugin_setup;
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
                .set_title("退出 YOGO Pet？")
                .set_description("还有未保存的修改。退出后将丢失这些修改。")
                .set_buttons(rfd::MessageButtons::OkCancelCustom(
                    "放弃修改并退出".into(),
                    "继续编辑".into(),
                ))
                .show();
            if result != rfd::MessageDialogResult::Ok
                && result != rfd::MessageDialogResult::Custom("放弃修改并退出".into())
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
fn main() {
    let app = tauri::Builder::default()
        .manage(UnsavedWindows::default())
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            show_main(app)
        }))
        .invoke_handler(tauri::generate_handler![
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
            plugin_setup::install_codex_plugin,
            plugin_setup::open_codex_plugin,
            codex_connection::get_codex_connection,
            codex_connection::open_codex_authorization,
            commands::start_service,
            commands::stop_service,
            commands::preview_state,
            commands::automatic,
            commands::save_preferences,
            commands::clear_logs,
            commands::open_settings,
            commands::export_plugin,
            commands::quit_app
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            let runtime = ServiceRuntime::spawn(data_dir(), move |snapshot| {
                let _ = handle.emit("snapshot-changed", snapshot);
            })
            .map_err(std::io::Error::other)?;
            app.manage(runtime);
            app.manage(std::sync::Arc::new(
                codex_connection::ConnectionProbe::default(),
            ));
            use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
            // Cocoa's predefined Quit directly terminates the app. Route Cmd+Q through
            // our async restore flow instead; keep a synchronous Exit fallback for OS termination.
            let quit_item =
                MenuItem::with_id(app, "app-quit", "退出 YOGO Pet", true, Some("CmdOrCtrl+Q"))?;
            let settings_item =
                MenuItem::with_id(app, "app-settings", "设置…", true, Some("CmdOrCtrl+,"))?;
            let application_menu = Submenu::with_items(
                app,
                "YOGO Pet",
                true,
                &[
                    &settings_item,
                    &PredefinedMenuItem::separator(app)?,
                    &quit_item,
                ],
            )?;
            let close_item =
                MenuItem::with_id(app, "app-close", "关闭窗口", true, Some("CmdOrCtrl+W"))?;
            let file_menu = Submenu::with_items(app, "文件", true, &[&close_item])?;
            let edit_menu = Submenu::with_items(
                app,
                "编辑",
                true,
                &[
                    &PredefinedMenuItem::undo(app, None)?,
                    &PredefinedMenuItem::redo(app, None)?,
                    &PredefinedMenuItem::separator(app)?,
                    &PredefinedMenuItem::cut(app, None)?,
                    &PredefinedMenuItem::copy(app, None)?,
                    &PredefinedMenuItem::paste(app, None)?,
                    &PredefinedMenuItem::select_all(app, None)?,
                ],
            )?;
            app.set_menu(Menu::with_items(
                app,
                &[&application_menu, &file_menu, &edit_menu],
            )?)?;
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
            let show = MenuItem::with_id(app, "show", "打开 YOGO Pet", true, None::<&str>)?;
            let settings = MenuItem::with_id(app, "settings", "设置…", true, None::<&str>)?;
            let stop = MenuItem::with_id(app, "stop", "停止并恢复原灯效", true, None::<&str>)?;
            let exit = MenuItem::with_id(app, "quit", "退出 YOGO Pet", true, None::<&str>)?;
            let sep = PredefinedMenuItem::separator(app)?;
            let menu = Menu::with_items(app, &[&show, &settings, &stop, &sep, &exit])?;
            tauri::tray::TrayIconBuilder::new()
                .title("Y·P")
                .tooltip("YOGO Pet · Codex 点阵伙伴")
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
        .expect("YOGO Pet 启动失败");
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
