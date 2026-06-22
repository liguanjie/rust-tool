# FinalShell 密码解密工具 — 走查报告

## 变更概览

- 新增纯 Rust FinalShell 密码解密能力，不依赖 Java 环境。
- 核心算法统一放在 `crates/rust_tool_core`，Web/Tauri/CLI 共用同一函数。
- 新增 Web/Tauri 前端页面 `/toolbox/finalshell-password`，工作台增加工具入口。
- 新增 CLI 子命令 `decode-finalshell-password`，便于同事无桌面环境时直接使用。

## 关键文件

- `crates/rust_tool_core/src/tools/finalshell_password.rs`：Base64 解析、Java Random 兼容实现、MD5 key 派生、DES/ECB/PKCS5Padding 解密、单元测试。
- `crates/rust_tool_server/src/routes/tools.rs`：新增 HTTP 解密接口。
- `frontend/src-tauri/src/lib.rs`：新增 Tauri command。
- `crates/rust_tool_cli/src/main.rs`：新增 CLI 子命令。
- `frontend/src/api/tools.ts`：新增 Web/Tauri 统一 API。
- `frontend/src/pages/FinalShellPasswordDecoder.vue`：新增工具页面。
- `frontend/src/pages/ToolboxDashboard.vue`、`frontend/src/router/index.ts`：新增入口与路由。

## 核心流程

1. 用户粘贴 FinalShell 加密密码。
2. 前端通过 `decodeFinalShellPassword` 自动选择 Tauri command 或 HTTP API。
3. 入口层调用 `rust_tool_core::decode_finalshell_password`。
4. Core 解析前 8 字节 head，按 Java `Random` 行为派生 DES key。
5. Core 执行 DES 解密、PKCS5 去填充、UTF-8 校验后返回明文。
6. 前端用密码框展示结果，默认隐藏，支持复制。

## 验证结果

- `cargo test`：通过，包含 core 样例测试、DES 标准向量测试、server route 测试。
- `pnpm --dir frontend run test:run`：通过，5 个测试文件、40 个测试。
- `pnpm --dir frontend run build`：通过；存在既有 Vite/Rollup 警告：VueUse PURE 注释提示和大 chunk 提示。
- `git diff --check`：通过，无空白错误。
- CLI 样例验证：`cargo run -p rust_tool_cli -- decode-finalshell-password eGANOUNaAUlE5cPFQXrtPfyej430A+k2ruM2JPtvU/I=` 输出与 Java 样例一致。
- UI 验证：in-app Browser 打开 `http://127.0.0.1:5173/toolbox/finalshell-password`，填入样例密文后返回预期明文，复制按钮启用。
- 响应式验证：390x844 视口下页面存在且无横向溢出。

## 风险与注意事项

- 解密结果属于敏感凭据；代码未写入本地配置、数据库或日志，但用户复制后需自行控制粘贴目标。
- Web API 返回明文 JSON，默认本地服务绑定 `127.0.0.1`；如用户手动将服务暴露到非本机网络，需要自行控制访问边界。
- 工作区存在与本任务无关的已修改/未跟踪文件，未纳入本次功能说明。

## 待用户验证

- 使用真实 FinalShell 配置中的 password 字段验证更多样本。
- 在目标同事机器上验证 CLI 或 Web 页面无需 Java 环境即可工作。
