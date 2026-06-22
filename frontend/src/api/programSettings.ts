export interface ProgramSettings {
  databasePath: string
}

export interface ProgramSettingsState {
  settings: ProgramSettings
  defaultDatabasePath: string
  effectiveDatabasePath: string
  databaseHealth: DatabaseHealth
  databaseStats: DatabaseFileStats
  databaseDiagnostics: DatabaseStorageDiagnostics
  legacyDatabase: LegacyDatabaseInfo
}

export type DatabaseHealthStatus = 'ready' | 'error' | 'unavailable'

export interface DatabaseHealth {
  databasePath: string
  status: DatabaseHealthStatus
  databaseExists: boolean
  parentDirectoryExists: boolean
  schemaVersion: number | null
  appliedMigrations: number
  message: string
}

export interface DatabaseFileStats {
  databasePath: string
  mainFileSizeBytes: number
  walFileSizeBytes: number
  shmFileSizeBytes: number
  totalSizeBytes: number
}

export interface DatabaseBackupResult {
  backupPath: string
  databaseStats: DatabaseFileStats
}

export interface DatabaseRestoreResult {
  safetyBackupPath: string
  state: ProgramSettingsState
}

export interface DatabaseRecordCount {
  key: string
  label: string
  count: number
}

export interface DatabaseStorageDiagnostics {
  totalRecords: number
  recordCounts: DatabaseRecordCount[]
}

export interface LegacyDatabaseInfo {
  path: string
  exists: boolean
  mainFileSizeBytes: number
  walFileSizeBytes: number
  shmFileSizeBytes: number
  totalSizeBytes: number
}

const PROGRAM_SETTINGS_STORAGE_KEY = 'rusttool:program-settings'
const WEB_DEFAULT_DATABASE_PATH = './data/rusttool.db'

export function defaultProgramSettings(): ProgramSettings {
  return {
    databasePath: '',
  }
}

export async function getProgramSettings(): Promise<ProgramSettingsState> {
  const tauriCore = await import('@tauri-apps/api/core')
  if (tauriCore.isTauri()) {
    return await tauriCore.invoke<ProgramSettingsState>('get_program_settings')
  }

  const raw = window.localStorage.getItem(PROGRAM_SETTINGS_STORAGE_KEY)
  if (!raw) return settingsState(defaultProgramSettings())

  try {
    return settingsState(normalizeProgramSettings(JSON.parse(raw) as Partial<ProgramSettings>))
  } catch {
    return settingsState(defaultProgramSettings())
  }
}

export async function saveProgramSettings(
  settings: ProgramSettings,
): Promise<ProgramSettingsState> {
  const normalized = normalizeProgramSettings(settings)
  const tauriCore = await import('@tauri-apps/api/core')
  if (tauriCore.isTauri()) {
    return await tauriCore.invoke<ProgramSettingsState>('save_program_settings', {
      settings: normalized,
    })
  }

  window.localStorage.setItem(PROGRAM_SETTINGS_STORAGE_KEY, JSON.stringify(normalized))
  return settingsState(normalized)
}

export async function backupProgramDatabase(): Promise<DatabaseBackupResult> {
  const tauriCore = await import('@tauri-apps/api/core')
  if (tauriCore.isTauri()) {
    return await tauriCore.invoke<DatabaseBackupResult>('backup_program_database')
  }

  throw new Error('Web 模式不支持备份本地 SQLite 数据库。')
}

export async function compactProgramDatabase(): Promise<ProgramSettingsState> {
  const tauriCore = await import('@tauri-apps/api/core')
  if (tauriCore.isTauri()) {
    return await tauriCore.invoke<ProgramSettingsState>('compact_program_database')
  }

  throw new Error('Web 模式不支持压缩本地 SQLite 数据库。')
}

export async function restoreProgramDatabase(backupPath: string): Promise<DatabaseRestoreResult> {
  const tauriCore = await import('@tauri-apps/api/core')
  if (tauriCore.isTauri()) {
    return await tauriCore.invoke<DatabaseRestoreResult>('restore_program_database', {
      request: {
        backupPath: backupPath.trim(),
      },
    })
  }

  throw new Error('Web 模式不支持恢复本地 SQLite 数据库。')
}

export async function openProgramDatabaseDirectory(): Promise<void> {
  const tauriCore = await import('@tauri-apps/api/core')
  if (tauriCore.isTauri()) {
    await tauriCore.invoke('open_program_database_directory')
    return
  }

  throw new Error('Web 模式不支持打开本地数据库目录。')
}

export async function deleteLegacyProgramDatabase(): Promise<ProgramSettingsState> {
  const tauriCore = await import('@tauri-apps/api/core')
  if (tauriCore.isTauri()) {
    return await tauriCore.invoke<ProgramSettingsState>('delete_legacy_program_database')
  }

  throw new Error('Web 模式不支持清理旧数据库。')
}

export async function clearProgramDatabaseHistory(): Promise<ProgramSettingsState> {
  const tauriCore = await import('@tauri-apps/api/core')
  if (tauriCore.isTauri()) {
    return await tauriCore.invoke<ProgramSettingsState>('clear_program_database_history')
  }

  return await getProgramSettings()
}

function normalizeProgramSettings(settings: Partial<ProgramSettings>): ProgramSettings {
  return {
    databasePath: settings.databasePath?.trim() ?? '',
  }
}

function settingsState(settings: ProgramSettings): ProgramSettingsState {
  const effectiveDatabasePath = settings.databasePath || WEB_DEFAULT_DATABASE_PATH
  return {
    settings,
    defaultDatabasePath: WEB_DEFAULT_DATABASE_PATH,
    effectiveDatabasePath,
    databaseHealth: {
      databasePath: effectiveDatabasePath,
      status: 'unavailable',
      databaseExists: false,
      parentDirectoryExists: false,
      schemaVersion: null,
      appliedMigrations: 0,
      message: 'Web 模式不会初始化本地 SQLite 数据库。',
    },
    databaseStats: {
      databasePath: effectiveDatabasePath,
      mainFileSizeBytes: 0,
      walFileSizeBytes: 0,
      shmFileSizeBytes: 0,
      totalSizeBytes: 0,
    },
    databaseDiagnostics: {
      totalRecords: 0,
      recordCounts: [],
    },
    legacyDatabase: {
      path: './data/rusttool.sqlite',
      exists: false,
      mainFileSizeBytes: 0,
      walFileSizeBytes: 0,
      shmFileSizeBytes: 0,
      totalSizeBytes: 0,
    },
  }
}
