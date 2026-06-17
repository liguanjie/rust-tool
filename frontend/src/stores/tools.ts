import { defineStore } from 'pinia'
import type { Component } from 'vue'
import { BookOpen, Cable, Shield, Terminal, LayoutDashboard } from 'lucide-vue-next'

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

export const useToolsStore = defineStore('tools', () => {
  const tools: ToolItem[] = [
    {
      id: 'dashboard',
      name: '工作台',
      path: '/dashboard',
      description: '全局工具集合与应用入口',
      badge: '聚合',
      summary: '聚合应用大盘',
      signals: [],
      accent: 'emerald',
      icon: LayoutDashboard,
    },
    {
      id: 'agent-skills',
      name: 'AI 技能',
      path: '/agent-skills',
      description: '管理并运行 本地自动化脚本与技能',
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
      id: 'osv-scanner',
      name: 'OSV 漏洞管理',
      path: '/osv-scanner',
      description: '预览并执行本地依赖漏洞扫描',
      badge: '供应链安全',
      summary: '诊断包源、预览命令、导出报告',
      signals: ['命令审计', 'JSON/HTML', '忽略规则'],
      accent: 'rose',
      icon: Shield,
    },
  ]

  return { tools }
})
