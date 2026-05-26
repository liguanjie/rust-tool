# VLESS 转 Mihomo 转换模块实施计划

## 模块目标

将 3x-ui 生成的 `vless://` 链接转换为 Clash Party/Mihomo 可导入的 YAML 配置。

该模块既要支持单节点快速转换，也要支持多条 VLESS 链接合并成同一份配置，后续可以扩展到 VMess、Trojan、Shadowsocks 等协议。

## 当前已落地

关键文件：

```text
crates/rust_tool_core/src/tools/vless_to_mihomo.rs
crates/rust_tool_cli/src/main.rs
crates/rust_tool_server/src/routes/tools.rs
frontend/src/api/tools.ts
frontend/src/stores/vlessToMihomo.ts
frontend/src/pages/VlessToMihomo.vue
frontend/src/api/download.ts
frontend/src-tauri/src/lib.rs
```

已完成能力：

- 解析 `vless://` 链接。
- 支持 Reality、TLS、TCP、WS、gRPC、HTTP Upgrade、XHTTP、H2 等常见参数映射。
- 支持完整配置与仅节点片段两种输出。
- 支持最小配置、基础分流、多节点分流模板。
- 默认使用多节点分流模板。
- 支持多条 VLESS 链接，一行一条。
- 多节点重名时自动追加 `-2`、`-3` 后缀。
- YAML 顶部输出节点地址注释。
- 支持下载文件名自定义。
- 单条链接时下载文件名可从 `#` 后面的节点名推断。
- 支持特殊直连域名，每行一条。
- 默认加入 `DST-PORT,22,DIRECT`，SSH 端口直连，不走代理。
- 默认加入 IPv4 与 IPv6 本地/局域网直连规则。
- 在 Tauri 桌面版中可保存 YAML 到下载目录。
- 在 Web 环境中可通过浏览器下载 YAML。
- VLESS 输入、模板、下载文件名、特殊直连域名可落盘保存。

## 输出模板

### 最小配置

适合只想让所有流量走代理的场景。

核心规则：

```text
本地/局域网/22端口直连
MATCH,PROXY
```

### 基础分流

适合当前单节点使用场景。

核心规则：

```text
本地/局域网/22端口直连
GEOIP,CN,DIRECT
MATCH,PROXY
```

### 多节点分流模板

适合未来多个节点或订阅源使用。

核心能力：

- 引用社区远程规则集。
- 广告、AI、媒体、Google、Telegram、TikTok 分组。
- 国内域名与国内 IP 直连。
- 非中国大陆域名走代理。
- 未匹配流量兜底走代理。

## 实施步骤

### 阶段 1：核心解析

1. 校验输入必须是 `vless://`。
2. 解析 UUID、server、port、query 参数、fragment 节点名。
3. 根据 `security` 判断 TLS/Reality。
4. 根据 `type` 或 `network` 映射 Mihomo network。
5. 将 3x-ui 常见参数映射为 Mihomo 字段。

### 阶段 2：YAML 模型

1. 建立 Rust 结构体表达 Mihomo config。
2. 使用 `serde_yaml` 输出 YAML。
3. 避免手写字符串拼接。
4. 对可选字段使用 `skip_serializing_if`。

### 阶段 3：模板化输出

1. 支持 `full_config` 和 `proxy_only`。
2. 支持 `minimal`、`standard`、`full_rules`。
3. 在完整配置中生成 proxies、proxy-groups、rules。
4. 在多节点模板中生成 rule-providers。

### 阶段 4：多链接支持

1. 输入框按行拆分。
2. 每行独立解析为一个 proxy。
3. 自动处理重复节点名。
4. 所有节点加入 PROXY 与 AUTO 组。
5. proxy-only 模式下，多节点输出 YAML 数组。

### 阶段 5：用户配置

1. 删除独立节点名称输入。
2. 下载文件名从单条 VLESS `#` 后面的名称自动推断。
3. 多条 VLESS 时下载文件名保留用户自定义或默认 `mihomo`。
4. 增加特殊直连域名输入区。
5. 将用户输入落盘到 SQLite 或 Web localStorage。

## 验收标准

- 单条 VLESS 可以正常转换。
- 多条 VLESS 一行一条可以生成多个 proxies。
- 重名节点导入 Clash Party 不冲突。
- `DST-PORT,22,DIRECT` 在兜底代理规则之前。
- 自定义直连域名生成 `DOMAIN-SUFFIX,<domain>,DIRECT`。
- 完整配置可导入 Clash Party。
- 仅节点片段可复制给高级用户二次组合。
- Rust 单元测试覆盖主要协议和模板。

## 已验证命令

```text
cargo fmt --all
cargo test -p rust_tool_core
cargo check --workspace
vue-tsc -b
vite build
```

## 后续扩展

- 支持批量导入 VMess。
- 支持 Trojan。
- 支持 Shadowsocks。
- 增加 YAML 校验器。
- 增加配置差异对比。
- 增加配置导入预览，显示节点数量、规则数量、分组数量。
