# AgentSkills 历史记录 SQLite 化 — 代码审计报告

## 审计总结

| 维度 | 结论 |
|------|------|
| 方案符合性 | ✅ 符合“先迁移 AgentSkills 历史仓储”的实施计划 |
| 红线约束 | ✅ 未发现 SQL 拼接、页面直连存储散落、无关重构 |
| 编码规范 | ✅ 前端调用已收口到 `api/`，Rust 使用强类型结构 |
| 编译 / 测试 | ✅ Rust 测试、桌面编译、前端测试、前端构建均通过 |
| 潜在风险 | `stdout/stderr` 未截断，后续建议增加输出体量治理 |

## 发现的问题

未发现阻断问题。

## 逐文件审计

### `crates/rust_tool_core/migrations/0002_agent_execution_history.sql`

- 改动摘要：新增执行历史表、脚本/时间索引、全局时间索引和 schema version 更新。
- 规则检查：DDL 幂等；字段使用 SQLite 基础类型；索引覆盖最近记录查询和后续脚本筛选。
- 结论：通过。

### `crates/rust_tool_core/src/storage.rs`

- 改动摘要：新增 `AgentExecutionHistoryRecord` 和保存、查询、清空 repository；保存逻辑包含校验、参数绑定、事务、去重和裁剪。
- 规则检查：符合 `SEC-002`，所有动态值使用 `.bind(...)`；符合强类型要求，无 Map 式弱结构；无静默吞异常。
- 结论：通过。

### `crates/rust_tool_core/src/lib.rs`

- 改动摘要：导出 SQLite 初始化和 AgentSkills 历史 repository API。
- 规则检查：导出范围与 Tauri 调用需要一致，未暴露无关模块。
- 结论：通过。

### `frontend/src-tauri/src/lib.rs`

- 改动摘要：新增 `get_agent_execution_history`、`save_agent_execution_history_record`、`clear_agent_execution_history` 三个命令，并通过程序配置解析有效数据库路径。
- 规则检查：Tauri 命令仅做参数入口和结果包装，数据库逻辑下沉到 core repository；错误向前端返回字符串；未写 SQL。
- 结论：通过。

### `frontend/src/api/agentSkillHistory.ts`

- 改动摘要：新增前端唯一 API 入口，封装 Tauri/SQLite 与 Web/localStorage fallback，兼容旧历史字段并支持空库导入。
- 规则检查：符合 `VUE-003`，页面不直接维护持久化细节；状态字段归一化集中处理。
- 结论：通过。

### `frontend/src/api/agentSkillHistory.test.ts`

- 改动摘要：覆盖 Web fallback 空数据、保存、去重、50 条裁剪、清空和非法 JSON。
- 规则检查：测试覆盖核心边界，未引入外部依赖。
- 结论：通过。

### `frontend/src/pages/AgentSkills.vue`

- 改动摘要：移除页面内 `localStorage` watcher，挂载时加载历史，执行后调用保存 API，清空时调用清空 API。
- 规则检查：持久化调用已收口；交互状态保持原状；未改动脚本执行主流程。
- 结论：通过。

## 红线合规性

| 红线 | 是否违反 | 说明 |
|------|----------|------|
| `SEC-002` 禁止拼接 SQL | 否 | Rust repository 使用 `sqlx::query` + `.bind(...)` |
| `VALID-003` 强类型对象流传 | 否 | Rust/TS 均定义执行历史记录类型 |
| `VUE-003` 前端请求层收口 | 否 | 新增 `frontend/src/api/agentSkillHistory.ts` |
| 禁止无关重构 | 否 | 未改动 OSV、VLESS 存储逻辑 |
| 禁止破坏 Web fallback | 否 | Web 模式继续使用原 `localStorage` key |

## 边界场景推演

| 场景 | 结论 | 说明 |
|------|------|------|
| 正向场景 | 通过 | 执行脚本后保存记录，列表返回最新记录 |
| 回归场景 | 通过 | Web fallback 使用旧 key，页面可正常加载 |
| NULL / 空数据 | 通过 | 无历史返回空数组；非法 JSON 返回空数组；空脚本名在 API/core 层拦截 |
| 并发 / 状态变化 | 可接受 | 保存逻辑使用事务；当前 UI 单次脚本执行串行触发 |
| 大输出 | 有残余风险 | 50 条限制可控，但 stdout/stderr 未截断 |

## 性能影响

- 调用频率：仅页面加载、脚本执行完成、清空历史时触发。
- 数据量级：固定保留 50 条。
- 索引 / JOIN 影响：无 JOIN；新增时间和脚本索引，写入成本极低。

## 验证记录

- 已运行：`cargo test -p rust_tool_core storage`
- 已运行：`cargo check -p rust_tool_desktop`
- 已运行：`pnpm --dir frontend test:run`
- 已运行：`pnpm --dir frontend build`
- 已运行：`git diff --check`
- 已运行：in-app Browser 打开 `/agent-skills`，选中 “AI 技能安装向导”，确认任务配置和执行记录区域正常，console error 为 0。
- 未运行：Tauri 桌面 App 可见窗口端到端执行脚本。

## 结论

本次变更符合实施计划和团队规范，未发现阻断问题；建议下一步在桌面 App 中执行一次真实脚本作为用户验收。
