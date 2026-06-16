export type OsvCommandKind = 'scan' | 'export' | 'fix'
export type OsvCommandStatus = 'succeeded' | 'failed'
export type OsvReportFormat = 'json' | 'html'
export type OsvSeverity = 'critical' | 'high' | 'medium' | 'low' | 'unknown'

export interface ApiErrorResponse {
  error?: {
    code?: string
    message?: string
  }
}

export interface OsvInstallStatus {
  installed: boolean
  binaryPath?: string
  version?: string
  message: string
}

export interface OsvScanOptions {
  recursive: boolean
  noIgnore: boolean
  includeGitRoot: boolean
  configPath?: string
  lockfiles: string[]
  experimentalExcludes: string[]
  allowNoLockfiles: boolean
  allPackages: boolean
  allVulns: boolean
  offline: boolean
  offlineVulnerabilities: boolean
  noResolve: boolean
  advancedArgs: string[]
}

export interface OsvCommandEditableOptions extends OsvScanOptions {}

export interface OsvScanCommandRequest {
  projectPath: string
  options: OsvScanOptions
}

export interface OsvScanRequest extends OsvScanCommandRequest {
  command: OsvCommandPreview
}

export interface OsvReportExportCommandRequest {
  projectPath: string
  options: OsvScanOptions
  format: OsvReportFormat
  outputPath: string
}

export interface OsvReportExportRequest extends OsvReportExportCommandRequest {
  command: OsvCommandPreview
}

export interface OsvIgnoreRequest {
  projectPath: string
  vulnerabilityId: string
  reason: string
}

export interface OsvCommandPreview {
  kind: OsvCommandKind
  binary: string
  workingDir: string
  argv: string[]
  displayCommand: string
  lockedArgs: string[]
  editableOptions: OsvCommandEditableOptions
  warnings: string[]
}

export interface OsvCommandExecutionRecord {
  id: string
  kind: OsvCommandKind
  projectPath: string
  workingDir: string
  argv: string[]
  displayCommand: string
  startedAt: string
  finishedAt?: string
  durationMs?: number
  exitCode?: number
  status: OsvCommandStatus
  summary: string
  stderrExcerpt?: string
}

export interface OsvProjectSettings {
  name: string
  path: string
  lastScanned?: string
  healthScore?: number
}

export interface OsvScannerSettings {
  projects: OsvProjectSettings[]
  autoScanSchedule: string
  commandHistory: OsvCommandExecutionRecord[]
}

export interface OsvPackageInfo {
  name: string
  version?: string
  ecosystem?: string
}

export interface OsvVulnerabilityFinding {
  id: string
  aliases: string[]
  summary?: string
  details?: string
  package: OsvPackageInfo
  severity: OsvSeverity
  affectedPaths: string[]
  fixedVersions: string[]
}

export interface OsvSeverityCounts {
  critical: number
  high: number
  medium: number
  low: number
  unknown: number
}

export interface OsvScanSummary {
  totalVulnerabilities: number
  severityCounts: OsvSeverityCounts
  highestSeverity: OsvSeverity
  healthScore: number
  message: string
}

export interface OsvScanResult {
  projectPath: string
  vulnerabilities: OsvVulnerabilityFinding[]
  summary: OsvScanSummary
  command: OsvCommandExecutionRecord
}

export interface OsvReportExportResult {
  format: OsvReportFormat
  outputPath: string
  command: OsvCommandExecutionRecord
}

export interface OsvIgnoreResult {
  projectPath: string
  configPath: string
  vulnerabilityId: string
  added: boolean
  message: string
}

const OSV_SETTINGS_STORAGE_KEY = 'rusttool:osv-scanner:settings'
const COMMAND_HISTORY_LIMIT = 50

export function defaultOsvScanOptions(): OsvScanOptions {
  return {
    recursive: true,
    noIgnore: false,
    includeGitRoot: false,
    configPath: '',
    lockfiles: [],
    experimentalExcludes: [],
    allowNoLockfiles: false,
    allPackages: false,
    allVulns: false,
    offline: false,
    offlineVulnerabilities: false,
    noResolve: false,
    advancedArgs: [],
  }
}

export function defaultOsvSettings(): OsvScannerSettings {
  return {
    projects: [],
    autoScanSchedule: 'none',
    commandHistory: [],
  }
}

export async function suggestOsvReportPath(
  projectPath: string,
  format: OsvReportFormat,
): Promise<string> {
  const filename = `${safeProjectName(projectPath)}-osv-report-${Date.now()}.${format}`
  const tauriCore = await import('@tauri-apps/api/core')
  if (tauriCore.isTauri()) {
    const pathApi = await import('@tauri-apps/api/path')
    return await pathApi.join(await pathApi.downloadDir(), filename)
  }

  return `/private/tmp/${filename}`
}

