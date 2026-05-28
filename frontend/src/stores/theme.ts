import { defineStore } from 'pinia'
import { ref } from 'vue'

export type ThemeMode = 'light' | 'dark' | 'system'

export const useThemeStore = defineStore('theme', () => {
  const themeMode = ref<ThemeMode>((localStorage.getItem('theme-mode') as ThemeMode) || 'system')
  const isDark = ref(true)

  // Media query to detect system light/dark preference
  const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)')

  function updateTheme() {
    if (themeMode.value === 'system') {
      isDark.value = mediaQuery.matches
    } else {
      isDark.value = themeMode.value === 'dark'
    }
  }

  function setThemeMode(mode: ThemeMode) {
    themeMode.value = mode
    localStorage.setItem('theme-mode', mode)
    updateTheme()
  }

  // Setup listener for system changes
  try {
    mediaQuery.addEventListener('change', updateTheme)
  } catch {
    // Fallback support for older Tauri webviews or browsers
    mediaQuery.addListener(updateTheme)
  }

  // Initialize theme status
  updateTheme()

  return {
    themeMode,
    isDark,
    setThemeMode,
  }
})
