# AgentSkills 历史记录 SQLite 化 — 影响分析

## 影响范围

| 模块 | 影响 | 说明 |
|------|------|------|
| SQLite schema | 有新增 | 新增 `agent_execution_history` 表和 2 个索引，`user_version = 2` |
| Rust core | 有新增 | 新增执行历史 repository API，沿用既有 SQLite 初始化层 |
| Tauri 桌面端 | 有新增 | 新增 3 个命令：查询、保存、清空 AgentSkills 历史 |
| 前端 API | 有新增 | 新增 `frontend/src/api/agentSkillHistory.ts` 作为唯一调用入口 |
| AgentSkills 页面 | 有修改 | 由页面内存储改为 API 适配层存储 |
| 其他工具 | 无直接改动 | OSV、VLESS、程序配置本次不迁移 |

## 数据库影响

- 表：`agent_execution_history`
- 主键：`id TEXT`
- 主要查询：按 `timestamp DESC` 查询最近 50 条；按 `script_name + timestamp` 支持后续筛选。
- 写入策略：先删除同 `script_name + args` 的旧记录，再插入新记录，并裁剪超出保留上限的数据。
- SQL 安全：所有动态值均使用 `sqlx::query(...).bind(...)` 参数绑定。

## 兼容性

- Web 模式：继续使用原 key `rusttool:codex:history`，不要求本地 SQLite。
- 桌面模式：优先读取 SQLite；首次空库时导入旧 `localStorage` 历史。
- 字段兼容：前端 API 兼容旧 `scriptName + exit_code`，也兼容可能来自后端或旧数据的 `script_name / exitCode`。

## 性能影响

- 每次保存历史会执行一次去重、一次插入、一次裁剪，数据量被限制在 50 条，影响很小。
- SQLite 连接通过现有初始化函数创建，当前 Tauri 命令按次初始化；后续若高频调用增多，可考虑 AppState 持有连接池。

## 回滚方案

- 如桌面端 SQLite 历史出现异常，可将 `AgentSkills.vue` 临时改回调用 Web fallback 逻辑，或让 `agentSkillHistory.ts` 在 Tauri 异常时退回 localStorage。
- migration 已通过 `CREATE TABLE IF NOT EXISTS` 和 `CREATE INDEX IF NOT EXISTS` 保持幂等；回滚代码后遗留表不会影响现有 JSON 配置。

## 残余风险

- `stdout/stderr` 未做长度截断，极端大输出可能让 SQLite 文件增长较快。
- 当前 UI 验证是在 Web dev 模式完成；Tauri 命令已通过 `cargo check` 和 core 测试验证，仍建议用户在桌面 App 实际跑一次脚本做端到端确认。
