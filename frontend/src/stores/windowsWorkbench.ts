import { computed, ref, watch } from 'vue'
import { defineStore } from 'pinia'
import {
  checkSub2apiHealth,
  clearOperationLogs,
  defaultWorkbenchConfig,
  detectClashParty,
  detectDocker,
  getClashPartyManagerState,
  getClashPartyStatus,
  getDockerStatus,
  getWorkbenchConfig,
  listOperationLogs,
  restartDocker,
  runSub2apiTask,
  saveWorkbenchConfig,
  selectWorkbenchDirectory,
  selectWorkbenchFile,
  startDocker,
  startClashParty,
  stopDocker,
  stopClashParty,
  shutdownWindows,
  switchClashPartyNode,
  switchClashPartySubscription,
  type ClashPartyManagerState,
  type ClashPartyProxyGroup,
  type ClashPartyStatus,
  type DockerStatus,
  type HealthStatus,
  type OperationLog,
  type OperationLogPage,
  type Sub2apiTask,
  type TaskRun,
  type WorkbenchConfig,
} from '../api/workbench'

type Sub2apiScriptField = 'sub2apiStartScript' | 'sub2apiStopScript' | 'sub2apiUpgradeScript'
type ConfigSection = 'docker' | 'clashParty' | 'sub2api'
type ConfirmTone = 'default' | 'danger'

interface PendingConfirm {
  title: string
  message: string
  warning?: string
  confirmText: string
  loadingKey: string
  tone: ConfirmTone
  action: () => Promise<void>
}

