<h1 align="center">RustTool</h1>

<p align="center">
  <strong>基于 Rust + Tauri 的跨平台高效桌面工作站与 AI 工具合集</strong>
</p>

<p align="center">
  <img alt="Tauri" src="https://img.shields.io/badge/Tauri-v2-blue?logo=tauri">
  <img alt="Rust" src="https://img.shields.io/badge/Rust-Backend-orange?logo=rust">
  <img alt="Vue" src="https://img.shields.io/badge/Vue.js-3.x-4fc08d?logo=vuedotjs">
  <img alt="Vite" src="https://img.shields.io/badge/Vite-Frontend-646cff?logo=vite">
  <img alt="License" src="https://img.shields.io/badge/License-MIT-green">
</p>

---

## ✨ 核心特性 (Features)

1. 🤖 **AI 技能安装向导 (Codex Installer)**
   - 支持动态扫描本地团队规范仓库，一键将底层的 Antigravity 核心引擎与架构规范（如 B2B、SCM 业务架构）注入到本地业务项目中，极速完成研发环境初始化。
2. 🔧 **网络代理增强工具 (Network Utility)**
   - 将 `vless://` 链接一键转换为 Clash Party / Mihomo YAML 格式，并提供本机节点即时测速及快速切换能力。

---

## 📸 应用预览 (Screenshots)

*(此处推荐上传应用界面截图或操作动图，增强展示效果)*
- `[占位：主界面概览截图]`
- `[占位：AI 技能向导操作演示]`

---

## 📦 下载与安装 (Installation)

### 普通用户下载 (推荐)
前往项目的 [Releases](../../releases) 页面下载最新编译版本：
- **macOS (Apple Silicon)**: 下载 `RustTool_*_aarch64.dmg` 文件，双击打开并将 `RustTool.app` 拖入应用程序文件夹即可。
- **macOS (Intel)** / **Windows**: *(请根据系统类型下载对应的压缩包或安装程序)*

> [!WARNING]
> **macOS 提示“文件已损坏”的解决方法**：
> 如果通过浏览器下载或隔空投送接收后，打开时提示**“文件已损坏，无法打开”**（这是 macOS 的安全隔离机制），请打开终端 (Terminal) 执行以下命令清除隔离属性：
> ```bash
> sudo xattr -cr /Applications/RustTool.app
> ```

### 初始配置指南
- 首次下载安装后，普通用户**无需任何配置**即可直接开箱使用。
- 若需绑定 `99_codex` 规范仓库源，可在应用的**设置页面**或对应模块面板中进行路径指派。

---

## 🛠 开发者指南 (Developer Guide)

本章节面向希望参与二次开发、自行编译代码的开发者。

### 技术选型 (Tech Stack)
- **核心框架**: Tauri v2 (Desktop), Axum + Tokio (Rust Server), Clap (CLI)
- **前端技术**: Vue 3 + Vite + TypeScript, Vue Router + Pinia, Ant Design Vue, Vitest
*(注：本项目的前端 UI 框架已全面迁移并只保留了 Ant Design Vue 模式，不再使用额外的自定义 CSS 库或 Tailwind 默认组件)*

### 目录结构 (Structure)
```text
crates/rust_tool_core   # 核心业务逻辑实现 (Core business logic)
crates/rust_tool_server # Axum API 服务端与前端静态托管
crates/rust_tool_cli    # 纯命令行入口 (Command-line wrapper)
frontend                # Vue 3 独立前端应用
frontend/src-tauri      # Tauri 桌面端宿主壳 (rust_tool_desktop)
```

### 本地开发与构建 (Setup & Build)

项目根目录提供了跨平台的 `./rt` 快捷控制脚本，大幅简化了全栈协同开发的复杂度：

```sh
./rt                # 打开带有数字选项的交互菜单
./rt install        # 一键安装前端 NPM 依赖与 Rust 依赖
./rt dev            # 联合启动：Rust 后端(5172端口) 和 Vue 前端开发热更服务器(5173端口)
./rt desktop        # 启动 Tauri 桌面开发版，便于调试原生能力
./rt build-desktop  # 编译打包 Tauri 桌面级 Release 最终应用 (生成 dmg / exe)
./rt test           # 运行全栈测试 (Rust 单元测试 + Vitest)
./rt clean          # 清理全部构建产物，释放空间
```

*(如果前端开发服务需要代理到自定义的新端口，启动前在环境中设置 `RUSTTOOL_SERVER_PORT` 即可。)*

构建发布前，强烈推荐统一执行校验：
```sh
./rt test
./rt build
```

---

## 🏗 架构设计与机制 (Architecture & Concepts)

### 1. 桌面与 Web 架构一致性
RustTool 的桌面版和 Web 版共用**同一套 Vue 页面**和 `rust_tool_core` 业务核心。
- **Web 模式**：通过 HTTP API (Axum) 发起请求调用 `rust_tool_server`。
- **桌面模式**：通过 Tauri command 发起 IPC 通信直接调用 Rust 核心，不依赖本地网卡端口，更加高效安全。
- **规范要求**：在二次开发新增接口时，需要同步维护 HTTP 路由、Tauri command 和前端的 `Api` 适配映射层。长期约定请参考根目录的 `AGENTS.md`。

### 2. 深入：高级配置
支持通过系统环境变量进行深度自定义 (极客玩法)：
```sh
export RUSTTOOL_CLASH_PARTY_API_URL="http://127.0.0.1:9998" # 指定底层代理测速接口
```

---

## 🤝 参与贡献 (Contributing)

1. 请在动手开发前，完整阅读本根目录下的 `AGENTS.md`，了解开发约定和协作规范。
2. 提交代码变更前，请务必在本地终端运行 `./rt test`，确保所有单元测试 100% 绿灯通过。

## 📄 开源协议 (License)
本项目代码遵循 [MIT License](LICENSE) 开源协议。
