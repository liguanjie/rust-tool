import { defineStore } from 'pinia'
import { Cable, MonitorCog } from '@lucide/vue'

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
      ],
    },
    {
      id: 'toolbox',
      name: '工具箱',
      items: [
        {
          id: 'vless-to-mihomo',
          name: 'VLESS 转 Mihomo',
          path: '/toolbox/vless-to-mihomo',
          description: '转换 3x-ui VLESS 链接',
          icon: Cable,
        },
      ],
    },
  ]

  const tools = groups.flatMap((group) => group.items)

  return { groups, tools }
})
