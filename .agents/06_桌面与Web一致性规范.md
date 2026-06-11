# 桌面与 Web 一致性规范

## 核心原则

RustTool 的桌面版和 Web 版必须是同一个产品，而不是两套功能。

1. 前端界面以 Vue 页面为唯一实现。
2. 业务核心以 `rust_tool_core` 为唯一实现。
3. Web 运行时通过 HTTP API 访问后端。
4. 桌面运行时通过 Tauri command 直接访问 Rust 核心，不依赖本地端口。
5. 前端页面不得直接散落 `fetch('/api/memo/...')`；必须通过统一 API 适配层调用。

## Memo API 适配规则

AI 安全文档页面统一调用 `frontend/src/services/memoApi.ts`。

- Web 模式：`memoApi` 使用 `fetch('/api/memo/...')`。
- 桌面模式：`memoApi` 使用 `@tauri-apps/api/core` 的 `invoke(...)`。

当前实现位置：

- 页面入口：`frontend/src/pages/AiMemo.vue`、`frontend/src/pages/SecretVault.vue`
- 前端适配层：`frontend/src/services/memoApi.ts`
- HTTP 路由：`crates/rust_tool_server/src/routes/memo.rs`
- Tauri command：`frontend/src-tauri/src/memo_commands.rs`
- Tauri 注册入口：`frontend/src-tauri/src/lib.rs`

新增、删除或修改 AI 安全文档接口时，必须同时更新：

1. `rust_tool_server/src/routes/memo.rs` 中的 HTTP 路由。
2. `frontend/src-tauri/src/memo_commands.rs` 中的 Tauri command。
3. `frontend/src/services/memoApi.ts` 中的前端适配映射。
4. 相关测试或手动验证说明。

后续 Codex 新窗口继续开发时，必须先检查：

1. `frontend/src/pages/AiMemo.vue` 中不应直接出现 `fetch('/api/memo` 或硬编码 `/api/memo`。
2. `frontend/src/services/memoApi.ts` 中的路径映射必须覆盖页面正在调用的所有 AI 文档接口。
3. `frontend/src-tauri/src/lib.rs` 的 `tauri::generate_handler!` 必须注册对应 command。
4. Web 与桌面的错误响应必须保持页面可统一读取：`{ error: { code, message } }`。
5. 完成后至少运行前端构建和 Rust 编译测试；如改到共享核心，还要运行 `./rt test`。

密码库页面规则：

1. 密码库页面只显示 secret 索引，不在列表接口中返回明文。
2. 明文只能通过单项 reveal/copy 触发，前端必须自动隐藏临时显示的值。
3. 密码库明文不得进入 AI 文档聊天、草稿、检索提示词或任何模型上下文。
4. 修改主密码必须先自动创建本地备份，再修改 KDBX 密码和 password verifier。
5. 修改主密码成功后必须立即锁定保险箱，要求用户使用新主密码重新解锁。
6. 删除、编辑、改名 secret 需要单独方案，必须处理 Markdown 占位符引用一致性。

## 数据目录规则

桌面和 Web 使用相同的 AI 安全文档资料库默认目录策略：

- macOS: `~/Library/Application Support/rust-tool`
- Windows: `%APPDATA%/rust-tool`
- Linux/Unix: `$XDG_DATA_HOME/rust-tool` 或 `~/.local/share/rust-tool`

Tauri 自身的应用配置目录 `com.ben.rusttool` 可以继续存放桌面壳设置，例如工具偏好；AI 安全文档资料库必须使用 `rust-tool` 目录策略，以保证桌面与 Web 的资料库一致。

## 用户体验规则

1. 桌面窗口必须能直接解锁和使用 AI 安全文档。
2. 桌面窗口不得要求用户手动运行 `./rt dev` 或后端端口。
3. 桌面和 Web 的页面文案、交互和功能默认保持一致。
4. 如某功能只能在某一运行时支持，必须在 UI 中明确显示原因，并在本文件记录例外。

## 参考

Tauri 2 官方建议通过 `#[tauri::command]` 暴露 Rust 函数，并在前端使用 `invoke(...)` 调用；命令可以异步、返回可序列化数据，也可以访问 managed state。