export async function getOsvSettings(): Promise<OsvScannerSettings> {
  const tauriCore = await import('@tauri-apps/api/core')
  if (tauriCore.isTauri()) {
    return await tauriCore.invoke<OsvScannerSettings>('get_osv_settings')
  }

  const raw = window.localStorage.getItem(OSV_SETTINGS_STORAGE_KEY)
  if (!raw) return defaultOsvSettings()

  try {
    return normalizeSettings(JSON.parse(raw) as Partial<OsvScannerSettings>)
  } catch {
    return defaultOsvSettings()
  }
}

export async function saveOsvSettings(settings: OsvScannerSettings): Promise<OsvScannerSettings> {
  const normalized = normalizeSettings(settings)
  const tauriCore = await import('@tauri-apps/api/core')
  if (tauriCore.isTauri()) {
    return await tauriCore.invoke<OsvScannerSettings>('save_osv_settings', {
      settings: normalized,
    })
  }

  window.localStorage.setItem(OSV_SETTINGS_STORAGE_KEY, JSON.stringify(normalized))
  return normalized
}

export async function checkOsvInstalled(): Promise<OsvInstallStatus> {
  const tauriCore = await import('@tauri-apps/api/core')
  if (tauriCore.isTauri()) {
    return await tauriCore.invoke<OsvInstallStatus>('check_osv_installed')
  }

  return await fetchJson<OsvInstallStatus>('/api/tools/osv-scanner/install-status')
}

export async function previewOsvScanCommand(
  request: OsvScanCommandRequest,
): Promise<OsvCommandPreview> {
  const tauriCore = await import('@tauri-apps/api/core')
  if (tauriCore.isTauri()) {
    return await tauriCore.invoke<OsvCommandPreview>('preview_osv_scan_command', { request })
  }

  return await fetchJson<OsvCommandPreview>('/api/tools/osv-scanner/scan/preview', request)
}

export async function scanOsvProject(request: OsvScanRequest): Promise<OsvScanResult> {
  const tauriCore = await import('@tauri-apps/api/core')
  if (tauriCore.isTauri()) {
    return await tauriCore.invoke<OsvScanResult>('scan_osv_project', { request })
  }

  return await fetchJson<OsvScanResult>('/api/tools/osv-scanner/scan', request)
}

export async function previewOsvReportExportCommand(
  request: OsvReportExportCommandRequest,
): Promise<OsvCommandPreview> {
  const tauriCore = await import('@tauri-apps/api/core')
  if (tauriCore.isTauri()) {
    return await tauriCore.invoke<OsvCommandPreview>('preview_osv_report_export_command', {
      request,
    })
  }

  return await fetchJson<OsvCommandPreview>('/api/tools/osv-scanner/export/preview', request)
}

export async function exportOsvReport(
  request: OsvReportExportRequest,
): Promise<OsvReportExportResult> {
  const tauriCore = await import('@tauri-apps/api/core')
  if (tauriCore.isTauri()) {
    return await tauriCore.invoke<OsvReportExportResult>('export_osv_report', { request })
  }

  return await fetchJson<OsvReportExportResult>('/api/tools/osv-scanner/export', request)
}

export async function ignoreOsvVulnerability(
  request: OsvIgnoreRequest,
): Promise<OsvIgnoreResult> {
  const tauriCore = await import('@tauri-apps/api/core')
  if (tauriCore.isTauri()) {
    return await tauriCore.invoke<OsvIgnoreResult>('ignore_osv_vulnerability', { request })
  }

  return await fetchJson<OsvIgnoreResult>('/api/tools/osv-scanner/ignore', request)
}

async function fetchJson<T>(url: string, body?: unknown): Promise<T> {
  const response = await fetch(url, {
    method: body === undefined ? 'GET' : 'POST',
    headers: body === undefined ? undefined : { 'Content-Type': 'application/json' },
    body: body === undefined ? undefined : JSON.stringify(body),
  })

  if (!response.ok) {
    let message = 'OSV 操作失败'
    try {
      const data = (await response.json()) as ApiErrorResponse
      message = data.error?.message || message
    } catch {
      message = await response.text()
    }
    throw new Error(message)
  }

  return (await response.json()) as T
}

function normalizeSettings(settings: Partial<OsvScannerSettings>): OsvScannerSettings {
  const defaults = defaultOsvSettings()
  return {
    projects: Array.isArray(settings.projects) ? settings.projects : defaults.projects,
    autoScanSchedule: settings.autoScanSchedule || defaults.autoScanSchedule,
    commandHistory: trimHistory(
      Array.isArray(settings.commandHistory) ? settings.commandHistory : defaults.commandHistory,
    ),
  }
}

function trimHistory(history: OsvCommandExecutionRecord[]): OsvCommandExecutionRecord[] {
  return history.slice(-COMMAND_HISTORY_LIMIT)
}

function safeProjectName(projectPath: string): string {
  const name = projectPath.split(/[\\/]/).filter(Boolean).pop() || 'project'
  return name
    .replace(/[^a-zA-Z0-9._-]+/g, '-')
    .replace(/^-+|-+$/g, '')
    .slice(0, 64) || 'project'
}
