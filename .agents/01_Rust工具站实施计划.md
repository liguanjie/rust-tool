# Rust 工具站实施计划

## 架构结论

本项目采用「Rust 核心库 + Rust API 服务 + Vue 前端 + CLI」的结构。

第一期不使用类 Spring Boot 的重型全家桶框架。Rust 生态的最佳实践更偏向明确分层和组合成熟 crate：核心业务逻辑保持为可测试 library，Web 服务、命令行、前端页面只是不同入口。这样后续增加新工具时，不会把转换逻辑绑死在 HTTP 或 UI 里。

推荐技术栈：

- 核心逻辑：Rust library crate
- 后端 API：Axum + Tokio
- CLI：Clap
- 序列化：Serde + serde_yaml
- URL 解析：url
- 错误处理：thiserror
- 日志与链路：tracing + tracing-subscriber
- 前端：Vue 3 + Vite + TypeScript
- 前端路由：Vue Router
- 前端状态：Pinia
- 样式：Tailwind CSS
- 图标：@lucide/vue
- 测试：Rust 单元测试 + API 集成测试 + Vitest + 前端构建检查

## 为什么这样选

### 后端框架

Axum 是 Rust Web/API 项目里非常稳妥的选择。它基于 Tokio、Tower、Hyper 生态，适合构建清晰、可组合的 HTTP API。对工具站来说，Axum 足够轻，也不会牺牲工程化。

Loco.rs 更接近 Rails 风格的全栈框架，适合需要数据库模型、后台任务、邮件、用户系统、生成器的应用。如果本项目后续升级为带账号、历史记录、团队协作、数据库存储的平台，可以重新评估 Loco。但第一期只是工具站，用 Loco 会偏重。

不建议第一期使用 Rocket 或 Actix Web 作为默认方案：

- Rocket 开发体验好，但生态组合自由度不如 Axum 当前主流。
- Actix Web 性能强，但对这个项目不是主要约束，整体工程风格也没有 Axum + Tower 组合自然。

### 前端框架

可以使用 Vue，而且建议使用 Vue。

工具站会逐步增加多个工具，每个工具都有输入区、选项区、结果区、复制、下载、错误提示。Vue 3 + Vite + TypeScript 能让这些交互保持清晰。相比服务端拼 HTML，Vue 更适合后续扩展。

第一期不引入大型 UI 组件库，但直接引入 Vue Router、Pinia、Tailwind CSS、Vitest 和 @lucide/vue。它们属于长期结构性依赖，越晚补越容易重构页面边界。Naive UI、Arco Design Vue 或 Radix Vue 暂缓，因为第一期的表单、按钮、结果面板可以用轻量组件自建。

## 项目结构

推荐采用 workspace，后端、核心库、前端分开。

```text
RustTool/
  Cargo.toml
  crates/
    rust_tool_core/
      Cargo.toml
      src/
        lib.rs
        tools/
          mod.rs
          vless_to_mihomo.rs
    rust_tool_server/
      Cargo.toml
      src/
        main.rs
        app.rs
        routes/
          mod.rs
          health.rs
          tools.rs
        static_files.rs
    rust_tool_cli/
      Cargo.toml
      src/
        main.rs
  frontend/
    package.json
    vite.config.ts
    tsconfig.json
    src/
      main.ts
      App.vue
      api/
        tools.ts
      router/
        index.ts
      stores/
        tools.ts
        vlessToMihomo.ts
      components/
        AppSidebar.vue
        ToolShell.vue
        CopyButton.vue
        DownloadButton.vue
        ResultPanel.vue
      pages/
        VlessToMihomo.vue
      styles/
        base.css
  tests/
    fixtures/
      vless_reality.txt
      vless_ws.txt
      vless_grpc.txt
  .agents/
    01_Rust工具站实施计划.md
```

## 分层原则

### core

`rust_tool_core` 只负责纯业务逻辑：

- 输入 `vless://` 字符串。
- 解析为中间结构。
- 映射为 Mihomo 配置模型。
- 输出 YAML。

禁止在 core 中出现：

- HTTP request/response 类型。
- Vue 或前端概念。
- 文件系统副作用。
- 进程退出。
- `println!`。

### server

`rust_tool_server` 只负责 HTTP：

- 暴露 API。
- 校验请求体。
- 调用 core。
- 返回 JSON。
- 托管前端 `dist` 静态文件。

API 建议：

```text
GET  /api/health
POST /api/tools/vless-to-mihomo
```

请求体：

```json
{
  "input": "vless://...",
  "mode": "full_config"
}
```

响应体：

```json
{
  "yaml": "mixed-port: 7890\n..."
}
```

错误响应：

```json
{
  "error": {
    "code": "invalid_vless_url",
    "message": "链接必须以 vless:// 开头"
  }
}
```

### cli

`rust_tool_cli` 复用 core：

```powershell
rust-tool convert-vless "vless://..."
rust-tool convert-vless --proxy-only "vless://..."
rust-tool convert-vless -o mihomo.yaml "vless://..."
```

CLI 不直接复制 server 的逻辑，只调用同一个 core 函数。

### frontend

Vue 前端只负责交互：

- 输入 VLESS 链接。
- 选择完整配置或仅节点片段。
- 调用 API。
- 展示 YAML。
- 复制结果。
- 下载结果。
- 展示错误信息。

