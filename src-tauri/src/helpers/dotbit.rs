use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

// ============================================================
// 数据结构
// ============================================================

/// DWeb 记录类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DWebRecordType {
    Ipfs,
    Ipns,
}

/// DWeb 记录
#[derive(Debug, Clone)]
pub struct DWebRecord {
    pub record_type: DWebRecordType,
    pub value: String,
}

// ============================================================
// .bit API 响应结构
// ============================================================

#[derive(Debug, Deserialize)]
struct DotBitResponse {
    err_no: i32,
    #[serde(default)]
    err_msg: Option<String>,
    #[serde(default)]
    data: Option<DotBitData>,
}

#[derive(Debug, Deserialize)]
struct DotBitData {
    #[serde(default)]
    records: Vec<DotBitRecord>,
}

#[derive(Debug, Deserialize)]
struct DotBitRecord {
    key: String,
    value: String,
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    ttl: Option<String>,
}

// ============================================================
// .bit 解析
// ============================================================

const DOTBIT_INDEXER_URL: &str = "https://indexer-v1.did.id";

/// 解析 .bit 域名的 DWeb 记录
///
/// 对标 Swift DotBitKit.resolve()
pub async fn resolve_dotbit(account: &str) -> Result<Option<DWebRecord>> {
    if account.len() <= 4 {
        return Ok(None);
    }

    let url = format!("{}/v1/account/records", DOTBIT_INDEXER_URL);

    #[derive(Serialize)]
    struct Request {
        account: String,
    }

    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .json(&Request {
            account: account.to_string(),
        })
        .send()
        .await
        .with_context(|| format!(".bit 解析请求失败: {}", account))?;

    if !resp.status().is_success() {
        warn!(".bit 解析 HTTP 错误: {} for {}", resp.status(), account);
        return Ok(None);
    }

    let data: DotBitResponse = resp
        .json()
        .await
        .with_context(|| format!("解析 .bit 响应失败: {}", account))?;

    if data.err_no != 0 {
        warn!(
            ".bit API 错误: err_no={}, msg={:?} for {}",
            data.err_no, data.err_msg, account
        );
        return Ok(None);
    }

    let records = match data.data {
        Some(d) => d.records,
        None => return Ok(None),
    };

    debug!(".bit account {} has {} records", account, records.len());

    // 遍历 records，查找 dweb 记录
    // 优先 IPNS，其次 IPFS
    let mut ipns_record: Option<DWebRecord> = None;
    let mut ipfs_record: Option<DWebRecord> = None;

    for record in &records {
        if record.key.starts_with("dweb") {
            if record.key.ends_with("ipns") {
                ipns_record = Some(DWebRecord {
                    record_type: DWebRecordType::Ipns,
                    value: record.value.clone(),
                });
            }
            if record.key.ends_with("ipfs") {
                ipfs_record = Some(DWebRecord {
                    record_type: DWebRecordType::Ipfs,
                    value: record.value.clone(),
                });
            }
        }
    }

    // 优先返回 IPNS，因为可以更新
    if let Some(r) = ipns_record {
        info!(".bit {} resolved to IPNS: {}", account, r.value);
        return Ok(Some(r));
    }
    if let Some(r) = ipfs_record {
        info!(".bit {} resolved to IPFS: {}", account, r.value);
        return Ok(Some(r));
    }

    info!(".bit {} has no dweb records", account);
    Ok(None)
}

// ============================================================
// 单元测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // 需要网络访问，手动运行: cargo test -- --ignored
    async fn test_resolve_dotbit() {
        let result = resolve_dotbit("test.bit").await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_short_account() {
        // 太短的账户应返回 None（长度 <= 4 直接返回 Ok(None)）
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(resolve_dotbit("ab"));
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }
}