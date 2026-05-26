export type VlessOutputMode = 'full_config' | 'proxy_only'
export type VlessTemplateMode = 'minimal' | 'standard' | 'full_rules'

export interface ConvertVlessRequest {
  input: string
  mode: VlessOutputMode
  template: VlessTemplateMode
  proxy_name?: string
  direct_domains?: string[]
}

export interface ConvertVlessResponse {
  yaml: string
}

export interface ApiErrorResponse {
  error?: {
    code?: string
    message?: string
  }
}

export interface VlessToolSettings {
  input: string
  mode: VlessOutputMode
  template: VlessTemplateMode
  downloadName: string
  directDomains: string
}

const VLESS_TOOL_SETTINGS_STORAGE_KEY = 'rusttool:vless-to-mihomo:settings'

export function defaultVlessToolSettings(): VlessToolSettings {
  return {
    input: '',
    mode: 'full_config',
    template: 'full_rules',
    downloadName: 'mihomo',
    directDomains: '',
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

  if (!response.ok) {
    let message = '转换失败'
    try {
      const data = (await response.json()) as ApiErrorResponse
      message = data.error?.message || message
    } catch {
      message = await response.text()
    }
    throw new Error(message)
  }

  return (await response.json()) as ConvertVlessResponse
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
