import { defineStore } from 'pinia'
import { Cable } from '@lucide/vue'

export interface ToolItem {
  id: string
  name: string
  path: string
  description: string
  icon: typeof Cable
}

export const useToolsStore = defineStore('tools', () => {
  const tools: ToolItem[] = [
    {
      id: 'vless-to-mihomo',
      name: 'VLESS 转 Mihomo',
      path: '/tools/vless-to-mihomo',
      description: '转换 3x-ui VLESS 链接',
      icon: Cable,
    },
  ]

  return { tools }
})
