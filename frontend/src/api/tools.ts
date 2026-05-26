export type VlessOutputMode = 'full_config' | 'proxy_only'
export type VlessTemplateMode = 'minimal' | 'standard' | 'full_rules'

export interface ConvertVlessRequest {
  input: string
  mode: VlessOutputMode
  template: VlessTemplateMode
  proxy_name?: string
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
