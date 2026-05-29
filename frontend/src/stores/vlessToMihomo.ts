import { computed, ref, watch } from 'vue'
import { defineStore } from 'pinia'
import {
  convertVlessToMihomo,
  getVlessToolSettings,
  saveVlessToolSettings,
  type VlessOutputMode,
  type VlessTemplateMode,
  type VlessTransitGroupType,
} from '../api/tools'

export const useVlessToMihomoStore = defineStore('vless-to-mihomo', () => {
  const input = ref('')
  const mode = ref<VlessOutputMode>('full_config')
  const template = ref<VlessTemplateMode>('full_rules')
  const downloadName = ref('mihomo')
  const downloadNameEdited = ref(false)
  const directDomains = ref('')
  const transitEnabled = ref(false)
  const transitProviderUrl = ref('')
  const transitProviderName = ref('transit')
  const transitProviderPath = ref('')
  const transitGroupName = ref('中转节点组')
  const transitGroupType = ref<VlessTransitGroupType>('url_test')
  const transitBypassDomains = ref('')
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

  watch(
    [
      input,
      mode,
      template,
      downloadName,
      directDomains,
      transitEnabled,
      transitProviderUrl,
      transitProviderName,
      transitProviderPath,
      transitGroupName,
      transitGroupType,
      transitBypassDomains,
    ],
    () => {
      scheduleSaveSettings()
    },
  )

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
      transitEnabled.value = Boolean(settings.transitEnabled)
      transitProviderUrl.value = settings.transitProviderUrl || ''
      transitProviderName.value = settings.transitProviderName || 'transit'
      transitProviderPath.value = settings.transitProviderPath || ''
      transitGroupName.value = settings.transitGroupName || '中转节点组'
      transitGroupType.value = normalizeTransitGroupType(settings.transitGroupType)
      transitBypassDomains.value = settings.transitBypassDomains || ''
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
        transit_proxy: buildTransitProxyPayload(),
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
        transitEnabled: transitEnabled.value,
        transitProviderUrl: transitProviderUrl.value,
        transitProviderName: transitProviderName.value,
        transitProviderPath: transitProviderPath.value,
        transitGroupName: transitGroupName.value,
        transitGroupType: transitGroupType.value,
        transitBypassDomains: transitBypassDomains.value,
      })
    } catch (caught) {
      console.warn('Failed to save VLESS tool settings', caught)
    } finally {
      savingSettings.value = false
    }
  }

  function buildTransitProxyPayload() {
    if (!transitEnabled.value) return undefined

    return {
      provider_name: emptyToUndefined(transitProviderName.value) || 'transit',
      provider_url: emptyToUndefined(transitProviderUrl.value),
      provider_path: emptyToUndefined(transitProviderPath.value),
      group_name: emptyToUndefined(transitGroupName.value) || '中转节点组',
      group_type: transitGroupType.value,
      bypass_domains: parseDirectDomains(transitBypassDomains.value),
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
    transitEnabled,
    transitProviderUrl,
    transitProviderName,
    transitProviderPath,
    transitGroupName,
    transitGroupType,
    transitBypassDomains,
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

function normalizeTransitGroupType(value: string): VlessTransitGroupType {
  if (value === 'select' || value === 'fallback' || value === 'url_test') {
    return value
  }

  return 'url_test'
}

function emptyToUndefined(value: string) {
  const trimmed = value.trim()
  return trimmed.length > 0 ? trimmed : undefined
}
