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
2. 🔒 **AI 安全文档 (Secure AI Notes)**
   - **本地优先**：纯本地存放 Markdown 文档与向量数据库，断网可用。
   - **高强加密**：内置 KDBX 级密码保险箱保存敏感密钥，主密码本地验证，绝不落盘。
   - **隐私脱敏**：AI 交互前自动对敏感字段进行提取和 `{{secret}}` 占位，避免隐私泄露给云端大模型。
   - **智能辅助**：无缝对接兼容 OpenAI 格式的大语言模型接口，支持智能检索问答。
3. 🔧 **网络代理增强工具 (Network Utility)**
   - 将 `vless://` 链接一键转换为 Clash Party / Mihomo YAML 格式，并提供本机节点即时测速及快速切换能力。

---

## 📸 应用预览 (Screenshots)

*(此处推荐上传应用界面截图或操作动图，增强展示效果)*
- `[占位：主界面概览截图]`
- `[占位：AI 技能向导操作演示]`
- `[占位：安全文档检索与密码库展示]`

---

## 📦 下载与安装 (Installation)

### 普通用户下载 (推荐)
前往项目的 [Releases](../../releases) 页面下载最新编译版本：
- **macOS (Apple Silicon)**: 下载 `RustTool_*_aarch64.dmg` 文件，双击打开并将 `RustTool.app` 拖入应用程序文件夹即可。
- **macOS (Intel)** / **Windows**: *(请根据系统类型下载对应的压缩包或安装程序)*

### 初始配置指南
- 首次下载安装后，系统默认将资料库安全地存储在您的系统应用数据目录中，普通用户**无需任何配置**即可直接开箱使用。
- 若需更改文档存储路径或绑定 `99_codex` 规范仓库源，可在应用的**设置页面**或对应模块面板中进行路径指派与迁移。

---

## 🛠 开发者指南 (Developer Guide)

本章节面向希望参与二次开发、自行编译代码的开发者。

### 技术选型 (Tech Stack)
- **核心框架**: Tauri v2 (Desktop), Axum + Tokio (Rust Server), Clap (CLI)
- **前端技术**: Vue 3 + Vite + TypeScript, Vue Router + Pinia, Tailwind CSS, Vitest

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

### 2. 深入：AI 安全文档机制
支持通过系统环境变量进行深度自定义 (极客玩法)：
```sh
export RUSTTOOL_DATA_DIR="$HOME/RustToolData" # 显式指定文档与保险箱的存放根目录
export RUSTTOOL_CLASH_PARTY_API_URL="http://127.0.0.1:9998" # 指定底层代理测速接口
```

**严格的安全行为准则：**
- **密码库机制**：任何敏感操作必须先调用 `unlock` 解锁保密库。主密码哈希校验全部在内存中进行，**明文密码绝对不落盘**。应用进入锁定状态后会立即清零释放内存中的 KDBX 对象与主密钥。
- **AI 脱敏保护**：文档正文的密码、Token 等敏感内容会被提取并替换为 `{{secret:key}}`，真实数值保存在 KDBX 保险箱内。当与大语言模型交互问答时，默认拒绝向云端发送明文密钥。

---

## 🤝 参与贡献 (Contributing)

1. 请在动手开发前，完整阅读本根目录下的 `AGENTS.md`，了解开发约定和协作规范。
2. 提交代码变更前，请务必在本地终端运行 `./rt test`，确保所有单元测试 100% 绿灯通过。
3. AI 安全文档功能**严禁**在桌面版和 Web 版之间拆成两套业务逻辑，必须严格遵从一致性适配规范。

## 📄 开源协议 (License)
本项目代码遵循 [MIT License](LICENSE) 开源协议。
