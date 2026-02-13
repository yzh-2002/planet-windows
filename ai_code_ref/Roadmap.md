# Planet Desktop (Tauri) 实施路线图

> 技术栈：Tauri 2.x + Rust + React + TypeScript + TailwindCSS
>
> 目标平台：Windows (主) + macOS (兼容)
>
> 预估总工期：15 周

---

## Phase 0：项目骨架搭建（第 1 周）

### 目标

搭建完整的 Tauri 项目结构，集成 Kubo 二进制，应用能启动并展示基本窗口。

### 任务清单

- [ ] 使用 `npm create tauri-app@latest` 初始化项目（React + TypeScript 模板）
- [ ] 配置 TailwindCSS
- [ ] 搭建 `src-tauri/src/` 目录结构（commands/ ipfs/ models/ template/ keystore/ helpers/ store/）
- [ ] 下载 Kubo 二进制文件（Windows amd64 / macOS arm64+amd64），放入 `src-tauri/resources/bin/`
- [ ] 在 `tauri.conf.json` 中配置 `bundle.resources` 嵌入 Kubo 二进制
- [ ] 实现 `helpers/paths.rs`：跨平台路径工具（`get_kubo_path`、`get_ipfs_repo_path`、`get_data_path`）
- [ ] 配置基本窗口（标题、尺寸、图标）
- [ ] 配置系统托盘（Tray Icon）骨架
- [ ] 前端实现空白三栏布局（Sidebar / ArticleList / ArticleDetail）
- [ ] 注册一个 hello world 级别的 `#[tauri::command]` 并在前端成功 `invoke` 调用
- [ ] 配置 GitHub Actions CI（`tauri-action` 自动构建 Windows + macOS）

### 验收标准

> **`npm run tauri dev` 启动应用，窗口正常显示三栏布局，前端点击按钮能通过 `invoke` 调用 Rust 后端并返回结果（如返回 Kubo 二进制路径），系统托盘图标可见。CI 能产出 Windows `.msi` 和 macOS `.dmg` 安装包。**

---

## Phase 1：Kubo 管理核心（第 2-3 周）

### 目标

实现 IPFS 守护进程的完整生命周期管理，对应原项目 `IPFSCommand.swift` + `IPFSDaemon.swift` + `IPFSState.swift`。

### 任务清单

- [ ] 实现 `ipfs/command.rs`（KuboCommand）
  - [ ] `run()` 同步执行 CLI 命令
  - [ ] `run_async()` 异步执行（用于 daemon）
  - [ ] `init_repo()` 初始化 IPFS 仓库
  - [ ] `update_swarm_port()` / `update_api_port()` / `update_gateway_port()`
  - [ ] `set_peers()` / `set_resolvers()` / `set_swarm_conn_mgr()`
  - [ ] `set_access_control_allow_origin()` / `set_access_control_allow_methods()`
  - [ ] `launch_daemon()` / `shutdown_daemon()`
  - [ ] `add_directory()` / `get_file_cid()`
  - [ ] `generate_key()` / `delete_key()` / `list_keys()` / `export_key()` / `import_key()`
- [ ] 实现 `ipfs/daemon.rs`（IpfsDaemon）
  - [ ] `setup()` 方法：检查 repo → init → 端口扫描 → 配置更新
  - [ ] `launch()` 方法：启动 daemon 子进程，监听 stdout 等待 "Daemon is ready"
  - [ ] `shutdown()` 方法：优雅关闭 daemon
  - [ ] `api()` 方法：通过 HTTP POST 调用 IPFS API (`http://127.0.0.1:{api_port}/api/v0/...`)
  - [ ] `scout_port()` 端口扫描工具函数
  - [ ] 内嵌 peers 和 DNS resolvers 配置（对应 Swift 中的 `IPFSDaemon.peers` 和 `IPFSDaemon.resolvers`）
- [ ] 实现 `ipfs/state.rs`（IpfsState）
  - [ ] 维护 daemon 状态（online、apiPort、gatewayPort、swarmPort、repoSize）
  - [ ] 通过 `app.emit("ipfs:state-changed", &state)` 推送状态变化到前端
