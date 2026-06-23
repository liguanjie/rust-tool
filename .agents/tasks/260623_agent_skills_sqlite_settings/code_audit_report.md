# AI 技能目录 SQLite 持久化 — 代码审计报告

## 审计总结

| 维度 | 结论 |
|------|------|
| 方案符合性 | ✅ 复用 `tool_settings`，核心读写在 `rust_tool_core` |
| 红线约束 | ✅ 未发现本次改动违反红线 |
| 编码规范 | ✅ 本次改动文件未发现阻断问题 |
| 编译 / 测试 | ✅ `cargo test`、前端测试、前端构建通过 |
| 潜在风险 | Web fallback 会在后端不可用时退回 localStorage；clippy 有既有阻断 |

## 发现的问题

未发现本次改动引入的阻断问题。

既有阻断：

- `crates/rust_tool_core/src/tools/finalshell_password.rs:149`：`cargo clippy --all-targets -- -D warnings` 报 `manual_is_multiple_of`。该文件不是本次改动范围，未在本任务中修改。

## 逐文件审计

### `crates/rust_tool_core/src/storage.rs`

- 改动摘要：新增 `AgentSkillsSettings`，通过 `tool_settings` 的 `agentSkills` key 读写技能目录。
- 规则检查：
  - `ARCH-001`：通过，核心 SQL 操作集中在 core。
  - `SEC-002`：通过，SQL 使用 `?` + `.bind(...)` 参数绑定。
  - `EX-001`：通过，错误通过 `Result` 返回。
- 结论：通过。

### `frontend/src-tauri/src/lib.rs`

- 改动摘要：新增 Tauri command，初始化当前应用数据库并调用 core 存储函数。
- 规则检查：
  - `ARCH-001`：通过，Tauri 层只做入口封装。
  - `EX-001`：通过，错误统一转换为字符串返回前端。
- 结论：通过。

### `crates/rust_tool_server/src/routes/workbench.rs`

- 改动摘要：新增 Web API GET / POST，使用服务端 SQLite 读写技能配置。
- 规则检查：
  - `ARCH-001`：通过，路由层只解析请求和包装响应。
  - `SEC-002`：通过，未在 server 层拼 SQL。
- 结论：通过。

### `crates/rust_tool_server/src/app.rs`

- 改动摘要：挂载 `/api/workbench/settings/agent-skills` 路由。
- 规则检查：路由注册改动，无业务逻辑。
- 结论：通过。

### `frontend/src/api/agentSkillsSettings.ts`

- 改动摘要：新增统一 API，兼容 Tauri invoke、Web HTTP、localStorage fallback。
- 规则检查：
  - `ARCH-002` / `VUE-003`：通过，页面通过 API 层访问后端能力。
  - `VUE-004`：通过，设置结构和默认值集中在 API 文件。
- 结论：通过。

### `frontend/src/pages/AgentSkills.vue`

- 改动摘要：页面加载时读取持久化设置，选择目录后保存设置并刷新脚本列表。
- 规则检查：
  - `ARCH-002`：通过，未在页面新增原始后端调用。
  - `VUE-002`：通过，仅做必要流程接入。
- 结论：通过。

### `frontend/src/api/agentSkillsSettings.test.ts`

- 改动摘要：覆盖 Web API 读取、保存、后端不可用 fallback 和默认值。
- 规则检查：测试覆盖新增 API 行为。
- 结论：通过。

## 红线合规性

| 红线 | 是否违反 | 说明 |
|------|----------|------|
| 不在前端直接访问 SQLite | 否 | 前端只调用 API / Tauri command |
| 不在入口层写核心 SQL | 否 | SQL 读写位于 `rust_tool_core::storage` |
| SQL 参数绑定 | 否 | 使用 `.bind(...)` |

## 边界场景推演

| 场景 | 结论 | 说明 |
|------|------|------|
| 首次加载 | 通过 | SQLite 无配置时返回默认空设置，页面 fallback 默认目录 |
| 保存目录 | 通过 | 保存时 trim，读回一致 |
| 刷新页面 | 通过 | UI reload 后仍显示 SQLite 中保存目录 |
| 后端不可用 | 通过 | Web 模式 fallback localStorage；Tauri 模式仍要求 SQLite command 成功 |

## 性能影响

- 调用频率：页面加载和选择目录时触发。
- 数据量级：单条工具配置记录。
- 索引 / JOIN 影响：使用 `tool_settings.tool_key` 主键，无 JOIN。

## 验证记录

- 已运行：
  - `cargo fmt`
  - `git diff --check`
  - `cargo test -p rust_tool_core agent_skills_settings_are_saved_and_loaded`
  - `cargo test -p rust_tool_server test_app_build`
  - `cargo test`
  - `pnpm --dir frontend exec vitest run src/api/agentSkillsSettings.test.ts`
  - `pnpm --dir frontend run test:run`
  - `pnpm --dir frontend run build`
  - `cargo clippy --all-targets -- -D warnings`
  - 本机 API 保存 / 读取验证
  - in-app Browser reload 验证
- 未通过：
  - `cargo clippy --all-targets -- -D warnings`，失败原因是既有 `finalshell_password.rs:149` clippy 提示。
- 未运行：
  - Windows 实机桌面端验证。

## 结论

本次改动已将 AI 技能目录接入 SQLite 持久化，桌面和 Web dev 模式均具备持久化路径；建议在同事机器上做一次真实刷新验收。
