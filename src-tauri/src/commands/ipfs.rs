use std::sync::Arc;
use tauri::{AppHandle, Manager, State};
use tokio::sync::Mutex;
use tracing::{error, info};

use crate::ipfs::models::IpfsStateSnapshot;
use crate::ipfs::state::{IpfsState, IpfsStateHandle};

// ============================================================
// Tauri Command: 获取 IPFS 状态
// 前端调用: invoke("ipfs_get_state")
// ============================================================

#[tauri::command]
pub async fn ipfs_get_state(
    state: State<'_, IpfsStateHandle>,
) -> Result<IpfsStateSnapshot, String> {
    let s = state.lock().await;
    Ok(s.snapshot())
}

// ============================================================
// Tauri Command: 手动 Setup + Launch
// 前端调用: invoke("ipfs_setup")
// ============================================================

#[tauri::command]
pub async fn ipfs_setup(
    state: State<'_, IpfsStateHandle>,
    app: AppHandle,
) -> Result<(), String> {
    info!("ipfs_setup command called");

    // 标记操作中
    {
        let mut s = state.lock().await;
        s.is_operating = true;
        s.emit_state_changed(&app);
    }

    // Setup
    let setup_result = {
        let mut s = state.lock().await;
        s.daemon.setup().await
    };

    if let Err(e) = setup_result {
        let mut s = state.lock().await;
        s.is_operating = false;
        s.error_message = Some(format!("Setup failed: {}", e));
        s.emit_state_changed(&app);
        return Err(format!("Setup failed: {}", e));
    }

    // Launch
    let launch_result = {
        let mut s = state.lock().await;
        s.daemon.launch().await
    };

    match launch_result {
        Ok(()) => {
            let mut s = state.lock().await;
            s.online = true;
            s.is_operating = false;
            s.error_message = None;

            if let Ok(info) = s.daemon.get_server_info().await {
                s.server_info = Some(info);
            }
            if let Ok(size) = s.daemon.get_repo_size().await {
                s.repo_size = Some(size);
            }

            s.emit_state_changed(&app);
            Ok(())
        }
        Err(e) => {
            let mut s = state.lock().await;
            s.online = false;
            s.is_operating = false;
            s.error_message = Some(format!("Launch failed: {}", e));
            s.emit_state_changed(&app);
            Err(format!("Launch failed: {}", e))
        }
    }
}

// ============================================================
// Tauri Command: 手动 Launch（仅启动，不 setup）
// 前端调用: invoke("ipfs_launch")
// ============================================================

#[tauri::command]
pub async fn ipfs_launch(
    state: State<'_, IpfsStateHandle>,
    app: AppHandle,
) -> Result<(), String> {
    info!("ipfs_launch command called");

    {
        let mut s = state.lock().await;
        s.is_operating = true;
        s.emit_state_changed(&app);
    }

    let result = {
        let mut s = state.lock().await;
        s.daemon.launch().await
    };

    match result {
        Ok(()) => {
            let mut s = state.lock().await;
            s.online = true;
            s.is_operating = false;
            s.error_message = None;

            if let Ok(info) = s.daemon.get_server_info().await {
                s.server_info = Some(info);
            }
            if let Ok(size) = s.daemon.get_repo_size().await {
                s.repo_size = Some(size);
            }

            s.emit_state_changed(&app);
            Ok(())
        }
        Err(e) => {
            let mut s = state.lock().await;
            s.online = false;
            s.is_operating = false;
            s.error_message = Some(format!("Launch failed: {}", e));
            s.emit_state_changed(&app);
            Err(format!("Launch failed: {}", e))
        }
    }
}

// ============================================================
// Tauri Command: 手动 Shutdown
// 前端调用: invoke("ipfs_shutdown")
// ============================================================

#[tauri::command]
pub async fn ipfs_shutdown(
    state: State<'_, IpfsStateHandle>,
    app: AppHandle,
) -> Result<(), String> {
    info!("ipfs_shutdown command called");

    {
        let mut s = state.lock().await;
        s.is_operating = true;
        s.emit_state_changed(&app);
    }

    let result = {
        let mut s = state.lock().await;
        s.daemon.shutdown().await
    };

    match result {
        Ok(()) => {
            let mut s = state.lock().await;
            s.online = false;
            s.is_operating = false;
            s.error_message = None;
            s.server_info = None;
            s.emit_state_changed(&app);
            Ok(())
        }
        Err(e) => {
            let mut s = state.lock().await;
            s.is_operating = false;
            s.error_message = Some(format!("Shutdown failed: {}", e));
            s.emit_state_changed(&app);
            Err(format!("Shutdown failed: {}", e))
        }
    }
}

// ============================================================
// Tauri Command: 垃圾回收
// 前端调用: invoke("ipfs_gc")
// ============================================================

#[tauri::command]
pub async fn ipfs_gc(
    state: State<'_, IpfsStateHandle>,
    app: AppHandle,
) -> Result<usize, String> {
    info!("ipfs_gc command called");

    let result = {
        let s = state.lock().await;
        s.daemon.gc().await
    };

    match result {
        Ok(count) => {
            // GC 后刷新 repo size
            let mut s = state.lock().await;
            if let Ok(size) = s.daemon.get_repo_size().await {
                s.repo_size = Some(size);
            }
            s.emit_state_changed(&app);
            Ok(count)
        }
        Err(e) => {
            error!("GC failed: {}", e);
            Err(format!("GC failed: {}", e))
        }
    }
}

// ============================================================
// Tauri Command: 刷新状态（手动触发 ServerInfo 更新）
// 前端调用: invoke("ipfs_refresh_status")
// ============================================================

#[tauri::command]
pub async fn ipfs_refresh_status(
    state: State<'_, IpfsStateHandle>,
    app: AppHandle,
) -> Result<IpfsStateSnapshot, String> {
    let mut s = state.lock().await;

    // 检查是否在线
    let online = s.daemon.check_online().await;
    s.online = online;

    if online {
        if let Ok(info) = s.daemon.get_server_info().await {
            s.server_info = Some(info);
        }
        if let Ok(size) = s.daemon.get_repo_size().await {
            s.repo_size = Some(size);
        }
    }

    s.emit_state_changed(&app);
    Ok(s.snapshot())
}