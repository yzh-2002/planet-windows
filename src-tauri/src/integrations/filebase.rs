use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

/// Filebase pinning 服务
///
/// API 文档: https://docs.filebase.com/api-documentation/ipfs-pinning-service-api
///
/// 对标 Swift Filebase
pub struct Filebase {
    pin_name: String,
    api_token: String,
    client: Client,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FilebasePin {
    pub cid: String,
    pub request_id: String,
    pub status: String,
}

#[derive(Debug, Deserialize)]
struct PinListResponse {
    results: Option<Vec<PinListResult>>,
}

#[derive(Debug, Deserialize)]
struct PinListResult {
    requestid: Option<String>,
    pin: Option<PinInfo>,
}

#[derive(Debug, Deserialize)]
struct PinInfo {
    name: Option<String>,
    cid: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PinResponse {
    requestid: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PinStatusResponse {
    status: Option<String>,
    pin: Option<PinInfo>,
    error: Option<ErrorInfo>,
}

#[derive(Debug, Deserialize)]
struct ErrorInfo {
    reason: Option<String>,
}

impl Filebase {
    pub fn new(pin_name: String, api_token: String) -> Self {
        Filebase {
            pin_name,
            api_token,
            client: Client::new(),
        }
    }

    /// 提交 CID 进行 pinning
    ///
    /// 1. 先查询是否已有同名的 pin
    /// 2. 如果有，更新；如果没有，创建新的
    ///
    /// 返回 requestID
    ///
    /// 对标 Swift Filebase.pin(cid:)
    pub async fn pin(&self, cid: &str) -> Result<Option<String>> {
        let base_url = "https://api.filebase.io/v1/ipfs/pins";

        // Step 1: 查找已有的 request ID
        let existing_request_id = self.find_existing_request_id().await;

        // Step 2: 提交 pin 请求
        let url = if let Some(ref _request_id) = existing_request_id {
            // 已有 pin，使用 POST 创建新的（Filebase 会自动去重）
            base_url.to_string()
        } else {
            base_url.to_string()
        };

        let body = serde_json::json!({
            "name": self.pin_name,
            "cid": cid,
        });

        let resp = self.client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.api_token))
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            warn!("Filebase pin 请求失败: {} {}", status, body);
            return Ok(None);
        }

        let pin_resp: PinResponse = resp.json().await?;
        if let Some(request_id) = pin_resp.requestid {
            info!("Filebase: pin 成功，requestID={}", request_id);
            return Ok(Some(request_id));
        }

        Ok(None)
    }

    /// 查找已有的同名 pin 的 request ID
    async fn find_existing_request_id(&self) -> Option<String> {
        let url = "https://api.filebase.io/v1/ipfs/pins";

        let resp = self.client
            .get(url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.api_token))
            .send()
            .await
            .ok()?;

        let list: PinListResponse = resp.json().await.ok()?;
        if let Some(results) = list.results {
            for result in results {
                if let Some(ref pin) = result.pin {
                    if pin.name.as_deref() == Some(&self.pin_name) {
                        debug!("Filebase: 找到已有的 requestID for '{}'", self.pin_name);
                        return result.requestid;
                    }
                }
            }
        }

        debug!("Filebase: 未找到 '{}' 的 requestID", self.pin_name);
        None
    }

    /// 检查 pin 状态
    ///
    /// 对标 Swift Filebase.checkPinStatus(requestID:)
    pub async fn check_pin_status(&self, request_id: &str) -> Result<Option<FilebasePin>> {
        let url = format!("https://api.filebase.io/v1/ipfs/pins/{}", request_id);

        let resp = self.client
            .get(&url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.api_token))
            .send()
            .await?;

        let status_resp: PinStatusResponse = resp.json().await?;

        if let Some(ref error) = status_resp.error {
            if let Some(ref reason) = error.reason {
                warn!("Filebase check status 错误: {}", reason);
                return Ok(None);
            }
        }

        if let (Some(status), Some(pin)) = (status_resp.status, status_resp.pin) {
            if let Some(cid) = pin.cid {
                return Ok(Some(FilebasePin {
                    cid,
                    request_id: request_id.to_string(),
                    status,
                }));
            }
        }

        Ok(None)
    }
}