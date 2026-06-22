# 数据库压缩、恢复与诊断 — 代码审计报告

## 审计总结

| 维度 | 结论 |
|------|------|
| 方案符合性 | ✅ |
| 红线约束 | ✅ |
| 编码规范 | ✅ |
| 编译 / 测试 | ✅ |
| 潜在风险 | 恢复动作替换数据库文件，需用户桌面端确认后触发 |

## 发现的问题

未发现阻断问题。

## 逐文件审计

### `crates/rust_tool_core/src/storage.rs`

- 改动摘要：新增 `vacuum_database`、`restore_database_file`、`database_storage_diagnostics` 和相关测试。
- 规则检查：SQL 为固定语句；诊断计数不接收用户 SQL；恢复路径有空值、存在性和“不能等于当前库”校验。
- 结论：通过。

### `frontend/src-tauri/src/lib.rs`

- 改动摘要：新增压缩、恢复、旧库删除命令，扩展配置响应。
- 规则检查：恢复前自动备份当前库；旧库删除只使用固定 app data 路径和固定文件名；命令层只做编排和结果包装。
- 结论：通过。

### `frontend/src/api/programSettings.ts`

- 改动摘要：新增压缩、恢复、旧库清理 API 和诊断类型。
- 规则检查：页面调用集中在 API 模块，符合 `VUE-003`。
- 结论：通过。

### `frontend/src/pages/ProgramSettings.vue`

- 改动摘要：新增维护按钮、旧库状态、存储诊断卡片。
- 规则检查：危险动作有确认；所有维护动作有 loading 状态，符合 `VUE-007`。
- 结论：通过。

### `frontend/src/api/programSettings.test.ts`

- 改动摘要：补充 Web fallback 默认值和桌面专属动作拒绝测试。
- 规则检查：覆盖新增字段与边界。
- 结论：通过。

## 红线合规性

| 红线 | 是否违反 | 说明 |
|------|----------|------|
| `SEC-002` 禁止拼接 SQL | 否 | SQL 固定，备份/恢复路径不拼入 SQL |
| `VALID-003` 强类型对象流传 | 否 | Tauri 请求和响应均为强类型 |
| `VUE-003` 请求层收归 | 否 | 页面通过 `programSettings.ts` 调用 |
| 恢复前必须自动备份 | 否 | `restore_program_database` 先生成 safety backup |
| 旧库只删除固定文件 | 否 | 仅删除 `rusttool.sqlite` 及 sidecar |

## 边界场景推演

| 场景 | 结论 | 说明 |
|------|------|------|
| 正向场景 | 通过 | 压缩、恢复、诊断命令均可编译 |
| 回归场景 | 通过 | 原设置保存、刷新、备份、清理历史流程保留 |
| NULL / 空数据 | 通过 | Web fallback 提供默认诊断与旧库状态 |
| 并发 / 状态变化 | 通过 | 维护动作共用 `maintenanceAction` 防重入 |

## 性能影响

- 调用频率：仅配置页刷新或用户手动维护时触发。
- 数据量级：诊断为少量 `COUNT(*)`；压缩/恢复为用户主动触发的重操作。
- 索引 / JOIN 影响：无新增 JOIN，无 schema 变更。

## 验证记录

- 已运行：
  - `cargo test -p rust_tool_core storage`
  - `cargo check -p rust_tool_desktop`
  - `pnpm --dir frontend test:run`
  - `pnpm --dir frontend build`
  - `git diff --check`
  - Browser UI 验证
- 未运行：
  - 未在真实 Tauri 桌面窗口中实际选择备份文件恢复。
  - 未在真实 Tauri 桌面窗口中实际删除用户目录里的旧库。

## 结论

本次数据库安全闭环能力符合当前架构和团队规范，可以进入桌面端用户验证。
