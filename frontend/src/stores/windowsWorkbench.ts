import { computed, ref } from 'vue'
import { defineStore } from 'pinia'
import {
  checkSub2apiHealth,
  defaultWorkbenchConfig,
  detectClashParty,
  detectDocker,
  getClashPartyManagerState,
  getClashPartyStatus,
  getDockerStatus,
  getWorkbenchConfig,
  listOperationLogs,
  listTaskRuns,
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
  type Sub2apiTask,
  type TaskRun,
  type WorkbenchConfig,
} from '../api/workbench'

type Sub2apiScriptField = 'sub2apiStartScript' | 'sub2apiStopScript' | 'sub2apiUpgradeScript'
type ConfigSection = 'docker' | 'clashParty' | 'sub2api'

export const useWindowsWorkbenchStore = defineStore('windows-workbench', () => {
  const config = ref<WorkbenchConfig>(defaultWorkbenchConfig())
  const dockerStatus = ref<DockerStatus | null>(null)
  const clashPartyStatus = ref<ClashPartyStatus | null>(null)
  const clashPartyManager = ref<ClashPartyManagerState | null>(null)
  const selectedClashPartySubscriptionId = ref('')
  const selectedClashPartyGroupName = ref('')
  const selectedClashPartyNodeName = ref('')
  const sub2apiHealth = ref<HealthStatus | null>(null)
  const taskRuns = ref<TaskRun[]>([])
  const operationLogs = ref<OperationLog[]>([])
  const activeConfig = ref<ConfigSection | ''>('')
  const loading = ref('')
  const autoChecking = ref(false)
  const desktopAvailable = ref(true)
  const shutdownConfirmOpen = ref(false)
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
  const latestRun = computed(() => taskRuns.value[0] ?? null)
  const selectedClashPartyGroup = computed<ClashPartyProxyGroup | null>(() => {
    const groups = clashPartyManager.value?.groups ?? []
    return groups.find((group) => group.name === selectedClashPartyGroupName.value) ?? groups[0] ?? null
  })

  async function load() {
    await withLoading('load', async () => {
      config.value = await getWorkbenchConfig()
      taskRuns.value = await listTaskRuns(12)
      await autoCheckConfiguredServices()
      await refreshOperationLogs()
    })
  }

  async function saveConfig() {
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
      if (path) config.value[field] = path
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
    await withLoading('docker-start', async () => {
      const run = await startDocker()
      await refreshRuns()
      showToast(isRunOk(run) ? 'success' : 'error', 'Docker 启动任务已执行', runSummary(run))
    })
  }

  async function shutdownDocker() {
    await withLoading('docker-stop', async () => {
      const run = await stopDocker()
      await refreshRuns()
      showToast(isRunOk(run) ? 'success' : 'error', 'Docker 停止任务已执行', runSummary(run))
      window.setTimeout(() => {
        void refreshDockerStatus()
      }, 1200)
    })
  }

  async function relaunchDocker() {
    await withLoading('docker-restart', async () => {
      const run = await restartDocker()
      await refreshRuns()
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
    await withLoading('clash-party-start', async () => {
      const run = await startClashParty()
      await refreshRuns()
      showToast(isRunOk(run) ? 'success' : 'error', 'Clash Party 启动任务已执行', runSummary(run))
      window.setTimeout(() => {
        void refreshClashPartyStatus()
      }, 1200)
    })
  }

  async function exitClashParty() {
    await withLoading('clash-party-stop', async () => {
      const run = await stopClashParty()
      await refreshRuns()
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
      selectedClashPartyNodeName.value =
        state.groups
          .find((group) => group.name === selectedClashPartyGroupName.value)
          ?.selected || ''
      showToast(state.apiAvailable ? 'success' : 'error', 'Clash Party 管理刷新完成', state.message)
    })
  }

  async function switchSubscription() {
    const subscriptionId = selectedClashPartySubscriptionId.value
    if (!subscriptionId) {
      showToast('error', '请选择订阅', '订阅列表为空或尚未刷新')
      return
    }

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
    shutdownConfirmOpen.value = true
  }

  function closeShutdownConfirm() {
    if (loading.value === 'system-shutdown') return
    shutdownConfirmOpen.value = false
  }

  async function confirmWindowsShutdown() {
    await withLoading('system-shutdown', async () => {
      const run = await shutdownWindows()
      await refreshRuns()
      shutdownConfirmOpen.value = false
      showToast(isRunOk(run) ? 'success' : 'error', '关机任务已执行', runSummary(run))
    })
  }

  async function runSub2api(task: Sub2apiTask) {
    await withLoading(`sub2api-${task}`, async () => {
      const run = await runSub2apiTask(task)
      await refreshRuns()
      showToast(isRunOk(run) ? 'success' : 'error', sub2apiTaskLabel(task), runSummary(run))
    })
  }

  async function refreshSub2apiHealth() {
    await withLoading('sub2api-health', async () => {
      sub2apiHealth.value = await checkSub2apiHealth()
      showToast(sub2apiHealth.value.ok ? 'success' : 'error', 'sub2api 健康检查', sub2apiHealth.value.message)
    })
  }

  async function refreshRuns() {
    taskRuns.value = await listTaskRuns(12)
  }

  async function refreshOperationLogs() {
    try {
      operationLogs.value = await listOperationLogs(80)
    } catch (caught) {
      const message = caught instanceof Error ? caught.message : String(caught)
      if (message.includes('Tauri 桌面版')) {
        desktopAvailable.value = false
      }
    }
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
              selectedClashPartyNodeName.value =
                state.groups.find((group) => group.name === selectedClashPartyGroupName.value)?.selected || ''
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
      if (desktopAvailable.value && key !== 'operation-logs') {
        void refreshOperationLogs()
      }
    }
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
    taskRuns,
    operationLogs,
    latestRun,
    activeConfig,
    loading,
    autoChecking,
    desktopAvailable,
    shutdownConfirmOpen,
    toast,
    dockerConfigured,
    clashPartyConfigured,
    clashPartyDataConfigured,
    clashPartyApiConfigured,
    sub2apiConfigured,
    selectedClashPartyGroup,
    load,
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
    closeShutdownConfirm,
    confirmWindowsShutdown,
    runSub2api,
    refreshSub2apiHealth,
    refreshRuns,
    refreshOperationLogs,
    openConfig,
    closeConfig,
    hideToast,
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
