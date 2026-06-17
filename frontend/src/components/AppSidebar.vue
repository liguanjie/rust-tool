<script setup lang="ts">
import { ref, watch } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { useToolsStore } from '../stores/tools'
import { useThemeStore } from '../stores/theme'

const toolsStore = useToolsStore()
const themeStore = useThemeStore()
const router = useRouter()
const route = useRoute()

const selectedKeys = ref<string[]>([])

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
    <div :style="{ height: '64px', display: 'flex', alignItems: 'center', justifyContent: 'center', fontWeight: 'bold', fontSize: '18px', borderBottom: themeStore.isDarkMode ? '1px solid #303030' : '1px solid #f0f0f0' }">
      <span v-if="!themeStore.isSidebarCollapsed">RustTool</span>
      <span v-else>RT</span>
    </div>
    
    <a-menu
      v-model:selectedKeys="selectedKeys"
      mode="inline"
      :style="{ borderRight: 0 }"
      @click="onMenuClick"
    >
      <a-menu-item v-for="tool in toolsStore.tools" :key="tool.id">
        <template #icon>
          <component :is="tool.icon" style="width: 16px; height: 16px;" />
        </template>
        {{ tool.name }}
      </a-menu-item>
    </a-menu>
  </a-layout-sider>
</template>
