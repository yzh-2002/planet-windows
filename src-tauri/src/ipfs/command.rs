use anyhow::{anyhow, Result};
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use tauri::AppHandle; 
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as TokioCommand;
use tokio::sync::mpsc;
use tracing::{debug, info};

use crate::helpers::paths;

/// Kubo CLI 命令封装
/// 对应原项目 IPFSCommand.swift
pub struct KuboCommand {
    app: AppHandle,
    args: Vec<String>,
}

impl KuboCommand {
    /// 创建新的 KuboCommand
    pub fn new(app: AppHandle) -> Self {
        Self {
            app,
            args: Vec::new(),
        }
    }

    /// 带参数创建
    pub fn with_args(app: AppHandle, args: Vec<String>) -> Self {
        Self { app, args }
    }
    // ============================================================
    // 路径
    // ============================================================

    /// Kubo 可执行文件路径
    pub fn executable_path(&self) -> PathBuf {
        paths::get_kubo_path(&self.app)
    }

    /// IPFS 仓库路径 (~/.planet/ipfs/)
    pub fn repo_path(&self) -> PathBuf {
        let repo = paths::get_ipfs_repo_path(&self.app);
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
        let exe = self.executable_path();
        let repo = self.repo_path();

        // 增强日志输出
        info!("=== Executing Kubo Command ===");
        info!("Executable: {:?}", exe);
        info!("Arguments: {:?}", self.args);
        info!("IPFS_PATH: {:?}", repo);
        info!("File exists: {}", exe.exists());

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
        let exe = self.executable_path();
        let repo = self.repo_path();
        // 增强日志输出
        info!("=== Launching Kubo Daemon (Streaming) ===");
        info!("Executable: {:?}", exe);
        info!("Arguments: {:?}", self.args);
        info!("IPFS_PATH: {:?}", repo);
        info!("File exists: {}", exe.exists());

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
    pub fn ipfs_init(app: AppHandle) -> Self {
        Self { app, args: vec!["init".into()] }
    }

    /// ipfs version
    pub fn ipfs_version(app: AppHandle) -> Self {
        Self { app, args: vec!["version".into()] }
    }

    /// ipfs config Addresses.API /ip4/127.0.0.1/tcp/{port}
    pub fn update_api_port(app: AppHandle, port: u16) -> Self {
        Self {
            app,
            args: vec![
                "config".into(),
                "Addresses.API".into(),
                format!("/ip4/127.0.0.1/tcp/{}", port),
            ],
        }
    }

    /// ipfs config Addresses.Gateway /ip4/127.0.0.1/tcp/{port}
    pub fn update_gateway_port(app: AppHandle, port: u16) -> Self {
        Self {
            app,
            args: vec![
                "config".into(),
                "Addresses.Gateway".into(),
                format!("/ip4/127.0.0.1/tcp/{}", port),
            ],
        }
    }

    /// ipfs config Addresses.Swarm [多地址] --json
    pub fn update_swarm_port(app: AppHandle, port: u16) -> Self {
        let swarm_json = format!(
            r#"["/ip4/0.0.0.0/tcp/{port}", "/ip6/::/tcp/{port}", "/ip4/0.0.0.0/udp/{port}/quic", "/ip6/::/udp/{port}/quic"]"#,
        );
        Self {
            app,
            args: vec![
                "config".into(),
                "Addresses.Swarm".into(),
                swarm_json,
                "--json".into(),
            ],
        }
    }

    /// ipfs config Peering.Peers {json} --json
    pub fn set_peers(app: AppHandle, peers_json: &str) -> Self {
        Self {
            app,
            args: vec![
                "config".into(),
                "Peering.Peers".into(),
                peers_json.into(),
                "--json".into(),
            ],
        }
    }

    /// ipfs config DNS.Resolvers {json} --json
    pub fn set_resolvers(app: AppHandle, resolvers_json: &str) -> Self {
        Self {
            app,
            args: vec![
                "config".into(),
                "DNS.Resolvers".into(),
                resolvers_json.into(),
                "--json".into(),
            ],
        }
    }

    /// ipfs config Swarm.ConnMgr {json} --json
    pub fn set_swarm_conn_mgr(app: AppHandle, json: &str) -> Self {
        Self {
            app,
            args: vec![
                "config".into(),
                "Swarm.ConnMgr".into(),
                json.into(),
                "--json".into(),
            ],
        }
    }

    /// ipfs config API.HTTPHeaders.Access-Control-Allow-Origin {json} --json
    pub fn set_access_control_allow_origin(app: AppHandle, json: &str) -> Self {
        Self {
            app,
            args: vec![
                "config".into(),
                "API.HTTPHeaders.Access-Control-Allow-Origin".into(),
                json.into(),
                "--json".into(),
            ],
        }
    }

    /// ipfs config API.HTTPHeaders.Access-Control-Allow-Methods {json} --json
    pub fn set_access_control_allow_methods(app: AppHandle, json: &str) -> Self {
        Self {
            app,
            args: vec![
                "config".into(),
                "API.HTTPHeaders.Access-Control-Allow-Methods".into(),
                json.into(),
                "--json".into(),
            ],
        }
    }

    /// ipfs daemon --migrate --enable-namesys-pubsub --enable-pubsub-experiment
    pub fn launch_daemon(app: AppHandle) -> Self {
        Self {
            app,
            args: vec![
                "daemon".into(),
                "--migrate".into(),
                "--enable-namesys-pubsub".into(),
                "--enable-pubsub-experiment".into(),
            ],
        }
    }

    /// ipfs shutdown
    pub fn shutdown_daemon(app: AppHandle) -> Self {
        Self {
            app,
            args: vec!["shutdown".into()],
        }
    }

    /// ipfs add -r -H {directory} --cid-version=1 --quieter
    pub fn add_directory(app: AppHandle, directory: &str) -> Self {
        Self {
            app,
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
    pub fn get_file_cid(app: AppHandle, file: &str) -> Self {
        Self {
            app,
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
    pub fn get_file_cid_v0(app: AppHandle, file: &str) -> Self {
        Self {
            app,
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
    pub fn generate_key(app: AppHandle, name: &str) -> Self {
        Self {
            app,
            args: vec!["key".into(), "gen".into(), name.into()],
        }
    }

    /// ipfs key rm {name}
    pub fn delete_key(app: AppHandle, name: &str) -> Self {
        Self {
            app,
            args: vec!["key".into(), "rm".into(), name.into()],
        }
    }

    /// ipfs key list
    pub fn list_keys(app: AppHandle) -> Self {
        Self {
            app,
            args: vec!["key".into(), "list".into()],
        }
    }

    /// ipfs key export {name} -o {target} [--format={format}]
    pub fn export_key(app: AppHandle, name: &str, target: &str, format: Option<&str>) -> Self {
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
        Self { app, args }
    }

    /// ipfs key import {name} {target} [--format={format}]
    pub fn import_key(app: AppHandle, name: &str, target: &str, format: Option<&str>) -> Self {
        let mut args = vec![
            "key".into(),
            "import".into(),
            name.into(),
            target.into(),
        ];
        if let Some(fmt) = format {
            args.push(format!("--format={}", fmt));
        }
        Self { app, args }
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