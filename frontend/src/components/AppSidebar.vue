<script setup lang="ts">
import { RouterLink } from 'vue-router'
import { Moon, PanelLeftClose, PanelLeftOpen } from '@lucide/vue'
import { useToolsStore } from '../stores/tools'
import { useThemeStore } from '../stores/theme'

const toolsStore = useToolsStore()
const themeStore = useThemeStore()
</script>

<template>
  <aside class="app-sidebar" :class="{ 'app-sidebar--collapsed': themeStore.isSidebarCollapsed }">
    <div class="flex items-center" :class="themeStore.isSidebarCollapsed ? 'justify-center' : 'justify-between gap-3'">
      <div v-show="!themeStore.isSidebarCollapsed" class="flex items-center gap-3">
        <span class="logo-box">
          RT
        </span>
        <div>
          <div class="flex items-baseline gap-1.5">
            <h1 class="sidebar-title">RustTool</h1>
            <span class="text-[9px] font-mono text-emerald-400 font-bold bg-emerald-500/10 px-1 rounded">v1.2.0</span>
          </div>
          <p class="sidebar-subtitle">本地工具站</p>
        </div>
      </div>
      
      <button 
        @click="themeStore.toggleSidebar" 
        class="p-1.5 hover:bg-emerald-500/10 rounded-lg text-emerald-500 transition-colors flex items-center justify-center border border-transparent hover:border-emerald-500/20"
        :title="themeStore.isSidebarCollapsed ? '展开导航' : '收起导航'"
        type="button"
      >
        <component :is="themeStore.isSidebarCollapsed ? PanelLeftOpen : PanelLeftClose" class="h-4 w-4" />
      </button>
    </div>

    <nav class="mt-8 flex-1 grid gap-5 content-start" aria-label="工具导航">
      <section v-for="group in toolsStore.groups" :key="group.id" class="nav-group">
        <h2 v-show="!themeStore.isSidebarCollapsed">{{ group.name }}</h2>
        <div class="grid gap-2">
          <RouterLink
            v-for="tool in group.items"
            :key="tool.id"
            :to="tool.path"
            class="sidebar-item"
            active-class="sidebar-item--active"
            :class="{ 
              'sidebar-item--active': tool.path === '/toolbox' && $route.path.startsWith('/toolbox'),
              'sidebar-item--collapsed': themeStore.isSidebarCollapsed
            }"
            :title="tool.name"
          >
            <component :is="tool.icon" class="sidebar-item-icon" aria-hidden="true" />
            <span v-show="!themeStore.isSidebarCollapsed">{{ tool.name }}</span>
          </RouterLink>
        </div>
      </section>
    </nav>

    <div class="theme-popover-container">
      <div
        class="theme-trigger-btn theme-trigger-btn--locked"
        :class="{ 'theme-trigger-btn--collapsed': themeStore.isSidebarCollapsed }"
        title="极客暗黑"
      >
        <Moon class="h-4 w-4" aria-hidden="true" />
        <span v-show="!themeStore.isSidebarCollapsed">极客暗黑</span>
      </div>
    </div>
  </aside>
</template>
