# 数据库维护能力 — 代码审计报告

## 审计总结

| 维度 | 结论 |
|------|------|
| 方案符合性 | ✅ |
| 红线约束 | ✅ |
| 编码规范 | ✅ |
| 编译 / 测试 | ✅ |
| 潜在风险 | 清理历史后数据库文件大小不一定下降，可后续增加压缩动作 |

## 发现的问题

未发现阻断问题。

## 逐文件审计

### `crates/rust_tool_core/src/storage.rs`

- 改动摘要：新增数据库文件统计、备份、OSV 命令历史清理与测试。
- 规则检查：`VACUUM INTO` 使用参数绑定；固定清理 SQL 不接收用户拼接；路径校验和文件错误有明确返回。
- 结论：通过。

### `frontend/src-tauri/src/lib.rs`

- 改动摘要：新增数据库维护 Tauri 命令，组合 health/stats 响应，打开系统目录。
- 规则检查：命令层只做参数、路径和结果包装，存储逻辑下沉 core；清理历史同步 JSON 快照，避免旧数据回流。
- 结论：通过。

### `frontend/src/api/programSettings.ts`

- 改动摘要：新增数据库统计类型和维护动作 API。
- 规则检查：页面不直接调用 Tauri 命令，符合 `VUE-003`。
- 结论：通过。

### `frontend/src/pages/ProgramSettings.vue`

- 改动摘要：新增数据库大小展示、维护按钮、清理确认和 loading 状态。
- 规则检查：危险动作有确认；异步动作有 loading，符合 `VUE-007`。
- 结论：通过。

### `frontend/src/api/programSettings.test.ts`

- 改动摘要：补充 Web fallback 下的 `databaseStats` 和维护动作测试。
- 规则检查：覆盖新增 API 边界。
- 结论：通过。

## 红线合规性

| 红线 | 是否违反 | 说明 |
|------|----------|------|
| `SEC-002` 禁止拼接 SQL | 否 | 备份路径使用 sqlx 参数绑定 |
| `VALID-003` 强类型对象流传 | 否 | 返回结构均为强类型 struct/interface |
| `VUE-003` 请求层收归 | 否 | 页面通过 `programSettings.ts` 调用 |
| 不误删当前数据 | 否 | 清理历史不删除项目和最后一次扫描结果 |

## 边界场景推演

| 场景 | 结论 | 说明 |
|------|------|------|
| 正向场景 | 通过 | 配置页加载后展示 health 与大小 |
| 回归场景 | 通过 | 原保存数据库路径流程保留 |
| NULL / 空数据 | 通过 | Web fallback 和空路径均有默认值 |
| 并发 / 状态变化 | 通过 | 前端维护动作使用 loading 防重入 |

## 性能影响

- 调用频率：仅用户打开配置页或主动维护时触发。
- 数据量级：文件大小读取为 metadata 操作；备份由 SQLite 执行。
- 索引 / JOIN 影响：无新增查询 JOIN，无 schema 变更。

## 验证记录

- 已运行：
  - `cargo test -p rust_tool_core storage`
  - `cargo check -p rust_tool_desktop`
  - `pnpm --dir frontend test:run`
  - `pnpm --dir frontend build`
  - `git diff --check`
  - Browser UI 验证
- 未运行：
  - 未在真实 Tauri 桌面窗口中点击系统文件管理器打开动作。

## 结论

本次数据库维护能力实现符合当前架构和团队规范，可以进入用户验证。
