# SQLite storage foundation — 影响分析

## 影响模块

- Rust core：新增 `storage` 模块和 SQLite migration。
- 桌面端：程序配置 command 从同步变为异步，并返回数据库健康状态。
- 前端：程序配置页新增健康状态展示。
- 构建依赖：新增 `sqlx 0.8.x` 及 SQLite 相关依赖。

## 上下游链路

- 上游：用户配置的数据库路径，仍来自 `rusttool-settings.json`。
- 中游：`rust_tool_core::storage` 负责打开数据库、执行 migration、读取 schema 状态。
- 下游：程序配置页展示健康状态；未来 Repository 会复用同一 storage 层。

## 数据影响

- 会在生效数据库路径创建 SQLite 文件。
- 第一版 migration 创建：
  - `app_metadata`
  - `app_events`
  - `tool_settings`
  - `export_records`
  - `_sqlx_migrations` 由 SQLx migrator 自动维护
- 不迁移现有 JSON/localStorage 数据。
- 不修改 OSV ignore 配置、导出报告、Clash Party 外部配置。

## 权限 / 菜单 / 配置影响

- 权限：无新增权限体系。
- 菜单：沿用上一轮“程序配置”入口，无新增菜单项。
- 配置：数据库路径继续保存在 bootstrap JSON 中，避免“打开数据库前要先读数据库”的循环依赖。

## 报表 / 定时任务 / 外部接口影响

- 报表：无影响。
- 定时任务：无影响。
- 外部接口：无新增 HTTP API。

## 回归风险

- `sqlx` 新依赖增加构建体积和首次编译耗时。
- 程序配置页在桌面端会初始化数据库文件；若用户配置到不可写路径，会显示健康异常。
- 后续迁移业务数据时，需要明确保留策略和回滚方案，避免把大报告或第三方配置塞进 SQLite。
