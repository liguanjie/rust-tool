import { defineStore } from 'pinia'
import { ref } from 'vue'

export const useThemeStore = defineStore('theme', () => {
  // Default to dark theme for a futuristic default appearance, but check localStorage first
  const stored = localStorage.getItem('theme-dark')
  const isDark = ref(stored === null ? true : stored === 'true')

  function toggleTheme() {
    isDark.value = !isDark.value
    localStorage.setItem('theme-dark', isDark.value.toString())
  }

  function setTheme(dark: boolean) {
    isDark.value = dark
    localStorage.setItem('theme-dark', dark.toString())
  }

  return {
    isDark,
    toggleTheme,
    setTheme,
  }
})
