# FinalShell 密码解密工具 — 代码审计报告

## 审计总结

| 维度 | 结论 |
|------|------|
| 方案符合性 | ✅ 核心算法位于 `rust_tool_core`，入口层保持薄封装 |
| 红线约束 | ✅ 未发现明文落盘、日志输出或跨层重复业务逻辑 |
| 编码规范 | ✅ 符合本项目 Rust/Vue 入口约束和统一 API 约束 |
| 编译 / 测试 | ✅ `cargo test`、`pnpm --dir frontend run test:run`、`pnpm --dir frontend run build`、`git diff --check` 均已通过 |
| 潜在风险 | 解密结果为敏感凭据，用户复制后需自行控制外部流转 |

## 发现的问题

未发现阻断问题。

## 逐文件审计

### `crates/rust_tool_core/src/tools/finalshell_password.rs`

- 改动摘要：新增 FinalShell 密码解密核心算法，包含 Java Random 兼容、MD5 key 派生、DES 解密、PKCS5 去填充和样例测试。
- 规则检查：符合 `ARCH-001`，业务算法位于 core；符合 `EX-001`，无 `unwrap()` 或 `panic!()` 参与生产路径；符合 `SEC-001`，中间 key/plaintext 使用 `Zeroizing`，错误信息不包含明文。
- 结论：通过。重点行：入口与错误处理在 32 行，key 派生在 57 行，DES 解密在 145 行。

### `crates/rust_tool_server/src/routes/tools.rs`

- 改动摘要：新增 `FinalShellPasswordDecodeRequest/Response` 和 HTTP 解密 handler。
- 规则检查：符合 `ARCH-001`，route 仅做参数接收、core 调用和结果包装；错误响应保持 `{ error: { code, message } }` 标准结构。
- 结论：通过。重点行：请求/响应 DTO 在 66 行，handler 在 126 行。

### `frontend/src-tauri/src/lib.rs`

- 改动摘要：新增 Tauri command `decode_finalshell_password` 并注册到 invoke handler。
- 规则检查：符合桌面入口薄封装要求；未将敏感输入加入 `DesktopSettings`，不持久化明文或密文。
- 结论：通过。

### `frontend/src/api/tools.ts`

- 改动摘要：新增 `decodeFinalShellPassword`，统一兼容 Tauri invoke 和 Web HTTP。
- 规则检查：符合 `ARCH-002` 和 `VUE-003`，前端页面不直接散落 `fetch` 或 `invoke`。
- 结论：通过。重点行：类型定义在 37 行，统一 API 在 101 行。

### `frontend/src/pages/FinalShellPasswordDecoder.vue`

- 改动摘要：新增解密页面，支持密文输入、解密、隐藏展示明文、复制和清空。
- 规则检查：页面状态仅在内存中维护；结果默认使用密码框隐藏；按钮 loading 防并发；无本地存储敏感值。
- 结论：通过。重点行：解密流程在 32 行，复制流程在 52 行，结果密码框在 162 行。

### `crates/rust_tool_cli/src/main.rs`

- 改动摘要：新增 `decode-finalshell-password` CLI 子命令。
- 规则检查：CLI 输出明文属于用户显式命令结果，不写日志、不落盘；错误信息不包含明文。
- 结论：通过。重点行：子命令定义在 53 行，执行分支在 171 行。

## 红线合规性

| 红线 | 是否违反 | 说明 |
|------|----------|------|
| 核心业务逻辑必须下沉到 core | 否 | DES/Random/MD5 算法只在 core 实现 |
| 禁止明文凭据落盘或写日志 | 否 | 页面不持久化，Tauri settings 不新增该字段，server/CLI 不记录日志 |
| 禁止前端直接散落 `fetch` / `invoke` | 否 | 页面只调用 `frontend/src/api/tools.ts` |
| 禁止粗暴 panic / unwrap | 否 | 生产路径使用 `Result` 返回错误 |

## 边界场景推演

| 场景 | 结论 | 说明 |
|------|------|------|
| 正向场景 | 通过 | 样例密文解密结果与 Java 代码一致 |
| 回归场景 | 通过 | VLESS/OSV 等既有 Rust 测试和前端测试通过 |
| NULL / 空数据 | 通过 | 空输入、非 Base64、过短 payload 有明确错误 |
| 并发 / 状态变化 | 通过 | 前端解密按钮 loading 时禁用，避免重复提交 |

## 性能影响

- 调用频率：手动工具调用，低频。
- 数据量级：单条密码字符串，DES 分块数量很小。
- 索引 / JOIN 影响：无数据库访问。

## 验证记录

- 已运行：`cargo test`
- 已运行：`pnpm --dir frontend run test:run`
- 已运行：`pnpm --dir frontend run build`
- 已运行：`git diff --check`
- 已运行：CLI 样例解密命令
- 已运行：in-app Browser 桌面视口和 390x844 移动视口验证
- 未运行：真实用户 FinalShell 配置批量样本验证

## 结论

本次变更符合团队规范和项目架构要求，可以进入用户真实样本验证阶段。
