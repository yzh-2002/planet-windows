// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod helpers;
mod ipfs;

// Phase 2+ 的模块占位
mod models;
mod store;
mod template;
mod keystore;

use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;
use tracing_subscriber;
use tauri::Manager;

use ipfs::state::{IpfsState, IpfsStateHandle};

fn main() {
    // 初始化日志 - 简化版本，避免 with_env_filter 错误
    tracing_subscriber::fmt()
        .with_target(false)
        .init();

    info!("Planet Desktop starting...");

    tauri::Builder::default()
        // 注册所有 Tauri Commands
        .invoke_handler(tauri::generate_handler![
            // Phase 0
            commands::app::get_kubo_path,
            commands::app::hello_world,
            // Phase 1 — IPFS
            commands::ipfs::ipfs_get_state,
            commands::ipfs::ipfs_setup,
            commands::ipfs::ipfs_launch,
            commands::ipfs::ipfs_shutdown,
            commands::ipfs::ipfs_gc,
            commands::ipfs::ipfs_refresh_status,
        ])
        // 应用启动钩子
        .setup(move |app| {
            let app_handle = app.handle().clone(); 
            
            // 创建 IPFS 全局状态
            let ipfs_state: IpfsStateHandle = Arc::new(Mutex::new(IpfsState::new(app_handle.clone())));
            
            // 注入全局状态
            app.manage(ipfs_state.clone());
            
            let state = ipfs_state.clone();

            // 异步启动 IPFS daemon
            tauri::async_runtime::spawn(async move {
                ipfs::state::auto_start(state, app_handle.clone()).await;
            });

            Ok(())
        })
        // 应用退出钩子（优雅关闭 daemon）
        .on_window_event(move |window, event| {
            if let tauri::WindowEvent::Destroyed = event {
                // 注意：这里需要通过 window.app_handle() 获取状态
                // 但在窗口销毁事件中处理可能不够可靠
                info!("Window destroyed");
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(move |app_handle, event| {
            match event {
                tauri::RunEvent::ExitRequested { .. } => {
                    info!("Application exit requested, shutting down IPFS...");
                    // 获取状态并关闭 daemon
                    let state: tauri::State<IpfsStateHandle> = app_handle.state();
                    let state_clone = state.inner().clone();
                    tauri::async_runtime::block_on(async {
                        ipfs::state::graceful_shutdown(state_clone).await;
                    });
                }
                tauri::RunEvent::Exit => {
                    info!("Application exiting");
                }
                _ => {}
            }
        });
}