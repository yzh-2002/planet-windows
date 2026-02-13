use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Manager, Emitter};
use tokio::sync::Mutex;
use tracing::{error, info};

use super::daemon::IpfsDaemon;
use super::models::{IpfsBandwidth, IpfsStateSnapshot, ServerInfo};

/// IPFS 全局状态
/// 对应原项目 IPFSState.swift
///
/// 在 Tauri 中通过 app.manage(Arc<Mutex<IpfsState>>) 注入全局状态
pub struct IpfsState {
    pub daemon: IpfsDaemon,
    pub online: bool,
    pub is_operating: bool,
    pub repo_size: Option<i64>,
    pub server_info: Option<ServerInfo>,
    pub error_message: Option<String>,
}

impl IpfsState {
    pub fn new(app: AppHandle) -> Self {
        Self {
            daemon: IpfsDaemon::new(app),
            online: false,
            is_operating: false,
            repo_size: None,
            server_info: None,
            error_message: None,
        }
    }

    /// 生成前端可用的状态快照
    pub fn snapshot(&self) -> IpfsStateSnapshot {
        IpfsStateSnapshot {
            online: self.online,
            is_operating: self.is_operating,
            api_port: self.daemon.api_port.unwrap_or(5981),
            gateway_port: self.daemon.gateway_port.unwrap_or(18181),
            swarm_port: self.daemon.swarm_port.unwrap_or(4001),
            repo_size: self.repo_size,
            server_info: self.server_info.clone(),
            error_message: self.error_message.clone(),
        }
    }

    /// 发送状态变化事件到前端
    pub fn emit_state_changed(&self, app: &AppHandle) {
        let snapshot = self.snapshot();
        if let Err(e) = app.emit("ipfs:state-changed", &snapshot) {
            error!("Failed to emit ipfs state: {}", e);
        }
    }
}

/// 类型别名，方便在 Tauri State 中使用
pub type IpfsStateHandle = Arc<Mutex<IpfsState>>;

// ============================================================
// 生命周期管理函数
// 供 main.rs 和 commands/ipfs.rs 调用
// ============================================================

/// 启动时自动 setup + launch
/// 对应 Swift: IPFSState.init() 中的 Task
pub async fn auto_start(state: IpfsStateHandle, app: AppHandle) {
    info!("Auto-starting IPFS daemon...");

    // 标记为操作中
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
        error!("IPFS setup failed: {}", e);
        let mut s = state.lock().await;
        s.is_operating = false;
        s.error_message = Some(format!("IPFS setup failed: {}", e));
        s.emit_state_changed(&app);
        return;
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

            // 获取 server info
            if let Ok(info) = s.daemon.get_server_info().await {
                s.server_info = Some(info);
            }

            // 获取 repo size
            if let Ok(size) = s.daemon.get_repo_size().await {
                s.repo_size = Some(size);
            }

            s.emit_state_changed(&app);
            info!("IPFS auto-start completed successfully");
        }
        Err(e) => {
            error!("IPFS launch failed: {}", e);
            let mut s = state.lock().await;
            s.online = false;
            s.is_operating = false;
            s.error_message = Some(format!("IPFS launch failed: {}", e));
            s.emit_state_changed(&app);
        }
    }
}

/// 应用退出时优雅关闭 daemon
/// 对应 Swift: PlanetStatusManager.terminate() 中 IPFSDaemon.shared.shutdown()
pub async fn graceful_shutdown(state: IpfsStateHandle) {
    info!("Graceful shutdown: stopping IPFS daemon...");
    let mut s = state.lock().await;
    if let Err(e) = s.daemon.shutdown().await {
        error!("Failed to shutdown IPFS daemon: {}", e);
    }
    s.online = false;
    s.is_operating = false;
    info!("IPFS daemon stopped");
}
