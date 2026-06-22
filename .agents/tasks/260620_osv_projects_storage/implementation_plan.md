# OSV 项目清单与命令历史 SQLite 存储实施计划

## 背景

OSV 扫描器当前已将每个项目的最后一次完整扫描结果存入 SQLite，但项目清单与轻量命令历史仍保存在桌面 JSON 配置中。随着项目数量、扫描次数增加，这类运行数据继续堆在配置文件中会让配置职责变重，也不利于后续查询和维护。

## 目标

- 新增 SQLite migration，持久化 OSV 项目清单与命令历史。
- 保持前端 `getOsvSettings` / `saveOsvSettings` API 形状不变，降低页面改动面。
- `autoScanSchedule` 保持在桌面配置 JSON 中，作为程序偏好继续管理。
- 首次读取时兼容旧 JSON 数据，将已有项目清单和命令历史迁入 SQLite。
- 命令历史继续限制为最近 50 条。

## 实施步骤

1. 新增 `osv_projects`、`osv_command_history` 数据表与索引。
2. 在 core storage 层新增项目清单和命令历史的保存、读取、替换逻辑。
3. 调整 Tauri `get_osv_settings` / `save_osv_settings`，从 SQLite 读写运行数据，从 JSON 读写扫描计划配置。
4. 补充 storage 单元测试，覆盖迁移版本、项目顺序、命令历史裁剪。
5. 执行 Rust/前端测试与构建验证，更新交付文档和代码审查报告。

## 风险与兼容

- 旧 JSON 中的项目/命令历史需要自动迁移；迁移后仍保留 JSON 结构以便回滚和兼容。
- 删除所有项目后，需要同步写回空 JSON，避免下次读取时旧数据被再次导入。
- 项目列表展示顺序需要保持用户原有顺序，因此表中记录 `sort_order`。
