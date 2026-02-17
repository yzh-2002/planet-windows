use anyhow::{Context, Result};
use hex;
use keyring::Entry;
use std::path::{Path, PathBuf};
use tracing::{debug, error, info, warn};

use crate::helpers::paths;
use crate::ipfs::daemon::IpfsDaemon;

/// Keystore 服务名称（对标 Swift 的 appServiceName）
const SERVICE_NAME: &str = "xyz.planetable.Planet";

/// 密钥存储管理器
///
/// 使用 `keyring-rs` 实现跨平台密钥安全存储：
/// - Windows: Credential Manager
/// - macOS: Keychain
/// - Linux: Secret Service (GNOME Keyring / KDE Wallet)
///
/// 对标 Swift KeychainHelper
pub struct Keystore;

impl Keystore {
    // ============================================================
    // 基础操作: save / load / check / delete
    // ============================================================

    /// 保存数据到安全存储
    ///
    /// 对标 Swift KeychainHelper.saveData(_:forKey:)
    pub fn save_data(key: &str, data: &[u8]) -> Result<()> {
        let entry = Entry::new(SERVICE_NAME, key)
            .map_err(|e| anyhow::anyhow!("创建 keyring entry 失败: {}", e))?;
        // keyring-rs 2.x 使用 set_password (字符串)，将字节数组编码为 hex 字符串
        let encoded = hex::encode(data);
        entry.set_password(&encoded)
            .map_err(|e| anyhow::anyhow!("保存密钥数据失败: {}", e))?;
        debug!("Keystore: 已保存 key={}", key);
        Ok(())
    }

    /// 从安全存储加载数据
    ///
    /// 对标 Swift KeychainHelper.loadData(forKey:)
    pub fn load_data(key: &str) -> Result<Vec<u8>> {
        let entry = Entry::new(SERVICE_NAME, key)
            .map_err(|e| anyhow::anyhow!("创建 keyring entry 失败: {}", e))?;
        let encoded = entry.get_password()
            .map_err(|e| anyhow::anyhow!("加载密钥数据失败: {}", e))?;
        let secret = hex::decode(&encoded)
            .map_err(|e| anyhow::anyhow!("解码密钥数据失败: {}", e))?;
        debug!("Keystore: 已加载 key={}, {} bytes", key, secret.len());
        Ok(secret)
    }

    /// 检查密钥是否存在
    ///
    /// 对标 Swift KeychainHelper.check(forKey:)
    pub fn check(key: &str) -> bool {
        match Entry::new(SERVICE_NAME, key) {
            Ok(entry) => entry.get_password().is_ok(),
            Err(_) => false,
        }
    }

    /// 删除密钥
    ///
    /// 对标 Swift KeychainHelper.delete(forKey:)
    pub fn delete(key: &str) -> Result<()> {
        let entry = Entry::new(SERVICE_NAME, key)
            .map_err(|e| anyhow::anyhow!("创建 keyring entry 失败: {}", e))?;
        entry.delete_password()
            .map_err(|e| anyhow::anyhow!("删除密钥失败: {}", e))?;
        info!("Keystore: 已删除 key={}", key);
        Ok(())
    }

    // ============================================================
    // IPFS 密钥操作
    // ============================================================

    /// 将 IPFS 密钥导出到安全存储 (备份)
    ///
    /// 流程：ipfs key export → 临时文件 → 读取 → 保存到 Keystore → 删除临时文件
    ///
    /// 对标 Swift KeychainHelper.exportKeyToKeychain(forPlanetKeyName:)
    pub fn export_key_to_keystore(daemon: &IpfsDaemon, key_name: &str, app: &tauri::AppHandle) -> Result<()> {
        let tmp_dir = crate::helpers::paths::get_temp_path(app);
        let tmp_key_path = tmp_dir.join(format!("{}.pem", key_name));

        // 确保临时目录存在
        std::fs::create_dir_all(&tmp_dir)?;

        // 如果临时文件已存在，先删除
        if tmp_key_path.exists() {
            std::fs::remove_file(&tmp_key_path)?;
        }

        // 从 IPFS 导出密钥到临时文件（需要将 PathBuf 转换为 &str）
        daemon.export_key(key_name, tmp_key_path.to_str().unwrap(), Some("pem-pkcs8-cleartext"))?;

        // 读取密钥数据
        let key_data = std::fs::read(&tmp_key_path)
            .with_context(|| format!("读取导出的密钥文件失败: {:?}", tmp_key_path))?;

        // 保存到安全存储
        Self::save_data(key_name, &key_data)?;

        // 清理临时文件
        let _ = std::fs::remove_file(&tmp_key_path);

        info!("Keystore: 已将 IPFS key '{}' 导出到安全存储", key_name);
        Ok(())
    }

