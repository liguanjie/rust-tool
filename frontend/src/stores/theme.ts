import { defineStore } from 'pinia'
import { ref } from 'vue'

export const useThemeStore = defineStore('theme', () => {
  const isSidebarCollapsed = ref(localStorage.getItem('sidebar-collapsed') !== 'false')
  const isDarkMode = ref(localStorage.getItem('theme-dark') === 'true')
  const isCompactMode = ref(localStorage.getItem('theme-compact') === 'true')

  function toggleSidebar() {
    isSidebarCollapsed.value = !isSidebarCollapsed.value
    localStorage.setItem('sidebar-collapsed', isSidebarCollapsed.value.toString())
  }

  function toggleDarkMode() {
    isDarkMode.value = !isDarkMode.value
    localStorage.setItem('theme-dark', isDarkMode.value.toString())
  }

  function toggleCompactMode() {
    isCompactMode.value = !isCompactMode.value
    localStorage.setItem('theme-compact', isCompactMode.value.toString())
  }

  return {
    isSidebarCollapsed,
    isDarkMode,
    isCompactMode,
    toggleSidebar,
    toggleDarkMode,
    toggleCompactMode,
  }
})
