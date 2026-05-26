import { computed, ref, watch } from 'vue'
import { defineStore } from 'pinia'
import {
  convertVlessToMihomo,
  getVlessToolSettings,
  saveVlessToolSettings,
  type VlessOutputMode,
  type VlessTemplateMode,
} from '../api/tools'

export const useVlessToMihomoStore = defineStore('vless-to-mihomo', () => {
  const input = ref('')
  const mode = ref<VlessOutputMode>('full_config')
  const template = ref<VlessTemplateMode>('full_rules')
  const downloadName = ref('mihomo')
  const downloadNameEdited = ref(false)
  const directDomains = ref('')
  const settingsLoaded = ref(false)
  const savingSettings = ref(false)
  const yaml = ref('')
  const error = ref('')
  const loading = ref(false)
  const copied = ref(false)
  let saveTimer: number | undefined

  const canConvert = computed(() => input.value.trim().length > 0 && !loading.value)
  const downloadFilename = computed(() => normalizeYamlFilename(downloadName.value))
  const proxyName = computed(() => stripYamlExtension(downloadName.value.trim() || 'mihomo'))
  const nodeAddress = computed(() => extractVlessAddress(input.value))

  watch(input, (value) => {
    if (downloadNameEdited.value) return

    const name = extractVlessName(value)
    downloadName.value = name || 'mihomo'
  })

  watch([input, mode, template, downloadName, directDomains], () => {
    scheduleSaveSettings()
  })

  async function load() {
    if (settingsLoaded.value) return

    try {
      const settings = await getVlessToolSettings()
      const extractedName = extractVlessName(settings.input)
      input.value = settings.input
      mode.value = normalizeOutputMode(settings.mode)
      template.value = normalizeTemplateMode(settings.template)
      downloadName.value = settings.downloadName || extractedName || 'mihomo'
      downloadNameEdited.value = Boolean(
        settings.downloadName?.trim() &&
          settings.downloadName !== 'mihomo' &&
          settings.downloadName !== extractedName,
      )
      directDomains.value = settings.directDomains || ''
    } catch (caught) {
      console.warn('Failed to load VLESS tool settings', caught)
    } finally {
      settingsLoaded.value = true
    }
  }

  function updateDownloadName(value: string) {
    downloadNameEdited.value = true
    downloadName.value = value
  }

  async function convert() {
    error.value = ''
    copied.value = false
    loading.value = true

    try {
      await persistSettings()
      const result = await convertVlessToMihomo({
        input: input.value,
        mode: mode.value,
        template: template.value,
        proxy_name: proxyName.value || undefined,
        direct_domains: parseDirectDomains(directDomains.value),
      })
      yaml.value = result.yaml
    } catch (caught) {
      yaml.value = ''
      error.value = caught instanceof Error ? caught.message : '转换失败'
    } finally {
      loading.value = false
    }
  }

  function scheduleSaveSettings() {
    if (!settingsLoaded.value) return

    if (saveTimer !== undefined) {
      window.clearTimeout(saveTimer)
    }

    saveTimer = window.setTimeout(() => {
      void persistSettings()
    }, 450)
  }

  async function persistSettings() {
    if (!settingsLoaded.value) return

    savingSettings.value = true
    try {
      await saveVlessToolSettings({
        input: input.value,
        mode: mode.value,
        template: template.value,
        downloadName: downloadName.value,
        directDomains: directDomains.value,
      })
    } catch (caught) {
      console.warn('Failed to save VLESS tool settings', caught)
    } finally {
      savingSettings.value = false
    }
  }

  function markCopied() {
    copied.value = true
    window.setTimeout(() => {
      copied.value = false
    }, 1600)
  }

  return {
    input,
    mode,
    template,
    proxyName,
    nodeAddress,
    downloadName,
    downloadFilename,
    directDomains,
    savingSettings,
    load,
    updateDownloadName,
    yaml,
    error,
    loading,
    copied,
    canConvert,
    convert,
    markCopied,
  }
})

function normalizeYamlFilename(value: string) {
  const name = value.trim() || 'mihomo'
  return /\.(ya?ml)$/i.test(name) ? name : `${name}.yaml`
}

function stripYamlExtension(value: string) {
  return value.replace(/\.(ya?ml)$/i, '').trim()
}

function extractVlessName(value: string) {
  const links = splitVlessLinks(value)
  if (links.length !== 1) return ''

  const raw = links[0] ?? ''
  if (!raw.toLowerCase().startsWith('vless://')) return ''

  try {
    const url = new URL(raw)
    return decodeURIComponent(url.hash.slice(1)).trim()
  } catch {
    const hashIndex = raw.indexOf('#')
    if (hashIndex < 0) return ''
    try {
      return decodeURIComponent(raw.slice(hashIndex + 1)).trim()
    } catch {
      return raw.slice(hashIndex + 1).trim()
    }
  }
}

function extractVlessAddress(value: string) {
  const links = splitVlessLinks(value)
  const addresses = links.map(extractSingleVlessAddress).filter(Boolean)
  return addresses.join(', ')
}

function extractSingleVlessAddress(raw: string) {
  if (!raw.toLowerCase().startsWith('vless://')) return ''

  try {
    const url = new URL(raw)
    const host = url.hostname
    if (!host) return ''

    const port = url.port || (isTlsVless(url) ? '443' : '80')
    return `${host}:${port}`
  } catch {
    return ''
  }
}

function isTlsVless(url: URL) {
  const security = url.searchParams.get('security')?.toLowerCase()
  return security === 'tls' || security === 'reality'
}

function splitVlessLinks(value: string) {
  return value
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean)
}

function parseDirectDomains(value: string) {
  return value
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line.length > 0 && !line.startsWith('#'))
}

function normalizeOutputMode(value: string): VlessOutputMode {
  return value === 'proxy_only' ? 'proxy_only' : 'full_config'
}

function normalizeTemplateMode(value: string): VlessTemplateMode {
  if (value === 'minimal' || value === 'standard' || value === 'full_rules') {
    return value
  }

  return 'full_rules'
}
