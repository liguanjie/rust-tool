# AgentSkills 历史记录 SQLite 化 — 走查报告

## 变更概览

- 新增 SQLite migration `0002_agent_execution_history.sql`，创建 `agent_execution_history` 表和查询索引，并将 `user_version` 升至 2。
- 在 `rust_tool_core::storage` 中新增 AgentSkills 执行历史 repository，支持保存、去重、裁剪、查询和清空。
- 在 Tauri 桌面入口新增执行历史查询、保存、清空命令，使用程序配置页的有效数据库路径。
- 新增前端 `agentSkillHistory` API 适配层：桌面端走 Tauri/SQLite，Web 模式继续走 `localStorage`。
- 改造 `AgentSkills.vue`，移除页面内直接 `localStorage` 读写，由 API 层统一处理。

## 关键文件

- `crates/rust_tool_core/migrations/0002_agent_execution_history.sql`
- `crates/rust_tool_core/src/storage.rs`
- `crates/rust_tool_core/src/lib.rs`
- `frontend/src-tauri/src/lib.rs`
- `frontend/src/api/agentSkillHistory.ts`
- `frontend/src/api/agentSkillHistory.test.ts`
- `frontend/src/pages/AgentSkills.vue`

## 核心流程

1. AgentSkills 页面挂载后调用 `listAgentSkillHistory()`。
2. Web 模式直接读取并规范化 `rusttool:codex:history`。
3. 桌面模式调用 `get_agent_execution_history`，通过程序配置页的数据库路径初始化 SQLite。
4. 如果桌面 SQLite 为空且旧 `localStorage` 存在历史，API 层按旧数据从旧到新导入 SQLite，并删除旧 key。
5. 每次脚本执行完成后，页面调用 `saveAgentSkillHistoryRecord()`；后端按 `script_name + args` 去重，保留最新 50 条。
6. 清空历史时，桌面端清空 SQLite 表，Web 模式删除 `localStorage` key。

## 验证结果

| 类型 | 命令 / 方法 | 结果 |
|------|-------------|------|
| Rust 仓储测试 | `cargo test -p rust_tool_core storage` | 通过，5 tests |
| 桌面编译 | `cargo check -p rust_tool_desktop` | 通过 |
| 前端测试 | `pnpm --dir frontend test:run` | 通过，34 tests |
| 前端构建 | `pnpm --dir frontend build` | 通过；仅保留既有 Rollup PURE 注释与 chunk size warning |
| 空白检查 | `git diff --check` | 通过，无输出 |
| UI 验证 | `pnpm --dir frontend dev --host 127.0.0.1` + in-app Browser | `/agent-skills` 可打开；任务列表、任务配置、执行记录区域可见；console error 为 0 |

## 风险与注意事项

- 桌面端旧 `localStorage` 历史只会在 SQLite 历史为空时导入，避免重复污染已有数据库。
- 历史输出中的 `stdout/stderr` 仍然直接存入 SQLite；当前按 50 条限制控制体量，后续如输出很大，建议补充输出截断或大日志文件化策略。
- 本次不迁移 OSV 配置、VLESS 配置和程序配置，避免一次性扩大存储变更面。

## 待用户验证

- 在桌面 App 中执行一次 AI 技能脚本，确认执行记录持久化到配置页当前指向的 SQLite 文件。
- 修改程序配置页数据库路径后，回到 AI 技能页确认历史记录会随数据库文件切换。
