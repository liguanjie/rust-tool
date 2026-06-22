import { computed, ref } from 'vue'
import { defineStore } from 'pinia'
import {
  checkOsvInstalled,
  defaultOsvScanOptions,
  deleteOsvLatestScanResult,
  diagnoseOsvProject,
  exportOsvReport,
  getOsvLatestScanResult,
  getOsvSettings,
  ignoreOsvVulnerability,
  previewOsvReportExportCommand,
  previewOsvScanCommand,
  saveOsvLatestScanResult,
  saveOsvSettings,
  scanOsvProject,
  suggestOsvReportPath,
  type OsvCommandExecutionRecord,
  type OsvCommandPreview,
  type OsvInstallStatus,
  type OsvProjectDiagnostic,
  type OsvProjectSettings,
  type OsvReportFormat,
  type OsvScanOptions,
  type OsvScanResult,
  type OsvVulnerabilityFinding,
} from '../api/osvScanner'

const COMMAND_HISTORY_LIMIT = 50

type OsvOperationKind =
  | 'idle'
  | 'diagnose'
  | 'preview-scan'
  | 'scan'
  | 'preview-export'
  | 'export'
  | 'ignore'

type OsvOperationStatus = 'idle' | 'running' | 'succeeded' | 'failed'

interface OsvOperationState {
  kind: OsvOperationKind
  status: OsvOperationStatus
  title: string
  message: string
  detail?: string
  command?: string
  startedAt?: number
  finishedAt?: number
}

function idleOperation(): OsvOperationState {
  return {
    kind: 'idle',
    status: 'idle',
    title: '',
    message: '',
  }
}