export const useWindowsWorkbenchStore = defineStore('windows-workbench', () => {
  const config = ref<WorkbenchConfig>(defaultWorkbenchConfig())
  const dockerStatus = ref<DockerStatus | null>(null)
  const clashPartyStatus = ref<ClashPartyStatus | null>(null)
  const clashPartyManager = ref<ClashPartyManagerState | null>(null)
  const selectedClashPartySubscriptionId = ref('')
  const selectedClashPartyGroupName = ref('')
  const selectedClashPartyNodeName = ref('')
  const sub2apiHealth = ref<HealthStatus | null>(null)
  const operationLogs = ref<OperationLog[]>([])
  const operationLogPage = ref<OperationLogPage>({
    logs: [],
    page: 1,
    pageSize: 50,
    total: 0,
    totalPages: 1,
  })
  const operationLogQuery = ref('')
  const activeConfig = ref<ConfigSection | ''>('')
  const loading = ref('')
  const autoChecking = ref(false)
  const initialized = ref(false)
  const desktopAvailable = ref(true)
  const pendingConfirm = ref<PendingConfirm | null>(null)
  const toast = ref<{ type: 'success' | 'error'; title: string; detail: string } | null>(null)
  let toastTimer: number | undefined

  const dockerConfigured = computed(
    () => Boolean(config.value.dockerDesktopPath.trim()) || Boolean(config.value.dockerCliPath.trim()),
  )
  const clashPartyConfigured = computed(() => Boolean(config.value.clashPartyPath.trim()))
  const clashPartyDataConfigured = computed(() => Boolean(config.value.clashPartyDataDir.trim()))
  const clashPartyApiConfigured = computed(() => Boolean(config.value.clashPartyApiUrl.trim()))
  const sub2apiConfigured = computed(
    () =>
      Boolean(config.value.sub2apiStartScript.trim()) ||
      Boolean(config.value.sub2apiStopScript.trim()) ||
      Boolean(config.value.sub2apiUpgradeScript.trim()),
  )
  const selectedClashPartyGroup = computed<ClashPartyProxyGroup | null>(() => {
    const groups = clashPartyManager.value?.groups ?? []
    return groups.find((group) => group.name === selectedClashPartyGroupName.value) ?? groups[0] ?? null
  })

  watch(selectedClashPartyGroupName, () => {
    syncSelectedClashPartyNode()
  })

  async function load() {
    await refreshDashboard()
  }

  async function ensureLoaded() {
    if (initialized.value) return
    await withLoading('load', async () => {
      await loadConfig()
    })
  }

  async function refreshDashboard() {
    await withLoading('load', async () => {
      if (!initialized.value) {
        await loadConfig()
      }
      await autoCheckConfiguredServices()
    })
  }

  async function loadConfig() {
    config.value = await getWorkbenchConfig()
    initialized.value = true
  }

  async function saveConfig() {
    requestConfirm({
      title: '确认保存配置？',
      message: '配置会写入本机 SQLite 数据库，并立即用于后续检测和操作。',
      confirmText: '保存配置',
      loadingKey: 'save-config',
      action: async () => {
        await saveConfigNow()
      },
    })
  }

  async function saveConfigNow() {
    await withLoading('save-config', async () => {
      config.value = await saveWorkbenchConfig(config.value)
      showToast('success', '配置已保存', 'Windows 工作台配置已更新')
      await autoCheckConfiguredServices()
    })
  }

  async function autoDetectDocker() {
    await withLoading('detect-docker', async () => {
      const detection = await detectDocker()
      if (detection.dockerDesktopPath) config.value.dockerDesktopPath = detection.dockerDesktopPath
      if (detection.dockerCliPath) config.value.dockerCliPath = detection.dockerCliPath
      const found = [detection.dockerDesktopPath, detection.dockerCliPath].filter(Boolean).length
      showToast(found ? 'success' : 'error', found ? '已找到 Docker' : '未找到 Docker', found ? '请确认路径后保存' : '可以手动填写 Docker 路径')
    })
  }

  async function selectDockerDesktopPath() {
    await withLoading('select-docker-desktop', async () => {
      const path = await selectWorkbenchFile('executable')
      if (path) config.value.dockerDesktopPath = path
    })
  }

  async function selectDockerCliPath() {
    await withLoading('select-docker-cli', async () => {
      const path = await selectWorkbenchFile('executable')
      if (path) config.value.dockerCliPath = path
    })
  }

  async function selectClashPartyPath() {
    await withLoading('select-clash-party', async () => {
      const path = await selectWorkbenchFile('executable')
      if (path) config.value.clashPartyPath = path
    })
  }

  async function autoDetectClashParty() {
    await withLoading('detect-clash-party', async () => {
      const detection = await detectClashParty()
      if (detection.clashPartyPath) config.value.clashPartyPath = detection.clashPartyPath
      if (detection.clashPartyDataDir) config.value.clashPartyDataDir = detection.clashPartyDataDir
      if (detection.clashPartyApiUrl) config.value.clashPartyApiUrl = detection.clashPartyApiUrl
      showToast(
        detection.clashPartyPath || detection.clashPartyDataDir ? 'success' : 'error',
        detection.clashPartyPath || detection.clashPartyDataDir ? '已找到 Clash Party' : '未找到 Clash Party',
        detection.clashPartyPath || detection.clashPartyDataDir ? '请确认路径后保存' : '可以手动填写 Clash Party 路径',
      )
    })
  }

  async function selectClashPartyDataDir() {
    await withLoading('select-clash-party-data-dir', async () => {
      const path = await selectWorkbenchDirectory()
      if (path) config.value.clashPartyDataDir = path
    })
  }

  async function selectSub2apiScript(field: Sub2apiScriptField) {
    await withLoading(`select-${field}`, async () => {
      const path = await selectWorkbenchFile('script')
      if (path) {
        config.value[field] = path
        if (!config.value.sub2apiWorkingDir.trim()) {
          config.value.sub2apiWorkingDir = parentDirectory(path)
        }
      }
    })
  }

  async function selectSub2apiWorkingDir() {
    await withLoading('select-sub2api-working-dir', async () => {
      const path = await selectWorkbenchDirectory()
      if (path) config.value.sub2apiWorkingDir = path
    })
  }

  async function refreshDockerStatus() {
    await withLoading('docker-status', async () => {
      dockerStatus.value = await getDockerStatus()
      showToast(dockerStatus.value.engineRunning ? 'success' : 'error', 'Docker 检测完成', dockerStatus.value.message)
    })
  }

  async function launchDocker() {
    requestConfirm({
      title: '确认启动 Docker？',
      message: '将启动本机 Docker Desktop，并可能触发 Docker Engine 初始化。',
      confirmText: '启动 Docker',
      loadingKey: 'docker-start',
      action: async () => {
        await launchDockerNow()
      },
    })
  }

  async function launchDockerNow() {
    await withLoading('docker-start', async () => {
      const run = await startDocker()
      showToast(isRunOk(run) ? 'success' : 'error', 'Docker 启动任务已执行', runSummary(run))
    })
  }

  async function shutdownDocker() {
    requestConfirm({
      title: '确认停止 Docker？',
      message: '将停止 Docker Desktop，依赖 Docker 的容器和服务可能中断。',
      warning: '请确认没有正在运行的重要容器任务。',
      confirmText: '停止 Docker',
      loadingKey: 'docker-stop',
      tone: 'danger',
      action: async () => {
        await shutdownDockerNow()
      },
    })
  }

  async function shutdownDockerNow() {
    await withLoading('docker-stop', async () => {
      const run = await stopDocker()
      showToast(isRunOk(run) ? 'success' : 'error', 'Docker 停止任务已执行', runSummary(run))
      window.setTimeout(() => {
        void refreshDockerStatus()
      }, 1200)
    })
  }

  async function relaunchDocker() {
    requestConfirm({
      title: '确认重启 Docker？',
      message: '将先停止再启动 Docker Desktop，运行中的容器连接可能短暂中断。',
      warning: '请确认可以接受 Docker 服务重启。',
      confirmText: '重启 Docker',
      loadingKey: 'docker-restart',
      tone: 'danger',
      action: async () => {
        await relaunchDockerNow()
      },
    })
  }

  async function relaunchDockerNow() {
    await withLoading('docker-restart', async () => {
      const run = await restartDocker()
      showToast(isRunOk(run) ? 'success' : 'error', 'Docker 重启任务已执行', runSummary(run))
      window.setTimeout(() => {
        void refreshDockerStatus()
      }, 2800)
    })
  }

  async function refreshClashPartyStatus() {
    await withLoading('clash-party-status', async () => {
      clashPartyStatus.value = await getClashPartyStatus()
      showToast(
        clashPartyStatus.value.running ? 'success' : 'error',
        'Clash Party 检测完成',
        clashPartyStatus.value.message,
      )
    })
  }

  async function launchClashParty() {
    requestConfirm({
      title: '确认启动 Clash Party？',
      message: '将启动配置中的 Clash Party 客户端。',
      confirmText: '启动 Clash Party',
      loadingKey: 'clash-party-start',
      action: async () => {
        await launchClashPartyNow()
      },
    })
  }

  async function launchClashPartyNow() {
    await withLoading('clash-party-start', async () => {
      const run = await startClashParty()
      showToast(isRunOk(run) ? 'success' : 'error', 'Clash Party 启动任务已执行', runSummary(run))
      window.setTimeout(() => {
        void refreshClashPartyStatus()
      }, 1200)
    })
  }

  async function exitClashParty() {
    requestConfirm({
      title: '确认退出 Clash Party？',
      message: '将结束 Clash Party 及相关内核进程，当前代理连接可能中断。',
      warning: '请确认当前网络代理可以临时中断。',
      confirmText: '退出 Clash Party',
      loadingKey: 'clash-party-stop',
      tone: 'danger',
      action: async () => {
        await exitClashPartyNow()
      },
    })
  }

  async function exitClashPartyNow() {
    await withLoading('clash-party-stop', async () => {
      const run = await stopClashParty()
      showToast(isRunOk(run) ? 'success' : 'error', 'Clash Party 退出任务已执行', runSummary(run))
      window.setTimeout(() => {
        void refreshClashPartyStatus()
      }, 800)
    })
  }

  async function refreshClashPartyManager() {
    await withLoading('clash-party-manager', async () => {
      const state = await getClashPartyManagerState()
      clashPartyManager.value = state
      selectedClashPartySubscriptionId.value =
        state.activeSubscriptionId || state.subscriptions[0]?.id || selectedClashPartySubscriptionId.value
      selectedClashPartyGroupName.value =
        chooseProxyGroupName(state.groups, selectedClashPartyGroupName.value) ?? ''
      syncSelectedClashPartyNode()
      showToast(state.apiAvailable ? 'success' : 'error', 'Clash Party 管理刷新完成', state.message)
    })
  }

  async function switchSubscription() {
    const subscriptionId = selectedClashPartySubscriptionId.value
    if (!subscriptionId) {
      showToast('error', '请选择订阅', '订阅列表为空或尚未刷新')
      return
    }
    const subscriptionName =
      clashPartyManager.value?.subscriptions.find((item) => item.id === subscriptionId)?.name || subscriptionId

    requestConfirm({
      title: '确认切换订阅？',
      message: `将把 Clash Party 当前订阅切换为“${subscriptionName}”。`,
      warning: '切换后代理组和当前节点可能变化。',
      confirmText: '切换订阅',
      loadingKey: 'clash-party-switch-subscription',
      action: async () => {
        await switchSubscriptionNow(subscriptionId)
      },
    })
  }

  async function switchSubscriptionNow(subscriptionId: string) {
    await withLoading('clash-party-switch-subscription', async () => {
      const result = await switchClashPartySubscription(subscriptionId)
      showToast(result.ok ? 'success' : 'error', '订阅切换请求已发送', result.message)
      window.setTimeout(() => {
        void refreshClashPartyManager()
      }, 1000)
    })
  }

  async function switchNode(nodeName?: string) {
    const groupName = selectedClashPartyGroupName.value
    const nextNodeName = nodeName || selectedClashPartyNodeName.value
    if (!groupName || !nextNodeName) {
      showToast('error', '请选择代理组和节点', '需要先刷新运行时代理组')
      return
    }

    requestConfirm({
      title: '确认切换节点？',
      message: `将把代理组“${groupName}”切换到节点“${nextNodeName}”。`,
      confirmText: '切换节点',
      loadingKey: 'clash-party-switch-node',
      action: async () => {
        await switchNodeNow(groupName, nextNodeName)
      },
    })
  }

  async function switchNodeNow(groupName: string, nextNodeName: string) {
    await withLoading('clash-party-switch-node', async () => {
      const result = await switchClashPartyNode(groupName, nextNodeName)
      selectedClashPartyNodeName.value = nextNodeName
      showToast(result.ok ? 'success' : 'error', '节点切换请求已发送', result.message)
      window.setTimeout(() => {
        void refreshClashPartyManager()
      }, 800)
    })
  }

  async function requestWindowsShutdown() {
    requestConfirm({
      title: '确认关闭 Windows？',
      message: '确认后会向系统发送关机命令，当前电脑将在 10 秒后关机。',
      warning: '请先保存正在编辑的文件，并确认没有正在运行的重要任务。',
      confirmText: '确认关机',
      loadingKey: 'system-shutdown',
      tone: 'danger',
      action: async () => {
        await confirmWindowsShutdown()
      },
    })
  }

  function closeConfirm() {
    if (loading.value && loading.value === pendingConfirm.value?.loadingKey) return
    pendingConfirm.value = null
  }

  async function confirmWindowsShutdown() {
    await withLoading('system-shutdown', async () => {
      const run = await shutdownWindows()
      showToast(isRunOk(run) ? 'success' : 'error', '关机任务已执行', runSummary(run))
    })
  }

  async function runSub2api(task: Sub2apiTask) {
    requestConfirm({
      title: `确认${sub2apiActionLabel(task)} sub2api？`,
      message: `将执行配置中的 sub2api ${sub2apiActionLabel(task)}脚本。`,
      warning: task === 'upgrade' ? '升级可能改变当前 sub2api 版本，请确认可以继续。' : undefined,
      confirmText: `${sub2apiActionLabel(task)} sub2api`,
      loadingKey: `sub2api-${task}`,
      tone: task === 'stop' ? 'danger' : 'default',
      action: async () => {
        await runSub2apiNow(task)
      },
    })
  }

  async function runSub2apiNow(task: Sub2apiTask) {
    await withLoading(`sub2api-${task}`, async () => {
      const run = await runSub2apiTask(task)
      showToast(isRunOk(run) ? 'success' : 'error', sub2apiTaskLabel(task), runSummary(run))
    })
  }

  async function refreshSub2apiHealth() {
    await withLoading('sub2api-health', async () => {
      sub2apiHealth.value = await checkSub2apiHealth()
      showToast(sub2apiHealth.value.ok ? 'success' : 'error', 'sub2api 健康检查', sub2apiHealth.value.message)
    })
  }

  async function refreshOperationLogs() {
    await withLoading('operation-logs', fetchOperationLogs)
  }

  async function fetchOperationLogs(page = operationLogPage.value.page, query = operationLogQuery.value) {
    const result = await listOperationLogs(page, query)
    operationLogPage.value = result
    operationLogs.value = result.logs
    operationLogQuery.value = query
  }

  async function searchOperationLogs(query: string) {
    operationLogQuery.value = query
    await withLoading('operation-logs', async () => {
      await fetchOperationLogs(1, query)
    })
  }

  async function goToOperationLogPage(page: number) {
    const nextPage = Math.min(Math.max(page, 1), operationLogPage.value.totalPages || 1)
    if (nextPage === operationLogPage.value.page && operationLogs.value.length) return
    await withLoading('operation-logs', async () => {
      await fetchOperationLogs(nextPage, operationLogQuery.value)
    })
  }

  async function clearLogs() {
    requestConfirm({
      title: '确认清理操作日志？',
      message: '将删除当前保存的操作日志记录，清理完成后只保留本次清理记录。',
      warning: '清理后的旧日志无法在 RustTool 内恢复。',
      confirmText: '清理日志',
      loadingKey: 'operation-logs-clear',
      tone: 'danger',
      action: async () => {
        await clearLogsNow()
      },
    })
  }

  async function clearLogsNow() {
    await withLoading('operation-logs-clear', async () => {
      const deleted = await clearOperationLogs()
      await fetchOperationLogs(1, operationLogQuery.value)
      showToast('success', '操作日志已清理', `已删除 ${deleted} 条记录`)
    })
  }

  async function autoCheckConfiguredServices() {
    autoChecking.value = true
    try {
      const checks: Promise<void>[] = []

      if (dockerConfigured.value && config.value.dockerCliPath.trim()) {
        checks.push(
          getDockerStatus()
            .then((status) => {
              dockerStatus.value = status
            })
            .catch((caught) => {
              dockerStatus.value = {
                desktopConfigured: Boolean(config.value.dockerDesktopPath.trim()),
                cliConfigured: Boolean(config.value.dockerCliPath.trim()),
                desktopRunning: false,
                cliAvailable: false,
                engineRunning: false,
                version: '',
                message: caught instanceof Error ? caught.message : String(caught),
              }
            }),
        )
      }

      if (clashPartyConfigured.value) {
        checks.push(
          getClashPartyStatus()
            .then((status) => {
              clashPartyStatus.value = status
            })
            .catch((caught) => {
              clashPartyStatus.value = {
                configured: Boolean(config.value.clashPartyPath.trim()),
                running: false,
                path: config.value.clashPartyPath,
                message: caught instanceof Error ? caught.message : String(caught),
              }
            }),
        )
      }

      if (clashPartyDataConfigured.value || clashPartyApiConfigured.value) {
        checks.push(
          getClashPartyManagerState()
            .then((state) => {
              clashPartyManager.value = state
              selectedClashPartySubscriptionId.value = state.activeSubscriptionId || state.subscriptions[0]?.id || ''
              selectedClashPartyGroupName.value = chooseProxyGroupName(state.groups, selectedClashPartyGroupName.value) ?? ''
              syncSelectedClashPartyNode()
            })
            .catch(() => {
              clashPartyManager.value = null
            }),
        )
      }

      if (sub2apiConfigured.value && config.value.sub2apiHealthUrl.trim()) {
        checks.push(
          checkSub2apiHealth()
            .then((health) => {
              sub2apiHealth.value = health
            })
            .catch((caught) => {
              sub2apiHealth.value = {
                ok: false,
                message: caught instanceof Error ? caught.message : String(caught),
              }
            }),
        )
      }

      await Promise.all(checks)
    } finally {
      autoChecking.value = false
    }
  }

  function openConfig(section: ConfigSection) {
    activeConfig.value = section
  }

  function closeConfig() {
    activeConfig.value = ''
  }

  function requestConfirm(options: Omit<PendingConfirm, 'tone'> & { tone?: ConfirmTone }) {
    pendingConfirm.value = {
      tone: 'default',
      ...options,
    }
  }

  async function confirmPendingAction() {
    const confirm = pendingConfirm.value
    if (!confirm) return
    await confirm.action()
    if (pendingConfirm.value === confirm) {
      pendingConfirm.value = null
    }
  }

  async function withLoading(key: string, action: () => Promise<void>) {
    loading.value = key
    try {
      await action()
    } catch (caught) {
      const message = caught instanceof Error ? caught.message : String(caught)
      if (message.includes('Tauri 桌面版')) {
        desktopAvailable.value = false
      }
      showToast('error', '操作失败', message)
    } finally {
      loading.value = ''
      if (
        desktopAvailable.value &&
        !key.startsWith('operation-logs') &&
        operationLogPage.value.page === 1 &&
        !operationLogQuery.value.trim()
      ) {
        void fetchOperationLogs().catch((caught) => {
          const message = caught instanceof Error ? caught.message : String(caught)
          if (message.includes('Tauri 桌面版')) {
            desktopAvailable.value = false
          }
        })
      }
    }
  }

  function syncSelectedClashPartyNode() {
    const group = clashPartyManager.value?.groups.find(
      (item) => item.name === selectedClashPartyGroupName.value,
    )
    if (!group) {
      selectedClashPartyNodeName.value = ''
      return
    }
    if (group.nodes.some((node) => node.name === selectedClashPartyNodeName.value)) {
      return
    }
    selectedClashPartyNodeName.value = group.nodes.some((node) => node.name === group.selected)
      ? group.selected
      : group.nodes[0]?.name || ''
  }

  function showToast(type: 'success' | 'error', title: string, detail: string) {
    toast.value = { type, title, detail }
    window.clearTimeout(toastTimer)
    toastTimer = window.setTimeout(() => {
      toast.value = null
    }, 4200)
  }

  function hideToast() {
    window.clearTimeout(toastTimer)
    toast.value = null
  }

  return {
    config,
    dockerStatus,
    clashPartyStatus,
    clashPartyManager,
    selectedClashPartySubscriptionId,
    selectedClashPartyGroupName,
    selectedClashPartyNodeName,
    sub2apiHealth,
    operationLogs,
    operationLogPage,
    operationLogQuery,
    activeConfig,
    loading,
    autoChecking,
    desktopAvailable,
    pendingConfirm,
    toast,
    dockerConfigured,
    clashPartyConfigured,
    clashPartyDataConfigured,
    clashPartyApiConfigured,
    sub2apiConfigured,
    selectedClashPartyGroup,
    load,
    ensureLoaded,
    refreshDashboard,
    saveConfig,
    autoDetectDocker,
    selectDockerDesktopPath,
    selectDockerCliPath,
    selectClashPartyPath,
    autoDetectClashParty,
    selectClashPartyDataDir,
    selectSub2apiScript,
    selectSub2apiWorkingDir,
    refreshDockerStatus,
    launchDocker,
    shutdownDocker,
    relaunchDocker,
    refreshClashPartyStatus,
    launchClashParty,
    exitClashParty,
    refreshClashPartyManager,
    switchSubscription,
    switchNode,
    requestWindowsShutdown,
    closeConfirm,
    confirmPendingAction,
    runSub2api,
    refreshSub2apiHealth,
    refreshOperationLogs,
    searchOperationLogs,
    goToOperationLogPage,
    clearLogs,
    openConfig,
    closeConfig,
    hideToast,
    initialized,
  }
})

