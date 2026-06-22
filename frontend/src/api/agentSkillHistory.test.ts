import { beforeEach, describe, expect, it, vi } from 'vitest'
import {
  clearAgentSkillHistory,
  listAgentSkillHistory,
  saveAgentSkillHistoryRecord,
  type AgentSkillHistoryRecord,
} from './agentSkillHistory'

vi.mock('@tauri-apps/api/core', () => ({
  isTauri: vi.fn(() => false),
  invoke: vi.fn(),
}))

describe('agentSkillHistory API adapter', () => {
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

  it('returns an empty list when Web history does not exist', async () => {
    await expect(listAgentSkillHistory()).resolves.toEqual([])
  })

  it('saves Web history with newest records first', async () => {
    await saveAgentSkillHistoryRecord(historyRecord('record-1', 'script-a', '--help', 1))
    const history = await saveAgentSkillHistoryRecord(
      historyRecord('record-2', 'script-b', '', 2),
    )

    expect(history.map((record) => record.id)).toEqual(['record-2', 'record-1'])
    await expect(listAgentSkillHistory()).resolves.toHaveLength(2)
  })

  it('deduplicates records with the same script and args', async () => {
    await saveAgentSkillHistoryRecord(historyRecord('record-1', 'script-a', '--help', 1))
    const history = await saveAgentSkillHistoryRecord(
      historyRecord('record-2', 'script-a', '--help', 2),
    )

    expect(history).toHaveLength(1)
    expect(history[0].id).toBe('record-2')
  })

  it('keeps only the latest fifty Web records', async () => {
    for (let index = 0; index < 55; index += 1) {
      await saveAgentSkillHistoryRecord(
        historyRecord(`record-${index}`, `script-${index}`, '', index),
      )
    }

    const history = await listAgentSkillHistory()

    expect(history).toHaveLength(50)
    expect(history[0].id).toBe('record-54')
    expect(history[49].id).toBe('record-5')
  })

  it('clears persisted Web history', async () => {
    await saveAgentSkillHistoryRecord(historyRecord('record-1', 'script-a', '', 1))
    await clearAgentSkillHistory()

    await expect(listAgentSkillHistory()).resolves.toEqual([])
  })

  it('ignores invalid stored Web history', async () => {
    window.localStorage.setItem('rusttool:codex:history', '{not-json')

    await expect(listAgentSkillHistory()).resolves.toEqual([])
  })
})

function historyRecord(
  id: string,
  scriptName: string,
  args: string,
  timestamp: number,
): AgentSkillHistoryRecord {
  return {
    id,
    timestamp,
    scriptName,
    args,
    exit_code: 0,
    success: true,
    stdout: 'ok',
    stderr: '',
  }
}
