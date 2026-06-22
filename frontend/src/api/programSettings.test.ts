import { beforeEach, describe, expect, it, vi } from 'vitest'
import {
  backupProgramDatabase,
  clearProgramDatabaseHistory,
  compactProgramDatabase,
  deleteLegacyProgramDatabase,
  defaultProgramSettings,
  getProgramSettings,
  openProgramDatabaseDirectory,
  restoreProgramDatabase,
  saveProgramSettings,
} from './programSettings'

vi.mock('@tauri-apps/api/core', () => ({
  isTauri: vi.fn(() => false),
  invoke: vi.fn(),
}))

describe('programSettings API adapter', () => {
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

  it('returns the Web default database path when no settings exist', async () => {
    const state = await getProgramSettings()

    expect(state.settings).toEqual(defaultProgramSettings())
    expect(state.defaultDatabasePath).toBe('./data/rusttool.db')
    expect(state.effectiveDatabasePath).toBe('./data/rusttool.db')
    expect(state.databaseHealth.status).toBe('unavailable')
    expect(state.databaseHealth.schemaVersion).toBeNull()
    expect(state.databaseStats.totalSizeBytes).toBe(0)
    expect(state.databaseStats.databasePath).toBe('./data/rusttool.db')
    expect(state.databaseDiagnostics.totalRecords).toBe(0)
    expect(state.databaseDiagnostics.recordCounts).toEqual([])
    expect(state.legacyDatabase.exists).toBe(false)
    expect(state.legacyDatabase.path).toBe('./data/rusttool.sqlite')
  })

  it('trims and persists the configured database path', async () => {
    const state = await saveProgramSettings({
      databasePath: '  /Users/ben/RustTool/rusttool.db  ',
    })

    expect(state.settings.databasePath).toBe('/Users/ben/RustTool/rusttool.db')
    expect(state.effectiveDatabasePath).toBe('/Users/ben/RustTool/rusttool.db')

    const loaded = await getProgramSettings()
    expect(loaded.settings.databasePath).toBe('/Users/ben/RustTool/rusttool.db')
  })

  it('falls back to defaults when stored settings are invalid', async () => {
    window.localStorage.setItem('rusttool:program-settings', '{not-json')

    const state = await getProgramSettings()

    expect(state.settings.databasePath).toBe('')
    expect(state.effectiveDatabasePath).toBe('./data/rusttool.db')
  })

  it('returns current settings when clearing history in Web mode', async () => {
    await saveProgramSettings({
      databasePath: '/Users/ben/RustTool/rusttool.db',
    })

    const state = await clearProgramDatabaseHistory()

    expect(state.effectiveDatabasePath).toBe('/Users/ben/RustTool/rusttool.db')
  })

  it('rejects desktop-only maintenance actions in Web mode', async () => {
    await expect(backupProgramDatabase()).rejects.toThrow('Web 模式不支持备份')
    await expect(compactProgramDatabase()).rejects.toThrow('Web 模式不支持压缩')
    await expect(restoreProgramDatabase('./backup.db')).rejects.toThrow('Web 模式不支持恢复')
    await expect(openProgramDatabaseDirectory()).rejects.toThrow('Web 模式不支持打开')
    await expect(deleteLegacyProgramDatabase()).rejects.toThrow('Web 模式不支持清理旧数据库')
  })
})
