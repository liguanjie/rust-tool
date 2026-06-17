<script setup lang="ts">
import { ChevronRight } from 'lucide-vue-next'
import { RouterLink } from 'vue-router'
import { useToolsStore } from '../stores/tools'

const toolsStore = useToolsStore()
</script>

<template>
  <div style="padding: 24px; max-width: 1200px; margin: 0 auto;">
    <a-page-header
      title="RustTool"
      sub-title="面向本地开发、安全审计和自动化运维的桌面工具站。"
      style="padding-left: 0; padding-right: 0;"
    >
      <template #tags>
        <a-tag color="blue">工作台</a-tag>
      </template>
    </a-page-header>

    <a-card style="margin-bottom: 24px;">
      <a-row :gutter="16">
        <a-col :span="12">
          <a-statistic title="工具总数" :value="toolsStore.tools.length" />
        </a-col>
        <a-col :span="12">
          <a-statistic title="云依赖" :value="0" />
        </a-col>
      </a-row>
    </a-card>

    <a-row :gutter="[16, 16]">
      <a-col :xs="24" :sm="12" :md="8" :lg="6" v-for="tool in toolsStore.tools" :key="tool.id">
        <RouterLink :to="tool.path" style="text-decoration: none;">
          <a-card hoverable style="height: 100%;">
            <a-card-meta :title="tool.name" :description="tool.summary">
              <template #avatar>
                <component :is="tool.icon" style="width: 24px; height: 24px; color: #1890ff;" />
              </template>
            </a-card-meta>
            <div style="margin-top: 16px;">
              <a-tag color="cyan">{{ tool.badge }}</a-tag>
            </div>
            <div style="margin-top: 8px;">
              <a-tag v-for="signal in tool.signals" :key="signal" style="margin-bottom: 4px;">{{ signal }}</a-tag>
            </div>
          </a-card>
        </RouterLink>
      </a-col>
    </a-row>
  </div>
</template>
