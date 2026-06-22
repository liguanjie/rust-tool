# 程序配置页 — 代码审计报告

## 审计总结

| 维度 | 结论 |
|------|------|
| 方案符合性 | ✅ 新增配置页与现有 Tauri JSON / Web localStorage 模式一致 |
| 红线约束 | ✅ 未触及 Controller、SQL、Map 请求体、BigDecimal 等致命红线 |
| 编码规范 | ✅ 前端配置读写集中在 `api/` 适配层，页面未直接散落持久化逻辑 |
| 编译 / 测试 | ✅ 已运行前端测试、前端构建、Tauri crate check、diff check |
| 潜在风险 | 后续真实 SQLite 初始化仍需补目录可写性校验和迁移流程 |

## 发现的问题

未发现阻断问题。

## 逐文件审计

### `frontend/src-tauri/src/lib.rs`

- 改动摘要：新增 `ProgramSettings`、默认数据库路径、Tauri 配置读写 command。
- 规则检查：配置结构强类型；无 SQL；无静默吞异常；错误信息保留上下文。
- 结论：通过。

### `frontend/src/api/programSettings.ts`

- 改动摘要：新增程序配置 API 适配层，桌面端走 Tauri command，Web 端走 localStorage。
- 规则检查：符合 `VUE-003` 集中 API 适配；默认值和异常 fallback 明确。
- 结论：通过。

### `frontend/src/pages/ProgramSettings.vue`

- 改动摘要：新增程序配置页面，支持数据库路径输入、选择目录、恢复默认和保存。
- 规则检查：使用现有 Ant Design Vue 控件；按钮防止换行；未绕过 API 层直接持久化。
- 结论：通过。

### `frontend/src/router/index.ts`

- 改动摘要：新增 `/program-settings` 路由和 `/settings` 重定向。
- 规则检查：未影响现有工具路由。
- 结论：通过。

### `frontend/src/stores/tools.ts`

- 改动摘要：新增左侧菜单“程序配置”入口。
- 规则检查：沿用现有工具菜单结构和图标模式。
- 结论：通过。

### `frontend/src/pages/ToolboxDashboard.vue`

- 改动摘要：工作台新增程序配置入口卡片，大屏列宽调整为四列。
- 规则检查：沿用现有工作台卡片结构。
- 结论：通过。

### `frontend/src/api/programSettings.test.ts`

- 改动摘要：新增 Web fallback 单测。
- 规则检查：覆盖默认值、路径 trim 保存、异常 JSON 恢复。
- 结论：通过。

## 红线合规性

| 红线 | 是否违反 | 说明 |
|------|----------|------|
| `ARCH-001` Controller 仅负责参数校验和包装 | 否 | 本次未改 Java Controller；Tauri command 只做配置读写包装 |
| `CLEAN-003` 禁止内联完整包名类名 | 否 | 本次 Rust/Vue 不涉及 Java FQCN |
| `VALID-003` 禁止 Map 请求体 | 否 | 配置结构使用强类型对象 |
| `FINANCE-001` BigDecimal divide 必须指定规则 | 否 | 不涉及金额计算 |
| `SEC-002` 禁止拼接 SQL | 否 | 本次不新增 SQL |

## 边界场景推演

| 场景 | 结论 | 说明 |
|------|------|------|
| 正向场景 | 通过 | 输入自定义数据库路径后保存，页面显示已同步 |
| 回归场景 | 通过 | 原 VLESS、OSV、AI 技能路由未变 |
| NULL / 空数据 | 通过 | 空路径表示使用默认路径；旧 JSON 缺字段可默认反序列化 |
| 并发 / 状态变化 | 可接受 | 配置保存为整份 JSON，当前无并发写入场景 |

## 性能影响

- 调用频率：低频用户配置操作。
- 数据量级：单个配置字段。
- 索引 / JOIN 影响：无数据库查询。

## 验证记录

- 已运行：
  - `pnpm --dir frontend test:run`
  - `pnpm --dir frontend build`
  - `cargo check -p rust_tool_desktop`
  - `git diff --check`
  - in-app Browser 验证 `/program-settings` 页面渲染与保存交互
- 未运行：
  - 未运行完整桌面 App 打包。

## 结论

本次改动符合当前架构边界，可以作为 SQLite 正式接入前的程序配置入口。
