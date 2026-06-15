import { defineStore } from 'pinia'
import { ref } from 'vue'

export const useThemeStore = defineStore('theme', () => {
  const isDark = ref(true)

  localStorage.removeItem('theme-mode')

  const isSidebarCollapsed = ref(localStorage.getItem('sidebar-collapsed') !== 'false')

  function toggleSidebar() {
    isSidebarCollapsed.value = !isSidebarCollapsed.value
    localStorage.setItem('sidebar-collapsed', isSidebarCollapsed.value.toString())
  }

  return {
    isDark,
    isSidebarCollapsed,
    toggleSidebar,
  }
})
