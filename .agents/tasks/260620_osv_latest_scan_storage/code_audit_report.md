# OSV 最近完整扫描结果持久化 — 代码审计报告

## 审计总结

| 维度 | 结论 |
|------|------|
| 方案符合性 | ✅ 符合“每个项目保存最近一次完整扫描结果，命令历史只保留轻量记录”的方案 |
| 红线约束 | ✅ 未发现 SQL 拼接、页面直连 Tauri、无关重构 |
| 编码规范 | ✅ 前端调用收口到 `api/`，Rust 使用强类型 `OsvScanResult` |
| 编译 / 测试 | ✅ Rust 测试、桌面编译、前端测试、前端构建均通过 |
| 潜在风险 | 完整 JSON 可能较大，但每项目只保留一条，风险可控 |

## 发现的问题

未发现阻断问题。

## 逐文件审计

### `crates/rust_tool_core/migrations/0003_osv_latest_scan_results.sql`

- 改动摘要：新增 `osv_latest_scan_results` 表，按 `project_path` 保存最近完整扫描结果。
- 规则检查：DDL 幂等；索引覆盖按时间查看/后续扩展；`user_version` 升至 3。
- 结论：通过。

### `crates/rust_tool_core/src/storage.rs`

- 改动摘要：新增 `save_osv_latest_scan_result`、`get_osv_latest_scan_result`、`delete_osv_latest_scan_result`。
- 规则检查：符合 `SEC-002`，动态值全部 `.bind(...)`；结果类型使用 `OsvScanResult` 强类型；空路径会被拒绝。
- 结论：通过。

### `crates/rust_tool_core/src/lib.rs`

- 改动摘要：导出 OSV 最近扫描结果仓储 API。
- 规则检查：导出范围与 Tauri 命令需要一致。
- 结论：通过。

### `frontend/src-tauri/src/lib.rs`

- 改动摘要：新增最近扫描结果的读取、保存、删除 Tauri 命令。
- 规则检查：Tauri 层仅负责参数入口和调用 core repository，无 SQL 和业务聚合逻辑。
- 结论：通过。

### `frontend/src/api/osvScanner.ts`

- 改动摘要：新增最近扫描结果 API；Web fallback 使用独立 localStorage key。
- 规则检查：符合 `VUE-003`；未把持久化细节散到页面或 store 外。
- 结论：通过。

### `frontend/src/stores/osvScanner.ts`

- 改动摘要：加载/切换项目恢复最近结果，扫描完成保存完整结果，删除项目清理结果；`hasCurrentScanResult` 校验结果属于当前项目。
- 规则检查：避免跨项目复用旧结果；保留原有命令历史 50 条策略；未改 UI 结构。
- 结论：通过。

### `frontend/src/api/osvScanner.test.ts`

- 改动摘要：新增 Web fallback 每项目覆盖保存和删除测试。
- 规则检查：覆盖关键边界，不依赖真实 OSV 后端。
- 结论：通过。

### `frontend/src/stores/osvScanner.test.ts`

- 改动摘要：新增加载恢复最近结果、扫描后保存完整结果、删除项目时清理最近结果测试。
- 规则检查：覆盖 store 与 API 的核心协作。
- 结论：通过。

## 红线合规性

| 红线 | 是否违反 | 说明 |
|------|----------|------|
| `SEC-002` 禁止拼接 SQL | 否 | SQLite 写入/读取使用 bind 参数 |
| `VALID-003` 强类型对象流传 | 否 | 使用 `OsvScanResult`、`OsvScannerSettings` 等强类型 |
| `VUE-003` 前端请求层收口 | 否 | 新 API 位于 `frontend/src/api/osvScanner.ts` |
| 完整结果不得按 50 条历史截断 | 否 | `osv_latest_scan_results` 每项目覆盖保存完整 JSON |
| 禁止无关重构 | 否 | 未改 AgentSkills、VLESS、路由结构 |

## 边界场景推演

| 场景 | 结论 | 说明 |
|------|------|------|
| 正向场景 | 通过 | 扫描完成后保存完整结果，重新进入项目可恢复 |
| 回归场景 | 通过 | 命令历史仍保留 50 条轻量记录 |
| NULL / 空数据 | 通过 | 无最近结果返回 `null`，页面显示空态 |
| 并发 / 状态变化 | 可接受 | store 恢复结果时校验当前项目路径，避免异步切换串项目 |
| 删除项目 | 通过 | 删除项目后同步删除该项目最近结果 |

## 性能影响

- 调用频率：加载/切换项目读取一次，扫描完成写入一次，删除项目删除一次。
- 数据量级：每个项目一条完整结果。
- 索引 / JOIN 影响：无 JOIN；主键读取，写入成本低。

## 验证记录

- 已运行：`cargo test -p rust_tool_core storage`
- 已运行：`cargo check -p rust_tool_desktop`
- 已运行：`pnpm --dir frontend test:run`
- 已运行：`pnpm --dir frontend build`
- 已运行：`git diff --check`
- 已运行：in-app Browser 打开 `/osv-scanner` 和项目工作区，确认页面、结果空态和 console error。
- 未运行：桌面 App 内真实 OSV 扫描端到端验收。

## 结论

本次变更符合用户纠正后的存储模型，未发现阻断问题；建议用户在桌面 App 中用真实项目执行一次扫描作为最终验收。
