# OSV 最近完整扫描结果持久化 — 走查报告

## 变更概览

- 新增 SQLite migration `0003_osv_latest_scan_results.sql`，每个项目路径保存一条最近完整扫描结果。
- 在 `rust_tool_core::storage` 中新增 OSV 最近结果仓储：保存、读取、删除。
- 在 Tauri 桌面入口新增 `get/save/delete_osv_latest_scan_result` 三个命令，统一使用程序配置页的有效 SQLite 路径。
- 在 `frontend/src/api/osvScanner.ts` 中新增 OSV 最近结果 API，桌面端走 SQLite，Web 端使用 `rusttool:osv-scanner:latest-results` fallback。
- 在 `useOsvScannerStore` 中接入：加载/切换项目恢复最近结果，扫描完成保存完整结果，删除项目删除对应结果。
- 命令历史仍保留 50 条轻量记录，不作为完整扫描结果的存储载体。

## 关键文件

- `crates/rust_tool_core/migrations/0003_osv_latest_scan_results.sql`
- `crates/rust_tool_core/src/storage.rs`
- `crates/rust_tool_core/src/lib.rs`
- `frontend/src-tauri/src/lib.rs`
- `frontend/src/api/osvScanner.ts`
- `frontend/src/api/osvScanner.test.ts`
- `frontend/src/stores/osvScanner.ts`
- `frontend/src/stores/osvScanner.test.ts`

## 核心流程

1. 用户执行 OSV 扫描后，`scanOsvProject` 返回完整 `OsvScanResult`。
2. store 写入 `latestResult`，追加轻量命令历史，更新项目摘要。
3. store 调用 `saveOsvLatestScanResult(result)`，桌面端写入 SQLite，Web 端写入 localStorage fallback。
4. 用户刷新页面、重新进入或切换项目时，store 调用 `getOsvLatestScanResult(path)` 恢复该项目最近完整结果。
5. 用户删除项目时，store 调用 `deleteOsvLatestScanResult(path)` 清理该项目的最近结果。

## 验证结果

| 类型 | 命令 / 方法 | 结果 |
|------|-------------|------|
| Rust 仓储测试 | `cargo test -p rust_tool_core storage` | 通过，7 tests |
| 桌面编译 | `cargo check -p rust_tool_desktop` | 通过 |
| 前端测试 | `pnpm --dir frontend test:run` | 通过，38 tests |
| 前端构建 | `pnpm --dir frontend build` | 通过；仅保留既有 Rollup PURE 注释与 chunk size warning |
| 空白检查 | `git diff --check` | 通过，无输出 |
| UI 验证 | `pnpm --dir frontend dev --host 127.0.0.1` + in-app Browser | `/osv-scanner` 大盘和项目工作区可打开；结果空态正常；console error 为 0 |

## 风险与注意事项

- 每个项目只保留最近一次完整扫描结果，不累积历史扫描快照。
- 完整结果以 JSON 存储在 `osv_latest_scan_results.result_json`，如果单次扫描结果很大，SQLite 文件会相应增长；但增长规模与项目数线性相关。
- Web dev 环境没有真实 OSV 后端扫描接口，本次 UI 验证覆盖页面加载和空态；完整扫描持久化由单元测试、Rust 测试和 Tauri 编译验证。

## 待用户验证

- 在桌面 App 中选择一个真实项目执行 OSV 扫描。
- 退出/刷新后重新进入该项目，确认“执行与结果”可恢复最近一次扫描结果。
- 删除项目后重新添加同一路径，确认旧最近结果不会继续残留。
