import { computed, ref } from 'vue'
import { defineStore } from 'pinia'
import {
  checkSub2apiHealth,
  defaultWorkbenchConfig,
  detectDocker,
  getDockerStatus,
  getWorkbenchConfig,
  listTaskRuns,
  restartDocker,
  runSub2apiTask,
  saveWorkbenchConfig,
  selectWorkbenchDirectory,
  selectWorkbenchFile,
  startDocker,
  stopDocker,
  type DockerStatus,
  type HealthStatus,
  type Sub2apiTask,
  type TaskRun,
  type WorkbenchConfig,
} from '../api/workbench'

type Sub2apiScriptField = 'sub2apiStartScript' | 'sub2apiStopScript' | 'sub2apiUpgradeScript'

export const useWindowsWorkbenchStore = defineStore('windows-workbench', () => {
  const config = ref<WorkbenchConfig>(defaultWorkbenchConfig())
  const dockerStatus = ref<DockerStatus | null>(null)
  const sub2apiHealth = ref<HealthStatus | null>(null)
  const taskRuns = ref<TaskRun[]>([])
  const activeConfig = ref<'docker' | 'sub2api' | ''>('')
  const loading = ref('')
  const autoChecking = ref(false)
  const desktopAvailable = ref(true)
  const toast = ref<{ type: 'success' | 'error'; title: string; detail: string } | null>(null)
  let toastTimer: number | undefined

  const dockerConfigured = computed(
    () => Boolean(config.value.dockerDesktopPath.trim()) || Boolean(config.value.dockerCliPath.trim()),
  )
  const sub2apiConfigured = computed(
    () =>
      Boolean(config.value.sub2apiStartScript.trim()) ||
      Boolean(config.value.sub2apiStopScript.trim()) ||
      Boolean(config.value.sub2apiUpgradeScript.trim()),
  )
  const latestRun = computed(() => taskRuns.value[0] ?? null)

  async function load() {
    await withLoading('load', async () => {
      config.value = await getWorkbenchConfig()
      taskRuns.value = await listTaskRuns(12)
      await autoCheckConfiguredServices()
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

  function openConfig(section: 'docker' | 'sub2api') {
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
    sub2apiHealth,
    taskRuns,
    latestRun,
    activeConfig,
    loading,
    autoChecking,
    desktopAvailable,
    toast,
    dockerConfigured,
    sub2apiConfigured,
    load,
    saveConfig,
    autoDetectDocker,
    selectDockerDesktopPath,
    selectDockerCliPath,
    selectSub2apiScript,
    selectSub2apiWorkingDir,
    refreshDockerStatus,
    launchDocker,
    shutdownDocker,
    relaunchDocker,
    runSub2api,
    refreshSub2apiHealth,
    refreshRuns,
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
