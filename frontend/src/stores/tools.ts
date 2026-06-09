import { defineStore } from 'pinia'
import { BookOpen, Cable, GitBranch, MonitorCog, ScrollText, Brain } from '@lucide/vue'

export interface ToolItem {
  id: string
  name: string
  path: string
  description: string
  icon: typeof Cable
}

export interface ToolGroup {
  id: string
  name: string
  items: ToolItem[]
}

export const useToolsStore = defineStore('tools', () => {
  const groups: ToolGroup[] = [
    {
      id: 'workbench',
      name: '工作台',
      items: [
        {
          id: 'local-workbench',
          name: '本机工作台',
          path: '/tools/local-workbench',
          description: '管理本机 Docker、代理客户端与脚本',
          icon: MonitorCog,
        },
        {
          id: 'api-management',
          name: 'API 管理',
          path: '/tools/api-management',
          description: '管理 Clash Party 订阅与节点',
          icon: GitBranch,
        },
        {
          id: 'api-docs',
          name: '接口文档',
          path: '/tools/api-docs',
          description: '查看 RustTool REST API',
          icon: BookOpen,
        },
        {
          id: 'operation-logs',
          name: '操作日志',
          path: '/tools/operation-logs',
          description: '记录工作台每步操作日志',
          icon: ScrollText,
        },
      ],
    },
    {
      id: 'toolbox',
      name: '工具箱',
      items: [
        {
          id: 'vless-to-mihomo',
          name: '工具箱',
          path: '/toolbox',
          description: '本地转换与辅助工具',
          icon: Cable,
        },
        {
          id: 'ai-memo',
          name: 'AI 离线备忘',
          path: '/tools/ai-memo',
          description: '100% 离线、支持加密与 AI 检索的个人备忘助手',
          icon: Brain,
        },
      ],
    },

  ]

  const tools = groups.flatMap((group) => group.items)

  return { groups, tools }
})
