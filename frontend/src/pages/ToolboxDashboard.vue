<script setup lang="ts">
import { useRouter } from 'vue-router'
import { Terminal, Cable, Shield, ArrowRight } from 'lucide-vue-next'
import { theme } from 'ant-design-vue'

const { token } = theme.useToken()
const router = useRouter()

const apps = [
  {
    id: 'codex',
    title: 'Codex AI 引擎',
    description: '集中管理本地执行脚本和自动化 AI 技能，驱动底层项目构建。',
    icon: Terminal,
    color: '#1890ff',
    route: '/toolbox/codex',
  },
  {
    id: 'vless',
    title: 'VLESS 转 Mihomo',
    description: '将 3x-ui VLESS 链接转换为 Clash Party/Mihomo 标准 YAML 格式。',
    icon: Cable,
    color: '#52c41a',
    route: '/toolbox/vless-to-mihomo',
  },
  {
    id: 'osv',
    title: 'OSV 漏洞扫描',
    description: '分析当前工作区中的依赖漏洞，提升应用与供应链安全性。',
    icon: Shield,
    color: '#f5222d',
    route: '/osv-scanner',
  },
]
</script>

<template>
  <div style="padding: 32px; max-width: 1200px; margin: 0 auto;">
    <div style="margin-bottom: 32px;">
      <h1 :style="{ fontSize: '28px', fontWeight: 'bold', color: token.colorTextHeading, marginBottom: '8px' }">
        应用大盘
      </h1>
      <p :style="{ fontSize: '16px', color: token.colorTextSecondary, margin: 0 }">
        探索并使用本地工具链，提升开发与运维效率。
      </p>
    </div>

    <a-row :gutter="[24, 24]">
      <a-col v-for="app in apps" :key="app.id" :xs="24" :md="12" :lg="8">
        <a-card
          hoverable
          @click="router.push(app.route)"
          :style="{ 
            height: '100%', 
            display: 'flex', 
            flexDirection: 'column', 
            borderRadius: '12px',
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
                borderRadius: '12px', 
                background: `${app.color}15`, 
                color: app.color,
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                marginRight: '16px'
              }"
            >
              <component :is="app.icon" :size="24" />
            </div>
            <div style="flex: 1;">
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
  </div>
</template>
