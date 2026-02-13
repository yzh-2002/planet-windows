use std::path::PathBuf;
use tauri::{AppHandle, Manager};

/// 获取 Kubo 可执行文件路径（跨平台）
pub fn get_kubo_path(app: &AppHandle) -> PathBuf {
    // 在开发模式下，使用 target/debug/resources
    // 在生产模式下，使用 app.path().resource_dir()
    let resource_dir = if cfg!(debug_assertions) {
        // 开发模式：target/debug/resources
        std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(|p| p.to_path_buf()))
            .map(|exe_dir| exe_dir.join("resources"))
            .unwrap_or_else(|| {
                // 如果上面的方法失败，尝试从当前工作目录推断
                std::env::current_dir()
                    .unwrap_or_default()
                    .join("target/debug/resources")
            })
    } else {
        // 生产模式：使用 Tauri 的资源目录
        app.path()
            .resource_dir()
            .expect("Failed to get resource dir")
    };

    #[cfg(target_os = "windows")]
    {
        #[cfg(target_arch = "x86_64")]
        {
            resource_dir.join("bin").join("kubo-windows-amd64.exe")
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            panic!("Unsupported Windows architecture")
        }
    }

    #[cfg(target_os = "macos")]
    {
        #[cfg(target_arch = "aarch64")]
        {
            resource_dir.join("bin").join("kubo-darwin-arm64")
        }
        #[cfg(target_arch = "x86_64")]
        {
            resource_dir.join("bin").join("kubo-darwin-amd64")
        }
        #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
        {
            panic!("Unsupported macOS architecture")
        }
    }

    #[cfg(target_os = "linux")]
    {
        resource_dir.join("bin").join("kubo-linux-amd64")
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        panic!("Unsupported operating system")
    }
}

/// IPFS repo 路径
pub fn get_ipfs_repo_path(app: &AppHandle) -> PathBuf {
    let app_data = app
        .path()
        .app_data_dir()
        .expect("Failed to get app data dir");
    let repo = app_data.join("ipfs");
    std::fs::create_dir_all(&repo).ok();
    repo
}

/// 数据存储路径（Planet 数据）
pub fn get_data_path(app: &AppHandle) -> PathBuf {
    let app_data = app
        .path()
        .app_data_dir()
        .expect("Failed to get app data dir");
    let planet = app_data.join("Planet");
    std::fs::create_dir_all(&planet).ok();
    planet
}

/// 文档路径
pub fn get_documents_path(app: &AppHandle) -> PathBuf {
    app.path()
        .document_dir()
        .expect("Failed to get document dir")
}

/// 临时文件路径
pub fn get_temp_path(app: &AppHandle) -> PathBuf {
    let temp = app.path().temp_dir().expect("Failed to get temp dir");
    let planet_temp = temp.join("planet-desktop");
    std::fs::create_dir_all(&planet_temp).ok();
    planet_temp
}