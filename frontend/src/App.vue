<script setup lang="ts">
import { RouterView } from 'vue-router'
import AppSidebar from './components/AppSidebar.vue'
import { useThemeStore } from './stores/theme'

const themeStore = useThemeStore()
</script>

<template>
  <div :class="{ 'dark': themeStore.isDark }" class="app-root">
    <Transition name="fade">
      <div v-if="themeStore.isDark" class="app-backdrop" aria-hidden="true" />
    </Transition>

    <main
      class="app-shell"
      :class="themeStore.isSidebarCollapsed ? 'grid-cols-[80px_minmax(0,1fr)] max-[880px]:grid-cols-1' : 'grid-cols-[280px_minmax(0,1fr)] max-[880px]:grid-cols-1'"
    >
      <AppSidebar class="max-[880px]:min-h-0 max-[880px]:border-b max-[880px]:border-r-0" />
      <section class="app-content">
        <RouterView />
      </section>
    </main>
  </div>
</template>

<style scoped>
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.4s ease;
}
.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
