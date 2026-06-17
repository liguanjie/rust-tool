<script setup lang="ts">
import { computed } from 'vue'
import { RouterView } from 'vue-router'
import { theme } from 'ant-design-vue'
import { Palette, Moon, Sun, Shrink, Expand } from '@lucide/vue'
import AppSidebar from './components/AppSidebar.vue'
import { useThemeStore } from './stores/theme'

const themeStore = useThemeStore()

const antdTheme = computed(() => {
  const algorithm = []
  if (themeStore.isDarkMode) {
    algorithm.push(theme.darkAlgorithm)
  } else {
    algorithm.push(theme.defaultAlgorithm)
  }
  if (themeStore.isCompactMode) {
    algorithm.push(theme.compactAlgorithm)
  }
  return { algorithm }
})
</script>

<template>
  <a-config-provider :theme="antdTheme">
    <a-layout style="min-height: 100vh">
      <AppSidebar />
      <a-layout>
        <a-layout-content style="padding: 24px; overflow: auto;">
          <RouterView />
        </a-layout-content>
      </a-layout>
    </a-layout>

    <a-float-button-group trigger="hover" type="primary" :style="{ right: '24px', bottom: '24px' }">
      <template #icon>
        <Palette class="h-4 w-4" style="margin: auto" />
      </template>
      <a-float-button :tooltip="themeStore.isDarkMode ? '切换明亮模式' : '切换暗黑模式'" @click="themeStore.toggleDarkMode">
        <template #icon>
          <Sun v-if="themeStore.isDarkMode" class="h-4 w-4" style="margin: auto" />
          <Moon v-else class="h-4 w-4" style="margin: auto" />
        </template>
      </a-float-button>
      <a-float-button :tooltip="themeStore.isCompactMode ? '取消紧凑模式' : '开启紧凑模式'" @click="themeStore.toggleCompactMode">
        <template #icon>
          <Expand v-if="themeStore.isCompactMode" class="h-4 w-4" style="margin: auto" />
          <Shrink v-else class="h-4 w-4" style="margin: auto" />
        </template>
      </a-float-button>
    </a-float-button-group>
  </a-config-provider>
</template>
