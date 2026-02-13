# Phase 0 详细执行步骤

> 目标：搭建完整的 Tauri 项目结构，集成 Kubo 二进制，应用能启动并展示基本窗口

---

## 前置准备

### 1. 安装必要工具

```bash
# 安装 Node.js (v18+)
node --version  # 确认版本 >= 18

# 安装 Rust (如果未安装)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustc --version  # 确认安装成功

# 安装 Tauri CLI
npm install -g @tauri-apps/cli@latest

# 安装 pnpm (推荐) 或使用 npm
npm install -g pnpm
```

### 2. 检查系统依赖

**Windows:**
- 确保已安装 Visual Studio Build Tools 或 Visual Studio Community
- 确保已安装 WebView2 Runtime（Windows 11 自带，Windows 10 需要单独安装）

**macOS:**
- 确保已安装 Xcode Command Line Tools: `xcode-select --install`

---

## 步骤 1：初始化 Tauri 项目

### 1.1 创建项目

```bash
# 在 Planet 项目根目录下创建新目录
cd /Users/zihan.yang/yangzihan/Planet
mkdir planet-desktop
cd planet-desktop

# 使用 Tauri CLI 创建项目（选择 React + TypeScript 模板）
npm create tauri-app@latest . -- --template react-ts

# 或者使用交互式命令
npm create tauri-app@latest
# 选择：
# - Project name: planet-desktop
# - Template: react-ts
# - Package manager: pnpm (或 npm)
```

### 1.2 验证项目结构

创建后应看到以下目录结构：

```
planet-desktop/
├── src/                    # 前端代码 (React)
│   ├── App.tsx
│   ├── main.tsx
│   └── ...
├── src-tauri/              # Rust 后端代码
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── src/
│   │   └── main.rs
│   └── ...
├── package.json
├── vite.config.ts
└── ...
```

### 1.3 测试运行

```bash
# 安装依赖
pnpm install  # 或 npm install

# 启动开发模式
pnpm tauri dev  # 或 npm run tauri dev
```

如果看到 Tauri 窗口正常打开，说明基础项目创建成功 ✅

---

## 步骤 2：配置 TailwindCSS

### 2.1 安装 TailwindCSS

```bash
cd planet-desktop
pnpm add -D tailwindcss postcss autoprefixer
pnpm exec tailwindcss init -p
```

### 2.2 配置 `tailwind.config.js`

```javascript
/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {},
  },
  plugins: [],
}
```

### 2.3 配置 `src/index.css`

```css
@tailwind base;
@tailwind components;
@tailwind utilities;
```

### 2.4 在 `src/main.tsx` 中引入 CSS

```typescript
import React from 'react'
import ReactDOM from 'react-dom/client'
import App from './App'
import './index.css'  // 添加这一行

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
)
```

### 2.5 验证 TailwindCSS

修改 `src/App.tsx` 测试：

```tsx
function App() {
  return (
    <div className="p-8 bg-gray-100">
      <h1 className="text-2xl font-bold text-blue-600">Planet Desktop</h1>
    </div>
  )
}

export default App
```

运行 `pnpm tauri dev`，如果看到蓝色标题，说明 TailwindCSS 配置成功 ✅

---

## 步骤 3：搭建 Rust 后端目录结构

### 3.1 创建目录

```bash
cd src-tauri/src
mkdir -p commands ipfs models template keystore helpers store
```

### 3.2 创建模块文件

```bash
# 创建各个模块的 mod.rs 文件
touch commands/mod.rs
touch ipfs/mod.rs
touch models/mod.rs
touch template/mod.rs
touch keystore/mod.rs
touch helpers/mod.rs
touch store/mod.rs
touch helpers/paths.rs
```

### 3.3 配置 `src-tauri/src/main.rs`

```rust
// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod helpers;
mod ipfs;
mod keystore;
mod models;
mod store;
mod template;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            // 暂时留空，后续添加 commands
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### 3.4 创建占位文件

在配置 `mod.rs` 之前，需要先创建占位文件，否则 Rust 编译器会报错找不到模块。

```bash
cd src-tauri/src

# 创建 commands 模块的占位文件
touch commands/app.rs
touch commands/ipfs.rs
touch commands/planet.rs
touch commands/article.rs

# 创建 ipfs 模块的占位文件
touch ipfs/command.rs
touch ipfs/daemon.rs
touch ipfs/state.rs
touch ipfs/models.rs

