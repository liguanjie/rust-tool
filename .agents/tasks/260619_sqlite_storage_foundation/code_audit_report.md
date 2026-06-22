# SQLite storage foundation — 代码审计报告

## 审计总结

| 维度 | 结论 |
|------|------|
| 方案符合性 | ✅ storage 能力位于 `rust_tool_core`，Tauri 只负责配置读取和状态包装 |
| 红线约束 | ✅ 未发现 SQL 拼接、Map 请求体、金额计算、Controller 直接持久化等问题 |
| 编码规范 | ✅ 前端通过 `api/programSettings.ts` 强类型适配，SQLite DDL 进入 migration |
| 编译 / 测试 | ✅ Rust / 前端验证均已运行通过 |
| 潜在风险 | `sqlx` 增加依赖体积；桌面端打开配置页会创建数据库文件 |

## 发现的问题

未发现阻断问题。

## 逐文件审计

### `Cargo.toml`

- 改动摘要：新增 `sqlx` workspace 依赖。
- 规则检查：依赖集中在 workspace，版本和 features 明确。
- 结论：通过。

### `crates/rust_tool_core/Cargo.toml`

- 改动摘要：core 引入 `sqlx`。
- 规则检查：数据库能力落在 core 层，符合共享业务核心边界。
- 结论：通过。

### `crates/rust_tool_core/src/storage.rs`

- 改动摘要：新增 SQLite 初始化、migration、健康检查和测试。
- 规则检查：未使用字符串拼接 SQL；运行期查询均为静态 SQL；错误不静默吞掉。
- 结论：通过。

### `crates/rust_tool_core/migrations/0001_storage_foundation.sql`

- 改动摘要：新增基础表。
- 规则检查：表主键明确；索引覆盖事件和导出记录常见查询方向；无动态 SQL。
- 结论：通过。

### `crates/rust_tool_core/src/lib.rs`

- 改动摘要：导出 storage API。
- 规则检查：公开边界清晰，未暴露内部 SQL 实现。
- 结论：通过。

### `frontend/src-tauri/src/lib.rs`

- 改动摘要：程序配置 command 接入数据库健康检查。
- 规则检查：Tauri command 不写 SQL，只调用 core storage；配置对象为强类型。
- 结论：通过。

### `frontend/src/api/programSettings.ts`

- 改动摘要：新增 `DatabaseHealth` 类型，Web fallback 标记为 unavailable。
- 规则检查：符合 `VUE-003`，前端页面不直接访问存储细节。
- 结论：通过。

### `frontend/src/pages/ProgramSettings.vue`

- 改动摘要：展示数据库健康状态、schema 版本、迁移数量和文件状态。
- 规则检查：沿用现有 Ant Design Vue 页面；保存按钮仍有 loading/disabled 控制。
- 结论：通过。

### `frontend/src/api/programSettings.test.ts`

- 改动摘要：补充 Web fallback 健康状态断言。
- 规则检查：覆盖默认路径和异常恢复。
- 结论：通过。

## 红线合规性

| 红线 | 是否违反 | 说明 |
|------|----------|------|
| `ARCH-001` Controller 仅负责参数校验和包装 | 否 | 本次无 Java Controller；Tauri command 委托 core storage |
| `CLEAN-003` 禁止内联完整包名类名 | 否 | 不涉及 Java FQCN |
| `VALID-003` 禁止 Map 请求体 | 否 | 配置和健康状态均为强类型结构 |
| `FINANCE-001` BigDecimal divide 必须指定规则 | 否 | 不涉及金额计算 |
| `SEC-002` 禁止拼接 SQL | 否 | 使用静态 SQL / SQLx migrator，无用户输入拼 SQL |

## 边界场景推演

| 场景 | 结论 | 说明 |
|------|------|------|
| 正向场景 | 通过 | 可创建临时 SQLite 并执行 migration |
| 回归场景 | 通过 | 原程序配置路径保存、Web fallback、现有工具测试通过 |
| NULL / 空数据 | 通过 | 空数据库路径返回错误健康状态，不 panic |
| 并发 / 状态变化 | 可接受 | SQLx pool 支持并发；当前仅配置页低频初始化 |

## 性能影响

- 调用频率：低频，主要在打开/保存程序配置时。
- 数据量级：首版基础表为空表，影响很小。
- 索引 / JOIN 影响：新增基础索引，无复杂 JOIN。

## 验证记录

- 已运行：
  - `cargo test -p rust_tool_core storage`
  - `cargo check -p rust_tool_desktop`
  - `pnpm --dir frontend test:run`
  - `pnpm --dir frontend build`
  - `git diff --check`
  - in-app Browser 验证 `/program-settings`
- 未运行：
  - 未运行完整 Tauri 桌面窗口人工验证。
  - 未运行完整应用打包。

## 结论

SQLite 基础设施层已具备可继续扩展 Repository 和业务数据迁移的基础，当前未发现阻断问题。
