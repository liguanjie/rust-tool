# sub2api 管理模块实施计划

## 模块目标

在 Windows 工作台中管理本机 sub2api：启动、停止、升级、健康检查，并支持维护认证信息。

该模块面向本机开发环境，不直接内置 sub2api 业务逻辑，而是通过用户配置的脚本进行受控执行。

## 当前已落地

关键文件：

```text
frontend/src/pages/WindowsWorkbench.vue
frontend/src/stores/windowsWorkbench.ts
frontend/src/api/workbench.ts
frontend/src-tauri/src/workbench.rs
frontend/src-tauri/src/lib.rs
```

已完成配置项：

```text
sub2apiStartScript
sub2apiStopScript
sub2apiUpgradeScript
sub2apiWorkingDir
sub2apiHealthUrl
sub2apiLoginUrl
sub2apiUsername
sub2apiPassword
```

已完成能力：

- 选择启动脚本。
- 选择停止脚本。
- 选择升级脚本。
- 选择工作目录。
- 配置健康检查地址，默认 `http://127.0.0.1:9999/v1/models`。
- 配置登录地址，默认 `http://127.0.0.1:9999/api/auth/login`。
- 配置用户名和密码。
- 健康检查遇到需要认证时，后端可先登录换取 token。
- 健康检查请求自动带 `Authorization: Bearer <token>`。
- 启动、停止、升级动作写入任务记录。
- 页面加载时如已配置，会自动检测健康状态。

## 业务判断

sub2api 的“启动、停止、升级”不应让前端传任意命令。

推荐方式：

- 前端只传 `start`、`stop`、`upgrade`。
- 后端从 SQLite 读取对应脚本路径。
- 后端校验脚本类型和工作目录。
- 后端执行白名单脚本。

这样既满足双击式工具使用体验，也避免把命令执行能力暴露给页面。

## 实施步骤

### 阶段 1：配置抽屉

1. 在 sub2api 卡片右上角提供齿轮入口。
2. 将启动、停止、升级脚本配置放入抽屉。
3. 将工作目录、健康检查地址、登录地址、账号、密码放入抽屉。
4. 保存配置到 SQLite。

### 阶段 2：脚本选择与校验

1. 支持 `.bat`、`.cmd`、`.ps1`、`.exe`。
2. 不允许不存在的脚本路径保存。
3. 工作目录必须是有效目录。
4. 前端只显示路径，不直接执行。

### 阶段 3：任务执行

1. 用户点击启动时，前端传 `start`。
2. 后端读取 `sub2apiStartScript`。
3. 按脚本类型构造执行命令。
4. 记录开始时间。
5. 执行完成后记录退出码、stdout、stderr。
6. 前端刷新最近任务。

### 阶段 4：健康检查与认证

1. 无账号密码时，直接请求健康检查地址。
2. 有账号密码时，先请求登录地址。
3. 从响应中解析 token、access_token、accessToken 或 data 下的 token。
4. 使用 Bearer token 请求健康检查地址。
5. 401 不展示成网络不通，而是提示认证失败或配置缺失。

## 验收标准

- 未配置脚本时，页面显示未配置。
- 配置脚本后，启动、停止、升级按钮可以执行对应任务。
- 健康检查成功时显示运行中。
- 健康检查 401 时给出认证相关提示。
- 用户填写账号密码后，健康检查可以先登录再请求 `/v1/models`。
- 任务记录能展示最近执行结果。
- 浏览器 Web 预览不执行本机脚本，并给出 Tauri 桌面版提示。

## 风险与约束

- 当前账号密码保存在 SQLite 明文 JSON 中，只适合本机早期开发。
- 后续建议接入 Windows Credential Manager 或系统密钥链。
- 脚本自身的安全性由用户维护，RustTool 只做路径和类型限制。

## 后续扩展

- 增加 sub2api 日志查看。
- 增加端口占用检测。
- 增加一键打开工作目录。
- 增加 Docker 方式部署 sub2api。
- 增加配置导入导出。
