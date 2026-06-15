import { defineStore } from 'pinia'
import { BookOpen, Cable, Terminal } from '@lucide/vue'

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
      id: 'toolbox',
      name: '工具箱',
      items: [
        {
          id: 'vless-to-mihomo',
          name: 'VLESS 转 Mihomo',
          path: '/toolbox/vless-to-mihomo',
          description: '将 3x-ui VLESS 链接转换为 Clash Party/Mihomo YAML',
          icon: Cable,
        },
        {
          id: 'api-docs',
          name: '接口文档',
          path: '/tools/api-docs',
          description: '查看 RustTool REST API',
          icon: BookOpen,
        },
        {
          id: 'codex',
          name: 'Codex 管理',
          path: '/toolbox/codex',
          description: '管理并运行 Codex 本地自动化脚本',
          icon: Terminal,
        },
      ],
    },
  ]

  const tools = groups.flatMap((group) => group.items)

  return { groups, tools }
})
