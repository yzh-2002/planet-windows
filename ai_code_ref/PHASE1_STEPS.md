# Phase 1：Kubo 管理核心 — 详细开发文档

> **目标**：实现 IPFS 守护进程的完整生命周期管理，对应原项目 `IPFSCommand.swift` + `IPFSDaemon.swift` + `IPFSState.swift`
>
> **预计工期**：2 周
>
> **验收标准**：应用启动后自动初始化并启动 IPFS daemon，前端 IPFS 状态面板显示 Online，端口号、peer 数量均正常。手动点击 Shutdown 后状态变为 Offline，再点击 Launch 可恢复。应用关闭时 daemon 自动停止。

---

## 目录

- [1. 整体架构](#1-整体架构)
- [2. 新增依赖](#2-新增依赖)
- [3. Step 1：实现 IPFS API 响应模型 (`ipfs/models.rs`)](#3-step-1实现-ipfs-api-响应模型-ipfsmodelsrs)
- [4. Step 2：实现 Kubo 命令封装 (`ipfs/command.rs`)](#4-step-2实现-kubo-命令封装-ipfscommandrs)
- [5. Step 3：实现端口扫描工具 (`helpers/net.rs`)](#5-step-3实现端口扫描工具-helpersnetrs)
- [6. Step 4：实现 IPFS Daemon 管理器 (`ipfs/daemon.rs`)](#6-step-4实现-ipfs-daemon-管理器-ipfsdaemonrs)
- [7. Step 5：实现 IPFS 状态管理 (`ipfs/state.rs`)](#7-step-5实现-ipfs-状态管理-ipfsstaaters)
- [8. Step 6：注册 Tauri Commands (`commands/ipfs.rs`)](#8-step-6注册-tauri-commands-commandsipfsrs)
- [9. Step 7：集成到 `main.rs` — 生命周期钩子](#9-step-7集成到-mainrs--生命周期钩子)
- [10. Step 8：前端实现 IPFS 状态面板](#10-step-8前端实现-ipfs-状态面板)
- [11. Step 9：测试与调试](#11-step-9测试与调试)
- [12. 文件清单](#12-文件清单)
- [13. Swift → Rust 对照表](#13-swift--rust-对照表)

---

## 1. 整体架构

```
┌─────────────────────────────────────────────────────┐
│                    前端 (React)                      │
│                                                     │
│  useIPFS hook ←── listen("ipfs:state-changed")      │
│  invoke("ipfs_setup") / invoke("ipfs_shutdown")     │
│  invoke("ipfs_get_state") / invoke("ipfs_gc")       │
│  IPFSStatusPanel 组件                               │
└──────────────────────┬──────────────────────────────┘
                       │ IPC (Tauri invoke / events)
┌──────────────────────▼──────────────────────────────┐
│                Rust 后端 (Tauri)                     │
│                                                     │
│  commands/ipfs.rs   ← Tauri Command 入口            │
│       │                                             │
│       ▼                                             │
│  ipfs/state.rs      ← 全局状态 (Mutex<IpfsState>)   │
│       │                                             │
│       ▼                                             │
│  ipfs/daemon.rs     ← Daemon 生命周期管理            │
│       │                                             │
│       ▼                                             │
│  ipfs/command.rs    ← Kubo CLI 命令封装              │
│       │                                             │
│       ▼                                             │
│  helpers/paths.rs   ← Kubo 二进制路径 (Phase 0)      │
│  helpers/net.rs     ← 端口扫描工具                   │
│  ipfs/models.rs     ← API 响应结构体                 │
└──────────────────────┬──────────────────────────────┘
                       │ std::process::Command / reqwest HTTP
┌──────────────────────▼──────────────────────────────┐
│              Kubo 守护进程 (ipfs daemon)              │
│                                                     │
│  API: http://127.0.0.1:{api_port}/api/v0/...        │
│  Gateway: http://127.0.0.1:{gateway_port}/...       │
│  Swarm: /ip4/0.0.0.0/tcp/{swarm_port}              │
└─────────────────────────────────────────────────────┘
```

---

## 2. 新增依赖

编辑 `src-tauri/Cargo.toml`，在 `[dependencies]` 中添加：

```toml
[dependencies]
tauri = { version = "1", features = ["shell-open"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["json"] }
anyhow = "1"
thiserror = "1"
tracing = "0.1"
tracing-subscriber = "0.3"
```

> **说明**：
> - `tokio`：Rust 异步运行时，用于 async/await
> - `reqwest`：HTTP 客户端，用于调用 IPFS API
> - `anyhow` / `thiserror`：错误处理
> - `tracing`：日志

---

## 3. Step 1：实现 IPFS API 响应模型 (`ipfs/models.rs`)

### 3.1 对应关系

| Swift 结构体 (`IPFSAPIModel.swift`) | Rust 结构体 |
|------|------|
| `IPFSVersion` | `IpfsVersion` |
| `IPFSRepoState` | `IpfsRepoState` |
| `IPFSID` | `IpfsId` |
| `IPFSPeers` / `IPFSPeer` | `IpfsPeers` / `IpfsPeer` |
| `IPFSPublished` | `IpfsPublished` |
| `IPFSResolved` | `IpfsResolved` |
| `IPFSBandwidth` | `IpfsBandwidth` |
| `IPFSPinned` / `IPFSPinInfo` | `IpfsPinned` / `IpfsPinInfo` |
| `ServerInfo` | `ServerInfo` |

### 3.2 完整代码

创建 `src-tauri/src/ipfs/models.rs`：

```rust
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
```

### 3.3 验证

```bash
cd src-tauri
cargo check
```

---

## 4. Step 2：实现 Kubo 命令封装 (`ipfs/command.rs`)

### 4.1 对应关系

对应原项目 `Planet/IPFS/IPFSCommand.swift`。

| Swift | Rust |
|------|------|
| `IPFSCommand.IPFSExecutablePath` | `KuboCommand::executable_path()` |
| `IPFSCommand.IPFSRepositoryPath` | `KuboCommand::repo_path()` |
| `IPFSCommand.run()` (同步) | `KuboCommand::run()` |
| `IPFSCommand.run(outHandler:errHandler:)` (异步回调) | `KuboCommand::run_streaming()` |
| `IPFSCommand.IPFSInit()` | `KuboCommand::ipfs_init()` |
| `IPFSCommand.launchDaemon()` | `KuboCommand::launch_daemon()` |
| `IPFSCommand.shutdownDaemon()` | `KuboCommand::shutdown_daemon()` |
| 等等所有静态工厂方法 | 对应 `KuboCommand` 的关联函数 |

### 4.2 完整代码

创建 `src-tauri/src/ipfs/command.rs`：

```rust
use anyhow::{anyhow, Result};
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as TokioCommand;
use tokio::sync::mpsc;
use tracing::{debug, error, info};

use crate::helpers::paths;

/// Kubo CLI 命令封装
/// 对应原项目 IPFSCommand.swift
pub struct KuboCommand {
    args: Vec<String>,
}

impl KuboCommand {
    // ============================================================
    // 路径
    // ============================================================

    /// Kubo 可执行文件路径
    pub fn executable_path() -> PathBuf {
        paths::get_kubo_path()
    }

    /// IPFS 仓库路径 (~/.planet/ipfs/)
    pub fn repo_path() -> PathBuf {
        let repo = paths::get_ipfs_repo_path();
        // 确保目录存在
        std::fs::create_dir_all(&repo).ok();
        repo
    }

    // ============================================================
    // 执行方法
    // ============================================================

    /// 同步执行 Kubo 命令，等待完成并返回结果
    /// 对应 Swift: IPFSCommand.run() -> (ret, out, err)
    pub fn run(&self) -> Result<CmdOutput> {
        let exe = Self::executable_path();
        let repo = Self::repo_path();

        debug!("Running kubo: {:?} {:?}", exe, self.args);

        let output: Output = Command::new(&exe)
            .args(&self.args)
            .env("IPFS_PATH", repo.to_str().unwrap_or(""))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| anyhow!("Failed to execute kubo command: {}", e))?;

        let ret = output.status.code().unwrap_or(-1);
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if ret != 0 {
            debug!(
                "Kubo command returned {}\n[stdout] {}\n[stderr] {}",
                ret, stdout, stderr
            );
        }

        Ok(CmdOutput {
            ret,
            stdout,
            stderr,
        })
    }

    /// 异步流式执行 Kubo 命令（用于 daemon）
    /// 对应 Swift: IPFSCommand.run(outHandler:errHandler:completionHandler:)
    ///
    /// 返回一个子进程 handle 和 stdout 行接收器
    pub async fn run_streaming(
        &self,
    ) -> Result<(tokio::process::Child, mpsc::Receiver<StreamLine>)> {
        let exe = Self::executable_path();
        let repo = Self::repo_path();

        info!("Launching kubo streaming: {:?} {:?}", exe, self.args);

        let mut child = TokioCommand::new(&exe)
            .args(&self.args)
            .env("IPFS_PATH", repo.to_str().unwrap_or(""))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true) // 当 Child 被 drop 时自动杀掉进程
            .spawn()
            .map_err(|e| anyhow!("Failed to spawn kubo daemon: {}", e))?;

        let (tx, rx) = mpsc::channel::<StreamLine>(100);

        // 读取 stdout
        if let Some(stdout) = child.stdout.take() {
            let tx_out = tx.clone();
            tokio::spawn(async move {
                let reader = BufReader::new(stdout);
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    let _ = tx_out.send(StreamLine::Stdout(line)).await;
                }
            });
        }

        // 读取 stderr
        if let Some(stderr) = child.stderr.take() {
            let tx_err = tx.clone();
            tokio::spawn(async move {
                let reader = BufReader::new(stderr);
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    let _ = tx_err.send(StreamLine::Stderr(line)).await;
                }
            });
        }

        Ok((child, rx))
    }

    // ============================================================
    // 工厂方法（对应 Swift 的所有 static func）
    // ============================================================

    /// ipfs init
    pub fn ipfs_init() -> Self {
        Self {
            args: vec!["init".into()],
        }
    }

    /// ipfs version
    pub fn ipfs_version() -> Self {
        Self {
            args: vec!["version".into()],
        }
    }

    /// ipfs config Addresses.API /ip4/127.0.0.1/tcp/{port}
    pub fn update_api_port(port: u16) -> Self {
        Self {
            args: vec![
                "config".into(),
                "Addresses.API".into(),
                format!("/ip4/127.0.0.1/tcp/{}", port),
            ],
        }
    }

    /// ipfs config Addresses.Gateway /ip4/127.0.0.1/tcp/{port}
    pub fn update_gateway_port(port: u16) -> Self {
        Self {
            args: vec![
                "config".into(),
                "Addresses.Gateway".into(),
                format!("/ip4/127.0.0.1/tcp/{}", port),
            ],
        }
    }

    /// ipfs config Addresses.Swarm [多地址] --json
    pub fn update_swarm_port(port: u16) -> Self {
        let swarm_json = format!(
            r#"["/ip4/0.0.0.0/tcp/{port}", "/ip6/::/tcp/{port}", "/ip4/0.0.0.0/udp/{port}/quic", "/ip6/::/udp/{port}/quic"]"#,
        );
        Self {
            args: vec![
                "config".into(),
                "Addresses.Swarm".into(),
                swarm_json,
                "--json".into(),
            ],
        }
    }

    /// ipfs config Peering.Peers {json} --json
    pub fn set_peers(peers_json: &str) -> Self {
        Self {
            args: vec![
                "config".into(),
                "Peering.Peers".into(),
                peers_json.into(),
                "--json".into(),
            ],
        }
    }

    /// ipfs config DNS.Resolvers {json} --json
    pub fn set_resolvers(resolvers_json: &str) -> Self {
        Self {
            args: vec![
                "config".into(),
                "DNS.Resolvers".into(),
                resolvers_json.into(),
                "--json".into(),
            ],
        }
    }

    /// ipfs config Swarm.ConnMgr {json} --json
    pub fn set_swarm_conn_mgr(json: &str) -> Self {
        Self {
            args: vec![
                "config".into(),
                "Swarm.ConnMgr".into(),
                json.into(),
                "--json".into(),
            ],
        }
    }

    /// ipfs config API.HTTPHeaders.Access-Control-Allow-Origin {json} --json
    pub fn set_access_control_allow_origin(json: &str) -> Self {
        Self {
            args: vec![
                "config".into(),
                "API.HTTPHeaders.Access-Control-Allow-Origin".into(),
                json.into(),
                "--json".into(),
            ],
        }
    }

    /// ipfs config API.HTTPHeaders.Access-Control-Allow-Methods {json} --json
    pub fn set_access_control_allow_methods(json: &str) -> Self {
        Self {
            args: vec![
                "config".into(),
                "API.HTTPHeaders.Access-Control-Allow-Methods".into(),
                json.into(),
                "--json".into(),
            ],
        }
    }

    /// ipfs daemon --migrate --enable-namesys-pubsub --enable-pubsub-experiment
    pub fn launch_daemon() -> Self {
        Self {
            args: vec![
                "daemon".into(),
                "--migrate".into(),
                "--enable-namesys-pubsub".into(),
                "--enable-pubsub-experiment".into(),
            ],
        }
    }

    /// ipfs shutdown
    pub fn shutdown_daemon() -> Self {
        Self {
            args: vec!["shutdown".into()],
        }
    }

    /// ipfs add -r -H {directory} --cid-version=1 --quieter
    pub fn add_directory(directory: &str) -> Self {
        Self {
            args: vec![
                "add".into(),
                "-r".into(),
                "-H".into(),
                directory.into(),
                "--cid-version=1".into(),
                "--quieter".into(),
            ],
        }
    }

    /// ipfs add {file} --quieter --cid-version=1 --only-hash
    pub fn get_file_cid(file: &str) -> Self {
        Self {
            args: vec![
                "add".into(),
                file.into(),
                "--quieter".into(),
                "--cid-version=1".into(),
                "--only-hash".into(),
            ],
        }
    }

    /// ipfs add {file} --quieter --cid-version=0 --pin
    pub fn get_file_cid_v0(file: &str) -> Self {
        Self {
            args: vec![
                "add".into(),
                file.into(),
                "--quieter".into(),
                "--cid-version=0".into(),
                "--pin".into(),
            ],
        }
    }

    /// ipfs key gen {name}
    pub fn generate_key(name: &str) -> Self {
        Self {
            args: vec!["key".into(), "gen".into(), name.into()],
        }
    }

    /// ipfs key rm {name}
    pub fn delete_key(name: &str) -> Self {
        Self {
            args: vec!["key".into(), "rm".into(), name.into()],
        }
    }

    /// ipfs key list
    pub fn list_keys() -> Self {
        Self {
            args: vec!["key".into(), "list".into()],
        }
    }

    /// ipfs key export {name} -o {target} [--format={format}]
    pub fn export_key(name: &str, target: &str, format: Option<&str>) -> Self {
        let mut args = vec![
            "key".into(),
            "export".into(),
            name.into(),
            "-o".into(),
            target.into(),
        ];
        if let Some(fmt) = format {
            args.push(format!("--format={}", fmt));
        }
        Self { args }
    }

    /// ipfs key import {name} {target} [--format={format}]
    pub fn import_key(name: &str, target: &str, format: Option<&str>) -> Self {
        let mut args = vec![
            "key".into(),
            "import".into(),
            name.into(),
            target.into(),
        ];
        if let Some(fmt) = format {
            args.push(format!("--format={}", fmt));
        }
        Self { args }
    }
}

// ============================================================
// 辅助类型
// ============================================================

/// 同步命令输出
pub struct CmdOutput {
    pub ret: i32,
    pub stdout: String,
    pub stderr: String,
}

/// 流式命令输出行
#[derive(Debug)]
pub enum StreamLine {
    Stdout(String),
    Stderr(String),
}
```

### 4.3 验证

```bash
cargo check
```

---

## 5. Step 3：实现端口扫描工具 (`helpers/net.rs`)

### 5.1 对应关系

对应 `IPFSDaemon.swift` 中的 `isPortOpen()` 和 `scoutPort()` 方法。

### 5.2 完整代码

创建 `src-tauri/src/helpers/net.rs`：

```rust
use std::net::TcpListener;
use tracing::debug;

/// 检查指定端口是否可用（尝试绑定）
/// 对应 Swift: IPFSDaemon.isPortOpen(port:)
pub fn is_port_available(port: u16) -> bool {
    match TcpListener::bind(("127.0.0.1", port)) {
        Ok(_listener) => {
            // listener 在这里被 drop，端口自动释放
            true
        }
        Err(_) => false,
    }
}

/// 在指定范围内扫描一个可用端口
/// 对应 Swift: IPFSDaemon.scoutPort(_:)
pub fn scout_port(range: std::ops::RangeInclusive<u16>) -> Option<u16> {
    for port in range {
        if is_port_available(port) {
            debug!("Found available port: {}", port);
            return Some(port);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_port_available() {
        // 高端口号通常是可用的
        assert!(is_port_available(59999));
    }

    #[test]
    fn test_scout_port() {
        let port = scout_port(59990..=59999);
        assert!(port.is_some());
    }
}
```

### 5.3 更新 `helpers/mod.rs`

```rust
pub mod paths;
pub mod net;  // 新增
```

### 5.4 验证

```bash
cargo check
cargo test helpers::net
```

---

## 6. Step 4：实现 IPFS Daemon 管理器 (`ipfs/daemon.rs`)

### 6.1 对应关系

对应原项目 `IPFSDaemon.swift`（actor），这是最核心、最复杂的模块。

| Swift 方法 | Rust 方法 |
|------|------|
| `setupIPFS(andLaunch:)` | `IpfsDaemon::setup()` |
| `launch()` | `IpfsDaemon::launch()` |
| `shutdown()` | `IpfsDaemon::shutdown()` |
| `api(path:args:timeout:)` | `IpfsDaemon::api()` |
| `generateKey(name:)` | `IpfsDaemon::generate_key()` |
| `removeKey(name:)` | `IpfsDaemon::remove_key()` |
| `checkKeyExists(name:)` | `IpfsDaemon::check_key_exists()` |
| `listKeys()` | `IpfsDaemon::list_keys()` |
| `addDirectory(url:)` | `IpfsDaemon::add_directory()` |
| `getFileCID(url:)` | `IpfsDaemon::get_file_cid()` |
| `getStatsBW()` | `IpfsDaemon::get_stats_bw()` |
| `resolveIPNSorDNSLink(name:)` | `IpfsDaemon::resolve_ipns()` |
| `pin(cid:)` / `unpin(cid:)` | `IpfsDaemon::pin()` / `IpfsDaemon::unpin()` |
| `IPFSDaemon.peers` (静态 JSON) | `IpfsDaemon::peers_json()` |
| `IPFSDaemon.resolvers` (静态 JSON) | `IpfsDaemon::resolvers_json()` |

### 6.2 完整代码

创建 `src-tauri/src/ipfs/daemon.rs`：

```rust
use anyhow::{anyhow, Result};
use reqwest::Client;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

use super::command::{KuboCommand, StreamLine};
use super::models::*;
use crate::helpers::net;

/// IPFS Daemon 管理器
/// 对应原项目 IPFSDaemon.swift (actor)
pub struct IpfsDaemon {
    setting_up: bool,
    pub swarm_port: Option<u16>,
    pub api_port: Option<u16>,
    pub gateway_port: Option<u16>,
    http_client: Client,
    /// daemon 子进程 handle（用于 shutdown 时 kill）
    daemon_child: Option<tokio::process::Child>,
}

impl IpfsDaemon {
    pub fn new() -> Self {
        info!("IpfsDaemon::new()");
        Self {
            setting_up: false,
            swarm_port: None,
            api_port: None,
            gateway_port: None,
            http_client: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("Failed to create HTTP client"),
            daemon_child: None,
        }
    }

    // ============================================================
    // Setup — 对应 Swift: setupIPFS(andLaunch:)
    // ============================================================

    /// 初始化 IPFS 仓库并配置各项参数
    pub async fn setup(&mut self) -> Result<()> {
        if self.setting_up {
            info!("IPFS is already being set up, skipping...");
            return Ok(());
        }
        self.setting_up = true;

        let result = self.do_setup().await;

        self.setting_up = false;
        result
    }

    async fn do_setup(&mut self) -> Result<()> {
        info!("Setting up IPFS...");

        // 1. 检查仓库是否为空，如为空则 init
        let repo_path = KuboCommand::repo_path();
        let is_empty = if repo_path.exists() {
            std::fs::read_dir(&repo_path)
                .map(|mut dir| dir.next().is_none())
                .unwrap_or(true)
        } else {
            true
        };

        if is_empty {
            info!("Initializing IPFS repo...");
            let output = KuboCommand::ipfs_init().run()?;
            if output.ret != 0 {
                error!("Failed to init IPFS: {}", output.stderr);
                return Err(anyhow!("Failed to init IPFS repo"));
            }
            info!("IPFS repo initialized");
        }

        // 2. 扫描并配置 Swarm 端口 (4001-4011)
        info!("Scanning swarm port...");
        let swarm_port = net::scout_port(4001..=4011)
            .ok_or_else(|| anyhow!("Unable to find open swarm port"))?;
        let output = KuboCommand::update_swarm_port(swarm_port).run()?;
        if output.ret != 0 {
            return Err(anyhow!("Failed to update swarm port: {}", output.stderr));
        }
        self.swarm_port = Some(swarm_port);
        info!("Swarm port: {}", swarm_port);

        // 3. 扫描并配置 API 端口 (5981-5991)
        info!("Scanning API port...");
        let api_port = net::scout_port(5981..=5991)
            .ok_or_else(|| anyhow!("Unable to find open API port"))?;
        let output = KuboCommand::update_api_port(api_port).run()?;
        if output.ret != 0 {
            return Err(anyhow!("Failed to update API port: {}", output.stderr));
        }
        self.api_port = Some(api_port);
        info!("API port: {}", api_port);

        // 4. 扫描并配置 Gateway 端口 (18181-18191)
        info!("Scanning gateway port...");
        let gateway_port = net::scout_port(18181..=18191)
            .ok_or_else(|| anyhow!("Unable to find open gateway port"))?;
        let output = KuboCommand::update_gateway_port(gateway_port).run()?;
        if output.ret != 0 {
            return Err(anyhow!(
                "Failed to update gateway port: {}",
                output.stderr
            ));
        }
        self.gateway_port = Some(gateway_port);
        info!("Gateway port: {}", gateway_port);

        // 5. 配置 Peering Peers
        info!("Setting peering peers...");
        let peers_json = Self::peers_json();
        let output = KuboCommand::set_peers(&peers_json).run()?;
        if output.ret != 0 {
            warn!("Failed to set peers: {}", output.stderr);
        }

        // 6. 配置 DNS Resolvers
        info!("Setting DNS resolvers...");
        let resolvers_json = Self::resolvers_json();
        let output = KuboCommand::set_resolvers(&resolvers_json).run()?;
        if output.ret != 0 {
            warn!("Failed to set DNS resolvers: {}", output.stderr);
        }

        // 7. 配置 Swarm Connection Manager (low: 10-20)
        info!("Setting Swarm Connection Manager...");
        let conn_mgr_json = serde_json::json!({
            "GracePeriod": "20s",
            "HighWater": 20,
            "LowWater": 10,
            "Type": "basic"
        })
        .to_string();
        let output = KuboCommand::set_swarm_conn_mgr(&conn_mgr_json).run()?;
        if output.ret != 0 {
            return Err(anyhow!(
                "Failed to set SwarmConnMgr: {}",
                output.stderr
            ));
        }

        // 8. 配置 CORS
        info!("Setting Access-Control headers...");
        let allow_origin = serde_json::json!(["https://webui.ipfs.io"]).to_string();
        let _ = KuboCommand::set_access_control_allow_origin(&allow_origin).run();
        let allow_methods = serde_json::json!(["PUT", "POST"]).to_string();
        let _ = KuboCommand::set_access_control_allow_methods(&allow_methods).run();

        info!("IPFS setup completed!");
        Ok(())
    }

    // ============================================================
    // Launch — 对应 Swift: launch()
    // ============================================================

    /// 启动 IPFS daemon 子进程
    /// 监听 stdout 中的 "Daemon is ready" 来确认启动成功
    pub async fn launch(&mut self) -> Result<()> {
        info!("Launching IPFS daemon...");

        // 端口必须已配置
        if self.api_port.is_none() || self.gateway_port.is_none() || self.swarm_port.is_none() {
            return Err(anyhow!(
                "IPFS ports not configured. Run setup() first."
            ));
        }

        // 先尝试关闭可能已经运行的 daemon
        let _ = KuboCommand::shutdown_daemon().run();

        // 流式启动 daemon
        let (child, mut rx) = KuboCommand::launch_daemon().run_streaming().await?;
        self.daemon_child = Some(child);

        // 读取 stdout/stderr，等待 "Daemon is ready"
        let api_port = self.api_port.unwrap();
        let ready = Arc::new(Mutex::new(false));
        let ready_clone = ready.clone();

        // 在后台任务中持续读取输出
        tokio::spawn(async move {
            while let Some(line) = rx.recv().await {
                match &line {
                    StreamLine::Stdout(text) => {
                        debug!("[IPFS stdout] {}", text);
                        if text.contains("Daemon is ready") {
                            info!("IPFS Daemon is ready! API port: {}", api_port);
                            let mut r = ready_clone.lock().await;
                            *r = true;
                        }
                    }
                    StreamLine::Stderr(text) => {
                        debug!("[IPFS stderr] {}", text);
                    }
                }
            }
            info!("IPFS daemon process output stream ended");
        });

        // 等待 daemon 就绪（最多 30 秒）
        for _ in 0..60 {
            tokio::time::sleep(Duration::from_millis(500)).await;
            let r = ready.lock().await;
            if *r {
                info!("IPFS daemon launched successfully");
                return Ok(());
            }
        }

        warn!("IPFS daemon did not report ready within 30 seconds");
        // 即使没检测到 "Daemon is ready"，也检查一下 API 是否可用
        if self.check_online().await {
            info!("IPFS daemon appears to be online despite no ready message");
            return Ok(());
        }

        Err(anyhow!("IPFS daemon failed to start within timeout"))
    }

    // ============================================================
    // Shutdown — 对应 Swift: shutdown()
    // ============================================================

    /// 优雅关闭 IPFS daemon
    pub async fn shutdown(&mut self) -> Result<()> {
        info!("Shutting down IPFS daemon...");

        // 方法1: 通过 CLI 发送 shutdown 命令
        let output = KuboCommand::shutdown_daemon().run();
        match output {
            Ok(o) if o.ret == 0 => {
                info!("Daemon shutdown via CLI successful");
            }
            _ => {
                warn!("CLI shutdown failed, trying to kill process...");
                // 方法2: 直接 kill 子进程
                if let Some(mut child) = self.daemon_child.take() {
                    let _ = child.kill().await;
                    info!("Daemon process killed");
                }
            }
        }

        self.daemon_child = None;
        info!("IPFS daemon shut down");
        Ok(())
    }

    // ============================================================
    // HTTP API 调用 — 对应 Swift: api(path:args:timeout:)
    // ============================================================

    /// 通过 HTTP POST 调用 IPFS API
    pub async fn api(
        &self,
        path: &str,
        args: Option<&HashMap<String, String>>,
        timeout_secs: Option<u64>,
    ) -> Result<Vec<u8>> {
        let api_port = self
            .api_port
            .ok_or_else(|| anyhow!("IPFS API port not set"))?;

        let mut url = format!("http://127.0.0.1:{}/api/v0/{}", api_port, path);

        // 添加查询参数
        if let Some(params) = args {
            let query: String = params
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join("&");
            url = format!("{}?{}", url, query);
        }

        let client = if let Some(t) = timeout_secs {
            Client::builder()
                .timeout(Duration::from_secs(t))
                .build()?
        } else {
            self.http_client.clone()
        };

        let response = client.post(&url).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            error!("IPFS API error: {} {} - {}", path, status, body);
            return Err(anyhow!("IPFS API error: {} {}", path, status));
        }

        let data = response.bytes().await?.to_vec();
        Ok(data)
    }

    /// 便捷方法：调用 API 并反序列化为 JSON
    pub async fn api_json<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        args: Option<&HashMap<String, String>>,
    ) -> Result<T> {
        let data = self.api(path, args, None).await?;
        let result = serde_json::from_slice(&data)?;
        Ok(result)
    }

    // ============================================================
    // 状态查询
    // ============================================================

    /// 检查 daemon 是否在线
    pub async fn check_online(&self) -> bool {
        let Some(api_port) = self.api_port else {
            return false;
        };
        let url = format!("http://127.0.0.1:{}/api/v0/id", api_port);
        let client = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();
        match client.post(&url).send().await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }

    /// 获取带宽统计
    /// 对应 Swift: getStatsBW()
    pub async fn get_stats_bw(&self) -> Result<IpfsBandwidth> {
        self.api_json("stats/bw", None).await
    }

    /// 获取 Server Info（聚合 id + version + swarm/peers）
    /// 对应 Swift: IPFSState.updateServerInfo()
    pub async fn get_server_info(&self) -> Result<ServerInfo> {
        // 获取 Peer ID
        let id_info: IpfsId = self.api_json("id", None).await.unwrap_or(IpfsId {
            id: String::new(),
            public_key: String::new(),
            addresses: vec![],
        });

        // 获取 IPFS 版本
        let version_info: IpfsVersion =
            self.api_json("version", None).await.unwrap_or(IpfsVersion {
                version: String::new(),
                repo: String::new(),
                system: String::new(),
            });

        // 获取 peers 数量
        let peers_info: IpfsPeers = self.api_json("swarm/peers", None).await.unwrap_or(IpfsPeers {
            peers: None,
        });
        let peer_count = peers_info.peers.as_ref().map_or(0, |p| p.len());

        // 获取主机名
        let host_name = hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_default();

        Ok(ServerInfo {
            host_name,
            version: env!("CARGO_PKG_VERSION").to_string(),
            ipfs_peer_id: id_info.id,
            ipfs_version: version_info.version,
            ipfs_peer_count: peer_count,
        })
    }

    /// 获取仓库大小
    pub async fn get_repo_size(&self) -> Result<i64> {
        let repo_state: IpfsRepoState = self.api_json("repo/stat", None).await?;
        Ok(repo_state.repo_size)
    }

    /// 获取本地 Gateway URL
    pub fn get_gateway(&self) -> Option<String> {
        self.gateway_port
            .map(|p| format!("http://127.0.0.1:{}", p))
    }

    // ============================================================
    // IPNS / Resolve
    // ============================================================

    /// 解析 IPNS 或 DNSLink
    /// 对应 Swift: resolveIPNSorDNSLink(name:)
    pub async fn resolve_ipns(&self, name: &str) -> Result<String> {
        let mut args = HashMap::new();
        args.insert("arg".into(), name.into());
        let resolved: IpfsResolved = self.api_json("name/resolve", Some(&args)).await?;
        if resolved.path.starts_with("/ipfs/") {
            Ok(resolved.path["/ipfs/".len()..].to_string())
        } else {
            Err(anyhow!("Unexpected resolve result: {}", resolved.path))
        }
    }

    // ============================================================
    // Pin
    // ============================================================

    pub async fn pin(&self, cid: &str) -> Result<()> {
        let mut args = HashMap::new();
        args.insert("arg".into(), cid.into());
        self.api("pin/add", Some(&args), Some(120)).await?;
        Ok(())
    }

    pub async fn unpin(&self, cid: &str) -> Result<()> {
        let mut args = HashMap::new();
        args.insert("arg".into(), cid.into());
        self.api("pin/rm", Some(&args), Some(120)).await?;
        Ok(())
    }

    pub async fn gc(&self) -> Result<usize> {
        let data = self.api("repo/gc", None, Some(120)).await?;
        let text = String::from_utf8_lossy(&data);
        let count = text.lines().filter(|l| l.contains("Key")).count();
        info!("GC removed {} objects", count);
        Ok(count)
    }

    // ============================================================
    // Key 管理
    // ============================================================

    /// 生成 IPFS 密钥对
    pub fn generate_key(&self, name: &str) -> Result<String> {
        let output = KuboCommand::generate_key(name).run()?;
        if output.ret == 0 {
            Ok(output.stdout.trim().to_string())
        } else {
            Err(anyhow!(
                "Failed to generate key: {}",
                output.stderr
            ))
        }
    }

    /// 删除 IPFS 密钥
    pub fn remove_key(&self, name: &str) -> Result<()> {
        let output = KuboCommand::delete_key(name).run()?;
        if output.ret == 0 {
            Ok(())
        } else {
            Err(anyhow!("Failed to delete key: {}", output.stderr))
        }
    }

    /// 列出所有密钥（排除 "self"）
    pub fn list_keys(&self) -> Result<Vec<String>> {
        let output = KuboCommand::list_keys().run()?;
        if output.ret == 0 {
            let keys: Vec<String> = output
                .stdout
                .trim()
                .lines()
                .filter(|l| !l.is_empty() && *l != "self")
                .map(|l| l.to_string())
                .collect();
            Ok(keys)
        } else {
            Err(anyhow!("Failed to list keys: {}", output.stderr))
        }
    }

    /// 检查密钥是否存在
    pub fn check_key_exists(&self, name: &str) -> Result<bool> {
        let output = KuboCommand::list_keys().run()?;
        if output.ret == 0 {
            let keys: Vec<&str> = output.stdout.trim().lines().collect();
            Ok(keys.contains(&name))
        } else {
            Ok(false)
        }
    }

    /// 导出密钥
    pub fn export_key(&self, name: &str, target: &str, format: Option<&str>) -> Result<()> {
        let output = KuboCommand::export_key(name, target, format).run()?;
        if output.ret == 0 {
            Ok(())
        } else {
            Err(anyhow!("Failed to export key: {}", output.stderr))
        }
    }

    /// 导入密钥
    pub fn import_key(&self, name: &str, target: &str, format: Option<&str>) -> Result<String> {
        let output = KuboCommand::import_key(name, target, format).run()?;
        if output.ret == 0 {
            Ok(output.stdout.trim().to_string())
        } else {
            Err(anyhow!("Failed to import key: {}", output.stderr))
        }
    }

    // ============================================================
    // Content 操作
    // ============================================================

    /// 添加目录到 IPFS
    pub fn add_directory(&self, dir: &str) -> Result<String> {
        let output = KuboCommand::add_directory(dir).run()?;
        if output.ret == 0 {
            Ok(output.stdout.trim().to_string())
        } else {
            Err(anyhow!(
                "Failed to add directory: {}",
                output.stderr
            ))
        }
    }

    /// 获取文件 CID（不实际添加到 IPFS）
    pub fn get_file_cid(&self, file: &str) -> Result<String> {
        let output = KuboCommand::get_file_cid(file).run()?;
        if output.ret == 0 {
            Ok(output.stdout.trim().to_string())
        } else {
            Err(anyhow!("Failed to get file CID: {}", output.stderr))
        }
    }

    /// 获取文件 CIDv0
    pub fn get_file_cid_v0(&self, file: &str) -> Result<String> {
        let output = KuboCommand::get_file_cid_v0(file).run()?;
        if output.ret == 0 {
            Ok(output.stdout.trim().to_string())
        } else {
            Err(anyhow!("Failed to get file CIDv0: {}", output.stderr))
        }
    }

    // ============================================================
    // 静态配置数据
    // 对应 Swift: IPFSDaemon.peers / IPFSDaemon.resolvers
    // ============================================================

    /// IPFS Peering Peers（硬编码，与原项目一致）
    pub fn peers_json() -> String {
        serde_json::json!([
            {
                "ID": "12D3KooWBJY6ZVV8Tk8UDDFMEqWoxn89Xc8wnpm8uBFSR3ijDkui",
                "Addrs": [
                    "/ip4/167.71.172.216/tcp/4001",
                    "/ip6/2604:a880:800:10::826:1/tcp/4001",
                    "/ip4/167.71.172.216/udp/4001/quic",
                    "/ip6/2604:a880:800:10::826:1/udp/4001/quic"
                ]
            },
            {
                "ID": "12D3KooWDaGQ3Fu3iLgFxrrg5Vfef9z5L3DQZoyqFxQJbKKPnCc8",
                "Addrs": [
                    "/ip4/143.198.18.166/tcp/4001",
                    "/ip6/2604:a880:800:10::735:7001/tcp/4001",
                    "/ip4/143.198.18.166/udp/4001/quic",
                    "/ip6/2604:a880:800:10::735:7001/udp/4001/quic"
                ]
            },
            {
                "ID": "12D3KooWJ6MTkNM8Bu8DzNiRm1GY3Wqh8U8Pp1zRWap6xY3MvsNw",
                "Addrs": ["/dnsaddr/node-1.ipfs.bit.site"]
            },
            {
                "ID": "12D3KooWQ85aSCFwFkByr5e3pUCQeuheVhobVxGSSs1DrRQHGv1t",
                "Addrs": ["/dnsaddr/node-1.ipfs.4everland.net"]
            },
            {
                "ID": "12D3KooWGtYkBAaqJMJEmywMxaCiNP7LCEFUAFiLEBASe232c2VH",
                "Addrs": ["/dns4/bitswap.filebase.io/tcp/443/wss"]
            }
        ])
        .to_string()
    }

    /// DNS over HTTPS resolvers
    pub fn resolvers_json() -> String {
        serde_json::json!({
            "bit.": "https://dweb-dns.v2ex.pro/dns-query",
            "sol.": "https://dweb-dns.v2ex.pro/dns-query",
            "fc.": "https://dweb-dns.v2ex.pro/dns-query",
            "eth.": "https://dns.eth.limo/dns-query"
        })
        .to_string()
    }
}
```

### 6.3 注意事项

1. **`kill_on_drop(true)`**：当 `Child` 被 drop 时自动杀掉 daemon 进程，防止僵尸进程
2. **`run_streaming()`**：daemon 使用异步流式读取，不会阻塞主线程
3. **`api()` 方法**：使用 `reqwest` 发送 HTTP POST 请求，与 Swift 的 `URLSession` 对应
4. **peers/resolvers**：硬编码 JSON，直接从原项目 `IPFSDaemon.swift` 复制

---

## 7. Step 5：实现 IPFS 状态管理 (`ipfs/state.rs`)

### 7.1 对应关系

对应原项目 `IPFSState.swift`（ObservableObject），在 Rust 中使用 `Mutex<IpfsState>` + Tauri Events 实现。

### 7.2 完整代码

创建 `src-tauri/src/ipfs/state.rs`：

```rust
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Manager};
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
    pub fn new() -> Self {
        Self {
            daemon: IpfsDaemon::new(),
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
        if let Err(e) = app.emit_all("ipfs:state-changed", &snapshot) {
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
```

### 7.3 更新 `ipfs/mod.rs`

```rust
pub mod command;
pub mod daemon;
pub mod models;
pub mod state;
```

### 7.4 新增依赖：`hostname`

`daemon.rs` 中用到了 `hostname::get()`，需要在 `Cargo.toml` 中添加：

```toml
hostname = "0.4"
```

### 7.5 验证

```bash
cargo check
```

---

## 8. Step 6：注册 Tauri Commands (`commands/ipfs.rs`)

### 8.1 说明

Tauri Commands 是前端通过 `invoke()` 调用后端的入口。每个 Command 函数需要用 `#[tauri::command]` 宏标注。

### 8.2 完整代码

创建 `src-tauri/src/commands/ipfs.rs`：

```rust
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
```

### 8.3 更新 `commands/mod.rs`

```rust
pub mod ipfs;
pub mod planet;   // Phase 2 实现
pub mod article;  // Phase 3 实现
pub mod app;      // Phase 0 已有占位
```

### 8.4 验证

```bash
cargo check
```

---

## 9. Step 7：集成到 `main.rs` — 生命周期钩子

### 9.1 说明

需要在 `main.rs` 中：
1. 初始化日志系统
2. 创建 `IpfsState` 并注入 Tauri 全局状态
3. 注册所有 Tauri Commands
4. 在 `setup` 钩子中异步启动 IPFS daemon
5. 在窗口关闭/应用退出时优雅关闭 daemon

### 9.2 完整代码

替换 `src-tauri/src/main.rs`：

```rust
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

use ipfs::state::{IpfsState, IpfsStateHandle};

fn main() {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter("planet_desktop=debug,info")
        .with_target(false)
        .init();

    info!("Planet Desktop starting...");

    // 创建 IPFS 全局状态
    let ipfs_state: IpfsStateHandle = Arc::new(Mutex::new(IpfsState::new()));

    tauri::Builder::default()
        // 注入全局状态
        .manage(ipfs_state.clone())
        // 注册所有 Tauri Commands
        .invoke_handler(tauri::generate_handler![
            // Phase 0
            commands::app::greet,
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
            let app_handle = app.handle();
            let state = ipfs_state.clone();

            // 异步启动 IPFS daemon
            tauri::async_runtime::spawn(async move {
                ipfs::state::auto_start(state, app_handle).await;
            });

            Ok(())
        })
        // 应用退出钩子（优雅关闭 daemon）
        .on_window_event(move |event| {
            if let tauri::WindowEvent::Destroyed = event.event() {
                // 注意：这里需要通过 event.window() 获取 app handle
                // 来拿到 state，但在窗口销毁事件中处理可能不够可靠
                // 更好的方式是使用 Builder::build().run() 的 exit 回调
                // 见下方替代方案
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
```

### 9.3 替代方案说明

上面的代码使用 `.build().run()` 模式而非 `.run(tauri::generate_context!())`，因为后者无法获取 `ExitRequested` 事件。这是处理应用退出时优雅关闭 daemon 的推荐方式。

### 9.4 验证

```bash
cargo check
```

---

## 10. Step 8：前端实现 IPFS 状态面板

### 10.1 说明

前端需要实现：
1. `useIPFS` React Hook — 订阅 IPFS 状态变化
2. `IPFSStatusPanel` 组件 — 显示状态、端口、peers、操作按钮
3. 集成到主界面布局

### 10.2 TypeScript 类型定义

创建 `src/types/ipfs.ts`：

```typescript
/** IPFS 状态快照 — 与 Rust IpfsStateSnapshot 一一对应 */
export interface IpfsStateSnapshot {
  online: boolean
  is_operating: boolean
  api_port: number
  gateway_port: number
  swarm_port: number
  repo_size: number | null
  server_info: ServerInfo | null
  error_message: string | null
}

/** 服务器信息 — 与 Rust ServerInfo 一一对应 */
export interface ServerInfo {
  host_name: string
  version: string
  ipfs_peer_id: string
  ipfs_version: string
  ipfs_peer_count: number
}
```

### 10.3 实现 `useIPFS` Hook

创建 `src/hooks/useIPFS.ts`：

```typescript
import { useState, useEffect, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/tauri'
import { listen } from '@tauri-apps/api/event'
import type { IpfsStateSnapshot } from '../types/ipfs'

/** 默认初始状态 */
const defaultState: IpfsStateSnapshot = {
  online: false,
  is_operating: false,
  api_port: 5981,
  gateway_port: 18181,
  swarm_port: 4001,
  repo_size: null,
  server_info: null,
  error_message: null,
}

/**
 * IPFS 状态管理 Hook
 *
 * 功能：
 * 1. 监听后端推送的 "ipfs:state-changed" 事件，自动更新状态
 * 2. 提供 setup / launch / shutdown / gc / refresh 操作方法
 *
 * 对应原项目 SwiftUI 中的 @EnvironmentObject IPFSState
 */
export function useIPFS() {
  const [state, setState] = useState<IpfsStateSnapshot>(defaultState)
  const [loading, setLoading] = useState(true)

  // 初始加载：获取当前状态
  useEffect(() => {
    invoke<IpfsStateSnapshot>('ipfs_get_state')
      .then((s) => {
        setState(s)
        setLoading(false)
      })
      .catch((e) => {
        console.error('Failed to get IPFS state:', e)
        setLoading(false)
      })
  }, [])

  // 监听后端事件
  useEffect(() => {
    const unlisten = listen<IpfsStateSnapshot>('ipfs:state-changed', (event) => {
      setState(event.payload)
    })

    return () => {
      unlisten.then((fn) => fn())
    }
  }, [])

  // 操作方法
  const setup = useCallback(async () => {
    try {
      await invoke('ipfs_setup')
    } catch (e) {
      console.error('IPFS setup failed:', e)
    }
  }, [])

  const launch = useCallback(async () => {
    try {
      await invoke('ipfs_launch')
    } catch (e) {
      console.error('IPFS launch failed:', e)
    }
  }, [])

  const shutdown = useCallback(async () => {
    try {
      await invoke('ipfs_shutdown')
    } catch (e) {
      console.error('IPFS shutdown failed:', e)
    }
  }, [])

  const gc = useCallback(async (): Promise<number | null> => {
    try {
      return await invoke<number>('ipfs_gc')
    } catch (e) {
      console.error('IPFS GC failed:', e)
      return null
    }
  }, [])

  const refresh = useCallback(async () => {
    try {
      const s = await invoke<IpfsStateSnapshot>('ipfs_refresh_status')
      setState(s)
    } catch (e) {
      console.error('IPFS refresh failed:', e)
    }
  }, [])

  return {
    state,
    loading,
    setup,
    launch,
    shutdown,
    gc,
    refresh,
  }
}
```

### 10.4 实现 `IPFSStatusPanel` 组件

创建 `src/components/IPFSStatusPanel.tsx`：

```tsx
import React, { useState } from 'react'
import { useIPFS } from '../hooks/useIPFS'

/**
 * IPFS 状态面板
 * 对应原项目 Planet/IPFS/Status Views/IPFSStatusView.swift
 *
 * 显示内容：
 * - Online/Offline 状态指示灯
 * - Local Gateway 地址
 * - Repo Size
 * - Peers 数量
 * - IPFS Version
 * - Launch/Shutdown 切换按钮
 * - GC 按钮
 */
export function IPFSStatusPanel() {
  const { state, loading, launch, shutdown, gc, refresh } = useIPFS()
  const [showGCConfirm, setShowGCConfirm] = useState(false)
  const [gcResult, setGcResult] = useState<string | null>(null)

  if (loading) {
    return (
      <div className="p-4 bg-gray-50 dark:bg-gray-800 rounded-lg">
        <div className="animate-pulse text-gray-400">Loading IPFS status...</div>
      </div>
    )
  }

  const gatewayUrl = `http://127.0.0.1:${state.gateway_port}`

  /** 格式化字节 */
  const formatBytes = (bytes: number): string => {
    if (bytes === 0) return '0 B'
    const k = 1024
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB']
    const i = Math.floor(Math.log(bytes) / Math.log(k))
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i]
  }

  /** 切换 daemon 状态 */
  const handleToggle = async () => {
    if (state.online) {
      await shutdown()
    } else {
      await launch()
    }
  }

  /** 执行 GC */
  const handleGC = async () => {
    setShowGCConfirm(false)
    const count = await gc()
    if (count !== null) {
      setGcResult(`Removed ${count} unused objects`)
      setTimeout(() => setGcResult(null), 5000)
    }
  }

  return (
    <div className="w-72 bg-white dark:bg-gray-800 rounded-lg shadow-md overflow-hidden">
      {/* 状态信息区域 */}
      <div className="p-3 space-y-2 text-sm">
        {/* Local Gateway */}
        <div className="flex justify-between items-center">
          <span className="text-gray-500 dark:text-gray-400">Local Gateway</span>
          <a
            href={gatewayUrl}
            target="_blank"
            rel="noopener noreferrer"
            className={`text-blue-500 hover:underline text-xs ${
              !state.online ? 'opacity-50 pointer-events-none' : ''
            }`}
          >
            {gatewayUrl}
          </a>
        </div>

        {/* Repo Size */}
        <div className="flex justify-between items-center">
          <span className="text-gray-500 dark:text-gray-400">Repo Size</span>
          <span className="text-gray-700 dark:text-gray-300">
            {state.repo_size !== null ? formatBytes(state.repo_size) : '—'}
          </span>
        </div>

        {/* Peers */}
        <div className="flex justify-between items-center">
          <span className="text-gray-500 dark:text-gray-400">Peers</span>
          <span className="text-gray-700 dark:text-gray-300">
            {state.online && state.server_info
              ? state.server_info.ipfs_peer_count
              : '—'}
          </span>
        </div>

        {/* IPFS Version */}
        <div className="flex justify-between items-center">
          <span className="text-gray-500 dark:text-gray-400">IPFS Version</span>
          <span className="text-gray-700 dark:text-gray-300">
            {state.server_info?.ipfs_version || '—'}
          </span>
        </div>
      </div>

      {/* 分割线 */}
      <div className="border-t border-gray-200 dark:border-gray-700" />

      {/* 错误信息 */}
      {state.error_message && (
        <div className="px-3 py-2 bg-red-50 dark:bg-red-900/20 text-red-600 dark:text-red-400 text-xs">
          {state.error_message}
        </div>
      )}

      {/* GC 结果 */}
      {gcResult && (
        <div className="px-3 py-2 bg-green-50 dark:bg-green-900/20 text-green-600 dark:text-green-400 text-xs">
          {gcResult}
        </div>
      )}

      {/* 操作栏 */}
      <div className="px-3 py-2 flex items-center justify-between">
        <div className="flex items-center gap-2">
          {state.is_operating ? (
            <div className="flex items-center gap-2">
              <div className="w-2.5 h-2.5 rounded-full bg-yellow-400 animate-pulse" />
              <span className="text-xs text-gray-500">Processing...</span>
            </div>
          ) : (
            <>
              <div
                className={`w-2.5 h-2.5 rounded-full ${
                  state.online ? 'bg-green-500' : 'bg-red-500'
                }`}
              />
              <span className="text-sm font-medium text-gray-700 dark:text-gray-300">
                {state.online ? 'Online' : 'Offline'}
              </span>
            </>
          )}
        </div>

        <div className="flex items-center gap-2">
          {/* GC 按钮 */}
          {!state.is_operating && (
            <button
              onClick={() => setShowGCConfirm(true)}
              disabled={!state.online}
              className="p-1 rounded hover:bg-gray-100 dark:hover:bg-gray-700 disabled:opacity-30"
              title="Run IPFS garbage collection"
            >
              <svg className="w-4 h-4 text-gray-500" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2}
                  d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
              </svg>
            </button>
          )}

          {/* Launch/Shutdown 切换 */}
          {!state.is_operating && (
            <label className="relative inline-flex items-center cursor-pointer">
              <input
                type="checkbox"
                className="sr-only peer"
                checked={state.online}
                onChange={handleToggle}
              />
              <div className="w-9 h-5 bg-gray-200 peer-focus:outline-none rounded-full peer dark:bg-gray-600
                peer-checked:after:translate-x-full peer-checked:after:border-white
                after:content-[''] after:absolute after:top-[2px] after:left-[2px]
                after:bg-white after:border-gray-300 after:border after:rounded-full
                after:h-4 after:w-4 after:transition-all
                peer-checked:bg-green-500" />
            </label>
          )}

          {/* 操作中转圈 */}
          {state.is_operating && (
            <div className="w-5 h-5 border-2 border-gray-300 border-t-blue-500 rounded-full animate-spin" />
          )}
        </div>
      </div>

      {/* GC 确认弹窗 */}
      {showGCConfirm && (
        <div className="px-3 py-2 bg-yellow-50 dark:bg-yellow-900/20 border-t border-yellow-200 dark:border-yellow-800">
          <p className="text-xs text-yellow-700 dark:text-yellow-400 mb-2">
            Run garbage collection to free disk space?
          </p>
          <div className="flex gap-2">
            <button
              onClick={handleGC}
              className="px-2 py-1 text-xs bg-red-500 text-white rounded hover:bg-red-600"
            >
              Run GC
            </button>
            <button
              onClick={() => setShowGCConfirm(false)}
              className="px-2 py-1 text-xs bg-gray-200 dark:bg-gray-600 rounded hover:bg-gray-300 dark:hover:bg-gray-500"
            >
              Cancel
            </button>
          </div>
        </div>
      )}
    </div>
  )
}
```

### 10.5 更新 `App.tsx` 集成状态面板

修改 `src/App.tsx`：

```tsx
import { IPFSStatusPanel } from './components/IPFSStatusPanel'

function App() {
  return (
    <div className="flex h-screen bg-gray-100 dark:bg-gray-900">
      {/* 左侧边栏 */}
      <div className="w-60 bg-white dark:bg-gray-800 border-r border-gray-200 dark:border-gray-700 flex flex-col">
        <div className="p-4 text-lg font-bold text-gray-800 dark:text-gray-200">
          Planet
        </div>
        <div className="flex-1 overflow-y-auto">
          {/* Phase 2 中填充 Planet 列表 */}
          <div className="p-4 text-sm text-gray-400">
            Planets will appear here...
          </div>
        </div>
      </div>

      {/* 中间内容区 */}
      <div className="flex-1 flex flex-col">
        <div className="flex-1 flex items-center justify-center text-gray-400">
          Select a planet to view articles
        </div>
      </div>

      {/* 右侧边栏 — IPFS 状态面板 */}
      <div className="w-72 border-l border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 p-4">
        <IPFSStatusPanel />
      </div>
    </div>
  )
}

export default App
```

### 10.6 验证

```bash
pnpm tauri dev
```

前端应显示三栏布局，右侧面板显示 IPFS 状态信息。

---

## 11. Step 9：测试与调试

### 11.1 Rust 单元测试

在 `src-tauri/src/helpers/net.rs` 中已包含单元测试，运行：

```bash
cd src-tauri
cargo test
```

### 11.2 手动集成测试

启动开发环境：

```bash
pnpm tauri dev
```

**测试清单：**

| # | 测试项 | 预期结果 | 通过? |
|---|--------|----------|-------|
| 1 | 应用启动 | 自动初始化 IPFS repo 并启动 daemon | ☐ |
| 2 | 状态面板 | 显示 Online、端口号、IPFS 版本 | ☐ |
| 3 | Peers 数量 | 启动 30 秒后 Peers > 0 | ☐ |
| 4 | Repo Size | 显示非零的仓库大小 | ☐ |
| 5 | Gateway 链接 | 点击可在浏览器打开 | ☐ |
| 6 | Shutdown 按钮 | 关闭后状态变为 Offline，端口不可访问 | ☐ |
| 7 | Launch 按钮 | 重新启动后状态恢复 Online | ☐ |
| 8 | GC 按钮 | 弹出确认框，执行后显示结果 | ☐ |
| 9 | 关闭应用 | daemon 进程自动停止 | ☐ |
| 10 | 错误处理 | 删除 kubo 可执行文件后启动，显示错误信息 | ☐ |

### 11.3 调试技巧

**查看 Rust 日志：**

```bash
# 开发模式下在终端查看
RUST_LOG=debug pnpm tauri dev
```

**查看 IPFS 是否在运行：**

```bash
# 检查进程
# Windows
tasklist | findstr ipfs

# macOS/Linux
ps aux | grep ipfs
```

**手动调用 IPFS API（验证 daemon 是否正常）：**

```bash
# 将端口替换为实际端口
curl -X POST http://127.0.0.1:5981/api/v0/id
curl -X POST http://127.0.0.1:5981/api/v0/swarm/peers
curl -X POST http://127.0.0.1:5981/api/v0/version
curl -X POST http://127.0.0.1:5981/api/v0/repo/stat
curl -X POST http://127.0.0.1:5981/api/v0/stats/bw
```

### 11.4 常见问题排查

| 问题 | 可能原因 | 解决方案 |
|------|---------|---------|
| Daemon 启动超时 | 端口被占用 | 检查端口范围是否有冲突 |
| "Failed to execute kubo command" | Kubo 二进制不存在或无执行权限 | 检查 `resources/bin/` 目录 |
| 前端收不到事件 | `emit_all` 失败 | 检查 `tauri.conf.json` 中的 `withGlobalTauri` |
| API 返回 500 | IPFS repo 损坏 | 删除 `~/.planet/ipfs/` 重新初始化 |
| Windows 上权限错误 | kubo.exe 被安全软件拦截 | 添加杀毒软件白名单 |

---

## 12. 文件清单

Phase 1 完成后，以下文件应已创建或修改：

### 12.1 新建文件

| 文件路径 | 说明 |
|----------|------|
| `src-tauri/src/ipfs/models.rs` | IPFS API 响应结构体 + 状态快照 |
| `src-tauri/src/ipfs/command.rs` | Kubo CLI 命令封装 |
| `src-tauri/src/ipfs/daemon.rs` | IPFS Daemon 生命周期管理 |
| `src-tauri/src/ipfs/state.rs` | IPFS 全局状态 + 生命周期函数 |
| `src-tauri/src/helpers/net.rs` | 端口扫描工具 |
| `src-tauri/src/commands/ipfs.rs` | Tauri Commands（IPFS 相关） |
| `src/types/ipfs.ts` | TypeScript 类型定义 |
| `src/hooks/useIPFS.ts` | React Hook：IPFS 状态管理 |
| `src/components/IPFSStatusPanel.tsx` | React 组件：IPFS 状态面板 |

### 12.2 修改文件

| 文件路径 | 改动内容 |
|----------|---------|
| `src-tauri/Cargo.toml` | 添加 tokio, reqwest, anyhow, thiserror, tracing, hostname 依赖 |
| `src-tauri/src/main.rs` | 集成 IPFS 状态、Commands、生命周期钩子 |
| `src-tauri/src/ipfs/mod.rs` | 声明 command/daemon/models/state 子模块 |
| `src-tauri/src/helpers/mod.rs` | 声明 net 子模块 |
| `src-tauri/src/commands/mod.rs` | 声明 ipfs 子模块 |
| `src/App.tsx` | 集成 IPFSStatusPanel 组件 |

---

## 13. Swift → Rust 对照表

| 分类 | Swift (原项目) | Rust (新项目) |
|------|---------------|--------------|
| **文件** | `IPFSCommand.swift` | `ipfs/command.rs` |
| **文件** | `IPFSDaemon.swift` | `ipfs/daemon.rs` |
| **文件** | `IPFSState.swift` | `ipfs/state.rs` |
| **文件** | `IPFSAPIModel.swift` | `ipfs/models.rs` |
| **文件** | `PlanetStore+ServerInfo.swift` | `ipfs/models.rs` (ServerInfo) |
| **文件** | `KuboSwarmConnections.swift` | `ipfs/daemon.rs` (硬编码 low:10-20) |
| **文件** | `IPFSStatusView.swift` | `components/IPFSStatusPanel.tsx` |
| **文件** | `IPFSTrafficView.swift` | Phase 1 暂不实现（Phase 3+） |
| **进程管理** | `Foundation.Process` | `std::process::Command` / `tokio::process` |
| **HTTP 客户端** | `URLSession` | `reqwest::Client` |
| **JSON 解析** | `JSONDecoder` + `Codable` | `serde_json` + `Serialize/Deserialize` |
| **全局状态** | `ObservableObject` + `@Published` | `Arc<Mutex<IpfsState>>` + `app.emit_all()` |
| **并发模型** | `actor` + `async/await` | `tokio::Mutex` + `async/await` |
| **端口扫描** | `Darwin.bind()`+`listen()` | `TcpListener::bind()` |
| **日志** | `os.Logger` | `tracing` crate |
| **错误处理** | `throws` + `PlanetError` | `Result<T, anyhow::Error>` + `thiserror` |
| **UI 状态推送** | `@Published` 自动刷新 SwiftUI | `app.emit_all("ipfs:state-changed")` → `listen()` |
| **生命周期** | `applicationDidFinishLaunching` | `tauri::Builder::setup()` |
| **优雅关闭** | `applicationShouldTerminate` | `RunEvent::ExitRequested` |

---

## 14. 执行顺序总结

```
Step 1 → models.rs      (数据结构，无依赖)
Step 2 → command.rs      (依赖 helpers/paths)
Step 3 → helpers/net.rs  (无依赖)
Step 4 → daemon.rs       (依赖 Step 1+2+3)
Step 5 → state.rs        (依赖 Step 4)
Step 6 → commands/ipfs.rs(依赖 Step 5)
Step 7 → main.rs 集成     (依赖 Step 6)
Step 8 → 前端组件          (依赖 Step 7)
Step 9 → 测试验证          (依赖 Step 8)
```

**每完成一个 Step 都运行 `cargo check` 确保编译通过。**

---

> Phase 1 完成后，你将拥有一个可以自动启动/关闭 IPFS daemon 的桌面应用，前端可以实时看到 IPFS 状态。这为后续的 Planet 发布（Phase 2）和内容管理（Phase 3）打下了坚实基础。