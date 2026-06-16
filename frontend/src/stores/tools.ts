import { defineStore } from 'pinia'
import { BookOpen, Cable, Shield, Terminal } from '@lucide/vue'

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
      name: '工具栏',
      items: [
        {
          id: 'codex',
          name: '工作台',
          path: '/toolbox/codex',
          description: '管理并运行 Codex 本地自动化脚本与工具',
          icon: Terminal,
        },
        {
          id: 'api-docs',
          name: '接口文档',
          path: '/tools/api-docs',
          description: '查看 RustTool REST API',
          icon: BookOpen,
        },
        {
          id: 'osv-scanner',
          name: 'OSV 漏洞管理',
          path: '/toolbox/osv-scanner',
          description: '预览并执行本地依赖漏洞扫描',
          icon: Shield,
        },
      ],
    },
  ]

  const tools = groups.flatMap((group) => group.items)

  return { groups, tools }
})
