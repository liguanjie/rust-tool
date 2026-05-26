export interface WorkbenchConfig {
  dockerDesktopPath: string
  dockerCliPath: string
  clashPartyPath: string
  clashPartyDataDir: string
  clashPartyApiUrl: string
  clashPartyApiSecret: string
  sub2apiStartScript: string
  sub2apiStopScript: string
  sub2apiUpgradeScript: string
  sub2apiWorkingDir: string
  sub2apiHealthUrl: string
  sub2apiLoginUrl: string
  sub2apiUsername: string
  sub2apiPassword: string
}

export interface DockerDetection {
  dockerDesktopPath: string
  dockerCliPath: string
}

export interface DockerStatus {
  desktopConfigured: boolean
  cliConfigured: boolean
  desktopRunning: boolean
  cliAvailable: boolean
  engineRunning: boolean
  version: string
  message: string
}

export interface ClashPartyDetection {
  clashPartyPath: string
  clashPartyDataDir: string
  clashPartyApiUrl: string
}

export interface ClashPartyStatus {
  configured: boolean
  running: boolean
  path: string
  message: string
}

export interface ClashPartySubscription {
  id: string
  name: string
  profileType: string
  path: string
  active: boolean
  nodeCount: number
  groupCount: number
  updatedAt: string
  usedBytes: number | null
  totalBytes: number | null
  expireAt: number | null
}

export interface ClashPartyNode {
  name: string
  nodeType: string
  server: string
  port: number | null
  delay: number | null
  active: boolean
}

export interface ClashPartyProxyGroup {
  name: string
  groupType: string
  selected: string
  nodes: ClashPartyNode[]
}

export interface ClashPartyManagerState {
  dataDir: string
  profileIndexPath: string
  apiUrl: string
  activeSubscriptionId: string
  subscriptions: ClashPartySubscription[]
  groups: ClashPartyProxyGroup[]
  apiAvailable: boolean
  message: string
}

export interface ClashPartySwitchResult {
  ok: boolean
  message: string
}

export type Sub2apiTask = 'start' | 'stop' | 'upgrade'

export type WorkbenchPathKind = 'executable' | 'script'

export interface TaskRun {
  id: number
  taskKey: string
  status: string
  startedAt: string
  finishedAt: string
  exitCode: number | null
  stdout: string
  stderr: string
}

export interface OperationLog {
  id: number
  module: string
  action: string
  status: string
  message: string
  detail: string
  createdAt: string
}

export interface HealthStatus {
  ok: boolean
  message: string
}

export function defaultWorkbenchConfig(): WorkbenchConfig {
  return {
    dockerDesktopPath: '',
    dockerCliPath: '',
    clashPartyPath: '',
    clashPartyDataDir: '',
    clashPartyApiUrl: 'http://127.0.0.1:9090',
    clashPartyApiSecret: '',
    sub2apiStartScript: '',
    sub2apiStopScript: '',
    sub2apiUpgradeScript: '',
    sub2apiWorkingDir: '',
    sub2apiHealthUrl: 'http://127.0.0.1:9999/v1/models',
    sub2apiLoginUrl: 'http://127.0.0.1:9999/api/auth/login',
    sub2apiUsername: '',
    sub2apiPassword: '',
  }
}

export async function getWorkbenchConfig() {
  return await invokeWorkbench<WorkbenchConfig>('get_workbench_config')
}

export async function saveWorkbenchConfig(config: WorkbenchConfig) {
  return await invokeWorkbench<WorkbenchConfig>('save_workbench_config', { config })
}

export async function detectDocker() {
  return await invokeWorkbench<DockerDetection>('detect_docker')
}

export async function detectClashParty() {
  return await invokeWorkbench<ClashPartyDetection>('detect_clash_party')
}

export async function selectWorkbenchFile(kind: WorkbenchPathKind) {
  return await invokeWorkbench<string | null>('select_workbench_file', { kind })
}

export async function selectWorkbenchDirectory() {
  return await invokeWorkbench<string | null>('select_workbench_directory')
}

export async function getDockerStatus() {
  return await invokeWorkbench<DockerStatus>('get_docker_status')
}

export async function startDocker() {
  return await invokeWorkbench<TaskRun>('start_docker')
}

export async function stopDocker() {
  return await invokeWorkbench<TaskRun>('stop_docker')
}

export async function restartDocker() {
  return await invokeWorkbench<TaskRun>('restart_docker')
}

export async function getClashPartyStatus() {
  return await invokeWorkbench<ClashPartyStatus>('get_clash_party_status')
}

export async function startClashParty() {
  return await invokeWorkbench<TaskRun>('start_clash_party')
}

export async function stopClashParty() {
  return await invokeWorkbench<TaskRun>('stop_clash_party')
}

export async function getClashPartyManagerState() {
  return await invokeWorkbench<ClashPartyManagerState>('get_clash_party_manager_state')
}

export async function switchClashPartySubscription(subscriptionId: string) {
  return await invokeWorkbench<ClashPartySwitchResult>('switch_clash_party_subscription', { subscriptionId })
}

export async function switchClashPartyNode(groupName: string, nodeName: string) {
  return await invokeWorkbench<ClashPartySwitchResult>('switch_clash_party_node', { groupName, nodeName })
}

export async function shutdownWindows() {
  return await invokeWorkbench<TaskRun>('shutdown_windows')
}

export async function runSub2apiTask(task: Sub2apiTask) {
  return await invokeWorkbench<TaskRun>('run_sub2api_task', { task })
}

export async function checkSub2apiHealth() {
  return await invokeWorkbench<HealthStatus>('check_sub2api_health')
}

export async function listTaskRuns(limit = 20) {
  return await invokeWorkbench<TaskRun[]>('list_task_runs', { limit })
}

export async function listOperationLogs(limit = 80) {
  return await invokeWorkbench<OperationLog[]>('list_operation_logs', { limit })
}

async function invokeWorkbench<T>(command: string, args?: Record<string, unknown>) {
  const tauriCore = await import('@tauri-apps/api/core')
  if (!tauriCore.isTauri()) {
    throw new Error('Windows 工作台需要在 Tauri 桌面版中使用')
  }
  return await tauriCore.invoke<T>(command, args)
}
