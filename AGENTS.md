# RustTool Agent Guidelines

本文件是后续 Codex/Agent 在本项目中继续开发时的长期规范。实施方案、阶段记录和临时分析文档可以放在 `.agents/`；稳定规则应沉淀到本文件。

## 项目定位

RustTool 是一个本地优先的工具站，当前主线是桌面优先的 AI 安全文档与密码库能力。

- 主要使用环境是 macOS。
- 桌面版和 Web 版必须共享同一套 Vue 页面。
- 业务核心必须沉淀在 `crates/rust_tool_core`。
- Web 模式通过 HTTP API 调用 `rust_tool_server`。
- 桌面模式通过 Tauri command 直接调用 Rust 核心，不依赖本地后端端口。

## 开发入口

macOS/Unix 优先使用项目根目录的 `./rt`：

- `./rt` 显示中文交互菜单。
- `./rt dev` 启动 Web 开发服务。
- `./rt desktop` 启动 Tauri 桌面开发版。
- `./rt test` 运行 Rust 与前端测试。
- `./rt build-desktop` 构建桌面应用。

不要新增只能在 Windows 上工作的主流程；Windows 只能作为兼容能力保留。

## 桌面与 Web 一致性

新增或修改 AI 安全文档/密码库功能时，必须保持桌面与 Web 行为一致。

- 页面入口：`frontend/src/pages/AiMemo.vue`、`frontend/src/pages/SecretVault.vue`
- 前端适配层：`frontend/src/services/memoApi.ts`
- HTTP 路由：`crates/rust_tool_server/src/routes/memo.rs`
- Tauri command：`frontend/src-tauri/src/memo_commands.rs`
- Tauri 注册入口：`frontend/src-tauri/src/lib.rs`

规则：

1. 前端页面不得直接散落 `fetch('/api/memo/...')` 或硬编码 `/api/memo`。
2. AI 安全文档/密码库页面必须通过 `memoRequest(...)` 调用后端能力。
3. 新增接口时必须同步更新 HTTP 路由、Tauri command 和 `memoApi.ts` 映射。
4. Web 与桌面的错误响应应保持页面可统一读取：`{ error: { code, message } }`。

## 敏感输入组件

所有密码、API Key、Token、WebDAV 密码、主密码等敏感输入框，必须使用：

```text
frontend/src/components/SecurePasswordInput.vue
```

不要在页面里手写：

```html
<input type="password" />
```

使用规则：

1. 主密码文案使用“显示主密码 / 隐藏主密码”。
2. API Key 文案使用“显示 API Key / 隐藏 API Key”。
3. WebDAV 密码文案使用“显示 WebDAV 密码 / 隐藏 WebDAV 密码”。
4. 输入框默认必须隐藏，只有用户点击眼睛按钮时临时显示。
5. 不要把密码可见状态散落成页面级 `showXxxPassword`，交给 `SecurePasswordInput` 内部管理。

## 安全规则

1. 明文主密码不得落盘、不得写入日志、不得进入错误消息。
2. 密码库列表接口只能返回 secret 索引，不得返回明文值。
3. secret 明文只能在单项 reveal/copy 时临时解密。
4. 默认不得把密码、Token、API Key、私钥等明文发送给大模型。
5. 如果用户显式开启“允许 AI 检索并读取解密后的密码”，UI 和代码路径必须清楚体现这是高风险行为。
6. 修改主密码必须先创建本地备份，成功后必须立即锁定保险箱。

## 资料库目录

AI 安全文档资料库默认目录策略：

- macOS: `~/Library/Application Support/rust-tool`
- Windows: `%APPDATA%/rust-tool`
- Linux/Unix: `$XDG_DATA_HOME/rust-tool` 或 `~/.local/share/rust-tool`

Tauri 自身应用配置目录可以继续存放桌面壳设置；AI 安全文档资料库必须使用上面的 `rust-tool` 目录策略，以保证桌面与 Web 的资料库一致。

## 验证要求

改动完成后按风险选择验证：

- 前端 UI/组件改动：至少运行 `pnpm run build`。
- Rust 核心或 API 改动：至少运行相关 `cargo test`。
- 跨前端、后端、Tauri 的共享改动：优先运行 `./rt test`。
- 明显影响用户界面的改动：使用浏览器或桌面窗口做一次真实交互验证。

验证真实数据相关功能时，优先使用临时资料目录，例如：

```sh
RUSTTOOL_DATA_DIR=/private/tmp/rusttool-verify ./rt server
```

不要在未说明的情况下操作用户真实资料库。

## 文档分工

- `AGENTS.md`：长期规则，后续 Agent 应优先阅读。
- `.agents/*.md`：实施方案、阶段记录、设计草案、临时决策缓存。
- `README.md`：面向使用者和开发者的项目说明。
