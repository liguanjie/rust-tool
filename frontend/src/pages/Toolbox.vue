<script setup lang="ts">
import { ChevronRight } from '@lucide/vue'
import { RouterLink } from 'vue-router'
import ToolShell from '../components/ToolShell.vue'
import { useToolsStore } from '../stores/tools'

const toolsStore = useToolsStore()
</script>

<template>
  <ToolShell
    title="RustTool"
    description="面向本地开发、安全审计和自动化运维的桌面工具站。"
    eyebrow="工作台"
    fluid
  >
    <section class="toolbox-overview">
      <div class="toolbox-overview-main">
        <span class="eyebrow">工具矩阵</span>
        <h3>把高频本地任务收在一个工作台里</h3>
      </div>
      <dl class="toolbox-overview-stats">
        <div>
          <dt>{{ toolsStore.tools.length }}</dt>
          <dd>工具</dd>
        </div>
        <div>
          <dt>0</dt>
          <dd>云依赖</dd>
        </div>
        <div>
          <dt>Web/Tauri</dt>
          <dd>同源界面</dd>
        </div>
      </dl>
    </section>

    <section class="toolbox-card-grid" aria-label="工具入口">
      <RouterLink
        v-for="tool in toolsStore.tools"
        :key="tool.id"
        class="toolbox-card"
        :class="`toolbox-card--${tool.accent}`"
        :to="tool.path"
      >
        <span class="service-icon toolbox-card-icon">
          <component :is="tool.icon" class="h-5 w-5" aria-hidden="true" />
        </span>
        <span class="toolbox-card-copy">
          <span class="status-pill status-pill--muted">{{ tool.badge }}</span>
          <strong>{{ tool.name }}</strong>
          <small>{{ tool.summary }}</small>
        </span>
        <span class="toolbox-card-signals">
          <small v-for="signal in tool.signals" :key="signal">{{ signal }}</small>
        </span>
        <ChevronRight class="h-5 w-5 api-entry-arrow" aria-hidden="true" />
      </RouterLink>
    </section>
  </ToolShell>
</template>
