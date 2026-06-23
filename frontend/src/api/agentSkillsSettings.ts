export interface AgentSkillsSettings {
  scriptDir: string
}

const AGENT_SKILLS_SETTINGS_STORAGE_KEY = 'rusttool:agent-skills:settings'

export function defaultAgentSkillsSettings(): AgentSkillsSettings {
  return {
    scriptDir: '',
  }
}

export async function getAgentSkillsSettings(): Promise<AgentSkillsSettings> {
  const tauriCore = await import('@tauri-apps/api/core')
  if (tauriCore.isTauri()) {
    const settings = await tauriCore.invoke<AgentSkillsSettings>('get_agent_skills_settings')
    return normalizeAgentSkillsSettings(settings)
  }

  try {
    const response = await fetch('/api/workbench/settings/agent-skills')
    const json = await response.json()
    if (json.success) {
      return normalizeAgentSkillsSettings(json.data)
    }
  } catch {
    return readLocalSettings()
  }

  return readLocalSettings()
}

export async function saveAgentSkillsSettings(
  settings: AgentSkillsSettings,
): Promise<AgentSkillsSettings> {
  const normalized = normalizeAgentSkillsSettings(settings)
  const tauriCore = await import('@tauri-apps/api/core')
  if (tauriCore.isTauri()) {
    return normalizeAgentSkillsSettings(
      await tauriCore.invoke<AgentSkillsSettings>('save_agent_skills_settings', {
        settings: normalized,
      }),
    )
  }

  try {
    const response = await fetch('/api/workbench/settings/agent-skills', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        settings: normalized,
      }),
    })
    const json = await response.json()
    if (json.success) {
      return normalizeAgentSkillsSettings(json.data)
    }
  } catch {
    persistLocalSettings(normalized)
    return normalized
  }

  persistLocalSettings(normalized)
  return normalized
}

function readLocalSettings(): AgentSkillsSettings {
  const raw = window.localStorage.getItem(AGENT_SKILLS_SETTINGS_STORAGE_KEY)
  if (!raw) return defaultAgentSkillsSettings()

  try {
    return normalizeAgentSkillsSettings(JSON.parse(raw) as Partial<AgentSkillsSettings>)
  } catch {
    return defaultAgentSkillsSettings()
  }
}

function persistLocalSettings(settings: AgentSkillsSettings) {
  window.localStorage.setItem(AGENT_SKILLS_SETTINGS_STORAGE_KEY, JSON.stringify(settings))
}

function normalizeAgentSkillsSettings(
  settings: Partial<AgentSkillsSettings> | null | undefined,
): AgentSkillsSettings {
  return {
    scriptDir: settings?.scriptDir?.trim() ?? '',
  }
}