- [ ] 实现 `ipfs/models.rs`
  - [ ] 定义 API 响应结构体：`IpfsVersion`、`IpfsId`、`IpfsPeers`、`IpfsBandwidth`、`IpfsPublished`、`IpfsResolved`、`IpfsPinned`、`IpfsRepoState`
- [ ] 注册 Tauri Commands
  - [ ] `ipfs_setup` / `ipfs_launch` / `ipfs_shutdown`
  - [ ] `ipfs_get_state` / `ipfs_gc`
- [ ] 前端实现 `useIPFS` hook + IPFS 状态面板（显示在线状态、端口、peer 数量）
- [ ] 实现应用启动时自动 setup + launch daemon
- [ ] 实现应用退出时优雅 shutdown daemon

### 验收标准

> **应用启动后自动初始化并启动 IPFS daemon，前端 IPFS 状态面板显示 `Online`，端口号、peer 数量均正常。手动点击 Shutdown 后状态变为 Offline，再点击 Launch 可恢复。应用关闭时 daemon 自动停止。在 Windows 和 macOS 上均能正常运行。**

---

## Phase 2：数据模型与持久化（第 4-5 周）

### 目标

实现 Planet、Article、Draft 的数据模型和 JSON 文件持久化，对应原项目 `MyPlanetModel.swift`、`FollowingPlanetModel.swift`、`MyArticleModel.swift`、`DraftModel.swift`。

### 任务清单

- [ ] 实现 `models/planet.rs`
  - [ ] `MyPlanet` 结构体（id, name, about, domain, ipns, created, updated, templateName, lastPublished, lastPublishedCID, 各种 enabled flags ...）
  - [ ] `FollowingPlanet` 结构体（id, name, about, created, planetType, link, cid, updated, lastRetrieved ...）
  - [ ] `PlanetType` 枚举（Planet / ENS / DNSLink / DNS / DotBit）
  - [ ] `PublicPlanet` 结构体（用于导出给模板的 JSON）
  - [ ] JSON 序列化/反序列化（serde）
  - [ ] `save()` / `load()` 方法（读写 `{data_path}/My/{id}/planet.json`）
  - [ ] `create()` / `delete()` 方法
- [ ] 实现 `models/article.rs`
  - [ ] `MyArticle` 结构体（id, title, content, created, updated, attachments, tags, slug ...）
  - [ ] `FollowingArticle` 结构体
  - [ ] JSON 持久化
  - [ ] 附件管理（文件复制到文章目录）
- [ ] 实现 `models/draft.rs`
  - [ ] `Draft` 结构体
  - [ ] 草稿的创建、保存、删除
  - [ ] 草稿转文章
- [ ] 实现 `store/mod.rs`（PlanetStore 全局状态）
  - [ ] 维护 `my_planets: Vec<MyPlanet>` 和 `following_planets: Vec<FollowingPlanet>`
  - [ ] 应用启动时从磁盘加载所有 Planet
  - [ ] 提供增删改查接口
- [ ] 注册 Tauri Commands
  - [ ] `planet_list` / `planet_create` / `planet_delete` / `planet_update`
  - [ ] `article_list` / `article_create` / `article_delete` / `article_update`
  - [ ] `draft_list` / `draft_save` / `draft_delete` / `draft_publish`
- [ ] 前端实现
  - [ ] Sidebar 展示 My Planets 和 Following Planets 列表
  - [ ] 点击 Planet 后在 ArticleList 展示文章列表
  - [ ] 点击 Article 后在 ArticleDetail 展示文章内容
  - [ ] "New Planet" 对话框
  - [ ] "New Article" 对话框

### 验收标准

> **可以创建 Planet，创建文章（含标题、Markdown 内容），文章列表正常展示，关闭应用重新打开后数据不丢失。能删除 Planet 和文章。数据存储在 `%APPDATA%/planet-desktop/Planet/` (Windows) 或 `~/Library/Application Support/planet-desktop/Planet/` (macOS)。**

