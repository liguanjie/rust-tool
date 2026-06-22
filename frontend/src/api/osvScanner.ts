import { readJsonResponse } from './http'

export type OsvCommandKind = 'scan' | 'export' | 'fix'
export type OsvCommandStatus = 'succeeded' | 'failed'
export type OsvReportFormat = 'json' | 'html'
export type OsvSeverity = 'critical' | 'high' | 'medium' | 'low' | 'unknown'

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

export interface OsvProjectDiagnosticRequest extends OsvScanCommandRequest {}

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

export type OsvDiagnosticLevel = 'info' | 'warning' | 'error'

export interface OsvDiagnosticMessage {
  level: OsvDiagnosticLevel
  code: string
  message: string
  suggestion?: string
}

export interface OsvPackageSource {
  path: string
  kind: string
  ecosystem: string
  explicit: boolean
}

export interface OsvProjectDiagnostic {
  projectPath: string
  packageSources: OsvPackageSource[]
  messages: OsvDiagnosticMessage[]
  canScan: boolean
  scannedEntries: number
  truncated: boolean
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
const OSV_LATEST_SCAN_RESULTS_STORAGE_KEY = 'rusttool:osv-scanner:latest-results'
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

export async function diagnoseOsvProject(
  request: OsvProjectDiagnosticRequest,
): Promise<OsvProjectDiagnostic> {
  const tauriCore = await import('@tauri-apps/api/core')
  if (tauriCore.isTauri()) {
    return await tauriCore.invoke<OsvProjectDiagnostic>('diagnose_osv_project', { request })
  }

  return await fetchJson<OsvProjectDiagnostic>('/api/tools/osv-scanner/diagnose', request)
}

export async function scanOsvProject(request: OsvScanRequest): Promise<OsvScanResult> {
  const tauriCore = await import('@tauri-apps/api/core')
  if (tauriCore.isTauri()) {
    return await tauriCore.invoke<OsvScanResult>('scan_osv_project', { request })
  }

  return await fetchJson<OsvScanResult>('/api/tools/osv-scanner/scan', request)
}

export async function getOsvLatestScanResult(projectPath: string): Promise<OsvScanResult | null> {
  const normalizedProjectPath = projectPath.trim()
  if (!normalizedProjectPath) return null

  const tauriCore = await import('@tauri-apps/api/core')
  if (tauriCore.isTauri()) {
    return await tauriCore.invoke<OsvScanResult | null>('get_osv_latest_scan_result', {
      projectPath: normalizedProjectPath,
    })
  }

  return readLatestScanResults()[normalizedProjectPath] ?? null
}

export async function saveOsvLatestScanResult(result: OsvScanResult): Promise<void> {
  const normalizedProjectPath = result.projectPath.trim()
  if (!normalizedProjectPath) {
    throw new Error('项目路径不能为空')
  }

  const normalizedResult = {
    ...result,
    projectPath: normalizedProjectPath,
  }
  const tauriCore = await import('@tauri-apps/api/core')
  if (tauriCore.isTauri()) {
    await tauriCore.invoke<void>('save_osv_latest_scan_result', {
      result: normalizedResult,
    })
    return
  }

  const latestResults = readLatestScanResults()
  latestResults[normalizedProjectPath] = normalizedResult
  window.localStorage.setItem(
    OSV_LATEST_SCAN_RESULTS_STORAGE_KEY,
    JSON.stringify(latestResults),
  )
}

export async function deleteOsvLatestScanResult(projectPath: string): Promise<void> {
  const normalizedProjectPath = projectPath.trim()
  if (!normalizedProjectPath) return

  const tauriCore = await import('@tauri-apps/api/core')
  if (tauriCore.isTauri()) {
    await tauriCore.invoke<void>('delete_osv_latest_scan_result', {
      projectPath: normalizedProjectPath,
    })
    return
  }

  const latestResults = readLatestScanResults()
  delete latestResults[normalizedProjectPath]
  window.localStorage.setItem(
    OSV_LATEST_SCAN_RESULTS_STORAGE_KEY,
    JSON.stringify(latestResults),
  )
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
    return await readJsonResponse<T>(response, 'OSV 操作失败')
  }

  return await readJsonResponse<T>(response, 'OSV 操作失败')
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

function readLatestScanResults(): Record<string, OsvScanResult> {
  const raw = window.localStorage.getItem(OSV_LATEST_SCAN_RESULTS_STORAGE_KEY)
  if (!raw) return {}

  try {
    const parsed = JSON.parse(raw) as unknown
    if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) return {}
    return Object.fromEntries(
      Object.entries(parsed as Record<string, unknown>).filter(
        (entry): entry is [string, OsvScanResult] => isOsvScanResult(entry[1]),
      ),
    )
  } catch {
    return {}
  }
}

function isOsvScanResult(value: unknown): value is OsvScanResult {
  if (!value || typeof value !== 'object') return false
  const result = value as Partial<OsvScanResult>
  return (
    typeof result.projectPath === 'string'
    && Array.isArray(result.vulnerabilities)
    && Boolean(result.summary)
    && Boolean(result.command)
  )
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
