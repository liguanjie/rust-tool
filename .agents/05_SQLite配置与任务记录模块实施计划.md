# SQLite 配置与任务记录模块实施计划

## 模块目标

为 RustTool 桌面端提供本地持久化能力，保存工具配置、工作台配置和任务执行记录。

数据库只服务本机应用，不作为远程服务数据库。首期重点是可靠、简单、可迁移。

## 当前已落地

关键文件：

```text
frontend/src-tauri/src/lib.rs
frontend/src-tauri/src/workbench.rs
frontend/src/api/tools.ts
frontend/src/api/workbench.ts
frontend/src/stores/vlessToMihomo.ts
frontend/src/stores/windowsWorkbench.ts
Cargo.toml
Cargo.lock
```

数据库位置：

```text
Tauri app data/rusttool.sqlite
Windows 当前示例：
C:\Users\ben\AppData\Roaming\com.ben.rusttool\rusttool.sqlite
```

已完成表：

```sql
CREATE TABLE IF NOT EXISTS app_settings (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS task_runs (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  task_key TEXT NOT NULL,
  status TEXT NOT NULL,
  started_at TEXT NOT NULL,
  finished_at TEXT,
  exit_code INTEGER,
  stdout TEXT NOT NULL DEFAULT '',
  stderr TEXT NOT NULL DEFAULT ''
);
```

已完成能力：

- 使用 Tauri app data 目录保存数据库。
- Web 环境回退到 localStorage。
- VLESS 工具配置保存到 `app_settings`。
- Windows 工作台配置保存到 `app_settings`。
- Docker/sub2api 执行记录保存到 `task_runs`。
- 任务记录包含任务类型、状态、开始时间、结束时间、退出码、stdout、stderr。

## 配置 Key 规划

当前已使用：

```text
toolbox.vless_to_mihomo.settings
windows_workbench.config
```

后续建议：

```text
toolbox.yaml_validator.settings
toolbox.config_merge.settings
workbench.docker.last_status
workbench.sub2api.last_health
app.ui.preferences
```

## 实施步骤

### 阶段 1：引入 SQLite

1. 在 Tauri crate 中引入 `rusqlite`。
2. 通过 `app.path().app_data_dir()` 定位数据目录。
3. 启动或访问配置时确保目录存在。
4. 打开 `rusttool.sqlite`。

### 阶段 2：建立通用配置表

1. 使用 `app_settings` 保存 JSON 配置。
2. 以 `key` 区分不同模块。
3. 使用 `updated_at` 记录最后更新时间。
4. 使用 upsert 保存配置。

### 阶段 3：建立任务记录表

1. 使用 `task_runs` 记录本机脚本和程序操作。
2. 执行前插入 started 记录。
3. 执行成功后更新为 success。
4. 执行失败后更新为 failed。
5. 前端读取最近 N 条展示在工作台。

### 阶段 4：前端状态持久化

1. 工具页面加载时读取配置。
2. 用户输入变化后 debounce 保存。
3. 点击转换前强制保存一次。
4. 桌面端使用 Tauri invoke，Web 端使用 localStorage。

## 验收标准

- Tauri 桌面版重启后配置仍存在。
- VLESS 输入和下载文件名可以恢复。
- Windows 工作台路径配置可以恢复。
- 执行 Docker/sub2api 操作后产生任务记录。
- Web 预览环境不依赖 SQLite，仍可使用 localStorage。

## 风险与约束

- 当前密码类配置以普通 JSON 存入 SQLite，适合本机早期开发，不适合长期保存敏感凭据。
- 后续如果保存 API Key、账号密码，应引入系统密钥链或至少做加密封装。
- `app_settings.value` 是 JSON，迁移时需要考虑版本号。

## 后续扩展

- 增加 `schema_migrations` 表。
- 增加配置版本字段。
- 增加敏感字段加密。
- 增加任务记录清理策略。
- 增加数据库查看/导出功能。