---

## Phase 3：发布流程（第 6-7 周）

### 目标

实现内容发布到 IPFS/IPNS 的完整流程，对应原项目 `MyPlanetModel.publish()`。

### 任务清单

- [ ] 实现 IPNS 密钥管理
  - [ ] 创建 Planet 时自动生成 IPFS key (`key gen {planet_id}`)
  - [ ] 删除 Planet 时移除 key (`key rm {planet_id}`)
  - [ ] 检查 key 是否存在 (`key list`)
- [ ] 实现 `keystore/mod.rs`
  - [ ] 使用 `keyring-rs` 实现跨平台密钥存储
  - [ ] `save()` / `load()` / `delete()` / `check()`
  - [ ] IPFS key 导出到 Keystore (`key export` → 加密存储)
  - [ ] 从 Keystore 导入到 IPFS (`key import`)
- [ ] 实现发布流程
  - [ ] `add_directory` 上传整个 public 目录到 IPFS → 获得 CID
  - [ ] `name/publish` 将 CID 发布到 IPNS
  - [ ] 更新 `lastPublished` 和 `lastPublishedCID`
  - [ ] 发布状态管理（isPublishing、publishStartedAt）
- [ ] 实现 Filebase Pinning 集成（可选）
  - [ ] Filebase API 调用：提交 CID 进行远程 pinning
  - [ ] 保存 pinning 状态
- [ ] 实现 Pinneable 集成（可选）
- [ ] 前端实现
  - [ ] Planet 详情页显示发布状态（上次发布时间、CID）
  - [ ] "Publish" 按钮，点击触发发布
  - [ ] 发布进度指示器
  - [ ] Filebase / Pinnable 设置面板

### 验收标准

> **创建 Planet → 写文章 → 点击 Publish，能看到发布进度，发布完成后显示 CID。通过 `http://127.0.0.1:{gateway_port}/ipns/{ipns_key}` 可以访问到发布的站点。重启应用后 IPNS key 仍然存在且可用。**

---

## Phase 4：关注与内容获取（第 8-9 周）

### 目标

实现关注其他 Planet 并获取内容的功能，对应原项目 `FollowingPlanetModel.swift` 的更新逻辑。

### 任务清单

- [ ] 实现 IPNS / DNSLink 解析
  - [ ] `name/resolve` 调用：IPNS name → CID
  - [ ] DNSLink 解析
- [ ] 实现 ENS 解析（`helpers/ens.rs`）
  - [ ] 使用 `alloy` (ethers-rs) 连接 Ethereum RPC
  - [ ] 解析 ENS contenthash → IPNS/CID
  - [ ] 支持 `.eth` / `.bit` 域名
- [ ] 实现 .bit 域名解析（`helpers/dotbit.rs`）
  - [ ] 调用 .bit indexer API
- [ ] 实现内容获取流程
  - [ ] 通过 IPFS Gateway 获取 `planet.json`
  - [ ] 解析 planet.json 获取文章列表
  - [ ] 下载文章内容和附件
  - [ ] Pin 获取的内容
- [ ] 实现 RSS 生成（`helpers/feed.rs`）
  - [ ] 使用 `feed-rs` 解析外部 RSS/Atom 源
  - [ ] 为 MyPlanet 生成 RSS XML
- [ ] 实现 Aggregation（聚合外部 RSS 源）
- [ ] 实现 Pin / Unpin 管理
- [ ] 实现定时更新
  - [ ] 后台定时检查 Following Planets 的更新
- [ ] 注册 Tauri Commands
  - [ ] `planet_follow` / `planet_unfollow`
  - [ ] `planet_update_following` / `planet_update_all_following`
- [ ] 前端实现
  - [ ] "Follow Planet" 对话框（支持输入 IPNS / ENS / DNSLink / RSS）
  - [ ] Following Planet 列表展示
  - [ ] 文章内容展示
  - [ ] "Update" 按钮，手动触发更新

### 验收标准

