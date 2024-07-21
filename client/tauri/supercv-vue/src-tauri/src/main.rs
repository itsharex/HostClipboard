#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use tauri::Manager;
use tauri::{CustomMenuItem, SystemTray, SystemTrayMenu, SystemTrayEvent};
use tauri::GlobalShortcutManager;
// 这里应该引入你的 Rust 剪贴板助手库
// use clipboard_helper::ClipboardHelper;

struct ClipboardHelper;

impl ClipboardHelper {
    fn new() -> Self {
        // 初始化逻辑
        ClipboardHelper
    }

    fn get_clipboard_entries(&self, _limit: usize) -> Vec<String> {
        // 实现获取剪贴板内容的逻辑
        vec!["Item 1".to_string(), "Item 2".to_string()]
    }

    fn search_clipboard_entries(&self, query: &str, _limit: usize) -> Vec<String> {
        // 实现搜索剪贴板内容的逻辑
        vec![format!("Search result for: {}", query)]
    }
}

#[tauri::command]
fn get_clipboard_content(state: tauri::State<ClipboardHelper>) -> Vec<String> {
    state.get_clipboard_entries(1000)
}

#[tauri::command]
fn search_clipboard(query: &str, state: tauri::State<ClipboardHelper>) -> Vec<String> {
    state.search_clipboard_entries(query, 1000)
}

#[tauri::command]
async fn toggle_window(window: tauri::Window) {
    if window.is_visible().unwrap() {
        window.hide().unwrap();
    } else {
        window.show().unwrap();
        window.set_focus().unwrap();
    }
}

fn main() {
    let clipboard_helper = ClipboardHelper::new();

    let quit = CustomMenuItem::new("quit".to_string(), "Quit");
    let tray_menu = SystemTrayMenu::new().add_item(quit);
    let system_tray = SystemTray::new().with_menu(tray_menu);

    tauri::Builder::default()
        .setup(|app| {
            let window = app.get_window("main").unwrap();
            window.set_decorations(false).unwrap();
            
            // 注册全局快捷键
            let mut global_shortcut = app.global_shortcut_manager();
            let window_handle = window.clone();
            global_shortcut.register("CommandOrControl+Shift+L", move || {
                let window = window_handle.clone();
                tauri::async_runtime::spawn(async move {
                    toggle_window(window).await;
                });
            }).unwrap();

            // 添加失去焦点事件处理
            let window_handle = window.clone();
            window.on_window_event(move |event| {
                if let tauri::WindowEvent::Focused(false) = event {
                    window_handle.hide().unwrap();
                }
            });
            

            Ok(())
        })
        .system_tray(system_tray)
        .on_system_tray_event(|app, event| match event {
            SystemTrayEvent::MenuItemClick { id, .. } => {
                match id.as_str() {
                    "quit" => {
                        std::process::exit(0);
                    }
                    _ => {}
                }
            }
            _ => {}
        })
        .manage(clipboard_helper)
        .invoke_handler(tauri::generate_handler![get_clipboard_content, search_clipboard, toggle_window])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}