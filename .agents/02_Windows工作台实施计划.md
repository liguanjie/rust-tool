# Windows 工作台实施计划

## 目标

在 RustTool 中新增一个 Tauri 桌面端专属的 Windows 工作台，用于管理本机 AI 开发环境中的常用服务与脚本。第一版聚焦 Docker Desktop 与 sub2api，做到状态可见、配置可维护、操作可追踪。

该模块不做成普通网页功能。浏览器环境不能安全地启动本机程序或脚本，相关能力只在 Tauri 桌面版中开放。

## 产品定位

Windows 工作台不是命令按钮集合，而是本机服务控制台。

核心原则：

- 状态优先：先告诉用户服务是否可用。
- 常用操作外露：启动、停止、升级、检测放在卡片主界面。
- 配置收起：路径、脚本、健康检查地址放到齿轮配置抽屉。
- 执行受控：前端只传 task id，后端按白名单执行。
- 日志可追踪：每次执行记录命令类型、开始时间、结束时间、退出码和输出摘要。

## 首期功能范围

### Docker 专区

主卡片能力：

- 显示 Docker Desktop 状态。
- 显示 docker CLI 是否可用。
- 显示 Docker 版本或 docker info 摘要。
- 启动 Docker Desktop。
- 重新检测 Docker 状态。

配置抽屉：

- Docker Desktop 程序路径。
- docker CLI 路径。
- 自动侦测按钮。
- 手动选择程序路径。
- 保存配置。

自动侦测候选路径：

```text
C:\Program Files\Docker\Docker\Docker Desktop.exe
C:\Program Files\Docker\Docker\resources\bin\docker.exe
```

同时可尝试：

```text
where docker
```

### sub2api 专区

主卡片能力：

- 显示 sub2api 健康状态。
- 启动 sub2api。
- 停止 sub2api。
- 升级 sub2api。
- 健康检查。
- 显示最近一次执行结果。

配置抽屉：

- 启动脚本路径。
- 停止脚本路径。
- 升级脚本路径。
- 工作目录。
- 健康检查地址，例如 `http://127.0.0.1:9999/v1/models`。
- 保存配置。

脚本类型限制：

```text
.bat
.cmd
.ps1
.exe
```

## UI 设计建议

### 页面结构

```text
Windows 工作台
├─ Docker Desktop 卡片
└─ sub2api 卡片
```

卡片统一结构：

```text
左侧：服务名称、说明、状态标签
中间：关键配置摘要、版本、路径、端口或健康检查地址
右侧：常用操作按钮、齿轮配置入口
底部：最近一次运行结果或日志摘要
```

状态标签：

```text
未配置
已停止
启动中
运行中
异常
需要更新
```

按钮顺序：

```text
Docker：启动、检测
sub2api：启动、停止、升级、检测
```

### 配置入口

每个专区右上角放齿轮图标。点击齿轮打开右侧抽屉，不跳转新页面。

右侧抽屉适合承载路径选择、自动侦测、脚本选择、健康检查地址、保存按钮。保存后关闭抽屉或保留抽屉，并显示 Toast。

### 反馈设计

- 成功：绿色 Toast，显示动作和结果。
- 失败：红色 Toast，显示失败原因和下一步建议。
- 长任务：按钮进入 loading 状态，防止重复点击。
- 执行日志：卡片底部显示最近一条，详情后续可展开。

错误文案示例：

```text
启动失败：找不到启动脚本
建议：点击齿轮，选择 sub2api 的 start.bat
```

## UX 流程

### 首次使用

```text
打开 Windows 工作台
-> Docker 显示未配置
-> 点击自动侦测
-> 用户确认路径
-> 保存
-> 点击检测
```

sub2api：

```text
打开 sub2api 配置抽屉
-> 选择启动/停止/升级脚本
-> 填写健康检查地址
-> 保存
-> 点击启动
-> 点击健康检查
```

### 日常使用

```text
打开 Windows 工作台
-> 查看 Docker/sub2api 状态
-> 按需点击启动、停止、升级或检测
```

## 技术方案

### 前端

技术栈沿用现有项目：

```text
Vue 3
TypeScript
Pinia
Tailwind CSS
lucide-vue
```

新增页面：

```text
frontend/src/pages/WindowsWorkbench.vue
```

建议新增组件：

