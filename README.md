# RustTool

RustTool 是一个本地工具站，后端使用 Rust，前端使用 Vue。

第一期工具：将 `vless://` 链接转换为 Clash Party/Mihomo YAML。

## 技术选型

- Rust workspace
- Axum + Tokio
- Clap CLI
- Vue 3 + Vite + TypeScript
- Vue Router + Pinia
- Tailwind CSS
- Vitest

## 目录结构

```text
crates/rust_tool_core   Core conversion logic
crates/rust_tool_server Axum API server and frontend static hosting
crates/rust_tool_cli    Command-line wrapper
frontend                Vue 3 + Vite + TypeScript app
```

## CLI 使用

```powershell
cargo run -p rust_tool_cli -- convert-vless "vless://..."
cargo run -p rust_tool_cli -- convert-vless --proxy-only "vless://..."
cargo run -p rust_tool_cli -- convert-vless -o mihomo.yaml "vless://..."
```

## 本地开发

最简单的方式：双击项目根目录的 `start.bat`，然后按数字选择功能。

命令行方式：

```powershell
rt install
rt dev
```

`rt dev` 会分别打开后端和前端窗口：

```text
Backend:  http://127.0.0.1:5172
Frontend: http://127.0.0.1:5173
```

也可以单独启动：

```powershell
rt server
rt frontend
```

后端默认监听 `127.0.0.1:5172`。端口被占用时可以改端口：

```powershell
rt server --port 18080
$env:RUSTTOOL_SERVER_PORT = "18080"; rt dev
```

如果前端开发服务也要代理到新端口，启动前设置同一个 `RUSTTOOL_SERVER_PORT` 即可。

## 本地 API

RustTool 后端提供本机 REST API，默认只监听 `127.0.0.1`。

```text
GET  /api/health
POST /api/tools/vless-to-mihomo
GET  /api/clash-party/health
GET  /api/clash-party/state
POST /api/clash-party/nodes/check
POST /api/clash-party/subscriptions/switch
POST /api/clash-party/nodes/switch
```

Clash Party / Mihomo API 默认读取：

```powershell
$env:RUSTTOOL_CLASH_PARTY_API_URL = "http://127.0.0.1:9998"
$env:RUSTTOOL_CLASH_PARTY_API_SECRET = "your-secret"
$env:RUSTTOOL_CLASH_PARTY_DATA_DIR = "$env:APPDATA\mihomo-party"
$env:RUSTTOOL_CLASH_PARTY_DELAY_TIMEOUT_MS = "5000"
$env:RUSTTOOL_CLASH_PARTY_DELAY_TEST_URL = "https://www.gstatic.com/generate_204"
```

切换节点前会主动调用 Mihomo 节点测速接口。检测超时或不可用时，RustTool 会拒绝切换。

检测节点：

```powershell
Invoke-RestMethod -Method Post -ContentType "application/json" `
  -Uri "http://127.0.0.1:5172/api/clash-party/nodes/check" `
  -Body '{"nodeName":"raw-node-name"}'
```

切换订阅：

```powershell
Invoke-RestMethod -Method Post -ContentType "application/json" `
  -Uri "http://127.0.0.1:5172/api/clash-party/subscriptions/switch" `
  -Body '{"subscriptionId":"your-profile-id"}'
```

切换节点：

```powershell
Invoke-RestMethod -Method Post -ContentType "application/json" `
  -Uri "http://127.0.0.1:5172/api/clash-party/nodes/switch" `
  -Body '{"groupName":"PROXY","nodeName":"raw-node-name"}'
```

## 常用命令

```powershell
rt install    # 安装前端依赖
rt dev        # 同时启动后端和前端开发服务
rt server     # 只启动 Rust 后端
rt frontend   # 只启动 Vue 前端
rt test       # 跑 Rust + 前端测试
rt build      # 构建前端 + Rust release exe
rt run        # 运行 release 后端服务
rt clean      # 清理构建产物
```

## 构建检查

推荐：

```powershell
rt test
rt build
```

手动命令：

```powershell
cargo test
cd frontend
pnpm run test:run
pnpm run build
```

发布时先执行 `npm run build`，再启动 `rust_tool_server`，后端会托管 `frontend/dist`。

## 当前验证状态

前端已通过：

```powershell
npm run test:run
npm run build
```

当前机器的 PATH 中没有检测到 `cargo`/`rustc`，Rust 侧需要安装 Rust 工具链后再运行 `cargo test`。
