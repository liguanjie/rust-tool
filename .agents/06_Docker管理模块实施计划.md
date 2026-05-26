# Docker 管理模块实施计划

## 模块目标

在 Windows 工作台中管理本机 Docker Desktop，降低日常开发时手动查找、启动、检测 Docker 的成本。

该模块只在 Tauri 桌面版开放，Web 预览页不执行本机命令。

## 当前已落地

关键文件：

```text
frontend/src/pages/WindowsWorkbench.vue
frontend/src/stores/windowsWorkbench.ts
frontend/src/api/workbench.ts
frontend/src-tauri/src/workbench.rs
frontend/src-tauri/src/lib.rs
```

已完成能力：

- Docker Desktop 路径配置。
- docker CLI 路径配置。
- 自动侦测常见 Docker 路径。
- 手动选择 `.exe`。
- 检测 Docker Desktop 进程状态。
- 检测 docker CLI 是否可用。
- 检测 Docker Engine 是否运行。
- 显示 Docker 版本摘要。
- 启动 Docker Desktop。
- 停止 Docker Desktop 相关进程。
- 重启 Docker Desktop。
- 页面加载后如果已配置路径，会自动检测状态。
- 根据运行状态调整按钮文案，例如运行中显示“已运行”。

## 状态模型

前端状态：

```text
desktopConfigured
cliConfigured
desktopRunning
cliAvailable
engineRunning
version
message
```

UI 解释：

- `engineRunning = true`：Docker 真正可用。
- `desktopRunning = true` 但 `engineRunning = false`：Docker Desktop 已打开，但 Engine 还没就绪。
- `cliAvailable = false`：docker.exe 路径可能未配置或不可执行。

## 实施步骤

### 阶段 1：配置入口

1. 在 Docker 卡片右上角提供齿轮入口。
2. 配置抽屉维护 Docker Desktop 路径和 docker CLI 路径。
3. 支持自动侦测。
4. 支持手动选择可执行文件。
5. 保存到 SQLite。

### 阶段 2：状态检测

1. 判断配置是否存在。
2. 检查 Docker Desktop 进程。
3. 执行 `docker version` 或等效命令确认 CLI 可用。
4. 执行 `docker info` 或等效命令确认 Engine 可用。
5. 将检测结果统一返回给前端。

### 阶段 3：管理动作

1. 启动：调用 Docker Desktop exe。
2. 停止：停止 Docker Desktop 相关进程。
3. 重启：先停止再启动。
4. 检测：仅刷新状态，不改变本机服务。
5. 每次动作写入 `task_runs`。

### 阶段 4：交互优化

1. 页面加载时自动检测已配置服务。
2. 操作后延迟刷新状态。
3. 执行中禁用对应按钮。
4. 用 toast 显示结果。
5. 最近任务区显示操作记录。

## 验收标准

- 未配置时提示用户配置或自动侦测。
- 已安装 Docker 时自动侦测能填入常见路径。
- Docker 运行时页面显示运行中。
- Docker 运行时启动按钮显示“已运行”并禁用。
- 停止后状态刷新为未运行或 Engine 未就绪。
- 重启后能重新拉起 Docker Desktop。
- 每次启动、停止、重启都有任务记录。

## 安全约束

- 前端不能传任意命令。
- 后端只执行内置 Docker 操作。
- Docker Desktop 路径必须是 `.exe`。
- 不在 Web 浏览器环境执行本机程序。

## 后续扩展

- Docker 容器列表。
- 常用容器启动/停止/重启。
- Docker Compose 项目管理。
- 查看容器日志。
- 清理 dangling images。
