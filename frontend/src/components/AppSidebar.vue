<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { RouterLink } from 'vue-router'
import { Sun, Moon, Monitor, Check, ChevronUp, ChevronDown, PanelLeftClose, PanelLeftOpen } from '@lucide/vue'
import { useToolsStore } from '../stores/tools'
import { useThemeStore } from '../stores/theme'

const toolsStore = useToolsStore()
const themeStore = useThemeStore()

const isMenuOpen = ref(false)
const popoverContainerRef = ref<HTMLElement | null>(null)

// Computed label and icon based on current mode
const activeThemeLabel = computed(() => {
  if (themeStore.themeMode === 'system') return '跟随系统'
  if (themeStore.themeMode === 'light') return '经典浅色'
  return '极客暗黑'
})

const activeThemeIcon = computed(() => {
  if (themeStore.themeMode === 'system') return Monitor
  if (themeStore.themeMode === 'light') return Sun
  return Moon
})

function selectTheme(mode: 'system' | 'light' | 'dark') {
  themeStore.setThemeMode(mode)
  isMenuOpen.value = false
}

// Close the menu when clicking outside of it
function handleOutsideClick(event: MouseEvent) {
  if (
    isMenuOpen.value &&
    popoverContainerRef.value &&
    !popoverContainerRef.value.contains(event.target as Node)
  ) {
    isMenuOpen.value = false
  }
}

onMounted(() => {
  window.addEventListener('click', handleOutsideClick)
})

onUnmounted(() => {
  window.removeEventListener('click', handleOutsideClick)
})
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

    <!-- Theme Switcher Popover (Gemini-style) at bottom -->
    <div ref="popoverContainerRef" class="theme-popover-container">
      <button 
        @click="isMenuOpen = !isMenuOpen" 
        class="theme-trigger-btn"
        :class="{ 
          'theme-trigger-btn--active': isMenuOpen,
          'theme-trigger-btn--collapsed': themeStore.isSidebarCollapsed
        }"
        type="button"
        title="设置主题"
      >
        <component :is="activeThemeIcon" class="h-4 w-4" aria-hidden="true" />
        <span v-show="!themeStore.isSidebarCollapsed">{{ activeThemeLabel }}</span>
        <component 
          v-show="!themeStore.isSidebarCollapsed" 
          :is="isMenuOpen ? ChevronDown : ChevronUp" 
          class="h-3 w-3 ml-auto text-gray-400" 
          aria-hidden="true" 
        />
      </button>

      <Transition name="slide-fade">
        <div 
          v-if="isMenuOpen" 
          class="theme-popover-menu"
          :class="{ 'theme-popover-menu--collapsed': themeStore.isSidebarCollapsed }"
        >
          <button 
            @click="selectTheme('system')" 
            class="theme-popover-item"
            :class="{ 'active': themeStore.themeMode === 'system' }"
            type="button"
          >
            <Monitor class="h-3.5 w-3.5" aria-hidden="true" />
            <span v-show="!themeStore.isSidebarCollapsed">跟随系统</span>
            <Check v-if="themeStore.themeMode === 'system'" class="h-3.5 w-3.5 ml-auto theme-check-icon" aria-hidden="true" />
          </button>
          
          <button 
            @click="selectTheme('light')" 
            class="theme-popover-item"
            :class="{ 'active': themeStore.themeMode === 'light' }"
            type="button"
          >
            <Sun class="h-3.5 w-3.5" aria-hidden="true" />
            <span v-show="!themeStore.isSidebarCollapsed">经典浅色</span>
            <Check v-if="themeStore.themeMode === 'light'" class="h-3.5 w-3.5 ml-auto theme-check-icon" aria-hidden="true" />
          </button>
          
          <button 
            @click="selectTheme('dark')" 
            class="theme-popover-item"
            :class="{ 'active': themeStore.themeMode === 'dark' }"
            type="button"
          >
            <Moon class="h-3.5 w-3.5" aria-hidden="true" />
            <span v-show="!themeStore.isSidebarCollapsed">极客暗黑</span>
            <Check v-if="themeStore.themeMode === 'dark'" class="h-3.5 w-3.5 ml-auto theme-check-icon" aria-hidden="true" />
          </button>
        </div>
      </Transition>
    </div>
  </aside>
</template>
