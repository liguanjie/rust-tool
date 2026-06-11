# RustTool

RustTool 是一个本地工具站，后端使用 Rust，前端使用 Vue。

第一期工具：将 `vless://` 链接转换为 Clash Party/Mihomo YAML。

新增工具：AI 安全文档。支持本地 Markdown 文档库、本地脱敏、敏感字段加密、OpenAI 兼容接口检索问答、备份与恢复。

## 技术选型

- Rust workspace
- Axum + Tokio
- Clap CLI
- Vue 3 + Vite + TypeScript
- Vue Router + Pinia
- Tailwind CSS
- Vitest

## 桌面与 Web 一致性

RustTool 的桌面版和 Web 版共用同一套 Vue 页面和 `rust_tool_core` 业务核心。Web 模式通过 HTTP API 调用 `rust_tool_server`；桌面模式通过 Tauri command 直接调用 Rust 核心，不依赖本地后端端口。AI 安全文档前端调用必须经过 `frontend/src/services/memoApi.ts` 适配层，新增接口时需要同步维护 HTTP 路由、Tauri command 和前端适配映射。长期规范见根目录 `AGENTS.md`，实施记录见 `.agents/06_桌面与Web一致性规范.md`。

后续使用 Codex 新窗口继续开发时，请先阅读根目录 `AGENTS.md`；AI 安全文档功能不得在桌面版和 Web 版之间拆成两套页面或两套业务逻辑。

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

### macOS / Unix

项目根目录提供 `./rt` 作为 macOS/Linux 开发入口：

```sh
./rt
./rt install
./rt dev
```

直接运行 `./rt` 会显示交互菜单，可按数字选择启动桌面版、开发服务、测试、安装依赖、构建桌面应用和清理构建产物等常用操作。

`./rt dev` 会同时启动后端和前端：

```text
Backend:  http://127.0.0.1:5172
Frontend: http://127.0.0.1:5173
```

高级调试时也可以单独启动：

```sh
./rt server
./rt frontend
```

后端默认监听 `127.0.0.1:5172`。端口被占用时可以改端口：

```sh
./rt server --port 18080
RUSTTOOL_SERVER_PORT=18080 ./rt dev
```

如果前端开发服务也要代理到新端口，启动前设置同一个 `RUSTTOOL_SERVER_PORT` 即可。

### Windows

Windows 下仍可使用：

```powershell
rt install
rt dev
rt server
rt frontend
```

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
POST /api/memo/unlock
POST /api/memo/lock
GET  /api/memo/status
POST /api/memo/settings
GET  /api/memo/data-dir
POST /api/memo/data-dir/migrate
POST /api/memo/test-connection
GET  /api/memo/list
GET  /api/memo/doc/:id
GET  /api/memo/secrets
POST /api/memo/secrets/reveal
POST /api/memo/change-master-password
POST /api/memo/save
POST /api/memo/draft
POST /api/memo/delete
POST /api/memo/query
POST /api/memo/chat
POST /api/memo/backup
POST /api/memo/restore
POST /api/memo/translate-key
```

Clash Party / Mihomo API 默认读取：

```sh
export RUSTTOOL_CLASH_PARTY_API_URL="http://127.0.0.1:9998"
export RUSTTOOL_CLASH_PARTY_API_SECRET="your-secret"
export RUSTTOOL_CLASH_PARTY_DATA_DIR="$HOME/Library/Application Support/mihomo-party"
export RUSTTOOL_CLASH_PARTY_DELAY_TIMEOUT_MS="5000"
export RUSTTOOL_CLASH_PARTY_DELAY_TEST_URL="https://www.gstatic.com/generate_204"
```

切换节点前会主动调用 Mihomo 节点测速接口。检测超时或不可用时，RustTool 会拒绝切换。

检测节点：

```sh
curl -sS -X POST "http://127.0.0.1:5172/api/clash-party/nodes/check" \
  -H "content-type: application/json" \
  -d '{"nodeName":"raw-node-name"}'
```

切换订阅：

```sh
curl -sS -X POST "http://127.0.0.1:5172/api/clash-party/subscriptions/switch" \
  -H "content-type: application/json" \
  -d '{"subscriptionId":"your-profile-id"}'
```

切换节点：

```sh
curl -sS -X POST "http://127.0.0.1:5172/api/clash-party/nodes/switch" \
  -H "content-type: application/json" \
  -d '{"groupName":"PROXY","nodeName":"raw-node-name"}'
