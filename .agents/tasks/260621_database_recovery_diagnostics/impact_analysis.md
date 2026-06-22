# 数据库压缩、恢复与诊断 — 影响分析

## 影响范围

- Core storage：新增数据库压缩、备份恢复文件替换、记录数诊断。
- Tauri：新增压缩数据库、从备份恢复、删除旧库命令。
- 前端 API：新增压缩、恢复、旧库清理 API 与诊断类型。
- 程序配置页：新增维护按钮、旧库状态、存储诊断卡片。

## 数据安全

- 恢复备份前会自动备份当前数据库到 `backups/rusttool-before-restore-*.db`。
- 恢复失败时会尝试回滚到恢复前备份。
- 删除旧库只删除固定文件名：
  - `rusttool.sqlite`
  - `rusttool.sqlite-wal`
  - `rusttool.sqlite-shm`
- 不支持旧库迁移，符合用户“后续不再支持旧库”的决策。

## 兼容性

- Web 模式下本地文件维护动作仍不可用。
- `ProgramSettingsState` 增加 `databaseDiagnostics` 和 `legacyDatabase` 字段，Web fallback 已补默认值。
- 原路径保存、刷新、备份、清理历史流程保留。

## 风险

- 恢复数据库会替换当前数据库文件；虽然已自动备份，仍需要用户在桌面 App 内确认后触发。
- 压缩数据库期间 SQLite 会短暂锁定，适合用户手动维护时执行。
- Browser 验证运行在 Web 模式，真实 Tauri 文件对话框与恢复动作仍需用户桌面端实测。
