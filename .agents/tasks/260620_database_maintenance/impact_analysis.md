# 数据库维护能力 — 影响分析

## 影响范围

- Core storage：新增数据库文件大小统计、SQLite 一致性备份、OSV 命令历史清理能力。
- Tauri：新增 `backup_program_database`、`open_program_database_directory`、`clear_program_database_history` 三个命令。
- 前端 API：`ProgramSettingsState` 增加 `databaseStats`，新增维护动作封装。
- 程序配置页：新增数据库大小展示和维护动作按钮。

## 数据安全

- 备份使用 SQLite `VACUUM INTO`，避免直接复制 WAL 模式下的不完整主库文件。
- 清理历史只删除：
  - `agent_execution_history`
  - `osv_command_history`
- 清理历史不会删除：
  - OSV 项目清单
  - 每个项目的最后一次完整扫描结果
  - 程序配置 JSON 中的数据库路径

## 兼容性

- Web fallback 下维护动作不执行本地文件操作。
- 清理 OSV 命令历史时同步清理 JSON 快照中的 `commandHistory`，避免空数据库边界下旧历史被重新迁入。
- 原 `getProgramSettings` / `saveProgramSettings` 调用仍可工作，只是返回结构增加 `databaseStats` 字段。

## 风险

- “打开目录”依赖操作系统文件管理器命令：macOS `open`、Windows `explorer`、Linux `xdg-open`。
- 清理历史后 SQLite 文件大小不一定立刻下降；当前提供备份能力，后续可补显式压缩/VACUUM。