> **输入一个已知的 IPNS 地址（如 `k51...`）或 ENS 域名（如 `vitalik.eth`），能成功关注并拉取到 planet.json 和文章列表。文章内容可在 ArticleDetail 中正常展示。手动点击 Update 能检查并拉取新内容。关闭重开后 Following 列表和已拉取的文章不丢失。**

---

## Phase 5：模板引擎（第 10 周）

### 目标

实现站点模板渲染系统，使 MyPlanet 发布时能生成完整的静态站点。

### 任务清单

- [ ] 实现 `template/engine.rs`
  - [ ] 使用 `Tera` 作为模板引擎（语法与 Stencil/Jinja2 接近）
  - [ ] 加载模板文件（`blog.html`、`index.html`、`tags.html`、`archive.html`）
  - [ ] 渲染 index.html（支持分页）
  - [ ] 渲染单篇文章页
  - [ ] 渲染 RSS XML
  - [ ] 渲染标签页
  - [ ] 渲染归档页
  - [ ] 渲染 robots.txt
- [ ] 模板管理
  - [ ] 定义 `Template` 结构体（name, description, author, version, settings ...）
  - [ ] 加载 `template.json` 元数据
  - [ ] 复制模板静态资源（assets/）到 public 目录
  - [ ] 支持模板设置（template_settings / user_settings）
- [ ] 迁移内置模板
  - [ ] 将原项目 `PlanetSiteTemplates` 中的内置模板转换为 Tera 语法
  - [ ] 注意：Stencil 的 `{% for %}` / `{% if %}` 与 Tera 基本兼容，少数过滤器需要适配
- [ ] Markdown 渲染
  - [ ] 使用 `pulldown-cmark` 渲染 Markdown → HTML
  - [ ] 支持 GFM 扩展（表格、任务列表、删除线等）
- [ ] 实现 `savePublic()` 方法
  - [ ] 生成完整的 public 目录结构
  - [ ] 渲染所有页面
  - [ ] 复制附件和静态资源
- [ ] 前端实现
  - [ ] 模板选择器
  - [ ] 模板预览（可选）

### 验收标准

> **创建 Planet → 选择模板 → 写文章 → Publish。生成的 public 目录包含完整的静态站点（index.html、文章页、RSS、assets）。通过 IPFS Gateway 访问站点，页面样式正确、文章内容完整、分页/标签/归档功能正常。**

---

## Phase 6：前端 UI 完善（第 11-13 周）

### 目标

实现完整的用户界面，达到与原 macOS 版本功能对等的使用体验。

### 任务清单

- [ ] **Markdown 编辑器**
  - [ ] 集成 CodeMirror 6 或 Milkdown 作为 Markdown 编辑器
  - [ ] 支持实时预览（双栏或切换模式）
  - [ ] 支持图片/文件拖拽上传（自动保存到文章附件目录）
  - [ ] 支持快捷键（加粗、斜体、链接、代码块等）
  - [ ] 支持粘贴图片
- [ ] **文章详情页**
  - [ ] Markdown 渲染展示（使用 `markdown-it` 或后端 `pulldown-cmark`）
  - [ ] 图片/音视频正确加载（通过 Tauri asset protocol）
  - [ ] 文章元信息展示（日期、标签、附件列表）
- [ ] **Sidebar 完善**
  - [ ] Planet 头像展示
  - [ ] 未读文章计数
  - [ ] 右键菜单（Publish / Settings / Delete ...）
  - [ ] 拖拽排序（可选）
- [ ] **设置页面**
  - [ ] 通用设置（数据目录、语言、开机启动）
  - [ ] IPFS 设置（Kubo 连接管理、公共网关选择）
  - [ ] Planet 设置（域名、社交链接、Analytics、Filebase/Pinnable 配置）
  - [ ] 模板设置（当前模板自定义参数）
- [ ] **IPFS 状态面板**
  - [ ] Daemon 状态（Online/Offline）
  - [ ] Peer ID、版本号
  - [ ] 连接的 Peer 数量
  - [ ] Repo 大小
  - [ ] 带宽统计图表
  - [ ] GC 按钮
