import { readJsonResponse } from './http'

export type VlessOutputMode = 'full_config' | 'proxy_only'
export type VlessTemplateMode = 'minimal' | 'standard' | 'full_rules'
export type VlessTransitGroupType = 'select' | 'url_test' | 'fallback'

export interface VlessTransitProxyRequest {
  provider_name: string
  provider_url?: string
  provider_path?: string
  group_name: string
  group_type: VlessTransitGroupType
  bypass_domains?: string[]
  providers?: VlessTransitProviderRequest[]
}

export interface VlessTransitProviderRequest {
  provider_name: string
  provider_url?: string
  provider_path?: string
  group_name: string
}

export interface ConvertVlessRequest {
  input: string
  mode: VlessOutputMode
  template: VlessTemplateMode
  proxy_name?: string
  direct_domains?: string[]
  transit_proxy?: VlessTransitProxyRequest
}

export interface ConvertVlessResponse {
  yaml: string
}

export interface VlessToolSettings {
  input: string
  mode: VlessOutputMode
  template: VlessTemplateMode
  downloadName: string
  directDomains: string
  transitEnabled: boolean
  transitProviderUrl: string
  transitProviderName: string
  transitProviderPath: string
  transitGroupName: string
  transitGroupType: VlessTransitGroupType
  transitBypassDomains: string
}

const VLESS_TOOL_SETTINGS_STORAGE_KEY = 'rusttool:vless-to-mihomo:settings'

export function defaultVlessToolSettings(): VlessToolSettings {
  return {
    input: '',
    mode: 'full_config',
    template: 'full_rules',
    downloadName: 'mihomo',
    directDomains: '',
    transitEnabled: false,
    transitProviderUrl: '',
    transitProviderName: 'transit',
    transitProviderPath: '',
    transitGroupName: '中转节点组',
    transitGroupType: 'url_test',
    transitBypassDomains: '',
  }
}

export async function convertVlessToMihomo(
  payload: ConvertVlessRequest,
): Promise<ConvertVlessResponse> {
  const tauriCore = await import('@tauri-apps/api/core')
  if (tauriCore.isTauri()) {
    const { invoke } = tauriCore
    return await invoke<ConvertVlessResponse>('convert_vless_to_mihomo', {
      request: payload,
    })
  }

  const response = await fetch('/api/tools/vless-to-mihomo', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(payload),
  })

  return await readJsonResponse<ConvertVlessResponse>(response, '转换失败')
}

export async function getVlessToolSettings(): Promise<VlessToolSettings> {
  const tauriCore = await import('@tauri-apps/api/core')
  if (tauriCore.isTauri()) {
    const { invoke } = tauriCore
    return await invoke<VlessToolSettings>('get_vless_tool_settings')
  }

  const raw = window.localStorage.getItem(VLESS_TOOL_SETTINGS_STORAGE_KEY)
  if (!raw) return defaultVlessToolSettings()

  try {
    return {
      ...defaultVlessToolSettings(),
      ...(JSON.parse(raw) as Partial<VlessToolSettings>),
    }
  } catch {
    return defaultVlessToolSettings()
  }
}

export async function saveVlessToolSettings(
  settings: VlessToolSettings,
): Promise<VlessToolSettings> {
  const tauriCore = await import('@tauri-apps/api/core')
  if (tauriCore.isTauri()) {
    const { invoke } = tauriCore
    return await invoke<VlessToolSettings>('save_vless_tool_settings', {
      settings,
    })
  }

  window.localStorage.setItem(VLESS_TOOL_SETTINGS_STORAGE_KEY, JSON.stringify(settings))
  return settings
}
