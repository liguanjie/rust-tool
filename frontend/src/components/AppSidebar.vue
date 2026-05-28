<script setup lang="ts">
import { RouterLink } from 'vue-router'
import { Sun, Moon } from '@lucide/vue'
import { useToolsStore } from '../stores/tools'
import { useThemeStore } from '../stores/theme'

const toolsStore = useToolsStore()
const themeStore = useThemeStore()
</script>

<template>
  <aside class="app-sidebar">
    <div class="flex items-center gap-3">
      <span class="logo-box">
        RT
      </span>
      <div>
        <h1 class="sidebar-title">RustTool</h1>
        <p class="sidebar-subtitle">本地工具站</p>
      </div>
    </div>

    <nav class="mt-8 flex-1 grid gap-5 content-start" aria-label="工具导航">
      <section v-for="group in toolsStore.groups" :key="group.id" class="nav-group">
        <h2>{{ group.name }}</h2>
        <div class="grid gap-2">
          <RouterLink
            v-for="tool in group.items"
            :key="tool.id"
            :to="tool.path"
            class="sidebar-item"
            active-class="sidebar-item--active"
            :class="{ 'sidebar-item--active': tool.path === '/toolbox' && $route.path.startsWith('/toolbox') }"
          >
            <component :is="tool.icon" class="sidebar-item-icon" aria-hidden="true" />
            <span>{{ tool.name }}</span>
          </RouterLink>
        </div>
      </section>
    </nav>

    <!-- Theme Switcher Segmented Capsule at bottom -->
    <div class="theme-switcher-container">
      <button 
        @click="themeStore.setTheme(false)" 
        class="theme-segment"
        :class="{ 'active': !themeStore.isDark }"
        type="button"
        title="经典浅色"
      >
        <Sun class="h-3.5 w-3.5" aria-hidden="true" />
        <span>经典浅色</span>
      </button>
      <button 
        @click="themeStore.setTheme(true)" 
        class="theme-segment"
        :class="{ 'active': themeStore.isDark }"
        type="button"
        title="极客暗黑"
      >
        <Moon class="h-3.5 w-3.5" aria-hidden="true" />
        <span>极客暗黑</span>
      </button>
    </div>
  </aside>
</template>