- [ ] **Key Manager**
  - [ ] 列出所有 IPFS keys
  - [ ] 导入/导出 key 文件
  - [ ] 从 Keychain/Credential Manager 恢复
- [ ] **Published Folders**
  - [ ] 选择本地目录发布到 IPFS/IPNS
  - [ ] 管理已发布的目录
- [ ] **搜索**
  - [ ] 全局搜索文章（标题、内容）
- [ ] **深色模式**
  - [ ] TailwindCSS dark mode 支持
  - [ ] 跟随系统主题

### 验收标准

> **整体 UI 流畅，三栏布局响应式调整。能创建 Planet、编写 Markdown 文章（含拖拽图片）、预览、发布。能关注其他 Planet 并浏览文章。IPFS 状态面板数据实时更新。设置项可持久化保存。深色模式正常工作。Key Manager 能导入导出密钥。**

---

## Phase 7：系统集成（第 14 周）

### 目标

完善桌面应用体验，集成系统级功能。

### 任务清单

- [ ] **系统托盘**
  - [ ] 托盘图标 + 右键菜单（Show/Hide、IPFS 状态、Publish All、Quit）
  - [ ] 关闭窗口时最小化到托盘（可配置）
  - [ ] 托盘图标反映 IPFS 在线状态
- [ ] **系统通知**
  - [ ] 使用 `tauri-plugin-notification`
  - [ ] 发布完成通知
  - [ ] Following Planet 有新文章时通知
  - [ ] GC 完成通知
- [ ] **开机自启**
  - [ ] 使用 `tauri-plugin-autostart`
  - [ ] 设置中可开关
- [ ] **自动更新**
  - [ ] 使用 `tauri-plugin-updater`
  - [ ] 配置更新服务器（GitHub Releases）
  - [ ] 静默检查更新 + 用户确认安装
- [ ] **自定义协议**
  - [ ] 注册 `planet://` URL scheme
  - [ ] 点击 `planet://follow/{ipns}` 自动关注
- [ ] **应用菜单栏**
  - [ ] File / Edit / View / Planet / Help 菜单
  - [ ] 键盘快捷键绑定
- [ ] **窗口状态持久化**
  - [ ] 记忆窗口大小和位置
  - [ ] 记忆 Sidebar 宽度

### 验收标准

> **应用最小化到托盘后可通过托盘图标恢复。发布完成时收到系统通知。开机自启可正常工作。通过 `planet://` 链接能唤起应用。菜单栏快捷键响应正确。窗口关闭再打开后保持上次的大小和位置。**

---

## Phase 8：打包、测试与分发（第 15 周）

### 目标

完成应用的最终打包、测试和分发流程。

### 任务清单

- [ ] **Windows 打包**
  - [ ] 配置 NSIS 安装包（`tauri.conf.json` → `bundle.windows.nsis`）
  - [ ] 配置 MSI 安装包（可选）
  - [ ] 安装包图标、许可协议、安装路径
  - [ ] 确保 Kubo 二进制正确打包到安装目录
  - [ ] 确保 WebView2 运行时自动安装（离线 bootstrapper）
- [ ] **macOS 打包**
  - [ ] 配置 DMG 安装包
  - [ ] 代码签名（Apple Developer ID）
  - [ ] 公证（Notarization）
  - [ ] Universal Binary（arm64 + x86_64）
- [ ] **CI/CD 完善**
  - [ ] GitHub Actions：push to main 自动构建
  - [ ] 自动上传 Release artifacts
  - [ ] 版本号管理
- [ ] **测试**
  - [ ] Rust 后端单元测试（Kubo 命令执行、模型序列化、端口扫描）
  - [ ] Rust 集成测试（daemon 生命周期、发布流程）
  - [ ] 前端组件测试（React Testing Library）
  - [ ] E2E 测试（可选，使用 Playwright 或 WebDriver）
- [ ] **性能优化**
  - [ ] 大量文章时的列表虚拟滚动
  - [ ] 图片懒加载
  - [ ] Daemon 启动耗时优化