function sub2apiTaskLabel(task: Sub2apiTask) {
  const labels: Record<Sub2apiTask, string> = {
    start: 'sub2api 启动任务已执行',
    stop: 'sub2api 停止任务已执行',
    upgrade: 'sub2api 升级任务已执行',
  }
  return labels[task]
}

function sub2apiActionLabel(task: Sub2apiTask) {
  const labels: Record<Sub2apiTask, string> = {
    start: '启动',
    stop: '停止',
    upgrade: '升级',
  }
  return labels[task]
}

function runSummary(run: TaskRun) {
  return run.stderr || run.stdout || `退出码: ${run.exitCode ?? '无'}`
}

function isRunOk(run: TaskRun) {
  return run.status === 'success' || run.status === 'started'
}

function chooseProxyGroupName(groups: ClashPartyProxyGroup[], current: string) {
  if (!groups.length) return null
  if (groups.some((group) => group.name === current)) return current
  return (
    groups.find((group) => group.name.toUpperCase() === 'PROXY')?.name ??
    groups.find((group) => group.name.toUpperCase() === 'GLOBAL')?.name ??
    groups[0].name
  )
}

function parentDirectory(path: string) {
  const normalized = path.replace(/\//g, '\\')
  const separatorIndex = normalized.lastIndexOf('\\')
  if (separatorIndex <= 0) return ''
  return normalized.slice(0, separatorIndex)
}
