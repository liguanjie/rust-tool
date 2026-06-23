import { beforeEach, describe, expect, it, vi } from 'vitest'
import {
  defaultAgentSkillsSettings,
  getAgentSkillsSettings,
  saveAgentSkillsSettings,
} from './agentSkillsSettings'

vi.mock('@tauri-apps/api/core', () => ({
  isTauri: vi.fn(() => false),
  invoke: vi.fn(),
}))

describe('agentSkillsSettings API adapter', () => {
  const storage = new Map<string, string>()

  beforeEach(() => {
    storage.clear()
    Object.defineProperty(window, 'localStorage', {
      configurable: true,
      value: {
        getItem: vi.fn((key: string) => storage.get(key) ?? null),
        setItem: vi.fn((key: string, value: string) => {
          storage.set(key, value)
        }),
        removeItem: vi.fn((key: string) => {
          storage.delete(key)
        }),
      },
    })
  })

  it('loads Web settings from the backend API', async () => {
    const fetchMock = vi.fn().mockResolvedValue(jsonResponse({
      success: true,
      data: {
        scriptDir: ' /Users/alice/work/99_codex ',
      },
    }))
    vi.stubGlobal('fetch', fetchMock)

    await expect(getAgentSkillsSettings()).resolves.toEqual({
      scriptDir: '/Users/alice/work/99_codex',
    })
    expect(fetchMock).toHaveBeenCalledWith('/api/workbench/settings/agent-skills')
  })

  it('saves Web settings through the backend API', async () => {
    const fetchMock = vi.fn().mockResolvedValue(jsonResponse({
      success: true,
      data: {
        scriptDir: '/Users/alice/work/99_codex',
      },
    }))
    vi.stubGlobal('fetch', fetchMock)

    await expect(
      saveAgentSkillsSettings({
        scriptDir: ' /Users/alice/work/99_codex ',
      }),
    ).resolves.toEqual({
      scriptDir: '/Users/alice/work/99_codex',
    })
    expect(fetchMock).toHaveBeenCalledWith('/api/workbench/settings/agent-skills', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        settings: {
          scriptDir: '/Users/alice/work/99_codex',
        },
      }),
    })
  })

  it('falls back to local settings when the backend is unavailable', async () => {
    vi.stubGlobal('fetch', vi.fn().mockRejectedValue(new Error('offline')))

    await saveAgentSkillsSettings({
      scriptDir: ' /Users/alice/work/99_codex ',
    })

    await expect(getAgentSkillsSettings()).resolves.toEqual({
      scriptDir: '/Users/alice/work/99_codex',
    })
  })

  it('returns defaults when no settings exist', async () => {
    vi.stubGlobal('fetch', vi.fn().mockRejectedValue(new Error('offline')))

    await expect(getAgentSkillsSettings()).resolves.toEqual(defaultAgentSkillsSettings())
  })
})

function jsonResponse(value: unknown) {
  return {
    json: () => Promise.resolve(value),
  }
}
