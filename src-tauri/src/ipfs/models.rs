use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================
// IPFS API 响应结构体
// 对应原项目 Planet/IPFS/IPFSAPIModel.swift
// ============================================================

/// IPFS 版本信息 — 对应 /api/v0/version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpfsVersion {
    #[serde(rename = "Version")]
    pub version: String,
    #[serde(rename = "Repo")]
    pub repo: String,
    #[serde(rename = "System")]
    pub system: String,
}

/// IPFS 仓库状态 — 对应 /api/v0/repo/stat
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpfsRepoState {
    #[serde(rename = "RepoSize")]
    pub repo_size: i64,
    #[serde(rename = "StorageMax")]
    pub storage_max: i64,
    #[serde(rename = "NumObjects")]
    pub num_objects: i64,
    #[serde(rename = "RepoPath")]
    pub repo_path: String,
    #[serde(rename = "Version")]
    pub version: String,
}

/// IPFS 节点 ID 信息 — 对应 /api/v0/id
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpfsId {
    #[serde(rename = "ID")]
    pub id: String,
    #[serde(rename = "PublicKey")]
    pub public_key: String,
    #[serde(rename = "Addresses")]
    pub addresses: Vec<String>,
}

/// IPFS Swarm Peers — 对应 /api/v0/swarm/peers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpfsPeers {
    #[serde(rename = "Peers")]
    pub peers: Option<Vec<IpfsPeer>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpfsPeer {
    #[serde(rename = "Addr")]
    pub addr: Option<String>,
}

/// IPFS Name Publish 结果 — 对应 /api/v0/name/publish
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpfsPublished {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Value")]
    pub value: String,
}

/// IPFS Name Resolve 结果 — 对应 /api/v0/name/resolve
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpfsResolved {
    #[serde(rename = "Path")]
    pub path: String,
}

/// IPFS 带宽统计 — 对应 /api/v0/stats/bw
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpfsBandwidth {
    #[serde(rename = "TotalIn")]
    pub total_in: i64,
    #[serde(rename = "TotalOut")]
    pub total_out: i64,
    #[serde(rename = "RateIn")]
    pub rate_in: f64,
    #[serde(rename = "RateOut")]
    pub rate_out: f64,
}

/// IPFS Pin 列表 — 对应 /api/v0/pin/ls
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpfsPinned {
    #[serde(rename = "Keys")]
    pub keys: HashMap<String, IpfsPinInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpfsPinInfo {
    #[serde(rename = "Type")]
    pub pin_type: String,
}

// ============================================================
// 应用级模型
// ============================================================

/// 服务器信息 — 对应原项目 PlanetStore+ServerInfo.swift
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub host_name: String,
    pub version: String,
    pub ipfs_peer_id: String,
    pub ipfs_version: String,
    pub ipfs_peer_count: usize,
}

// ============================================================
// IPFS 状态（推送到前端的数据结构）
// ============================================================

/// 前端可见的 IPFS 状态快照
/// 通过 app.emit("ipfs:state-changed", &state) 推送
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpfsStateSnapshot {
    pub online: bool,
    pub is_operating: bool,
    pub api_port: u16,
    pub gateway_port: u16,
    pub swarm_port: u16,
    pub repo_size: Option<i64>,
    pub server_info: Option<ServerInfo>,
    pub error_message: Option<String>,
}