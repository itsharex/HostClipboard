#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use log::debug;
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
            let window_main = app.get_window("main").unwrap();
            window_main.set_decorations(false).unwrap();

            // 注册全局快捷键
            let mut global_shortcut = app.global_shortcut_manager();
            let window_handle = window_main.clone();
            global_shortcut
                .register("CommandOrControl+Shift+L", move || {
                    let window = window_handle.clone();
                    tauri::async_runtime::spawn(async move {
                        if window.is_visible().unwrap() {
                            window.hide().unwrap();
                        } else {
                            window.show().unwrap();
                            window.set_focus().unwrap();
                        }
                    });
                })
                .unwrap();

            // let window_handle = window_main.clone();
            // global_shortcut
            //     .register("CommandOrControl+,", move || {
            //         let window = window_handle.clone();
            //         tauri::async_runtime::spawn(async move {
            //             window.show().unwrap();
            //             window.set_focus().unwrap();
            //             // 触发切换到配置页面的事件
            //             window.emit("toggle_config_page", ()).unwrap();
            //         });
            //     })
            //     .unwrap();

            // 添加失去焦点事件处理
            let window_handle = window_main.clone();
            window_main.on_window_event(move |event| {
                if let tauri::WindowEvent::Focused(false) = event {
                    window_handle.hide().unwrap();
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
                    // if let Some(window) = app.get_window("main") {
                    //     window.show().unwrap();
                    //     window.set_focus().unwrap();
                    //     // 触发切换到配置页面的事件
                    //     window.emit("toggle_config_page", ()).unwrap();
                    // }
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
            rs_invoke_set_user_config
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
