# 程序配置页 — 影响分析

## 影响模块

- 桌面端配置：`rusttool-settings.json` 增加 `program.databasePath` 字段。
- 前端页面：新增 `/program-settings` 页面。
- 导航入口：左侧菜单和工作台新增程序配置入口。

## 上下游链路

- 上游：用户在程序配置页输入或选择数据库文件路径。
- 当前下游：配置持久化到桌面 JSON 或 Web localStorage。
- 未来下游：SQLite 初始化模块读取生效路径并打开数据库。

## 数据影响

- 既有 JSON 缺少 `program` 字段时，Serde 通过 `#[serde(default)]` 自动补默认值。
- 不迁移现有 VLESS、OSV、AgentSkills 数据。
- 不创建真实数据库文件，不生成 DDL。

## 权限 / 菜单 / 配置影响

- 菜单：新增“程序配置”本地入口，无后端权限体系变更。
- 配置：新增数据库路径配置；空值表示使用默认路径。
- 桌面权限：复用已安装的 dialog 插件选择目录。

## 报表 / 定时任务 / 外部接口影响

- 报表：无影响。
- 定时任务：无影响。
- 外部接口：无新增网络接口。

## 回归风险

- 路由新增不会覆盖原有 `/dashboard`、`/agent-skills`、`/osv-scanner`、`/toolbox/vless-to-mihomo`。
- 工作台卡片从三列调整为四列，大屏为四列布局，中屏仍为两列。
- Tauri 设置读写仍复用原 JSON 文件，需关注后续真实 SQLite 接入时的路径校验。
