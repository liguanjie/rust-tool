<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { useToolsStore } from '../stores/tools'
import { useThemeStore } from '../stores/theme'

const toolsStore = useToolsStore()
const themeStore = useThemeStore()
const router = useRouter()
const route = useRoute()

const selectedKeys = ref<string[]>([])
const footerToolIds = new Set(['program-settings'])
const primaryTools = computed(() => toolsStore.tools.filter(tool => !footerToolIds.has(tool.id)))
const footerTools = computed(() => toolsStore.tools.filter(tool => footerToolIds.has(tool.id)))
const appVersion = __APP_VERSION__

watch(
  () => route.path,
  () => {
    const activeTool = toolsStore.tools.find(
      (tool) => route.path === tool.path || route.path.startsWith(`${tool.path}/`)
    )
    if (activeTool) {
      selectedKeys.value = [activeTool.id]
    }
  },
  { immediate: true }
)

const onMenuClick = ({ key }: { key: string }) => {
  const tool = toolsStore.tools.find(t => t.id === key)
  if (tool) {
    router.push(tool.path)
  }
}
</script>

<template>
  <a-layout-sider
    v-model:collapsed="themeStore.isSidebarCollapsed"
    collapsible
    :theme="themeStore.isDarkMode ? 'dark' : 'light'"
    :style="{ borderRight: themeStore.isDarkMode ? '1px solid #303030' : '1px solid #f0f0f0' }"
  >
    <div
      class="sidebar-brand"
      :style="{ borderBottom: themeStore.isDarkMode ? '1px solid #303030' : '1px solid #f0f0f0' }"
    >
      <span class="sidebar-brand-title">{{ themeStore.isSidebarCollapsed ? 'RT' : 'RustTool' }}</span>
      <span class="sidebar-brand-version">v{{ appVersion }}</span>
    </div>

    <a-menu
      v-model:selectedKeys="selectedKeys"
      class="sidebar-menu sidebar-menu-primary"
      mode="inline"
      :style="{ borderRight: 0 }"
      @click="onMenuClick"
    >
      <a-menu-item v-for="tool in primaryTools" :key="tool.id">
        <template #icon>
          <component :is="tool.icon" style="width: 16px; height: 16px;" />
        </template>
        {{ tool.name }}
      </a-menu-item>
    </a-menu>

    <a-menu
      v-model:selectedKeys="selectedKeys"
      class="sidebar-menu sidebar-menu-footer"
      mode="inline"
      :style="{ borderRight: 0 }"
      @click="onMenuClick"
    >
      <a-menu-item v-for="tool in footerTools" :key="tool.id">
        <template #icon>
          <component :is="tool.icon" style="width: 16px; height: 16px;" />
        </template>
        {{ tool.name }}
      </a-menu-item>
    </a-menu>
  </a-layout-sider>
</template>

<style scoped>
.sidebar-menu {
  border-inline-end: 0;
}

.sidebar-brand {
  display: flex;
  flex: 0 0 auto;
  height: 80px;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 5px;
  line-height: 1;
}

.sidebar-brand-title {
  color: rgba(0, 0, 0, 0.88);
  font-size: 18px;
  font-weight: 700;
}

.sidebar-brand-version {
  color: rgba(0, 0, 0, 0.45);
  font-size: 12px;
  font-variant-numeric: tabular-nums;
  font-weight: 500;
}

.sidebar-menu-primary {
  flex: 1;
  min-height: 0;
  overflow-x: hidden;
  overflow-y: auto;
}

.sidebar-menu-footer {
  flex: 0 0 auto;
}

:global(.ant-layout-sider-dark) .sidebar-brand-title {
  color: rgba(255, 255, 255, 0.88);
}

:global(.ant-layout-sider-dark) .sidebar-brand-version {
  color: rgba(255, 255, 255, 0.45);
}
</style>
