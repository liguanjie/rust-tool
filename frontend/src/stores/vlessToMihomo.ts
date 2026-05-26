import { computed, ref, watch } from 'vue'
import { defineStore } from 'pinia'
import {
  convertVlessToMihomo,
  type VlessOutputMode,
  type VlessTemplateMode,
} from '../api/tools'

export const useVlessToMihomoStore = defineStore('vless-to-mihomo', () => {
  const input = ref('')
  const mode = ref<VlessOutputMode>('full_config')
  const template = ref<VlessTemplateMode>('full_rules')
  const downloadName = ref('mihomo')
  const downloadNameEdited = ref(false)
  const yaml = ref('')
  const error = ref('')
  const loading = ref(false)
  const copied = ref(false)

  const canConvert = computed(() => input.value.trim().length > 0 && !loading.value)
  const downloadFilename = computed(() => normalizeYamlFilename(downloadName.value))
  const proxyName = computed(() => stripYamlExtension(downloadName.value.trim() || 'mihomo'))
  const nodeAddress = computed(() => extractVlessAddress(input.value))

  watch(input, (value) => {
    if (downloadNameEdited.value) return

    const name = extractVlessName(value)
    downloadName.value = name || 'mihomo'
  })

  function updateDownloadName(value: string) {
    downloadNameEdited.value = true
    downloadName.value = value
  }

  async function convert() {
    error.value = ''
    copied.value = false
    loading.value = true

    try {
      const result = await convertVlessToMihomo({
        input: input.value,
        mode: mode.value,
        template: template.value,
        proxy_name: proxyName.value || undefined,
      })
      yaml.value = result.yaml
    } catch (caught) {
      yaml.value = ''
      error.value = caught instanceof Error ? caught.message : '转换失败'
    } finally {
      loading.value = false
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
  const raw = value.trim()
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
  const raw = value.trim()
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
