// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod helpers;
mod ipfs;
mod keystore;
mod models;
mod store;
mod template;

use tauri::{Manager, tray::TrayIconBuilder};

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            // 创建系统托盘图标
            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .on_tray_icon_event(|tray, event| {
                    // 处理托盘事件
                    match event {
                        tauri::tray::TrayIconEvent::Click {
                            button: tauri::tray::MouseButton::Left,
                            button_state: tauri::tray::MouseButtonState::Up,
                            ..
                        } => {
                            // 点击托盘图标显示/隐藏窗口
                            if let Some(window) = tray.app_handle().get_webview_window("main") {
                                if window.is_visible().unwrap() {
                                    let _ = window.hide();
                                } else {
                                    let _ = window.show();
                                    let _ = window.set_focus();
                                }
                            }
                        }
                        _ => {}
                    }
                })
                .build(app)?;
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app::get_kubo_path,
            commands::app::hello_world,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}