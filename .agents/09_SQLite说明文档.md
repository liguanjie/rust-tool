# SQLite 说明文档

## 数据库定位

RustTool 桌面端使用 SQLite 保存本机配置和任务执行记录。

数据库文件名：

```text
rusttool.sqlite
```

数据库不放在项目目录，而是放在 Tauri 应用数据目录，避免开发目录清理、打包或 Git 操作影响用户数据。

Windows 当前路径示例：

```text
C:\Users\ben\AppData\Roaming\com.ben.rusttool\rusttool.sqlite
```

代码定位：

```text
frontend/src-tauri/src/lib.rs
frontend/src-tauri/src/workbench.rs
```

路径来源：

```rust
app.path().app_data_dir()
```

## 当前表结构

### app_settings

用于保存各功能模块的配置。

```sql
CREATE TABLE IF NOT EXISTS app_settings (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
```

字段说明：

```text
key        配置项唯一标识
value      JSON 字符串
updated_at Unix 秒级时间戳字符串
```

当前已使用 key：

```text
toolbox.vless_to_mihomo.settings
windows_workbench.config
```

### task_runs

用于保存本机任务执行记录，例如 Docker 启动、停止、重启，sub2api 启动、停止、升级。

```sql
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

字段说明：

```text
id          自增主键
task_key    任务标识，例如 docker.start、sub2api.start
status      任务状态，例如 started、success、failed
started_at  开始时间，Unix 秒级时间戳字符串
finished_at 结束时间，Unix 秒级时间戳字符串，可为空
exit_code   进程退出码，可为空
stdout      标准输出
stderr      标准错误
```

## 当前保存的数据类型

### VLESS 转 Mihomo 配置

key：

```text
toolbox.vless_to_mihomo.settings
```

value 示例结构：

```json
{
  "input": "vless://...",
  "mode": "full_config",
  "template": "full_rules",
  "downloadName": "mihomo",
  "directDomains": "github.com\nexample.com"
}
```

说明：

- `input` 保存用户输入的 VLESS 链接。
- 支持单条或多条 VLESS，一行一条。
- `mode` 保存输出模式。
- `template` 保存配置模板。
- `downloadName` 保存下载文件名。
- `directDomains` 保存特殊直连域名，每行一条。

### Windows 工作台配置

key：

```text
windows_workbench.config
```

value 示例结构：

```json
{
  "dockerDesktopPath": "C:\\Program Files\\Docker\\Docker\\Docker Desktop.exe",
  "dockerCliPath": "C:\\Program Files\\Docker\\Docker\\resources\\bin\\docker.exe",
  "sub2apiStartScript": "",
  "sub2apiStopScript": "",
  "sub2apiUpgradeScript": "",
  "sub2apiWorkingDir": "",
  "sub2apiHealthUrl": "http://127.0.0.1:9999/v1/models",
  "sub2apiLoginUrl": "http://127.0.0.1:9999/api/auth/login",
  "sub2apiUsername": "",
  "sub2apiPassword": ""
}
```

说明：

- Docker 路径用于启动、停止、重启和检测 Docker Desktop。
- sub2api 脚本路径用于启动、停止、升级。
- sub2api 健康检查地址用于判断服务是否可用。
- sub2api 登录地址、账号、密码用于健康检查前自动换取 token。

## 数据写入方式

### 配置写入

配置使用 upsert 写入：

```sql
INSERT INTO app_settings (key, value, updated_at)
VALUES (?1, ?2, ?3)
ON CONFLICT(key) DO UPDATE SET
  value = excluded.value,
  updated_at = excluded.updated_at;
```

这样每个模块只保留一份最新配置。

### 任务记录写入

任务执行前插入记录：

```sql
INSERT INTO task_runs (task_key, status, started_at)
VALUES (?1, ?2, ?3);
```

任务结束后更新记录：

```sql
UPDATE task_runs
SET status = ?1,
    finished_at = ?2,
    exit_code = ?3,
    stdout = ?4,
    stderr = ?5
WHERE id = ?6;
```

## 查看数据库

如果本机安装了 sqlite3，可以使用：

```bat
sqlite3 "%APPDATA%\com.ben.rusttool\rusttool.sqlite"
```

常用查看命令：

```sql
.tables
.schema app_settings
.schema task_runs
SELECT key, value, updated_at FROM app_settings;
SELECT id, task_key, status, started_at, finished_at, exit_code FROM task_runs ORDER BY id DESC LIMIT 20;
```

如果没有 sqlite3，也可以用 Python 只读查看：

```powershell
python -c "import sqlite3, os; p=os.path.expandvars(r'%APPDATA%\\com.ben.rusttool\\rusttool.sqlite'); c=sqlite3.connect('file:'+p+'?mode=ro', uri=True); print(c.execute('select name from sqlite_master where type=\"table\"').fetchall())"
```

## Web 环境回退策略

Tauri 桌面版使用 SQLite。

普通 Web 开发预览环境不能访问本机 SQLite，因此 VLESS 工具配置回退到 localStorage：

```text
rusttool:vless-to-mihomo:settings
```

Windows 工作台依赖本机程序和脚本执行能力，因此只能在 Tauri 桌面版中使用。

## 安全注意事项

当前 SQLite 是本机明文数据库。

需要特别注意：

- VLESS 链接可能包含服务器地址、UUID、公钥等敏感信息。
- sub2api 账号密码当前也会保存到 SQLite。
- 本机开发阶段可以接受，但长期使用建议接入系统密钥链。

后续建议：

```text
Windows：Credential Manager
macOS：Keychain
Linux：Secret Service / libsecret
```

可选改造方向：

- app_settings 继续保存普通配置。
- 敏感字段只保存引用 id。
- 真正的密码、token、API Key 放入系统密钥链。

## 迁移建议

当前没有 schema migrations 表。

后续如果表结构继续增长，建议新增：

```sql
CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at TEXT NOT NULL
);
```

迁移策略：

1. 每次 schema 变化新增版本号。
2. 启动时检查当前版本。
3. 按顺序执行缺失迁移。
4. 迁移完成后写入 `schema_migrations`。

## 当前设计取舍

选择 `app_settings(key, value)` 的原因：

- 首期模块少，配置结构变化快。
- JSON 适合保存页面表单配置。
- 不需要为每个工具单独建表。
- 新增工具时只要约定新的 key。

选择 `task_runs` 独立建表的原因：

- 任务记录是列表数据，不适合覆盖式保存。
- 后续需要分页、筛选、清理。
- stdout/stderr 需要保留排查问题。

## 后续扩展

- 增加数据库查看页面。
- 增加配置导入导出。
- 增加任务记录清理按钮。
- 增加敏感字段加密。
- 增加 schema migrations。
- 增加工具使用历史，例如最近转换的文件名和时间。
