# OSV 项目清单与命令历史 SQLite 存储影响分析

## 影响范围

- 数据库：新增 migration `0004_osv_projects_and_command_history.sql`，schema version 升至 4。
- Core：`storage.rs` 新增 OSV 项目清单、命令历史 repository。
- 桌面端：`get_osv_settings` / `save_osv_settings` 改为异步命令，返回结构保持不变。
- 前端页面：无 UI 结构变化，现有 API 调用不需要调整。

## 数据流变化

- `autoScanSchedule`：继续保存在桌面 JSON 配置中。
- `projects`：改为保存在 SQLite `osv_projects`。
- `commandHistory`：改为保存在 SQLite `osv_command_history`，并保留最近 50 条。
- 旧 JSON 项目和命令历史：当 SQLite 中尚无项目/命令历史时，首次读取自动导入。

## 兼容性

- 前端 `OsvScannerSettings` 返回格式不变。
- Web fallback localStorage 不变。
- 保存设置时会同步写回 JSON 快照，避免删除全部项目后旧 JSON 被再次导入。

## 风险

- 若用户手动切换到一个全新空数据库，系统会从 JSON 快照导入已有 OSV 项目和历史。这符合“配置跟随用户”的当前策略；若未来需要完全隔离数据库，可增加“导入/清空”显式动作。
- 当前命令历史仍按轻量记录保存，不包含完整扫描结果；完整结果继续由 `osv_latest_scan_results` 存储。
