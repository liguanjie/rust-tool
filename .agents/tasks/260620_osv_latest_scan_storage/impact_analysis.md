# OSV 最近完整扫描结果持久化 — 影响分析

## 影响范围

| 模块 | 影响 | 说明 |
|------|------|------|
| SQLite schema | 有新增 | 新增 `osv_latest_scan_results` 表和 `scanned_at` 索引，`user_version = 3` |
| Rust core | 有新增 | 新增 OSV 最近扫描结果仓储 API |
| Tauri 桌面端 | 有新增 | 新增 3 个命令：读取、保存、删除最近完整扫描结果 |
| 前端 API | 有新增 | 新增 `get/save/deleteOsvLatestScanResult`，Web fallback 单独存储 |
| OSV store | 有修改 | 加载/切换/扫描/删除项目时同步最近结果 |
| AgentSkills | 无改动 | 保持最近 50 条执行历史策略 |

## 数据库影响

- 表：`osv_latest_scan_results`
- 主键：`project_path`
- 关键字段：
  - `result_json`：完整 `OsvScanResult`
  - `scanned_at`：优先取命令完成时间，否则取开始时间
  - `updated_at`：数据库写入更新时间
- 写入策略：`ON CONFLICT(project_path) DO UPDATE`，同一项目只覆盖最近一次。
- SQL 安全：所有动态值均使用 `sqlx::query(...).bind(...)` 参数绑定。

## 数据策略

- 完整扫描结果：每个项目一条，覆盖保存，不截断。
- 命令历史：轻量操作历史，继续保留最近 50 条。
- 项目摘要：仍由现有设置保存 `lastScanned`、`healthScore`，用于大盘展示。

## 兼容性

- 桌面端：使用当前程序配置的 SQLite 路径。
- Web 模式：使用 `rusttool:osv-scanner:latest-results`，不依赖 SQLite。
- 无历史结果：返回 `null`，页面保持原“暂无风险数据”状态。

## 性能影响

- 读取：进入或切换项目时按主键读取一条 JSON。
- 写入：扫描完成后按主键 upsert 一条 JSON。
- 数据量：与监控项目数量相关，不随扫描次数无限增长。

## 回滚方案

- 前端可移除 `get/save/deleteOsvLatestScanResult` 调用，恢复原内存 `latestResult` 行为。
- SQLite 表为新增表，回滚代码后不会影响现有 settings JSON 或 AgentSkills 历史。

## 残余风险

- 极大项目的完整漏洞 JSON 可能占用较多 SQLite 空间。
- 目前没有保留多次扫描快照；若后续需要趋势分析，应另建聚合指标表或有限快照表，而不是复用轻量命令历史。