```

## AI 安全文档

AI 安全文档默认使用本机资料库根目录：

```sh
export RUSTTOOL_DATA_DIR="$HOME/RustToolData" # 可选
```

未设置时，macOS 默认使用 `~/Library/Application Support/rust-tool`，Windows 默认使用 `%APPDATA%\rust-tool`，其他 Unix 系统默认使用 `$XDG_DATA_HOME/rust-tool` 或 `~/.local/share/rust-tool`。

资料库内当前结构：

```text
config.json  非敏感配置、主密码校验材料和资料库迁移指针
documents/   Markdown 文档库，frontmatter 保存文档元数据
embeddings/  向量缓存文件，可按需重建
secrets.kdbx KDBX 密码库，用于保存 API Key 和文档 secret
```

文档列表以 `documents/**/*.md` 为准。用户直接在 `documents/` 下新增 Markdown 文件后，应用会在文档列表中显示；应用保存的文档会自动写入 frontmatter，记录 `id`、`title`、`summary` 和 `updatedAt`。

安装后默认使用系统应用数据目录，普通用户无需配置即可使用。设置页提供资料库目录迁移：迁移时会要求保密库已解锁，自动创建备份，复制 `config.json`、`documents/`、`embeddings/` 和 `secrets.kdbx` 到新目录，并把新路径写入默认目录下的 `config.json`。迁移成功后当前会话会锁定；重启应用后使用新目录，旧目录会保留。

默认 OpenAI 兼容接口配置：

```text
Base URL:                 https://api.openai.com/v1
Chat Model:               gpt-5.5
Embedding Model:          text-embedding-3-small
Reasoning Effort:         xhigh
Disable Response Storage: true
```

安全行为：

- 保密库需要先调用 `/api/memo/unlock` 解锁。锁定状态访问文档、密码库、检索、备份、恢复、设置保存等接口会返回 `401`。
- 明文主密码不会落盘；服务只在 `config.json` 中保存盐和加密校验 token。
- 解锁成功后会用同一个主密码打开或创建 `secrets.kdbx`；如果旧资料库没有文件版校验 token 但已有 `secrets.kdbx`，会先用 KDBX 验证密码并初始化新的校验材料。锁定后会从内存中释放 KDBX 对象和主密钥。
- AI 调用前会先在本地脱敏，密码、Token、API Key、私钥等会替换为 `{{secret:pending_n}}`，默认不会把明文 secret 发送给大模型。
- 文档正文以 Markdown 文件保存，敏感字段以 `{{secret:key}}` 占位；新保存的 secret 会写入 `secrets.kdbx`。
- 密码库页面只读取 secret 索引；明文值只在单项查看或复制时通过 `/api/memo/secrets/reveal` 临时解密返回，默认不会进入 AI 上下文。
- 主密码可在保险箱已解锁时修改。修改时会先创建本地备份，再更新 password verifier 和 KDBX 保存密码；成功后会自动锁定，要求使用新主密码重新解锁。
- `/api/memo/status` 只返回 `hasApiKey`，不会回显已保存的 API Key。设置页 API Key 留空会保留原值。
- 文档 `fileName` 必须是库内相对路径，不能使用绝对路径或 `..`；同名文件会被拒绝。
- 备份会包含 `config.json`、`documents/`、`embeddings/` 和 `secrets.kdbx`；恢复会先解压到临时目录并校验其中包含 RustTool 资料库数据，校验成功后才替换现有数据，并在恢复成功后回到锁定状态。
- 开启“允许 AI 检索并读取解密后的密码”后，解密 secret 可能随上下文发送给配置的大模型服务；云端模型场景建议保持关闭。

错误响应格式：

```json
{
  "error": {
    "code": "vault_locked",
    "message": "Vault is locked. Please unlock first."
  }
}
```

常见状态码：

```text
401 vault_locked  保密库未解锁
400 bad_request   参数、路径、备份包或文件名冲突错误
404 not_found     文档不存在
500 internal_error 服务端处理失败
```

## 常用命令

```sh
./rt                # 打开交互菜单
./rt install        # 安装前端依赖
./rt dev            # 同时启动后端和前端开发服务
./rt desktop        # 启动 Tauri 桌面开发版
./rt test           # 跑 Rust + 前端测试
./rt build-desktop  # 构建 Tauri 桌面应用
./rt clean          # 清理构建产物
```

高级命令仍然可直接输入：

```sh
./rt server    # 只启动 Rust 后端
./rt frontend  # 只启动 Vue 前端
./rt build     # 构建前端 + Rust release 二进制
./rt run       # 运行 release 后端服务
```

## 构建检查

推荐：

```sh
./rt test
./rt build
```

手动命令：

```sh
cargo test
cd frontend
pnpm run test:run
pnpm run build
```

发布时先执行 `pnpm run build`，再启动 `rust_tool_server`，后端会托管 `frontend/dist`。

## 当前验证状态

前端已通过：

```sh
pnpm run test:run
pnpm run build
```

当前机器的 PATH 中没有检测到 `cargo`/`rustc`，Rust 侧需要安装 Rust 工具链后再运行 `cargo test`。
