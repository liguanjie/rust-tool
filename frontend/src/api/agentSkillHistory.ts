export interface AgentSkillHistoryRecord {
  id: string
  timestamp: number
  scriptName: string
  args: string
  exit_code: number
  success: boolean
  stdout: string
  stderr: string
}

const AGENT_SKILL_HISTORY_STORAGE_KEY = 'rusttool:codex:history'
const AGENT_SKILL_HISTORY_LIMIT = 50

type UnknownRecord = Partial<AgentSkillHistoryRecord> & {
  script_name?: unknown
  exitCode?: unknown
}

type TauriInvoke = <T>(command: string, args?: Record<string, unknown>) => Promise<T>

export async function listAgentSkillHistory(): Promise<AgentSkillHistoryRecord[]> {
  const tauriCore = await import('@tauri-apps/api/core')
  if (tauriCore.isTauri()) {
    const databaseHistory = normalizeHistory(
      await tauriCore.invoke<AgentSkillHistoryRecord[]>('get_agent_execution_history'),
    )
    if (databaseHistory.length > 0) return databaseHistory

    return await migrateLocalHistoryToTauri(tauriCore.invoke)
  }

  return readLocalHistory()
}

export async function saveAgentSkillHistoryRecord(
  record: AgentSkillHistoryRecord,
): Promise<AgentSkillHistoryRecord[]> {
  const normalizedRecord = normalizeHistoryRecord(record)
  if (!normalizedRecord) {
    throw new Error('执行历史记录无效')
  }

  const tauriCore = await import('@tauri-apps/api/core')
  if (tauriCore.isTauri()) {
    return normalizeHistory(
      await tauriCore.invoke<AgentSkillHistoryRecord[]>('save_agent_execution_history_record', {
        record: normalizedRecord,
      }),
    )
  }

  const history = upsertLocalHistory(normalizedRecord)
  persistLocalHistory(history)
  return history
}

export async function clearAgentSkillHistory(): Promise<void> {
  const tauriCore = await import('@tauri-apps/api/core')
  if (tauriCore.isTauri()) {
    await tauriCore.invoke<void>('clear_agent_execution_history')
  }

  window.localStorage.removeItem(AGENT_SKILL_HISTORY_STORAGE_KEY)
}

async function migrateLocalHistoryToTauri(invoke: TauriInvoke): Promise<AgentSkillHistoryRecord[]> {
  const localHistory = readLocalHistory()
  if (localHistory.length === 0) return []

  let importedHistory: AgentSkillHistoryRecord[] = []
  for (const record of [...localHistory].reverse()) {
    importedHistory = normalizeHistory(
      await invoke<AgentSkillHistoryRecord[]>('save_agent_execution_history_record', { record }),
    )
  }

  window.localStorage.removeItem(AGENT_SKILL_HISTORY_STORAGE_KEY)
  return importedHistory
}

function readLocalHistory(): AgentSkillHistoryRecord[] {
  const raw = window.localStorage.getItem(AGENT_SKILL_HISTORY_STORAGE_KEY)
  if (!raw) return []

  try {
    const parsed = JSON.parse(raw) as unknown
    return Array.isArray(parsed) ? normalizeHistory(parsed) : []
  } catch {
    return []
  }
}

function upsertLocalHistory(record: AgentSkillHistoryRecord): AgentSkillHistoryRecord[] {
  return [
    record,
    ...readLocalHistory().filter(
      (item) => !(item.scriptName === record.scriptName && item.args === record.args),
    ),
  ].slice(0, AGENT_SKILL_HISTORY_LIMIT)
}

function persistLocalHistory(history: AgentSkillHistoryRecord[]) {
  window.localStorage.setItem(AGENT_SKILL_HISTORY_STORAGE_KEY, JSON.stringify(history))
}

function normalizeHistory(records: unknown[]): AgentSkillHistoryRecord[] {
  const normalizedRecords = records
    .map(normalizeHistoryRecord)
    .filter((record): record is AgentSkillHistoryRecord => record !== null)
    .sort((left, right) => right.timestamp - left.timestamp)

  const seen = new Set<string>()
  const deduplicated: AgentSkillHistoryRecord[] = []
  for (const record of normalizedRecords) {
    const deduplicateKey = `${record.scriptName}\u0000${record.args}`
    if (seen.has(deduplicateKey)) continue
    seen.add(deduplicateKey)
    deduplicated.push(record)
  }

  return deduplicated.slice(0, AGENT_SKILL_HISTORY_LIMIT)
}

function normalizeHistoryRecord(record: unknown): AgentSkillHistoryRecord | null {
  if (!record || typeof record !== 'object') return null

  const value = record as UnknownRecord
  const scriptName = stringValue(value.scriptName ?? value.script_name).trim()
  if (!scriptName) return null

  return {
    id: stringValue(value.id).trim() || createHistoryRecordId(),
    timestamp: numberValue(value.timestamp, Date.now()),
    scriptName,
    args: stringValue(value.args),
    exit_code: numberValue(value.exit_code ?? value.exitCode, 0),
    success: Boolean(value.success),
    stdout: stringValue(value.stdout),
    stderr: stringValue(value.stderr),
  }
}

function stringValue(value: unknown): string {
  return typeof value === 'string' ? value : ''
}

function numberValue(value: unknown, fallback: number): number {
  const number = Number(value)
  return Number.isFinite(number) ? number : fallback
}

function createHistoryRecordId(): string {
  if (globalThis.crypto?.randomUUID) {
    return globalThis.crypto.randomUUID()
  }

  return `history-${Date.now()}-${Math.random().toString(16).slice(2)}`
}