    /// 从安全存储恢复 IPFS 密钥
    ///
    /// 流程：从 Keystore 加载 → 写入临时文件 → ipfs key import → 删除临时文件
    ///
    /// 对标 Swift KeychainHelper.importKeyFromKeychain(forPlanetKeyName:)
    pub fn import_key_from_keystore(daemon: &IpfsDaemon, key_name: &str, app: &tauri::AppHandle) -> Result<()> {
        if !Self::check(key_name) {
            anyhow::bail!("Keystore 中不存在密钥: {}", key_name);
        }

        let key_data = Self::load_data(key_name)?;
        let tmp_dir = crate::helpers::paths::get_temp_path(app);
        let tmp_key_path = tmp_dir.join(format!("{}.pem", key_name));

        std::fs::create_dir_all(&tmp_dir)?;

        if tmp_key_path.exists() {
            std::fs::remove_file(&tmp_key_path)?;
        }

        // 写入临时文件
        std::fs::write(&tmp_key_path, &key_data)
            .with_context(|| format!("写入临时密钥文件失败: {:?}", tmp_key_path))?;

        // 导入到 IPFS（需要将 PathBuf 转换为 &str）
        let result = daemon.import_key(key_name, tmp_key_path.to_str().unwrap(), Some("pem-pkcs8-cleartext"));

        // 清理临时文件
        let _ = std::fs::remove_file(&tmp_key_path);

        result?;
        info!("Keystore: 已从安全存储恢复 IPFS key '{}'", key_name);
        Ok(())
    }

    /// 将外部密钥文件导入到 IPFS 和 Keystore
    ///
    /// 对标 Swift KeychainHelper.importKeyFile(forPlanetKeyName:fileURL:)
    pub fn import_key_file(daemon: &IpfsDaemon, key_name: &str, file_path: &Path) -> Result<()> {
        // 读取密钥文件
        let key_data = std::fs::read(file_path)
            .with_context(|| format!("读取密钥文件失败: {:?}", file_path))?;

        // 导入到 IPFS（需要将 Path 转换为 &str）
        daemon.import_key(key_name, file_path.to_str().unwrap(), Some("pem-pkcs8-cleartext"))?;

        // 保存到 Keystore
        Self::save_data(key_name, &key_data)?;

        info!("Keystore: 已导入外部密钥 '{}' 到 IPFS 和安全存储", key_name);
        Ok(())
    }

    /// 将 IPFS 密钥导出到文件 (用于备份/分享)
    ///
    /// 对标 Swift KeychainHelper.exportKeyFile(forPlanetName:planetKeyName:toDirectory:)
    pub fn export_key_file(
        daemon: &IpfsDaemon,
        planet_name: &str,
        key_name: &str,
        target_dir: &Path,
        app: &tauri::AppHandle,
    ) -> Result<PathBuf> {
        let safe_name = planet_name.replace(|c: char| !c.is_alphanumeric() && c != '-' && c != '_', "_");
        let target_path = target_dir.join(format!("{}.pem", safe_name));

        if target_path.exists() {
            anyhow::bail!("目标文件已存在: {:?}", target_path);
        }

        let tmp_dir = crate::helpers::paths::get_temp_path(app);
        let tmp_key_path = tmp_dir.join(format!("{}.pem", safe_name));

        std::fs::create_dir_all(&tmp_dir)?;
        if tmp_key_path.exists() {
            std::fs::remove_file(&tmp_key_path)?;
        }

        // 从 IPFS 导出到临时文件（需要将 PathBuf 转换为 &str）
        daemon.export_key(key_name, tmp_key_path.to_str().unwrap(), Some("pem-pkcs8-cleartext"))?;

        // 复制到目标路径
        std::fs::copy(&tmp_key_path, &target_path)?;

        // 清理临时文件
        let _ = std::fs::remove_file(&tmp_key_path);

        info!("Keystore: 已导出密钥到 {:?}", target_path);
        Ok(target_path)
    }
}