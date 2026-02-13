use anyhow::{anyhow, Result};
use reqwest::Client;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tauri::AppHandle;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

use super::command::{KuboCommand, StreamLine};
use super::models::*;
use crate::helpers::net;

/// IPFS Daemon 管理器
/// 对应原项目 IPFSDaemon.swift (actor)
pub struct IpfsDaemon {
    app: AppHandle,
    setting_up: bool,
    pub swarm_port: Option<u16>,
    pub api_port: Option<u16>,
    pub gateway_port: Option<u16>,
    http_client: Client,
    /// daemon 子进程 handle（用于 shutdown 时 kill）
    daemon_child: Option<tokio::process::Child>,
}

impl IpfsDaemon {
    pub fn new(app: AppHandle) -> Self {
        info!("IpfsDaemon::new()");
        Self {
            app,
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

        // 创建一个临时的 KuboCommand 实例来获取 repo_path
        let cmd = KuboCommand::new(self.app.clone());
        let repo_path = cmd.repo_path();
        let is_empty = if repo_path.exists() {
            std::fs::read_dir(&repo_path)
                .map(|mut dir| dir.next().is_none())
                .unwrap_or(true)
        } else {
            true
        };

        if is_empty {
            info!("Initializing IPFS repo...");
            let output = KuboCommand::ipfs_init(self.app.clone()).run()?;
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
        let output = KuboCommand::update_swarm_port(self.app.clone(), swarm_port).run()?;
        if output.ret != 0 {
            return Err(anyhow!("Failed to update swarm port: {}", output.stderr));
        }
        self.swarm_port = Some(swarm_port);
        info!("Swarm port: {}", swarm_port);

        // 3. 扫描并配置 API 端口 (5981-5991)
        info!("Scanning API port...");
        let api_port = net::scout_port(5981..=5991)
            .ok_or_else(|| anyhow!("Unable to find open API port"))?;
        let output = KuboCommand::update_api_port(self.app.clone(), api_port).run()?;
        if output.ret != 0 {
            return Err(anyhow!("Failed to update API port: {}", output.stderr));
        }
        self.api_port = Some(api_port);
        info!("API port: {}", api_port);

        // 4. 扫描并配置 Gateway 端口 (18181-18191)
        info!("Scanning gateway port...");
        let gateway_port = net::scout_port(18181..=18191)
            .ok_or_else(|| anyhow!("Unable to find open gateway port"))?;
        let output = KuboCommand::update_gateway_port(self.app.clone(), gateway_port).run()?;
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
        let output = KuboCommand::set_peers(self.app.clone(), &peers_json).run()?;
        if output.ret != 0 {
            warn!("Failed to set peers: {}", output.stderr);
        }

        // 6. 配置 DNS Resolvers
        info!("Setting DNS resolvers...");
        let resolvers_json = Self::resolvers_json();
        let output = KuboCommand::set_resolvers(self.app.clone(), &resolvers_json).run()?;
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
        let output = KuboCommand::set_swarm_conn_mgr(self.app.clone(), &conn_mgr_json).run()?;
        if output.ret != 0 {
            return Err(anyhow!(
                "Failed to set SwarmConnMgr: {}",
                output.stderr
            ));
        }

        // 8. 配置 CORS
        info!("Setting Access-Control headers...");
        let allow_origin = serde_json::json!(["https://webui.ipfs.io"]).to_string();
        let _ = KuboCommand::set_access_control_allow_origin(self.app.clone(), &allow_origin).run();
        let allow_methods = serde_json::json!(["PUT", "POST"]).to_string();
        let _ = KuboCommand::set_access_control_allow_methods(self.app.clone(), &allow_methods).run();

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
        let _ = KuboCommand::shutdown_daemon(self.app.clone()).run();

        // 流式启动 daemon
        let (child, mut rx) = KuboCommand::launch_daemon(self.app.clone()).run_streaming().await?;
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
        let output = KuboCommand::shutdown_daemon(self.app.clone()).run();
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
        let output = KuboCommand::generate_key(self.app.clone(), name).run()?;
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
        let output = KuboCommand::delete_key(self.app.clone(), name).run()?;
        if output.ret == 0 {
            Ok(())
        } else {
            Err(anyhow!("Failed to delete key: {}", output.stderr))
        }
    }

    /// 列出所有密钥（排除 "self"）
    pub fn list_keys(&self) -> Result<Vec<String>> {
        let output = KuboCommand::list_keys(self.app.clone()).run()?;
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
        let output = KuboCommand::list_keys(self.app.clone()).run()?;
        if output.ret == 0 {
            let keys: Vec<&str> = output.stdout.trim().lines().collect();
            Ok(keys.contains(&name))
        } else {
            Ok(false)
        }
    }

    /// 导出密钥
    pub fn export_key(&self, name: &str, target: &str, format: Option<&str>) -> Result<()> {
        let output = KuboCommand::export_key(self.app.clone(), name, target, format).run()?;
        if output.ret == 0 {
            Ok(())
        } else {
            Err(anyhow!("Failed to export key: {}", output.stderr))
        }
    }

    /// 导入密钥
    pub fn import_key(&self, name: &str, target: &str, format: Option<&str>) -> Result<String> {
        let output = KuboCommand::import_key(self.app.clone(), name, target, format).run()?;
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
        let output = KuboCommand::add_directory(self.app.clone(), dir).run()?;
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
        let output = KuboCommand::get_file_cid(self.app.clone(), file).run()?;
        if output.ret == 0 {
            Ok(output.stdout.trim().to_string())
        } else {
            Err(anyhow!("Failed to get file CID: {}", output.stderr))
        }
    }

    /// 获取文件 CIDv0
    pub fn get_file_cid_v0(&self, file: &str) -> Result<String> {
        let output = KuboCommand::get_file_cid_v0(self.app.clone(), file).run()?;
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