# 创建 models 模块的占位文件
touch models/planet.rs
touch models/article.rs
touch models/draft.rs
```

### 3.5 配置各个模块的 `mod.rs`

**`src-tauri/src/commands/mod.rs`:**
```rust
pub mod app;
// 以下模块在后续阶段实现，先创建占位文件
pub mod ipfs;
pub mod planet;
pub mod article;
```

**`src-tauri/src/ipfs/mod.rs`:**
```rust
// 以下模块在 Phase 1 实现，先创建占位文件
pub mod command;
pub mod daemon;
pub mod state;
pub mod models;
```

**`src-tauri/src/models/mod.rs`:**
```rust
// 以下模块在 Phase 2 实现，先创建占位文件
pub mod planet;
pub mod article;
pub mod draft;
```

**`src-tauri/src/helpers/mod.rs`:**
```rust
pub mod paths;
```

**`src-tauri/src/store/mod.rs`:**
```rust
// Phase 2 实现
```

**`src-tauri/src/template/mod.rs`:**
```rust
// Phase 5 实现
```

**`src-tauri/src/keystore/mod.rs`:**
```rust
// Phase 3 实现
```

### 3.6 创建占位文件内容

为了让代码能编译通过，需要为占位文件添加最小内容：

**`src-tauri/src/commands/ipfs.rs`:**
```rust
// Phase 1 实现
```

**`src-tauri/src/commands/planet.rs`:**
```rust
// Phase 2 实现
```

**`src-tauri/src/commands/article.rs`:**
```rust
// Phase 2 实现
```

**`src-tauri/src/ipfs/command.rs`:**
```rust
// Phase 1 实现
```

**`src-tauri/src/ipfs/daemon.rs`:**
```rust
// Phase 1 实现
```

**`src-tauri/src/ipfs/state.rs`:**
```rust
// Phase 1 实现
```

**`src-tauri/src/ipfs/models.rs`:**
```rust
// Phase 1 实现
```

**`src-tauri/src/models/planet.rs`:**
```rust
// Phase 2 实现
```

**`src-tauri/src/models/article.rs`:**
```rust
// Phase 2 实现
```

**`src-tauri/src/models/draft.rs`:**
```rust
// Phase 2 实现
```

### 3.7 验证编译

```bash
cd src-tauri
cargo check
```

如果编译通过，说明目录结构搭建成功 ✅

---

## 步骤 4：下载并集成 Kubo 二进制

### 4.1 创建资源目录

```bash
cd src-tauri
mkdir -p resources/bin
```

### 4.2 下载 Kubo 二进制文件

**Windows amd64:**
```bash
# 从 https://github.com/ipfs/kubo/releases 下载最新版本
# 例如：kubo_v0.26.0_windows-amd64.zip
# 解压后找到 ipfs.exe，重命名为 kubo-windows-amd64.exe

# 或者使用 curl (Windows 需要安装 Git Bash 或 WSL)
curl -L https://github.com/ipfs/kubo/releases/download/v0.26.0/kubo_v0.26.0_windows-amd64.zip -o kubo.zip
unzip kubo.zip
mv kubo/ipfs.exe resources/bin/kubo-windows-amd64.exe
rm -rf kubo kubo.zip
```

**macOS arm64 (Apple Silicon):**
```bash
curl -L https://github.com/ipfs/kubo/releases/download/v0.26.0/kubo_v0.26.0_darwin-arm64.tar.gz -o kubo.tar.gz
tar -xzf kubo.tar.gz
mv kubo/ipfs resources/bin/kubo-darwin-arm64
chmod +x resources/bin/kubo-darwin-arm64
rm -rf kubo kubo.tar.gz
```

**macOS amd64 (Intel):**
```bash
curl -L https://github.com/ipfs/kubo/releases/download/v0.26.0/kubo_v0.26.0_darwin-amd64.tar.gz -o kubo.tar.gz
tar -xzf kubo.tar.gz
mv kubo/ipfs resources/bin/kubo-darwin-amd64
chmod +x resources/bin/kubo-darwin-amd64
rm -rf kubo kubo.tar.gz
```

### 4.3 验证二进制文件

```bash
# Windows
./resources/bin/kubo-windows-amd64.exe version

