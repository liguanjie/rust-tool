import { computed, ref } from 'vue'
import { defineStore } from 'pinia'
import {
  checkOsvInstalled,
  defaultOsvScanOptions,
  exportOsvReport,
  getOsvSettings,
  ignoreOsvVulnerability,
  previewOsvReportExportCommand,
  previewOsvScanCommand,
  saveOsvSettings,
  scanOsvProject,
  suggestOsvReportPath,
  type OsvCommandExecutionRecord,
  type OsvCommandPreview,
  type OsvInstallStatus,
  type OsvProjectSettings,
  type OsvReportFormat,
  type OsvScanOptions,
  type OsvScanResult,
  type OsvVulnerabilityFinding,
} from '../api/osvScanner'

const COMMAND_HISTORY_LIMIT = 50

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
  const options = ref<OsvScanOptions>(defaultOsvScanOptions())

  const activeProject = computed(() =>
    projects.value.find((project) => project.path === activeProjectPath.value) ?? null,
  )
  const hasCurrentScanResult = computed(() => Boolean(activeProjectPath.value && latestResult.value))
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
    }
  }

  async function removeProject(path: string) {
    projects.value = projects.value.filter((project) => project.path !== path)
    if (activeProjectPath.value === path) {
      activeProjectPath.value = projects.value[0]?.path ?? ''
      invalidateCommandPreviews()
      latestResult.value = null
    }
    await persistSettings()
  }

  function selectProject(path: string) {
    const changed = activeProjectPath.value !== path
    activeProjectPath.value = path
    invalidateCommandPreviews()
    if (changed) latestResult.value = null
    notice.value = ''
    error.value = ''
  }

  async function previewScan(path = activeProjectPath.value) {
    error.value = ''
    notice.value = ''
    if (!path) {
      error.value = '请先选择项目'
      return
    }

    try {
      currentPreview.value = await previewOsvScanCommand({
        projectPath: path,
        options: options.value,
      })
    } catch (caught) {
      currentPreview.value = null
      error.value = caught instanceof Error ? caught.message : '生成扫描命令失败'
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
    try {
      const result = await scanOsvProject({
        projectPath: activeProjectPath.value,
        options: options.value,
        command: currentPreview.value,
      })
      latestResult.value = result
      appendHistory(result.command)
      updateProjectAfterScan(activeProjectPath.value, result.summary.healthScore)
      notice.value = result.summary.message
      await persistSettings()
    } catch (caught) {
      error.value = caught instanceof Error ? caught.message : '扫描失败'
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
      currentExportPreview.value = await previewOsvReportExportCommand({
        projectPath: activeProjectPath.value,
        options: options.value,
        format,
        outputPath,
      })
    } catch (caught) {
      currentExportPreview.value = null
      error.value = caught instanceof Error ? caught.message : '生成导出命令失败'
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
      await persistSettings()
    } catch (caught) {
      error.value = caught instanceof Error ? caught.message : '导出失败'
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
      const result = await ignoreOsvVulnerability({
        projectPath: activeProjectPath.value,
        vulnerabilityId: finding.id,
        reason,
      })
      notice.value = result.message
    } catch (caught) {
      error.value = caught instanceof Error ? caught.message : '写入忽略规则失败'
    }
  }

  function appendHistory(record: OsvCommandExecutionRecord) {
    commandHistory.value = [...commandHistory.value, record].slice(-COMMAND_HISTORY_LIMIT)
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
    vulnerabilities,
    options,
    globalHealthScore,
    load,
    refreshInstallStatus,
    addProject,
    removeProject,
    selectProject,
    previewScan,
    runScan,
    previewExport,
    runExport,
    suggestedReportPath,
    ignoreFinding,
    invalidateScanPreview,
    invalidateExportPreview,
    invalidateCommandPreviews,
  }
})

function projectNameFromPath(path: string): string {
  return path.split(/[\\/]/).filter(Boolean).pop() || path
}
