<script setup lang="ts">
import { Cable, ChevronRight } from '@lucide/vue'
import { RouterLink } from 'vue-router'
import ToolShell from '../components/ToolShell.vue'
</script>

<template>
  <ToolShell title="工具箱" description="选择要使用的本地工具。" eyebrow="工具箱">
    <section class="api-card-grid">
      <RouterLink class="api-entry-card" to="/toolbox/vless-to-mihomo">
        <span class="service-icon">
          <Cable class="h-5 w-5" aria-hidden="true" />
        </span>
        <span class="api-entry-copy">
          <strong>VLESS 转 Mihomo</strong>
          <small>转换 3x-ui VLESS 链接为 Clash Party/Mihomo YAML</small>
        </span>
        <span class="api-entry-meta">
          <span class="status-pill status-pill--muted">本地转换</span>
          <small>支持完整配置与节点片段</small>
        </span>
        <ChevronRight class="h-5 w-5 api-entry-arrow" aria-hidden="true" />
      </RouterLink>
    </section>
  </ToolShell>
</template>
