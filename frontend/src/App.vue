<script setup lang="ts">
import { RouterView } from 'vue-router'
import AppSidebar from './components/AppSidebar.vue'
import { useThemeStore } from './stores/theme'

const themeStore = useThemeStore()
</script>

<template>
  <div :class="{ 'dark': themeStore.isDark }" class="relative h-screen overflow-hidden bg-[var(--bg-app)] text-[var(--text-base)] transition-colors duration-300">
    <!-- Background glowing ambient spots and circuit patterns -->
    <Transition name="fade">
      <div v-if="themeStore.isDark" class="pointer-events-none absolute inset-0 z-0">
        <div class="absolute -top-[20%] -left-[10%] h-[700px] w-[700px] rounded-full bg-[#10b981]/10 blur-[150px]"></div>
        <div class="absolute top-[40%] -right-[10%] h-[600px] w-[600px] rounded-full bg-[#059669]/5 blur-[120px]"></div>
        
        <!-- Tech/Circuit background pattern using CSS gradients -->
        <div class="absolute inset-0 opacity-[0.03]" style="background-image: radial-gradient(#10b981 1px, transparent 1px); background-size: 20px 20px;"></div>
        <div class="absolute inset-0 opacity-[0.04]" style="background-image: linear-gradient(to right, #10b981 1px, transparent 1px), linear-gradient(to bottom, #10b981 1px, transparent 1px); background-size: 80px 80px;"></div>
        <div class="absolute inset-0 bg-gradient-to-br from-[#050a0f]/80 to-transparent"></div>
      </div>
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