前端不重复实现转换逻辑，避免 Rust 和 TypeScript 两套规则不一致。路由、工具列表、页面状态从第一期就分层，新增工具时应新增页面、store、API 方法和工具元数据，而不是堆到单个 `App.vue`。

## 第一工具：VLESS 转 Mihomo

### 支持字段

第一期支持 3x-ui 常见字段：

- `uuid`
- `server`
- `port`
- `type` / `network`
- `security`
- `sni` / `servername`
- `fp` / `fingerprint` / `client-fingerprint`
- `flow`
- `pbk` / `public-key`
- `sid` / `short-id`
- `spx` / `spider-x`
- `path`
- `host`
- `serviceName` / `service-name`
- `mode`
- `authority`
- `alpn`
- `allowInsecure` / `skip-cert-verify`
- `packetEncoding` / `packet-encoding`

### 支持传输方式

- `tcp`
- `ws`
- `grpc`
- `httpupgrade`
- `xhttp`
- `h2`
- `http`，映射为 Mihomo 的 `h2`

### 输出模式

- `full_config`：完整 Mihomo 配置，可直接导入 Clash Party。
- `proxy_only`：只输出单个 proxy 节点，方便粘贴到已有配置。

## 数据模型建议

```rust
pub enum OutputMode {
    FullConfig,
    ProxyOnly,
}

pub struct ConvertOptions {
    pub output_mode: OutputMode,
}

pub fn convert_vless_to_yaml(input: &str, options: ConvertOptions) -> Result<String, ConvertError>;
```

错误类型建议：

```rust
pub enum ConvertError {
    EmptyInput,
    InvalidScheme,
    MissingUuid,
    MissingServer,
    InvalidPort,
    UnsupportedTransport(String),
    YamlSerializeFailed(String),
}
```

## 开发步骤

### 阶段 1：Workspace 与基础工程

1. 创建 Rust workspace。
2. 创建 `rust_tool_core`、`rust_tool_server`、`rust_tool_cli` 三个 crate。
3. 创建 Vue 3 + Vite + TypeScript 前端项目。
4. 配置统一格式化：
   - Rust: `cargo fmt`
   - Rust lint: `cargo clippy`
   - Frontend: `npm run build` 先作为最低校验
5. 添加 README 初稿。

### 阶段 2：核心转换库

1. 将现有 Python 脚本能力迁移到 `rust_tool_core`。
2. 建立 VLESS URL 解析模型。
3. 建立 Mihomo proxy/config 输出模型。
4. 使用 `serde_yaml` 输出 YAML。
5. 补齐 Reality、WS、gRPC、非法输入测试。

### 阶段 3：CLI

1. 用 Clap 实现 `convert-vless` 命令。
2. 支持 `--proxy-only`。
3. 支持 `-o/--output` 写入文件。
4. 支持标准输出，方便管道使用。
5. CLI 错误信息使用面向用户的中文提示。

### 阶段 4：API 服务

1. 用 Axum 创建 `/api/health`。
2. 创建 `/api/tools/vless-to-mihomo`。
3. 请求和响应结构使用 `serde`。
4. 错误统一映射为 JSON。
5. 添加 CORS，仅开发环境允许前端 Vite dev server。
6. 支持托管 Vue 构建后的 `dist`。

### 阶段 5：Vue 前端

1. 创建工具站外壳布局。
2. 创建工具列表和 VLESS 转换页面。
3. 输入区支持粘贴大段链接。
4. 输出区使用等宽字体展示 YAML。
5. 增加复制和下载按钮。
6. 增加错误提示和 loading 状态。
7. 移动端布局不溢出。

### 阶段 6：集成验证

1. `cargo test` 通过。
2. `cargo clippy` 无关键警告。
3. `npm run build` 通过。
4. 启动后端服务。
5. 打开 Vue 页面完成一次真实脱敏 VLESS 转换。
6. 将输出 YAML 导入 Clash Party 验证。

### 阶段 7：发布

1. Windows release：
   - `rust-tool-server.exe`
   - `rust-tool.exe`
2. macOS Apple Silicon release：
   - `rust-tool-server-aarch64-apple-darwin`
   - `rust-tool-aarch64-apple-darwin`
3. macOS Intel release：
   - `rust-tool-server-x86_64-apple-darwin`
   - `rust-tool-x86_64-apple-darwin`
4. README 写明 Windows 与 macOS 的使用方式。

## 验收标准

1. `cargo test` 全部通过。
2. `cargo clippy` 不出现会影响正确性的警告。
3. `npm run build` 成功。
4. CLI 能转换 VLESS 链接并输出 YAML。
5. Web 页面能转换 VLESS 链接并复制或下载 YAML。
6. Reality、WS、gRPC 三类样例输出正确。
7. 输出文件可以导入 Clash Party/Mihomo。
8. 新增第二个工具时，不需要改动 VLESS 核心逻辑。

## 后续扩展

- VMess 转 Mihomo
- Trojan 转 Mihomo
- Shadowsocks 转 Mihomo
- 多节点订阅转换
- Base64 订阅解码
- Mihomo YAML 校验
- Mihomo 配置合并
- 本地转换历史
- 工具收藏与排序
