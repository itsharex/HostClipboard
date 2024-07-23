#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use log::{debug, error, info};
use std::sync::Arc;
use tauri::GlobalShortcutManager;
use tauri::Manager;
use tauri::{CustomMenuItem, SystemTray, SystemTrayEvent, SystemTrayMenu};

use crate::clipboard_helper::{
    rs_invoke_get_clipboards, rs_invoke_get_user_config, rs_invoke_is_initialized,
    rs_invoke_search_clipboards, rs_invoke_set_user_config, ClipboardHelper,
};

mod apis;
mod clipboard_helper;
mod db;
mod schema;
mod search_engine;
mod utils;

#[tauri::command]
fn open_settings(window: tauri::Window) {
    if let Some(settings_window) = window.get_window("settings") {
        settings_window.show().unwrap();
    }
}


async fn toggle_windows(main_window: &tauri::Window, settings_window: &tauri::Window) -> Result<(), tauri::Error> {
    let main_visible = main_window.is_visible()?;
    let settings_visible = settings_window.is_visible()?;

    if !settings_visible {
        info!("Showing settings window");
        settings_window.show()?;
        settings_window.set_focus()?;
        if main_visible {
            main_window.hide()?;
        }
    } else {
        info!("Hiding settings window");
        settings_window.hide()?;
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    let clipboard_helper = ClipboardHelper::new();
    let clipboard_helper = Arc::new(clipboard_helper);
    let clipboard_helper_for_setup = clipboard_helper.clone();

    let quit = CustomMenuItem::new("quit".to_string(), "退出");
    let show_window = CustomMenuItem::new("show_window".to_string(), "显示页面");
    let setting = CustomMenuItem::new("setting".to_string(), "设置");
    let tray_menu = SystemTrayMenu::new()
        .add_item(show_window)
        .add_item(setting)
        .add_item(quit);
    let system_tray = SystemTray::new().with_menu(tray_menu);

    tauri::Builder::default()
        .setup(move |app| {
            let clipboard_helper = clipboard_helper_for_setup.clone();
            tauri::async_runtime::spawn(async move {
                let result = time_it!(async  clipboard_helper.init(None, Some(2)).await ).await;

                if let Err(e) = result {
                    eprintln!("Failed to initialize ClipboardHelper: {}", e);
                }
            });
            // windows
            let window_main = app.get_window("main").unwrap();
            let w_main_handle = window_main.clone();
            window_main.set_decorations(false).unwrap();
            let window_settings = app.get_window("settings").unwrap();
            window_settings.hide()?;


            // 注册全局快捷键
            let mut global_shortcut = app.global_shortcut_manager();
            // let window_handle = window_main.clone();
            global_shortcut
                .register("CommandOrControl+Shift+L", move || {
                    let w_main = w_main_handle.clone();
                    tauri::async_runtime::spawn(async move {
                        if w_main.is_visible().unwrap() {
                            w_main.hide().unwrap();
                        } else {
                            w_main.show().unwrap();
                            w_main.set_focus().unwrap();
                        }
                    });
                })
                .unwrap();

            let w_main_handle = window_main.clone();
            let w_set_handle = window_settings.clone();
            global_shortcut
                .register("CommandOrControl+,", move || {
                    let w_main = w_main_handle.clone();
                    let w_set = w_set_handle.clone();
                    tauri::async_runtime::spawn(async move {
                        if let Err(e) = toggle_windows(&w_main, &w_set).await {
                            error!("Error toggling windows: {}", e);
                        }
                    });
                })
                .unwrap_or_else(|e| error!("Failed to register shortcut: {}", e));


            // 添加失去焦点事件处理
            let window_handle = window_main.clone();
            window_main.on_window_event(move |event| {
                if let tauri::WindowEvent::Focused(false) = event {
                    window_handle.hide().unwrap();
                }
            });

            let settings_handle = window_settings.clone();
            window_settings.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    // 阻止窗口关闭
                    api.prevent_close();
                    // 仅隐藏窗口
                    settings_handle.hide().unwrap();
                }
            });

            Ok(())
        })
        .system_tray(system_tray)
        .on_system_tray_event(|app, event| match event {
            SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
                "quit" => {
                    std::process::exit(0);
                }
                "show_window" => {
                    if let Some(window) = app.get_window("main") {
                        window.show().unwrap();
                        window.set_focus().unwrap();
                    }
                }
                "setting" => {
                    if let Some(settings) = app.get_window("settings") {
                        settings.show().unwrap();
                        settings.set_focus().unwrap();
                    }
                }
                _ => {}
            },
            _ => {}
        })
        .manage(clipboard_helper)
        .invoke_handler(tauri::generate_handler![
            rs_invoke_get_clipboards,
            rs_invoke_search_clipboards,
            rs_invoke_is_initialized,
            rs_invoke_get_user_config,
            rs_invoke_set_user_config,
            open_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
