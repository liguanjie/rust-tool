import { defineStore } from 'pinia'
import { Cable, MonitorCog, GitBranch, ScrollText } from '@lucide/vue'

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
          id: 'windows-workbench',
          name: 'Windows 工作台',
          path: '/tools/windows-workbench',
          description: '管理本机 Docker 与 sub2api',
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
      ],
    },
  ]

  const tools = groups.flatMap((group) => group.items)

  return { groups, tools }
})