# macOS
./resources/bin/kubo-darwin-arm64 version
# 或
./resources/bin/kubo-darwin-amd64 version
```

如果显示版本号，说明下载成功 ✅

---

## 步骤 5：配置 Tauri 资源绑定

### 5.1 编辑 `src-tauri/tauri.conf.json`

找到 `bundle` 部分，添加 `resources` 配置：

```json
{
  "build": {
    "beforeDevCommand": "pnpm dev",
    "devPath": "http://localhost:1420",
    "beforeBuildCommand": "pnpm build",
    "frontendDist": "../dist"
  },
  "package": {
    "productName": "planet-desktop",
    "version": "0.1.0"
  },
  "tauri": {
    "allowlist": {
      "all": false,
      "shell": {
        "all": false,
        "open": true
      },
      "fs": {
        "all": false,
        "readFile": true,
        "writeFile": true,
        "createDir": true,
        "removeDir": true,
        "scope": ["$APPDATA", "$DOCUMENT", "$RESOURCE"]
      }
    },
    "bundle": {
      "active": true,
      "targets": "all",
      "identifier": "xyz.planetable.planet-desktop",
      "icon": [
        "icons/32x32.png",
        "icons/128x128.png",
        "icons/128x128@2x.png",
        "icons/icon.icns",
        "icons/icon.ico"
      ],
      "resources": [
        "resources/bin/*"
      ]
    },
    "security": {
      "csp": null
    },
    "windows": [
      {
        "fullscreen": false,
        "resizable": true,
        "title": "Planet Desktop",
        "width": 1200,
        "height": 800
      }
    ],
    "systemTray": {
      "iconPath": "icons/icon.png",
      "iconAsTemplate": true
    }
  }
}
```

### 5.2 验证资源路径

在 Rust 代码中可以通过 `tauri::api::path::resource_dir()` 访问资源目录（后续在 `helpers/paths.rs` 中实现）

---

## 步骤 6：实现跨平台路径工具

### 6.1 编辑 `src-tauri/src/helpers/paths.rs`

```rust
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

