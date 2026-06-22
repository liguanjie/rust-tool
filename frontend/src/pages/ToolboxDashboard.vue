<script setup lang="ts">
import { useRouter } from 'vue-router'
import { ArrowRight, Cable, KeyRound, Settings, Shield, Terminal } from 'lucide-vue-next'
import { theme } from 'ant-design-vue'

const { token } = theme.useToken()
const router = useRouter()

const appGroups = [
  {
    id: 'automation',
    title: '开发与自动化',
    description: '沉淀脚本入口、AI 技能和可复用的本地构建流程。',
    apps: [
      {
        id: 'agent-skills',
        title: 'AI 技能',
        description: '管理本地技能与执行脚本，让常用自动化流程可检索、可复用。',
        icon: Terminal,
        color: '#1890ff',
        route: '/agent-skills',
      },
    ],
  },
  {
    id: 'security',
    title: '安全与网络',
    description: '集中处理依赖风险、节点配置和本机凭据核验。',
    apps: [
      {
        id: 'osv',
        title: 'OSV 漏洞扫描',
        description: '扫描项目依赖漏洞，生成风险概览和后续修复依据。',
        icon: Shield,
        color: '#f5222d',
        route: '/osv-scanner',
      },
      {
        id: 'vless',
        title: 'VLESS 转 Mihomo',
        description: '把 VLESS 节点链接转换为 Mihomo YAML，便于客户端导入与复核。',
        icon: Cable,
        color: '#52c41a',
        route: '/toolbox/vless-to-mihomo',
      },
      {
        id: 'finalshell-password',
        title: 'FinalShell 密码解密',
        description: '解密 FinalShell 保存的密码字段，辅助凭据迁移和本机核验。',
        icon: KeyRound,
        color: '#722ed1',
        route: '/toolbox/finalshell-password',
      },
    ],
  },
  {
    id: 'operations',
    title: '配置与运维',
    description: '维护程序运行环境、数据存储位置和本机级设置。',
    apps: [
      {
        id: 'program-settings',
        title: '程序配置',
        description: '调整 SQLite 数据目录、维护数据库状态和本机运行参数。',
        icon: Settings,
        color: '#faad14',
        route: '/program-settings',
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
        汇集开发自动化、运行配置、安全审计和凭据核验的本地工具站。
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
