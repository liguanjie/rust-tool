# Clash Party 管理 API 文档

## 目标

在 Windows 工作台中提供 Clash Party 管理能力：

- 读取 Clash Party/Mihomo Party 本地订阅索引。
- 列出每个订阅下的节点数量、代理组数量、流量信息。
- 通过 Mihomo API 列出当前运行时代理组与节点。
- 切换当前订阅。
- 切换指定代理组下的节点。

## 配置项

配置保存在本机 SQLite 的 `app_settings` 表中，key 为 `windows_workbench.config`。

| 字段 | 说明 | 默认值 |
| --- | --- | --- |
| `clashPartyPath` | Clash Party/Mihomo Party 程序路径 | 自动侦测 |
| `clashPartyDataDir` | Clash Party 数据目录，通常包含 `profile.yaml` 和 `profiles` 目录 | `%APPDATA%\mihomo-party` |
| `clashPartyApiUrl` | Mihomo 外部控制 API 地址 | `http://127.0.0.1:9090` |
| `clashPartyApiSecret` | Mihomo API Secret | 空 |

## Tauri 命令 API

### `detect_clash_party`

自动侦测 Clash Party 程序路径、数据目录和默认 API 地址。

返回：

```ts
interface ClashPartyDetection {
  clashPartyPath: string
  clashPartyDataDir: string
  clashPartyApiUrl: string
}
```

### `get_clash_party_manager_state`

读取订阅列表，并尝试通过 Mihomo API 读取运行时代理组。

返回：

```ts
interface ClashPartyManagerState {
  dataDir: string
  profileIndexPath: string
  apiUrl: string
  activeSubscriptionId: string
  subscriptions: ClashPartySubscription[]
  groups: ClashPartyProxyGroup[]
  apiAvailable: boolean
  message: string
}
```

### `switch_clash_party_subscription`

切换当前订阅。底层调用 Mihomo API：

```http
PUT /configs
Content-Type: application/json

{"path":"订阅 YAML 文件路径"}
```

入参：

```ts
{
  subscriptionId: string
}
```

返回：

```ts
interface ClashPartySwitchResult {
  ok: boolean
  message: string
}
```

### `switch_clash_party_node`

切换指定代理组下的节点。底层调用 Mihomo API：

```http
PUT /proxies/{groupName}
Content-Type: application/json

{"name":"节点名称"}
```

入参：

```ts
{
  groupName: string
  nodeName: string
}
```

返回：

```ts
interface ClashPartySwitchResult {
  ok: boolean
  message: string
}
```

## 前端封装

前端封装位于 `frontend/src/api/workbench.ts`：

- `getClashPartyManagerState()`
- `switchClashPartySubscription(subscriptionId)`
- `switchClashPartyNode(groupName, nodeName)`

状态管理位于 `frontend/src/stores/windowsWorkbench.ts`：

- `clashPartyManager`
- `selectedClashPartySubscriptionId`
- `selectedClashPartyGroupName`
- `selectedClashPartyNodeName`
- `refreshClashPartyManager()`
- `switchSubscription()`
- `switchNode(nodeName?)`

## 数据来源

本地订阅读取：

- `profile.yaml`：订阅索引与当前订阅 ID。
- `profiles/{id}.yaml`：订阅对应的 Mihomo 配置文件。

运行时节点读取：

- `GET /proxies`：读取当前 Mihomo 内核内的代理组和节点。

## 注意事项

- 订阅 URL 不在工作台界面展示，避免暴露敏感订阅地址。
- 如果 Clash Party 当前只开放 Windows 命名管道 API，普通 HTTP API 不可访问，工作台仍可读取订阅文件，但不能执行切换。
- 切换订阅和切换节点都依赖 `clashPartyApiUrl` 可访问；如果设置了 API Secret，需要在配置中填写。
