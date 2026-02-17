use anyhow::{Context, Result};
use serde::Deserialize;
use tracing::{debug, info, warn};

use crate::ipfs::daemon::IpfsDaemon;

// ============================================================
// IPNS 判断
// ============================================================

/// 判断一个字符串是否为 IPNS 地址
///
/// 对标 Swift ENSUtils.isIPNS()
pub fn is_ipns(s: &str) -> bool {
    if !s.starts_with('k') {
        return false;
    }
    // CIDv1 in libp2p-key codec: k51... (62 chars)
    if s.starts_with("k51") && s.len() == 62 {
        return true;
    }
    // CIDv1 in older format: k2... (56 chars)
    if s.starts_with("k2") && s.len() == 56 {
        return true;
    }
    false
}

// ============================================================
// ENS 解析
// ============================================================

/// ENS 解析结果
#[derive(Debug, Clone)]
pub struct EnsResolution {
    pub content_hash: Option<String>,
    pub address: Option<String>,
    pub avatar: Option<String>,
}

/// 解析 ENS 域名的 contenthash
///
/// 对标 Swift ENSDataKit 的 resolve()
///
/// 使用公共 HTTP API (enstate.rs) 进行解析
pub async fn resolve_ens(name: &str) -> Result<EnsResolution> {
    // 方案1: 使用 enstate.rs API (免费、可靠)
    let url = format!("https://enstate.rs/n/{}", name);
    let resp = reqwest::get(&url)
        .await
        .with_context(|| format!("ENS 解析请求失败: {}", name))?;

    if !resp.status().is_success() {
        anyhow::bail!("ENS 解析失败 (HTTP {}): {}", resp.status(), name);
    }

    let data: EnstateResponse = resp
        .json()
        .await
        .with_context(|| format!("解析 ENS 响应失败: {}", name))?;

    info!("ENS 解析成功: {} -> contenthash={:?}", name, data.contenthash);

    Ok(EnsResolution {
        content_hash: data.contenthash,
        address: data.address,
        avatar: data.avatar,
    })
}

/// enstate.rs API 响应结构
#[derive(Debug, Deserialize)]
struct EnstateResponse {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub address: Option<String>,
    #[serde(default)]
    pub avatar: Option<String>,
    #[serde(default)]
    pub contenthash: Option<String>,
    #[serde(default)]
    pub display: Option<String>,
}

// ============================================================
// contenthash → CID
// ============================================================

/// 从 contenthash 解析出 CID
///
/// 对标 Swift ENSUtils.getCID(from:)
///
/// contenthash 格式示例：
///   - "ipns://k51..." → 调用 IPFS resolve
///   - "ipfs://Qm..." → 直接返回 CID
///   - "ipfs://bafy..." → 直接返回 CID
///   - "k51..." → 当作 IPNS 解析
///   - "Qm..." → 当作 CID 直接返回
///   - "bafy..." → 当作 CID 直接返回
pub async fn resolve_contenthash_to_cid(
    contenthash: &str,
    daemon: &IpfsDaemon,
) -> Result<String> {
    let hash = contenthash.trim();

    // 已经是纯 CID
    if hash.starts_with("Qm") || hash.starts_with("bafy") {
        return Ok(hash.to_string());
    }

    // ipfs:// 协议
    if let Some(cid) = hash.strip_prefix("ipfs://") {
        return Ok(cid.to_string());
    }

    // ipns:// 协议 → 需要 IPFS resolve
    if let Some(ipns) = hash.strip_prefix("ipns://") {
        let cid = daemon
            .resolve_ipns(ipns)
            .await
            .with_context(|| format!("IPNS 解析失败: {}", ipns))?;
        return Ok(cid);
    }

    // k51... 或 k2... → IPNS 地址
    if is_ipns(hash) {
        let cid = daemon
            .resolve_ipns(hash)
            .await
            .with_context(|| format!("IPNS 解析失败: {}", hash))?;
        return Ok(cid);
    }

    anyhow::bail!("不支持的 contenthash 格式: {}", hash)
}

/// 组装 contenthash URL
///
/// 根据 contenthash 字符串判断是 ipns 还是 ipfs
pub fn contenthash_to_url(contenthash: &str) -> String {
    if contenthash.starts_with("k51") {
        format!("ipns://{}", contenthash)
    } else if contenthash.starts_with("Qm") || contenthash.starts_with("bafy") {
        format!("ipfs://{}", contenthash)
    } else {
        format!("ipfs://{}", contenthash)
    }
}

// ============================================================
// 单元测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_ipns() {
        // k51... (62 chars)
        assert!(is_ipns("k51qzi5uqu5dgutgmsbx7rlff2kj1coemc7tgrchii04upey7s75btk2dpceeu"));
        // k2... (56 chars)
        assert!(is_ipns("k2k4r8jx63q4e5n7s0gu5b8wq0m7j2a1k4l9n3p5h7d8f2c6abcdefgh"));
        // 太短
        assert!(!is_ipns("k51short"));
        // 不以 k 开头
        assert!(!is_ipns("Qm1234567890"));
        // k2 长度不对
        assert!(!is_ipns("k2k4r8jx63q4e5n7"));
    }

    #[tokio::test]
    #[ignore] // 需要网络访问，手动运行: cargo test -- --ignored
    async fn test_resolve_ens() {
        let result = resolve_ens("vitalik.eth").await;
        assert!(result.is_ok());
        let res = result.unwrap();
        assert!(res.content_hash.is_some());
    }
}