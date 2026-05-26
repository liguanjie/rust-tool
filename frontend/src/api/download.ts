export interface SaveYamlResponse {
  path: string
}

export async function downloadYaml(text: string, filename: string): Promise<SaveYamlResponse | null> {
  if (!text) return null

  const tauriCore = await import('@tauri-apps/api/core')
  if (tauriCore.isTauri()) {
    return await tauriCore.invoke<SaveYamlResponse>('save_yaml_file', {
      filename,
      content: text,
    })
  }

  const blob = new Blob([text], { type: 'text/yaml;charset=utf-8' })
  const url = URL.createObjectURL(blob)
  const link = document.createElement('a')
  link.href = url
  link.download = filename
  link.click()
  URL.revokeObjectURL(url)

  return null
}
