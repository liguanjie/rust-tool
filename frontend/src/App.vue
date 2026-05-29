<script setup lang="ts">
import { RouterView } from 'vue-router'
import AppSidebar from './components/AppSidebar.vue'
import { useThemeStore } from './stores/theme'

const themeStore = useThemeStore()
</script>

<template>
  <div :class="{ 'dark': themeStore.isDark }" class="relative h-screen overflow-hidden bg-[var(--bg-app)] text-[var(--text-base)] transition-colors duration-300">
    <!-- Background glowing ambient spots (Only active in dark mode) -->
    <Transition name="fade">
      <div v-if="themeStore.isDark" class="pointer-events-none">
        <div class="absolute -top-[30%] -left-[10%] h-[700px] w-[700px] rounded-full bg-emerald-500/5 blur-[120px]"></div>
        <div class="absolute -bottom-[20%] -right-[10%] h-[600px] w-[600px] rounded-full bg-blue-500/5 blur-[100px]"></div>
        <!-- Dot-matrix developer grid background -->
        <div class="absolute inset-0 bg-[radial-gradient(rgba(255,255,255,0.015)_1px,transparent_1px)] [background-size:24px_24px]"></div>
      </div>
    </Transition>

    <main
      class="app-shell"
      :class="themeStore.isSidebarCollapsed ? 'grid-cols-[80px_minmax(0,1fr)]' : 'grid-cols-[280px_minmax(0,1fr)]'"
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
