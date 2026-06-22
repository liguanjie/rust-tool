<script setup lang="ts">
import { useRouter } from 'vue-router'
import { ArrowRight, Cable, Settings, Shield, Terminal } from 'lucide-vue-next'
import { theme } from 'ant-design-vue'

const { token } = theme.useToken()
const router = useRouter()

const appGroups = [
  {
    id: 'automation',
    title: '开发与自动化',
    description: '本地脚本、技能编排和项目构建入口。',
    apps: [
      {
        id: 'agent-skills',
        title: 'AI 技能',
        description: '集中管理本地执行脚本和自动化 AI 技能，驱动底层项目构建。',
        icon: Terminal,
        color: '#1890ff',
        route: '/agent-skills',
      },
    ],
  },
  {
    id: 'operations',
    title: '配置与运维',
    description: '管理本机运行设置与常用配置转换。',
    apps: [
      {
        id: 'program-settings',
        title: '程序配置',
        description: '管理 SQLite 数据库存放位置和本机运行配置。',
        icon: Settings,
        color: '#faad14',
        route: '/program-settings',
      },
      {
        id: 'vless',
        title: 'VLESS 转 Mihomo',
        description: '将 3x-ui VLESS 链接转换为 Clash Party/Mihomo 标准 YAML 格式。',
        icon: Cable,
        color: '#52c41a',
        route: '/toolbox/vless-to-mihomo',
      },
    ],
  },
  {
    id: 'security',
    title: '安全与审计',
    description: '面向依赖风险、供应链安全和审计留痕。',
    apps: [
      {
        id: 'osv',
        title: 'OSV 漏洞扫描',
        description: '分析当前工作区中的依赖漏洞，提升应用与供应链安全性。',
        icon: Shield,
        color: '#f5222d',
        route: '/osv-scanner',
      },
    ],
  },
]

const openApp = (route: string) => {
  router.push(route)
}
</script>

<template>
  <div style="padding: 32px; max-width: 1200px; margin: 0 auto;">
    <div style="margin-bottom: 32px;">
      <h1 :style="{ fontSize: '28px', fontWeight: 'bold', color: token.colorTextHeading, marginBottom: '8px' }">
        工作台
      </h1>
      <p :style="{ fontSize: '16px', color: token.colorTextSecondary, margin: 0 }">
        面向本地开发、安全审计和自动化运维的桌面工具站。
      </p>
    </div>

    <section
      v-for="group in appGroups"
      :key="group.id"
      style="margin-bottom: 36px;"
    >
      <div style="margin-bottom: 16px;">
        <h2 :style="{ fontSize: '18px', fontWeight: 700, color: token.colorTextHeading, margin: '0 0 4px 0' }">
          {{ group.title }}
        </h2>
        <p :style="{ fontSize: '14px', color: token.colorTextSecondary, margin: 0 }">
          {{ group.description }}
        </p>
      </div>

      <a-row :gutter="[24, 24]">
        <a-col v-for="app in group.apps" :key="app.id" :xs="24" :sm="12" :lg="8" :xl="6">
          <a-card
            hoverable
            @click="openApp(app.route)"
            :style="{
              height: '100%',
              display: 'flex',
              flexDirection: 'column',
              borderRadius: '8px',
              border: `1px solid ${token.colorBorderSecondary}`,
              transition: 'all 0.3s ease'
            }"
            :bodyStyle="{ flex: 1, display: 'flex', flexDirection: 'column', padding: '24px' }"
          >
            <div style="display: flex; align-items: flex-start; margin-bottom: 16px;">
              <div
                :style="{
                  width: '48px',
                  height: '48px',
                  borderRadius: '8px',
                  background: `${app.color}15`,
                  color: app.color,
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  marginRight: '16px',
                  flex: '0 0 auto'
                }"
              >
                <component :is="app.icon" :size="24" />
              </div>
              <div style="flex: 1; min-width: 0;">
                <h3 :style="{ fontSize: '18px', fontWeight: 'bold', margin: '0 0 4px 0', color: token.colorText }">
                  {{ app.title }}
                </h3>
              </div>
            </div>
            <p :style="{ color: token.colorTextSecondary, margin: '0 0 24px 0', flex: 1, lineHeight: '1.5' }">
              {{ app.description }}
            </p>
            <div :style="{ display: 'flex', alignItems: 'center', color: token.colorPrimary, fontWeight: 500 }">
              <span>立即开启</span>
              <ArrowRight :size="16" style="margin-left: 4px;" />
            </div>
          </a-card>
        </a-col>
      </a-row>
    </section>
  </div>
</template>