```text
frontend/src/components/workbench/ServiceCard.vue
frontend/src/components/workbench/ConfigDrawer.vue
frontend/src/components/workbench/PathPicker.vue
frontend/src/components/workbench/TaskLogList.vue
frontend/src/stores/windowsWorkbench.ts
```

### Tauri 后端

新增 Tauri commands：

```text
get_workbench_config()
save_workbench_config(config)
detect_docker()
get_docker_status()
start_docker()
run_sub2api_task(task)
check_sub2api_health()
list_task_runs(limit)
```

`run_sub2api_task(task)` 中的 `task` 只能是枚举：

```text
start
stop
upgrade
```

前端不能传任意命令。

### 本地数据库

建议引入 SQLite。

Rust 依赖：

```toml
rusqlite = { version = "0.32", features = ["bundled"] }
```

数据库文件放 Tauri app data 目录，避免写到项目目录。

用途：

- 保存 Docker 路径。
- 保存 sub2api 脚本路径。
- 保存健康检查地址。
- 保存任务运行记录。
- 保存用户偏好。

### 数据表初稿

```sql
CREATE TABLE app_settings (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE task_runs (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  task_key TEXT NOT NULL,
  status TEXT NOT NULL,
  started_at TEXT NOT NULL,
  finished_at TEXT,
  exit_code INTEGER,
  stdout TEXT,
  stderr TEXT
);
```

配置可以先用 key-value JSON 存储，后续稳定后再拆表。

建议 key：

```text
docker.desktop_path
docker.cli_path
sub2api.start_script
sub2api.stop_script
sub2api.upgrade_script
sub2api.working_dir
sub2api.health_url
```

## 安全策略

必须遵守：

- 前端只传任务 id，不传命令字符串。
- 后端只执行保存过的白名单路径。
- 执行前校验文件存在。
- 执行前校验扩展名。
- bat/cmd 执行使用 `cmd /c`，路径必须使用参数传递或严格引用。
- ps1 执行使用 `powershell -ExecutionPolicy Bypass -File`，仅对用户配置的明确脚本生效。
- 不支持任意命令输入框。
- 不自动提权。需要管理员权限时提示用户。

路径校验：

- Docker Desktop 必须是 `.exe`。
- docker CLI 必须是 `.exe`。
- sub2api 脚本必须是 `.bat`、`.cmd`、`.ps1` 或 `.exe`。

## 实施阶段

### 阶段 1：数据与后端基础

- 引入 SQLite。
- 建立 app data 数据库。
- 实现配置读写。
- 实现任务运行记录。
- 实现基础路径校验。

验收标准：

- Tauri 桌面版能读写配置。
- 配置重启后仍然存在。
- 无配置时返回明确状态。

### 阶段 2：Docker 专区

- 实现 Docker 自动侦测。
- 实现 Docker Desktop 启动。
- 实现 docker CLI 状态检测。
- 前端完成 Docker 卡片与配置抽屉。

验收标准：

- 未配置时提示自动侦测。
- 自动侦测能找到常见 Docker 路径。
- 点击启动能打开 Docker Desktop。
- 点击检测能显示 docker CLI 是否可用。

### 阶段 3：sub2api 专区

- 实现启动/停止/升级脚本配置。
- 实现脚本执行。
- 实现健康检查。
- 实现任务日志记录。
- 前端完成 sub2api 卡片与配置抽屉。

验收标准：

- 未配置脚本时按钮置灰或提示配置。
- 启动/停止/升级只能执行对应脚本。
- 健康检查能显示成功/失败。
- 最近一次执行结果可见。

### 阶段 4：体验打磨

- 统一 Toast。
- 统一状态标签。
- 增加日志详情抽屉或展开区。
- 增加按钮 loading 与防重复点击。
- 完善错误提示。

验收标准：

- 常见失败场景有明确提示。
- 用户不需要记命令。
- 日常操作路径清晰。

## 暂不做

第一版不做：

- Docker 容器列表管理。
- 任意命令执行器。
- 复杂计划任务。
- 多机器远程管理。
- 用户权限提升。
- Web 版执行本机程序。

这些可以在 Windows 工作台稳定后再扩展。

## 验证清单

- `cargo test`
- `pnpm run test:run`
- `pnpm run build`
- `pnpm run tauri:build`
- Windows 上双击 exe 验证 Docker 启动。
- Windows 上验证 sub2api start/stop/upgrade 脚本。
- 验证错误路径、缺失脚本、健康检查失败场景。