/// 获取 Kubo 可执行文件路径（跨平台）
pub fn get_kubo_path(app: &AppHandle) -> PathBuf {
    let resource_dir = app
        .path()
        .resource_dir()
        .expect("Failed to get resource dir");

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
```

### 6.2 更新 `src-tauri/src/helpers/mod.rs`

```rust
pub mod paths;
```

### 6.3 验证编译

```bash
cd src-tauri
cargo check
```

---

## 步骤 7：配置基本窗口

### 7.1 编辑 `src-tauri/tauri.conf.json`

窗口配置已在步骤 5 中完成，确认以下配置：

```json
"windows": [
  {
    "fullscreen": false,
    "resizable": true,
    "title": "Planet Desktop",
    "width": 1200,
    "height": 800,
    "minWidth": 800,
    "minHeight": 600
  }
]
```

### 7.2 创建应用图标（可选，但推荐）

```bash
cd src-tauri
mkdir -p icons

# 准备图标文件：
# - 32x32.png
# - 128x128.png
# - 128x128@2x.png
# - icon.icns (macOS)
# - icon.ico (Windows)

# 可以使用在线工具生成：
# https://icon.kitchen/
# 或使用 ImageMagick 转换
```

---

## 步骤 8：配置系统托盘

### 8.1 更新 `src-tauri/Cargo.toml`

确保包含 `tray-icon` feature：

```toml
[dependencies]
tauri = { version = "2", features = ["tray-icon"] }
```

### 8.2 编辑 `src-tauri/src/main.rs`

```rust
use tauri::{Manager, SystemTray, SystemTrayEvent, SystemTrayMenu};

fn main() {
    // 创建系统托盘菜单
    let tray_menu = SystemTrayMenu::new();
    let system_tray = SystemTray::new().with_menu(tray_menu);

    tauri::Builder::default()
        .system_tray(system_tray)
        .on_system_tray_event(|app, event| {
            match event {
                SystemTrayEvent::LeftClick {
                    position: _,
                    size: _,
                    ..
                } => {
                    // 点击托盘图标显示/隐藏窗口
                    let window = app.get_window("main").unwrap();
                    if window.is_visible().unwrap() {
                        window.hide().unwrap();
                    } else {
                        window.show().unwrap();
                        window.set_focus().unwrap();
                    }
                }
                SystemTrayEvent::RightClick {
                    position: _,
                    size: _,
                    ..
                } => {
                    // 右键菜单（后续完善）
                }
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            // 暂时留空
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### 8.3 验证托盘图标

运行 `pnpm tauri dev`，应该能在系统托盘看到图标 ✅

---

## 步骤 9：实现前端三栏布局

### 9.1 创建组件目录

```bash
cd src
mkdir -p components/Sidebar components/ArticleList components/ArticleDetail
```

### 9.2 创建 `src/components/Sidebar/Sidebar.tsx`

```tsx
import React from 'react'

export function Sidebar() {
  return (
    <div className="w-64 bg-gray-100 border-r border-gray-300 h-full overflow-y-auto">
      <div className="p-4">
        <h2 className="text-lg font-semibold mb-4">My Planets</h2>
        <div className="space-y-2">
          {/* 占位内容 */}
          <div className="p-2 bg-white rounded cursor-pointer hover:bg-gray-50">
            <div className="font-medium">Planet Name</div>
            <div className="text-sm text-gray-500">Last updated: ...</div>
          </div>
        </div>
      </div>
      <div className="p-4 border-t border-gray-300">
        <h2 className="text-lg font-semibold mb-4">Following</h2>
        <div className="space-y-2">
          {/* 占位内容 */}
        </div>
      </div>
    </div>
  )
}
```

### 9.3 创建 `src/components/ArticleList/ArticleList.tsx`

```tsx
import React from 'react'

export function ArticleList() {
  return (
    <div className="w-80 bg-white border-r border-gray-300 h-full overflow-y-auto">
      <div className="p-4">
        <h2 className="text-lg font-semibold mb-4">Articles</h2>
        <div className="space-y-2">
          {/* 占位内容 */}
          <div className="p-3 border-b border-gray-200 cursor-pointer hover:bg-gray-50">
            <div className="font-medium">Article Title</div>
            <div className="text-sm text-gray-500 mt-1">2024-01-01</div>
          </div>
        </div>
      </div>
    </div>
  )
}
```

### 9.4 创建 `src/components/ArticleDetail/ArticleDetail.tsx`

```tsx
import React from 'react'

export function ArticleDetail() {
  return (
    <div className="flex-1 bg-white h-full overflow-y-auto">
      <div className="max-w-4xl mx-auto p-8">
        <h1 className="text-3xl font-bold mb-4">Article Title</h1>
        <div className="text-gray-500 mb-6">2024-01-01</div>
        <div className="prose max-w-none">
          {/* 占位内容 */}
          <p>Article content will be displayed here...</p>
        </div>
      </div>
    </div>
  )
}
```

### 9.5 更新 `src/App.tsx`

```tsx
import React from 'react'
import { Sidebar } from './components/Sidebar/Sidebar'
import { ArticleList } from './components/ArticleList/ArticleList'
import { ArticleDetail } from './components/ArticleDetail/ArticleDetail'

function App() {
  return (
    <div className="flex h-screen bg-gray-50">
      <Sidebar />
      <ArticleList />
      <ArticleDetail />
    </div>
  )
}

export default App
```

### 9.6 验证布局

运行 `pnpm tauri dev`，应该看到三栏布局 ✅

---

## 步骤 10：实现 Hello World Tauri Command

### 10.1 创建 `src-tauri/src/commands/app.rs`

```rust
use tauri::AppHandle;

#[tauri::command]
pub fn get_kubo_path(app: AppHandle) -> Result<String, String> {
    let kubo_path = crate::helpers::paths::get_kubo_path(&app);
    Ok(kubo_path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn hello_world(name: String) -> String {
    format!("Hello, {}! This is Planet Desktop.", name)
}
```

### 10.2 更新 `src-tauri/src/commands/mod.rs`

```rust
pub mod app;
pub mod ipfs;
pub mod planet;
pub mod article;
```

### 10.3 更新 `src-tauri/src/main.rs`

```rust
mod commands;
mod helpers;
mod ipfs;
mod keystore;
mod models;
mod store;
mod template;

use commands::app;

fn main() {
    // ... 系统托盘代码 ...

    tauri::Builder::default()
        .system_tray(system_tray)
        .on_system_tray_event(|app, event| {
            // ... 托盘事件处理 ...
        })
        .invoke_handler(tauri::generate_handler![
            app::get_kubo_path,
            app::hello_world,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### 10.4 前端调用测试

更新 `src/App.tsx`：

```tsx
import React, { useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { Sidebar } from './components/Sidebar/Sidebar'
import { ArticleList } from './components/ArticleList/ArticleList'
import { ArticleDetail } from './components/ArticleDetail/ArticleDetail'

function App() {
  const [kuboPath, setKuboPath] = useState<string>('')
  const [message, setMessage] = useState<string>('')

  const handleTestInvoke = async () => {
    try {
      // 测试 get_kubo_path
      const path = await invoke<string>('get_kubo_path')
      setKuboPath(path)

      // 测试 hello_world
      const msg = await invoke<string>('hello_world', { name: 'World' })
      setMessage(msg)
    } catch (error) {
      console.error('Invoke error:', error)
    }
  }

  return (
    <div className="flex h-screen bg-gray-50">
      <Sidebar />
      <ArticleList />
      <div className="flex-1 bg-white h-full overflow-y-auto">
        <div className="max-w-4xl mx-auto p-8">
          <h1 className="text-3xl font-bold mb-4">Planet Desktop</h1>
          <button
            onClick={handleTestInvoke}
            className="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700"
          >
            Test Tauri Invoke
          </button>
          {kuboPath && (
            <div className="mt-4 p-4 bg-gray-100 rounded">
              <p className="font-semibold">Kubo Path:</p>
              <p className="text-sm text-gray-600">{kuboPath}</p>
            </div>
          )}
          {message && (
            <div className="mt-4 p-4 bg-green-100 rounded">
              <p>{message}</p>
            </div>
          )}
        </div>
      </div>
    </div>
  )
}

export default App
```

### 10.5 验证 IPC 通信

运行 `pnpm tauri dev`，点击按钮，应该能看到 Kubo 路径和 Hello World 消息 ✅

---

## 步骤 11：配置 GitHub Actions CI

### 11.1 创建 `.github/workflows/build.yml`

```yaml
name: Build Tauri App

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]
  workflow_dispatch:

jobs:
  build:
    strategy:
      fail-fast: false
      matrix:
        include:
          - platform: 'windows-latest'
            args: '--target x86_64-pc-windows-msvc'
          - platform: 'macos-latest'
            args: '--target aarch64-apple-darwin'
          - platform: 'macos-latest'
            args: '--target x86_64-apple-darwin'

    runs-on: ${{ matrix.platform }}
    steps:
      - name: Checkout repository
        uses: actions/checkout@v4

      - name: Install dependencies (ubuntu only)
        if: matrix.platform == 'ubuntu-20.04'
        run: |
          sudo apt-get update
          sudo apt-get install -y libwebkit2gtk-4.1-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.args }}

      - name: Rust cache
        uses: swatinem/rust-cache@v2
        with:
          workspaces: './src-tauri -> target'

      - name: Sync node version and setup pnpm
        uses: pnpm/action-setup@v2
        with:
          version: 8

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: 18
          cache: 'pnpm'

      - name: Install frontend dependencies
        run: pnpm install

      - name: Build the app
        uses: tauri-apps/tauri-action@v0
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        with:
          projectPath: .
          args: ${{ matrix.args }}
          tagName: v__VERSION__
          releaseName: 'Planet Desktop v__VERSION__'
          releaseBody: 'See the assets to download this version and install.'
          releaseDraft: true
          prerelease: false
```

### 11.2 验证 CI

提交代码到 GitHub，查看 Actions 是否成功运行并生成安装包 ✅

---

## 最终验证清单

运行以下命令验证所有功能：

```bash
# 1. 项目能编译
cd src-tauri
cargo check

# 2. 前端能构建
cd ..
pnpm build

# 3. 开发模式能运行
pnpm tauri dev

# 4. 生产构建能打包（本地测试）
pnpm tauri build
```

### 验收标准检查

- [ ] `pnpm tauri dev` 启动应用，窗口正常显示
- [ ] 三栏布局（Sidebar / ArticleList / ArticleDetail）正常显示
- [ ] 点击测试按钮，能通过 `invoke` 调用 Rust 后端并返回 Kubo 路径
- [ ] 系统托盘图标可见
- [ ] GitHub Actions CI 能成功构建 Windows `.msi` 和 macOS `.dmg` 安装包

---

## 常见问题排查

### 问题 1：`cargo check` 失败

**可能原因：** Rust 工具链未正确安装  
**解决方案：**
```bash
rustup update stable
rustup component add rustfmt clippy
```

### 问题 2：TailwindCSS 样式不生效

**可能原因：** CSS 文件未正确引入  
**解决方案：** 确认 `src/main.tsx` 中引入了 `./index.css`

### 问题 3：Kubo 二进制找不到

**可能原因：** 资源路径配置错误  
**解决方案：** 检查 `tauri.conf.json` 中的 `bundle.resources` 配置，确保路径正确

### 问题 4：系统托盘图标不显示

**可能原因：** 图标文件缺失或路径错误  
**解决方案：** 检查 `tauri.conf.json` 中的 `systemTray.iconPath` 配置

### 问题 5：`invoke` 调用失败

**可能原因：** Command 未正确注册  
**解决方案：** 检查 `main.rs` 中的 `invoke_handler` 是否包含对应的 command

---

## 下一步

Phase 0 完成后，可以开始 Phase 1：实现 Kubo 管理核心功能。
