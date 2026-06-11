import { defineStore } from 'pinia'
import { BookOpen, Cable, Brain, KeyRound } from '@lucide/vue'

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
          name: 'AI 安全文档',
          path: '/tools/ai-memo',
          description: '本地 Markdown 文档库、secret 脱敏与 AI 检索问答',
          icon: Brain,
        },
        {
          id: 'secret-vault',
          name: '密码库',
          path: '/tools/secrets',
          description: '查看本地保密库中的 secret 索引并临时解密复制',
          icon: KeyRound,
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