- [ ] **错误处理与日志**
  - [ ] Rust 端统一错误类型 (`thiserror`)
  - [ ] 日志写入文件 (`tracing` + `tracing-appender`)
  - [ ] 前端错误边界（Error Boundary）
  - [ ] 用户友好的错误提示

### 验收标准

> **Windows NSIS 安装包可正常安装、运行、卸载。macOS DMG 安装包可正常挂载、拖拽安装、通过 Gatekeeper 检查。CI 自动构建并产出双平台安装包。核心功能全部可用：创建 Planet → 写文章 → 发布 → 通过 IPFS 访问；关注 Planet → 拉取内容 → 浏览文章。无崩溃级 bug。**

---

## 里程碑总览

```
Week  1     2     3     4     5     6     7     8     9     10    11    12    13    14    15
      ├─────┼─────┼─────┼─────┼─────┼─────┼─────┼─────┼─────┼─────┼─────┼─────┼─────┼─────┤
      │ P0  │    P1     │    P2     │    P3     │    P4     │ P5  │        P6        │ P7  │ P8  │
      │骨架  │  Kubo核心  │  数据模型  │  发布流程  │ 关注&获取  │模板  │     前端UI完善     │集成  │打包  │
      ├─────┼───────────┼───────────┼───────────┼───────────┼─────┼──────────────────┼─────┼─────┤
      ▲     ▲           ▲           ▲           ▲           ▲     ▲                  ▲     ▲
      M0    M1          M2          M3          M4          M5    M6                 M7    M8
```

| 里程碑 | 关键交付物 |
|--------|-----------|
| **M0** | 项目能运行，invoke 通信OK |
| **M1** | IPFS daemon 可启动/停止/监控 |
| **M2** | 可创建 Planet 和文章，数据持久化 |
| **M3** | 可发布到 IPFS/IPNS，通过网关可访问 |
| **M4** | 可关注其他 Planet 并浏览内容 |
| **M5** | 模板渲染生成完整静态站点 |
| **M6** | 完整的编辑器和UI交互 |
| **M7** | 系统托盘/通知/自启/自更新 |
| **M8** | 双平台安装包，可分发 |

---

## 技术风险与应对

| 风险 | 影响 | 应对策略 |
|------|------|----------|
| Rust 学习曲线 | 开发效率降低 | Phase 0 前预留 1-2 周 Rust 基础学习 |
| Tera 模板语法与 Stencil 不完全兼容 | 模板迁移工作量增大 | 提前梳理差异，编写自定义 Tera filter/function |
| Windows WebView2 兼容性 | 部分 Windows 10 用户缺少 WebView2 | NSIS 安装包内嵌 WebView2 bootstrapper |
| Kubo 二进制体积（~30MB） | 安装包偏大 | 可选下载（首次启动时下载）或 UPX 压缩 |
| IPFS daemon 进程管理 | 僵尸进程、异常退出 | 应用退出时强制 kill，启动时检查残留进程 |
| ethers-rs / alloy 版本迭代快 | API 变动 | 锁定版本，定期跟进 |

---

## 附录：核心 Cargo.toml 依赖

```toml
[dependencies]
tauri = { version = "2", features = ["tray-icon", "protocol-asset"] }
tauri-plugin-store = "2"
tauri-plugin-notification = "2"
tauri-plugin-clipboard-manager = "2"
tauri-plugin-dialog = "2"
tauri-plugin-fs = "2"
tauri-plugin-shell = "2"
tauri-plugin-autostart = "2"
tauri-plugin-updater = "2"
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["json"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tera = "1"
keyring = "3"
feed-rs = "2"
scraper = "0.20"
alloy = { version = "0.8", features = ["providers", "contract"] }
rusqlite = { version = "0.32", features = ["bundled"] }
pulldown-cmark = "0.12"
image = "0.25"
tracing = "0.1"
tracing-subscriber = "0.3"
tracing-appender = "0.2"
anyhow = "1"
thiserror = "2"
base64 = "0.22"
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
```
