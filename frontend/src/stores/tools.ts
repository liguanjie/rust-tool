import { defineStore } from 'pinia'
import type { Component } from 'vue'
import { BookOpen, Cable, Shield, Terminal } from '@lucide/vue'

export interface ToolItem {
  id: string
  name: string
  path: string
  description: string
  icon: Component
  badge: string
  summary: string
  signals: string[]
  accent: 'emerald' | 'sky' | 'amber' | 'rose'
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
          badge: '自动化',
          summary: '脚本编排、参数输入、执行记录',
          signals: ['本地执行', '记录追溯', '参数化'],
          accent: 'emerald',
          icon: Terminal,
        },
        {
          id: 'vless-to-mihomo',
          name: 'VLESS 转 Mihomo',
          path: '/toolbox/vless-to-mihomo',
          description: '转换 3x-ui VLESS 链接为 Clash Party/Mihomo YAML',
          badge: '网络配置',
          summary: '链接解析、订阅整理、配置导出',
          signals: ['本地转换', '模板配置', '下载 YAML'],
          accent: 'sky',
          icon: Cable,
        },
        {
          id: 'api-docs',
          name: '接口文档',
          path: '/tools/api-docs',
          description: '查看 RustTool REST API',
          badge: '开发参考',
          summary: 'REST API、请求结构、返回示例',
          signals: ['接口索引', '调试参考', 'Web/Tauri 共用'],
          accent: 'amber',
          icon: BookOpen,
        },
        {
          id: 'osv-scanner',
          name: 'OSV 漏洞管理',
          path: '/toolbox/osv-scanner',
          description: '预览并执行本地依赖漏洞扫描',
          badge: '供应链安全',
          summary: '诊断包源、预览命令、导出报告',
          signals: ['命令审计', 'JSON/HTML', '忽略规则'],
          accent: 'rose',
          icon: Shield,
        },
      ],
    },
  ]

  const tools = groups.flatMap((group) => group.items)

  return { groups, tools }
})
