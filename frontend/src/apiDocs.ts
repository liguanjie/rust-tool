export type ApiMethod = 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE'

export interface ApiEndpointDoc {
  id: string
  method: ApiMethod
  path: string
  title: string
  description: string
  request?: string
  response: string
  notes?: string[]
}

export interface ApiDocModule {
  id: string
  name: string
  summary: string
  baseUrl: string
  environment: Array<{ name: string; description: string; defaultValue?: string }>
  endpoints: ApiEndpointDoc[]
}

export const apiDocModules: ApiDocModule[] = [
  {
    id: 'clash-party',
    name: 'Clash Party / Mihomo',
    summary: '读取 Clash Party 订阅与运行时代理组，并通过 Mihomo API 检测、切换订阅和节点。',
    baseUrl: 'http://127.0.0.1:8080',
    environment: [
      {
        name: 'RUSTTOOL_SERVER_PORT',
        description: 'RustTool REST 服务端口。端口被占用时可以改成其他值。',
        defaultValue: '8080',
      },
      {
        name: 'RUSTTOOL_CLASH_PARTY_DATA_DIR',
        description: 'Clash Party 数据目录，用于读取 profile.yaml 和订阅配置。',
      },
      {
        name: 'RUSTTOOL_CLASH_PARTY_API_URL',
        description: 'Mihomo external-controller 地址。',
        defaultValue: 'http://127.0.0.1:9998',
      },
      {
        name: 'RUSTTOOL_CLASH_PARTY_API_SECRET',
        description: 'Mihomo API Secret。未设置 Secret 时可以留空。',
      },
      {
        name: 'RUSTTOOL_CLASH_PARTY_DELAY_TIMEOUT_MS',
        description: '节点检测超时时间。超时节点不会被切换。',
        defaultValue: '5000',
      },
    ],
    endpoints: [
      {
        id: 'state',
        method: 'GET',
        path: '/api/clash-party/state',
        title: '读取管理状态',
        description: '返回订阅列表、运行时代理组、当前选中的订阅和节点状态。',
        response: `{
  "apiAvailable": true,
  "message": "已读取 3 个订阅和 4 个运行时代理组",
  "activeSubscriptionId": "profile-1",
  "subscriptions": [],
  "groups": []
}`,
        notes: ['当 Mihomo API 不可用时，接口仍会尽量读取本地订阅配置。'],
      },
      {
        id: 'health',
        method: 'GET',
        path: '/api/clash-party/health',
        title: '检查 Mihomo API',
        description: '检查 RustTool 是否能连接 Mihomo external-controller。',
        response: `{
  "available": true,
  "message": "Mihomo API 已连接",
  "version": "v1.19.24"
}`,
      },
      {
        id: 'switch-subscription',
        method: 'POST',
        path: '/api/clash-party/subscriptions/switch',
        title: '切换订阅',
        description: '按订阅 ID 切换 Clash Party 当前 profile。',
        request: `{
  "subscriptionId": "profile-1"
}`,
        response: `{
  "ok": true,
  "message": "订阅切换请求已发送"
}`,
      },
      {
        id: 'check-node',
        method: 'POST',
        path: '/api/clash-party/nodes/check',
        title: '检测节点',
        description: '对指定节点执行 Mihomo delay 检测，返回延迟和是否可用。',
        request: `{
  "nodeName": "日本 | 02 | AWS"
}`,
        response: `{
  "nodeName": "日本 | 02 | AWS",
  "available": true,
  "delay": 80,
  "message": "节点可用，延迟 80ms"
}`,
        notes: ['检测超时或 API 返回错误时，available 会是 false。'],
      },
      {
        id: 'switch-node',
        method: 'POST',
        path: '/api/clash-party/nodes/switch',
        title: '切换节点',
        description: '切换代理组目标节点。切换前会强制检测节点，超时节点不会被切换。',
        request: `{
  "groupName": "凌云",
  "nodeName": "自动选择"
}`,
        response: `{
  "ok": true,
  "message": "节点已切换"
}`,
        notes: ['如果节点检测超时，接口会拒绝切换并返回失败消息。'],
      },
    ],
  },
]

export function findApiDocModule(moduleId: unknown) {
  return apiDocModules.find((module) => module.id === moduleId) ?? apiDocModules[0]
}

export function buildCurlExample(module: ApiDocModule, endpoint: ApiEndpointDoc) {
  const url = `${module.baseUrl}${endpoint.path}`
  if (!endpoint.request) return `curl "${url}"`
  return [
    `curl -X ${endpoint.method} "${url}"`,
    '  -H "Content-Type: application/json"',
    `  -d '${endpoint.request.replace(/\r?\n/g, '')}'`,
  ].join(' \\\n')
}
