import { defineStore } from 'pinia'
import { BookOpen, Cable, Brain } from '@lucide/vue'

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
          id: 'ai-memo',
          name: 'AI 离线备忘',
          path: '/tools/ai-memo',
          description: '100% 离线、支持加密与 AI 检索的个人备忘助手',
          icon: Brain,
        },
        {
          id: 'api-docs',
          name: '接口文档',
          path: '/tools/api-docs',
          description: '查看 RustTool REST API',
          icon: BookOpen,
        },
      ],
    },
  ]

  const tools = groups.flatMap((group) => group.items)

  return { groups, tools }
})