export const useOsvScannerStore = defineStore('osv-scanner', () => {
  const projects = ref<OsvProjectSettings[]>([])
  const autoScanSchedule = ref('none')
  const commandHistory = ref<OsvCommandExecutionRecord[]>([])
  const installStatus = ref<OsvInstallStatus | null>(null)
  const settingsLoaded = ref(false)
  const loading = ref(false)
  const scanningPath = ref('')
  const exporting = ref(false)
  const error = ref('')
  const notice = ref('')
  const activeProjectPath = ref('')
  const currentPreview = ref<OsvCommandPreview | null>(null)
  const currentExportPreview = ref<OsvCommandPreview | null>(null)
  const latestResult = ref<OsvScanResult | null>(null)
  const diagnostic = ref<OsvProjectDiagnostic | null>(null)
  const diagnosing = ref(false)
  const options = ref<OsvScanOptions>(defaultOsvScanOptions())
  const operation = ref<OsvOperationState>(idleOperation())

  const activeProject = computed(() =>
    projects.value.find((project) => project.path === activeProjectPath.value) ?? null,
  )
  const hasCurrentScanResult = computed(() =>
    Boolean(activeProjectPath.value && latestResult.value?.projectPath === activeProjectPath.value),
  )
  const vulnerabilities = computed<OsvVulnerabilityFinding[]>(
    () => latestResult.value?.vulnerabilities ?? [],
  )
  const globalHealthScore = computed(() => {
    const scores = projects.value
      .map((project) => project.healthScore)
      .filter((score): score is number => typeof score === 'number')
    if (!scores.length) return undefined
    return Math.round(scores.reduce((sum, score) => sum + score, 0) / scores.length)
  })

  async function load() {
    if (settingsLoaded.value) return

    try {
      const settings = await getOsvSettings()
      projects.value = settings.projects
      activeProjectPath.value = settings.projects[0]?.path ?? ''
      autoScanSchedule.value = settings.autoScanSchedule
      commandHistory.value = settings.commandHistory
      await refreshInstallStatus()
      if (activeProjectPath.value) {
        await restoreLatestScanResult(activeProjectPath.value)
        await diagnoseActiveProject(activeProjectPath.value, { quiet: true })
      }
    } catch (caught) {
      error.value = caught instanceof Error ? caught.message : '加载 OSV 配置失败'
    } finally {
      settingsLoaded.value = true
    }
  }

  async function refreshInstallStatus() {
    try {
      installStatus.value = await checkOsvInstalled()
    } catch (caught) {
      installStatus.value = {
        installed: false,
        message: caught instanceof Error ? caught.message : '检测 osv-scanner 失败',
      }
      error.value = caught instanceof Error ? caught.message : '检测 osv-scanner 失败'
    }
  }

  async function persistSettings() {
    const settings = await saveOsvSettings({
      projects: projects.value,
      autoScanSchedule: autoScanSchedule.value,
      commandHistory: commandHistory.value.slice(-COMMAND_HISTORY_LIMIT),
    })
    projects.value = settings.projects
    autoScanSchedule.value = settings.autoScanSchedule
    commandHistory.value = settings.commandHistory
  }

  async function addProject(path: string) {
    const trimmed = path.trim()
    if (!trimmed) return
    if (!projects.value.some((project) => project.path === trimmed)) {
      projects.value.push({
        name: projectNameFromPath(trimmed),
        path: trimmed,
      })
      activeProjectPath.value = trimmed
      invalidateCommandPreviews()
      latestResult.value = null
      await persistSettings()
      await diagnoseActiveProject(trimmed)
    }
  }

  async function removeProject(path: string) {
    projects.value = projects.value.filter((project) => project.path !== path)
    if (activeProjectPath.value === path) {
      activeProjectPath.value = projects.value[0]?.path ?? ''
      invalidateCommandPreviews()
      latestResult.value = null
      diagnostic.value = null
    }
    await persistSettings()
    await deleteLatestScanResult(path)
    if (activeProjectPath.value) await diagnoseActiveProject()
  }

  function selectProject(path: string) {
    const changed = activeProjectPath.value !== path
    activeProjectPath.value = path
    invalidateCommandPreviews()
    if (changed) {
      latestResult.value = null
      diagnostic.value = null
      void restoreLatestScanResult(path)
      void diagnoseActiveProject(path, { quiet: true })
    }
    notice.value = ''
    error.value = ''
  }

  async function diagnoseActiveProject(
    path = activeProjectPath.value,
    settings: { quiet?: boolean } = {},
  ) {
    error.value = ''
    if (!path) {
      diagnostic.value = null
      return
    }

    diagnosing.value = true
    if (!settings.quiet) {
      startOperation(
        'diagnose',
        '扫描诊断中',
        '正在识别项目包源、配置文件和会阻断扫描的参数。',
        { detail: path },
      )
    }
    try {
      diagnostic.value = await diagnoseOsvProject({
        projectPath: path,
        options: options.value,
      })
      if (!settings.quiet) {
        const sourceCount = diagnostic.value.packageSources.length
        const message = diagnostic.value.canScan
          ? `诊断完成，发现 ${sourceCount} 个包源。`
          : '诊断完成，但存在需要先处理的阻断项。'
        completeOperation(message, diagnostic.value.projectPath)
      }
    } catch (caught) {
      diagnostic.value = null
      error.value = caught instanceof Error ? caught.message : '扫描诊断失败'
      if (!settings.quiet) failOperation(error.value)
    } finally {
      diagnosing.value = false
    }
  }

  async function previewScan(path = activeProjectPath.value) {
    error.value = ''
    notice.value = ''
    if (!path) {
      error.value = '请先选择项目'
      return
    }

    try {
      startOperation(
        'preview-scan',
        '生成扫描命令',
        '正在校验当前配置，并生成执行前可确认的命令。',
        { detail: path },
      )
      await diagnoseActiveProject(path, { quiet: true })
      if (diagnostic.value && !diagnostic.value.canScan) {
        currentPreview.value = null
        const blocking = diagnostic.value.messages.find((message) => message.level === 'error')
        error.value = [blocking?.message, blocking?.suggestion].filter(Boolean).join(' ')
        failOperation(error.value || '诊断未通过，扫描命令未生成。')
        return
      }
      currentPreview.value = await previewOsvScanCommand({
        projectPath: path,
        options: options.value,
      })
      completeOperation('扫描命令已生成，执行前仍可复制和核对。', currentPreview.value.workingDir, {
        command: currentPreview.value.displayCommand,
      })
    } catch (caught) {
      currentPreview.value = null
      error.value = caught instanceof Error ? caught.message : '生成扫描命令失败'
      failOperation(error.value)
    }
  }

  async function runScan() {
    error.value = ''
    notice.value = ''
    if (!activeProjectPath.value || !currentPreview.value) {
      error.value = '请先生成并确认扫描命令'
      return
    }

    loading.value = true
    scanningPath.value = activeProjectPath.value
    startOperation('scan', '扫描执行中', 'osv-scanner 正在分析依赖包源和漏洞数据。', {
      detail: activeProjectPath.value,
      command: currentPreview.value.displayCommand,
    })
    try {
      const result = await scanOsvProject({
        projectPath: activeProjectPath.value,
        options: options.value,
        command: currentPreview.value,
      })
      latestResult.value = result
      appendHistory(result.command)
      updateProjectAfterScan(activeProjectPath.value, result.summary.healthScore)
      await saveOsvLatestScanResult(result)
      notice.value = result.summary.message
      completeOperation(result.summary.message, result.command.summary, {
        command: result.command.displayCommand,
      })
      await persistSettings()
    } catch (caught) {
      error.value = caught instanceof Error ? caught.message : '扫描失败'
      failOperation(error.value, currentPreview.value.displayCommand)
    } finally {
      loading.value = false
      scanningPath.value = ''
    }
  }

  async function previewExport(format: OsvReportFormat, outputPath: string) {
    error.value = ''
    notice.value = ''
    if (!activeProjectPath.value) {
      error.value = '请先选择项目'
      return
    }
    if (!hasCurrentScanResult.value) {
      error.value = '请先完成当前项目扫描后再导出报告'
      return
    }
    try {
      startOperation(
        'preview-export',
        '生成导出命令',
        `正在生成 ${format.toUpperCase()} 报告导出命令。`,
        { detail: outputPath },
      )
      currentExportPreview.value = await previewOsvReportExportCommand({
        projectPath: activeProjectPath.value,
        options: options.value,
        format,
        outputPath,
      })
      completeOperation('导出命令已生成，执行前仍可复制和核对。', outputPath, {
        command: currentExportPreview.value.displayCommand,
      })
    } catch (caught) {
      currentExportPreview.value = null
      error.value = caught instanceof Error ? caught.message : '生成导出命令失败'
      failOperation(error.value)
    }
  }

  async function suggestedReportPath(format: OsvReportFormat, path = activeProjectPath.value) {
    if (!path) return ''
    try {
      return await suggestOsvReportPath(path, format)
    } catch {
      return ''
    }
  }

  async function runExport(format: OsvReportFormat, outputPath: string) {
    error.value = ''
    notice.value = ''
    if (!activeProjectPath.value || !currentExportPreview.value) {
      error.value = '请先生成并确认导出命令'
      return
    }
    if (!hasCurrentScanResult.value) {
      currentExportPreview.value = null
      error.value = '请先完成当前项目扫描后再导出报告'
      return
    }

    exporting.value = true
    startOperation('export', '报告导出中', `正在写入 ${format.toUpperCase()} 报告。`, {
      detail: outputPath,
      command: currentExportPreview.value.displayCommand,
    })
    try {
      const result = await exportOsvReport({
        projectPath: activeProjectPath.value,
        options: options.value,
        format,
        outputPath,
        command: currentExportPreview.value,
      })
      appendHistory(result.command)
      notice.value = `已导出 ${result.format.toUpperCase()} 报告：${result.outputPath}`
      completeOperation(notice.value, result.outputPath, {
        command: result.command.displayCommand,
      })
      await persistSettings()
    } catch (caught) {
      error.value = caught instanceof Error ? caught.message : '导出失败'
      failOperation(error.value, currentExportPreview.value.displayCommand)
    } finally {
      exporting.value = false
    }
  }

  async function ignoreFinding(finding: OsvVulnerabilityFinding, reason: string) {
    error.value = ''
    notice.value = ''
    if (!activeProjectPath.value) {
      error.value = '请先选择项目'
      return
    }

    try {
      startOperation('ignore', '写入忽略规则', `正在记录 ${finding.id} 的忽略原因。`, {
        detail: finding.package.name,
      })
      const result = await ignoreOsvVulnerability({
        projectPath: activeProjectPath.value,
        vulnerabilityId: finding.id,
        reason,
      })
      notice.value = result.message
      completeOperation(result.message, result.configPath)
    } catch (caught) {
      error.value = caught instanceof Error ? caught.message : '写入忽略规则失败'
      failOperation(error.value)
    }
  }

  function appendHistory(record: OsvCommandExecutionRecord) {
    commandHistory.value = [...commandHistory.value, record].slice(-COMMAND_HISTORY_LIMIT)
  }

  async function restoreLatestScanResult(path: string) {
    if (!path) {
      latestResult.value = null
      return
    }

    try {
      const result = await getOsvLatestScanResult(path)
      if (activeProjectPath.value === path) {
        latestResult.value = result
      }
    } catch (caught) {
      if (activeProjectPath.value === path) {
        latestResult.value = null
        error.value = caught instanceof Error ? caught.message : '恢复最近扫描结果失败'
      }
    }
  }

  async function deleteLatestScanResult(path: string) {
    try {
      await deleteOsvLatestScanResult(path)
    } catch (caught) {
      error.value = caught instanceof Error ? caught.message : '删除最近扫描结果失败'
    }
  }

  function invalidateScanPreview() {
    currentPreview.value = null
  }

  function invalidateExportPreview() {
    currentExportPreview.value = null
  }

  function invalidateCommandPreviews() {
    invalidateScanPreview()
    invalidateExportPreview()
    clearSettledOperation()
  }

  function clearDiagnostic() {
    diagnostic.value = null
  }

  function startOperation(
    kind: Exclude<OsvOperationKind, 'idle'>,
    title: string,
    message: string,
    metadata: Pick<OsvOperationState, 'detail' | 'command'> = {},
  ) {
    operation.value = {
      kind,
      status: 'running',
      title,
      message,
      detail: metadata.detail,
      command: metadata.command,
      startedAt: Date.now(),
    }
  }

  function completeOperation(
    message: string,
    detail?: string,
    metadata: Pick<OsvOperationState, 'command'> = {},
  ) {
    operation.value = {
      ...operation.value,
      status: 'succeeded',
      message,
      detail: detail ?? operation.value.detail,
      command: metadata.command ?? operation.value.command,
      finishedAt: Date.now(),
    }
  }

  function failOperation(message: string, command?: string) {
    operation.value = {
      ...operation.value,
      status: 'failed',
      message,
      command: command ?? operation.value.command,
      finishedAt: Date.now(),
    }
  }

  function dismissOperation() {
    if (operation.value.status === 'running') return
    operation.value = idleOperation()
  }

  function clearSettledOperation() {
    if (operation.value.status === 'running') return
    operation.value = idleOperation()
  }

  function updateProjectAfterScan(path: string, healthScore: number) {
    const project = projects.value.find((item) => item.path === path)
    if (!project) return
    project.lastScanned = Date.now().toString()
    project.healthScore = healthScore
  }

  return {
    projects,
    autoScanSchedule,
    commandHistory,
    installStatus,
    settingsLoaded,
    loading,
    scanningPath,
    exporting,
    error,
    notice,
    activeProjectPath,
    activeProject,
    hasCurrentScanResult,
    currentPreview,
    currentExportPreview,
    latestResult,
    diagnostic,
    diagnosing,
    operation,
    vulnerabilities,
    options,
    globalHealthScore,
    load,
    refreshInstallStatus,
    addProject,
    removeProject,
    selectProject,
    diagnoseActiveProject,
    previewScan,
    runScan,
    previewExport,
    runExport,
    suggestedReportPath,
    ignoreFinding,
    invalidateScanPreview,
    invalidateExportPreview,
    invalidateCommandPreviews,
    clearDiagnostic,
    dismissOperation,
  }
})

function projectNameFromPath(path: string): string {
  return path.split(/[\\/]/).filter(Boolean).pop() || path
}
