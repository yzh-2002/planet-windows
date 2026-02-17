use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

/// Pinnable.xyz pinning 服务
///
/// 对标 Swift Pinnable
pub struct Pinnable {
    api: String,
    client: Client,
}

/// Pin 状态
///
/// 对标 Swift PinnablePinStatus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinnablePinStatus {
    pub status: String,
    pub last_known_ipns: Option<String>,
    pub last_known_cid: Option<String>,
    pub size: Option<i64>,
    pub created: Option<i64>,
    pub last_checked: Option<i64>,
    pub last_pinned: Option<i64>,
}

impl Pinnable {
    pub fn new(api: String) -> Self {
        Pinnable {
            api,
            client: Client::new(),
        }
    }

    /// 发送 pin 请求
    ///
    /// 对标 Swift Pinnable.pin()
    pub async fn pin(&self) -> Result<()> {
        let resp = self.client
            .get(&self.api)
            .send()
            .await?;

        let status = resp.status();
        if status.as_u16() == 202 {
            info!("Pinnable: pin 请求已接受 (202)");
            Ok(())
        } else {
            warn!("Pinnable: 意外的状态码 {}", status);
            anyhow::bail!("Pinnable pin 失败，状态码: {}", status);
        }
    }

    /// 查询 pin 状态
    ///
    /// 对标 Swift Pinnable.status()
    pub async fn status(&self) -> Result<Option<PinnablePinStatus>> {
        let url = format!("{}/status", self.api);
        debug!("Pinnable: 查询状态 {}", url);

        let resp = self.client
            .get(&url)
            .send()
            .await?;

        if !resp.status().is_success() {
            warn!("Pinnable: 查询状态失败 {}", resp.status());
            return Ok(None);
        }

        let status: PinnablePinStatus = resp.json().await?;
        Ok(Some(status))
    }
}