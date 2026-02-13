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