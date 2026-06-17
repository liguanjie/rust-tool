interface StandardErrorResponse {
  error?: {
    message?: string
  }
}

export async function readJsonResponse<T>(response: Response, fallbackMessage: string): Promise<T> {
  const text = await response.text()
  const trimmed = text.trim()
  let payload: unknown = null

  if (trimmed) {
    try {
      payload = JSON.parse(trimmed) as unknown
    } catch {
      const plainMessage = plainErrorText(trimmed)
      if (!response.ok) {
        throw new Error(plainMessage || fallbackMessage)
      }
      throw new Error(`${fallbackMessage}：后端返回了无效 JSON 响应`)
    }
  }

  if (!response.ok) {
    throw new Error(errorMessageFromPayload(payload) || plainErrorText(trimmed) || fallbackMessage)
  }

  if (!trimmed) {
    throw new Error(`${fallbackMessage}：后端返回空响应`)
  }

  return payload as T
}

function errorMessageFromPayload(payload: unknown): string {
  if (!payload || typeof payload !== 'object') return ''
  const message = (payload as StandardErrorResponse).error?.message
  return typeof message === 'string' ? message.trim() : ''
}

function plainErrorText(text: string): string {
  if (!text) return ''
  const lower = text.slice(0, 64).toLowerCase()
  if (lower.startsWith('<!doctype') || lower.startsWith('<html')) return ''
  return text.slice(0, 500)
}
