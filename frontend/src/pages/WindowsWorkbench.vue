<script setup lang="ts">
import {
  Activity,
  CheckCircle2,
  Cog,
  Container,
  GitBranch,
  MonitorCog,
  Play,
  Power,
  RefreshCw,
  Rocket,
  ScrollText,
  Square,
  UploadCloud,
  X,
  XCircle,
} from '@lucide/vue'
import { computed, onMounted } from 'vue'
import ToolShell from '../components/ToolShell.vue'
import { useWindowsWorkbenchStore } from '../stores/windowsWorkbench'

const workbench = useWindowsWorkbenchStore()

const dockerState = computed(() => {
  if (!workbench.dockerConfigured) return { label: '未配置', tone: 'muted' }
  if (workbench.autoChecking && !workbench.dockerStatus) return { label: '检测中', tone: 'muted' }
  if (workbench.dockerStatus?.engineRunning) return { label: '运行中', tone: 'good' }
  if (workbench.dockerStatus?.desktopRunning) return { label: '启动中', tone: 'warn' }
  if (workbench.dockerStatus) return { label: '需检查', tone: 'warn' }
  return { label: '待检测', tone: 'muted' }
})

const dockerLaunchDisabled = computed(
  () =>
    workbench.loading === 'docker-start' ||
    Boolean(workbench.dockerStatus?.engineRunning) ||
    Boolean(workbench.dockerStatus?.desktopRunning),
)

const dockerLaunchLabel = computed(() => {
  if (workbench.loading === 'docker-start') return '启动中'
  if (workbench.dockerStatus?.engineRunning) return '已运行'
  if (workbench.dockerStatus?.desktopRunning) return '启动中'
  return '启动 Docker'
})

const sub2apiState = computed(() => {
  if (!workbench.sub2apiConfigured) return { label: '未配置', tone: 'muted' }
  if (workbench.autoChecking && !workbench.sub2apiHealth) return { label: '检测中', tone: 'muted' }
  if (workbench.sub2apiHealth?.ok) return { label: '运行中', tone: 'good' }
  if (workbench.sub2apiHealth) return { label: '异常', tone: 'warn' }
  return { label: '待检测', tone: 'muted' }
})

const clashPartyState = computed(() => {
  if (!workbench.clashPartyConfigured) return { label: '未配置', tone: 'muted' }
  if (workbench.autoChecking && !workbench.clashPartyStatus) return { label: '检测中', tone: 'muted' }
  if (workbench.clashPartyStatus?.running) return { label: '运行中', tone: 'good' }
  if (workbench.clashPartyStatus) return { label: '未运行', tone: 'warn' }
  return { label: '待检测', tone: 'muted' }
})

const activeSubscription = computed(() =>
  workbench.clashPartyManager?.subscriptions.find((item) => item.active),
)

const selectedSubscription = computed(() =>
  workbench.clashPartyManager?.subscriptions.find(
    (item) => item.id === workbench.selectedClashPartySubscriptionId,
  ),
)

const selectedGroupNodes = computed(() => workbench.selectedClashPartyGroup?.nodes ?? [])

function formatBytes(value: number | null | undefined) {
  if (!value || value <= 0) return '无'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  let next = value
  let index = 0
  while (next >= 1024 && index < units.length - 1) {
    next /= 1024
    index += 1
  }
  return `${next.toFixed(index === 0 ? 0 : 2)} ${units[index]}`
}

function formatLogTime(value: string) {
  const epochSeconds = Number(value)
  if (!Number.isFinite(epochSeconds) || epochSeconds <= 0) return value || '未知时间'
  return new Date(epochSeconds * 1000).toLocaleString()
}

function operationLogTone(status: string) {
  if (['success', 'started'].includes(status)) return 'status-pill--good'
  if (['failed', 'warn'].includes(status)) return 'status-pill--warn'
  return 'status-pill--muted'
}

onMounted(() => {
  void workbench.load()
})
</script>

<template>
  <ToolShell title="Windows 工作台" description="管理本机 Docker、sub2api、Clash Party 和常用脚本。" eyebrow="工作台">
    <p v-if="!workbench.desktopAvailable" class="desktop-only-message">
      Windows 工作台需要在 Tauri 桌面版中使用，Web 开发服务只支持页面预览。
    </p>

    <div class="workbench-grid">
      <section class="service-card">
        <header class="service-card-header">
          <div class="service-title">
            <span class="service-icon">
              <Container class="h-5 w-5" aria-hidden="true" />
            </span>
            <div>
              <h3>Docker Desktop</h3>
              <p>启动 Docker，并检测 docker CLI 是否可用。</p>
            </div>
          </div>
          <div class="service-actions">
            <span class="status-pill" :class="`status-pill--${dockerState.tone}`">{{ dockerState.label }}</span>
            <button class="icon-only-button" type="button" title="Docker 配置" @click="workbench.openConfig('docker')">
              <Cog class="h-4 w-4" aria-hidden="true" />
            </button>
          </div>
        </header>

        <dl class="service-facts">
          <div>
            <dt>Docker Desktop</dt>
            <dd>{{ workbench.config.dockerDesktopPath || '未配置' }}</dd>
          </div>
          <div>
            <dt>docker CLI</dt>
            <dd>{{ workbench.config.dockerCliPath || '未配置' }}</dd>
          </div>
          <div>
            <dt>版本</dt>
            <dd>{{ workbench.dockerStatus?.version || '待检测' }}</dd>
          </div>
        </dl>

        <div class="card-button-row">
          <button class="secondary-button" type="button" :disabled="dockerLaunchDisabled" @click="workbench.launchDocker">
            <Play class="h-4 w-4" aria-hidden="true" />
            <span>{{ dockerLaunchLabel }}</span>
          </button>
          <button
            class="secondary-button"
            type="button"
            :disabled="workbench.loading.startsWith('docker-') || !workbench.dockerConfigured"
            @click="workbench.shutdownDocker"
          >
            <Square class="h-4 w-4" aria-hidden="true" />
            <span>{{ workbench.loading === 'docker-stop' ? '停止中' : '停止' }}</span>
          </button>
          <button
            class="secondary-button"
            type="button"
            :disabled="workbench.loading.startsWith('docker-') || !workbench.dockerConfigured"
            @click="workbench.relaunchDocker"
          >
            <RefreshCw class="h-4 w-4" aria-hidden="true" />
            <span>{{ workbench.loading === 'docker-restart' ? '重启中' : '重启' }}</span>
          </button>
          <button class="secondary-button" type="button" :disabled="workbench.loading === 'docker-status'" @click="workbench.refreshDockerStatus">
            <RefreshCw class="h-4 w-4" aria-hidden="true" />
            <span>检测</span>
          </button>
        </div>

        <p class="service-note">{{ workbench.dockerStatus?.message || '首次使用请点击齿轮配置或自动侦测 Docker 路径。' }}</p>
      </section>

      <section class="service-card">
        <header class="service-card-header">
          <div class="service-title">
            <span class="service-icon">
              <Rocket class="h-5 w-5" aria-hidden="true" />
            </span>
            <div>
              <h3>sub2api</h3>
              <p>通过白名单脚本启动、停止、升级 sub2api。</p>
            </div>
          </div>
          <div class="service-actions">
            <span class="status-pill" :class="`status-pill--${sub2apiState.tone}`">{{ sub2apiState.label }}</span>
            <button class="icon-only-button" type="button" title="sub2api 配置" @click="workbench.openConfig('sub2api')">
              <Cog class="h-4 w-4" aria-hidden="true" />
            </button>
          </div>
        </header>

        <dl class="service-facts">
          <div>
            <dt>启动脚本</dt>
            <dd>{{ workbench.config.sub2apiStartScript || '未配置' }}</dd>
          </div>
          <div>
            <dt>健康检查</dt>
            <dd>{{ workbench.config.sub2apiHealthUrl || '未配置' }}</dd>
          </div>
          <div>
            <dt>最近状态</dt>
            <dd>{{ workbench.sub2apiHealth?.message || '待检测' }}</dd>
          </div>
        </dl>

        <div class="card-button-row">
          <button class="secondary-button" type="button" :disabled="workbench.loading === 'sub2api-start'" @click="workbench.runSub2api('start')">
            <Play class="h-4 w-4" aria-hidden="true" />
            <span>启动</span>
          </button>
          <button class="secondary-button" type="button" :disabled="workbench.loading === 'sub2api-stop'" @click="workbench.runSub2api('stop')">
            <Square class="h-4 w-4" aria-hidden="true" />
            <span>停止</span>
          </button>
          <button class="secondary-button" type="button" :disabled="workbench.loading === 'sub2api-upgrade'" @click="workbench.runSub2api('upgrade')">
            <UploadCloud class="h-4 w-4" aria-hidden="true" />
            <span>升级</span>
          </button>
          <button class="secondary-button" type="button" :disabled="workbench.loading === 'sub2api-health'" @click="workbench.refreshSub2apiHealth">
            <Activity class="h-4 w-4" aria-hidden="true" />
            <span>检测</span>
          </button>
        </div>

        <p class="service-note">脚本只从配置中读取，前端不会执行任意命令。</p>
      </section>

      <section class="service-card">
        <header class="service-card-header">
          <div class="service-title">
            <span class="service-icon">
              <MonitorCog class="h-5 w-5" aria-hidden="true" />
            </span>
            <div>
              <h3>Clash Party</h3>
              <p>启动、退出并检测 Clash Party 客户端。</p>
            </div>
          </div>
          <div class="service-actions">
            <span class="status-pill" :class="`status-pill--${clashPartyState.tone}`">{{ clashPartyState.label }}</span>
            <button class="icon-only-button" type="button" title="Clash Party 配置" @click="workbench.openConfig('clashParty')">
              <Cog class="h-4 w-4" aria-hidden="true" />
            </button>
          </div>
        </header>

        <dl class="service-facts">
          <div>
            <dt>程序路径</dt>
            <dd>{{ workbench.config.clashPartyPath || '未配置' }}</dd>
          </div>
          <div>
            <dt>最近状态</dt>
            <dd>{{ workbench.clashPartyStatus?.message || '待检测' }}</dd>
          </div>
        </dl>

        <div class="card-button-row">
          <button
            class="secondary-button"
            type="button"
            :disabled="workbench.loading === 'clash-party-start' || workbench.clashPartyStatus?.running"
            @click="workbench.launchClashParty"
          >
            <Play class="h-4 w-4" aria-hidden="true" />
            <span>{{ workbench.clashPartyStatus?.running ? '已运行' : '启动' }}</span>
          </button>
          <button
            class="secondary-button"
            type="button"
            :disabled="workbench.loading === 'clash-party-stop' || !workbench.clashPartyConfigured"
            @click="workbench.exitClashParty"
          >
            <Square class="h-4 w-4" aria-hidden="true" />
            <span>退出</span>
          </button>
          <button class="secondary-button" type="button" :disabled="workbench.loading === 'clash-party-status'" @click="workbench.refreshClashPartyStatus">
            <RefreshCw class="h-4 w-4" aria-hidden="true" />
            <span>检测</span>
          </button>
        </div>

        <p class="service-note">首次使用请点击齿轮配置或自动侦测 Clash Party 路径。</p>
      </section>

      <section class="service-card service-card--danger">
        <header class="service-card-header">
          <div class="service-title">
            <span class="service-icon service-icon--danger">
              <Power class="h-5 w-5" aria-hidden="true" />
            </span>
            <div>
              <h3>系统电源</h3>
              <p>关闭当前 Windows 电脑。</p>
            </div>
          </div>
          <span class="status-pill status-pill--warn">高风险</span>
        </header>

        <p class="service-note">点击后会二次确认；确认后 Windows 将在 10 秒后关机。</p>

        <div class="card-button-row">
          <button class="danger-button" type="button" :disabled="workbench.loading === 'system-shutdown'" @click="workbench.requestWindowsShutdown">
            <Power class="h-4 w-4" aria-hidden="true" />
            <span>{{ workbench.loading === 'system-shutdown' ? '关机指令已发送' : '关机' }}</span>
          </button>
        </div>
      </section>
    </div>

    <section class="clash-manager-panel">
      <header class="service-card-header">
        <div class="service-title">
          <span class="service-icon">
            <GitBranch class="h-5 w-5" aria-hidden="true" />
          </span>
          <div>
            <h3>Clash Party 管理</h3>
            <p>读取订阅配置，查看运行时代理组，并通过 Mihomo API 切换订阅和节点。</p>
          </div>
        </div>
        <div class="service-actions">
          <span class="status-pill" :class="workbench.clashPartyManager?.apiAvailable ? 'status-pill--good' : 'status-pill--warn'">
            {{ workbench.clashPartyManager?.apiAvailable ? 'API 已连接' : '仅读取文件' }}
          </span>
          <button
            class="icon-button"
            type="button"
            :disabled="workbench.loading === 'clash-party-manager'"
            @click="workbench.refreshClashPartyManager"
          >
            <RefreshCw class="h-4 w-4" aria-hidden="true" />
            <span>刷新</span>
          </button>
        </div>
      </header>

      <p class="service-note">
        {{ workbench.clashPartyManager?.message || '刷新后会列出 Clash Party profile.yaml 中的订阅；切换节点需要 Clash Party 开放 Mihomo API。' }}
      </p>

      <div class="clash-manager-grid">
        <section class="manager-block">
          <div class="manager-block-header">
            <h4>订阅</h4>
            <span>{{ workbench.clashPartyManager?.subscriptions.length ?? 0 }} 个</span>
          </div>

          <label class="field-control">
            <span class="field-label">选择订阅</span>
            <select v-model="workbench.selectedClashPartySubscriptionId" class="text-input">
              <option value="">请选择订阅</option>
              <option
                v-for="subscription in workbench.clashPartyManager?.subscriptions ?? []"
                :key="subscription.id"
                :value="subscription.id"
              >
                {{ subscription.active ? '当前 - ' : '' }}{{ subscription.name }}
              </option>
            </select>
          </label>

          <dl class="service-facts compact-facts">
            <div>
              <dt>当前订阅</dt>
              <dd>{{ activeSubscription?.name || '未读取' }}</dd>
            </div>
            <div>
              <dt>节点/分组</dt>
              <dd>{{ selectedSubscription ? `${selectedSubscription.nodeCount}/${selectedSubscription.groupCount}` : '无' }}</dd>
            </div>
            <div>
              <dt>流量</dt>
              <dd>
                {{
                  selectedSubscription
                    ? `${formatBytes(selectedSubscription.usedBytes)} / ${formatBytes(selectedSubscription.totalBytes)}`
                    : '无'
                }}
              </dd>
            </div>
          </dl>

          <div class="card-button-row">
            <button
              class="secondary-button"
              type="button"
              :disabled="!workbench.selectedClashPartySubscriptionId || workbench.loading === 'clash-party-switch-subscription'"
              @click="workbench.switchSubscription"
            >
              <RefreshCw class="h-4 w-4" aria-hidden="true" />
              <span>切换订阅</span>
            </button>
          </div>
        </section>

        <section class="manager-block">
          <div class="manager-block-header">
            <h4>代理组与节点</h4>
            <span>{{ workbench.clashPartyManager?.groups.length ?? 0 }} 组</span>
          </div>

          <label class="field-control">
            <span class="field-label">代理组</span>
            <select v-model="workbench.selectedClashPartyGroupName" class="text-input">
              <option value="">请选择代理组</option>
              <option v-for="group in workbench.clashPartyManager?.groups ?? []" :key="group.name" :value="group.name">
                {{ group.name }}{{ group.selected ? ` - ${group.selected}` : '' }}
              </option>
            </select>
          </label>

          <label class="field-control">
            <span class="field-label">目标节点</span>
            <select v-model="workbench.selectedClashPartyNodeName" class="text-input">
              <option value="">请选择节点</option>
              <option v-for="node in selectedGroupNodes" :key="node.name" :value="node.name">
                {{ node.active ? '当前 - ' : '' }}{{ node.name }}{{ node.delay ? ` (${node.delay}ms)` : '' }}
              </option>
            </select>
          </label>

          <div class="node-list" v-if="selectedGroupNodes.length">
            <button
              v-for="node in selectedGroupNodes.slice(0, 18)"
              :key="node.name"
              class="node-chip"
              :class="{ 'node-chip--active': node.active }"
              type="button"
              @click="workbench.switchNode(node.name)"
            >
              <span>{{ node.name }}</span>
              <small>{{ node.delay ? `${node.delay}ms` : node.nodeType }}</small>
            </button>
          </div>
          <p v-else class="empty-state">未读取到可切换节点。请确认 API 地址可用后刷新。</p>

          <div class="card-button-row">
            <button
              class="secondary-button"
              type="button"
              :disabled="!workbench.selectedClashPartyGroupName || !workbench.selectedClashPartyNodeName || workbench.loading === 'clash-party-switch-node'"
              @click="workbench.switchNode()"
            >
              <GitBranch class="h-4 w-4" aria-hidden="true" />
              <span>切换节点</span>
            </button>
          </div>
        </section>
      </div>
    </section>

    <section class="task-log-panel">
      <header>
        <h3>最近任务</h3>
        <button class="icon-button" type="button" @click="workbench.refreshRuns">
          <RefreshCw class="h-4 w-4" aria-hidden="true" />
          <span>刷新</span>
        </button>
      </header>
      <div v-if="workbench.taskRuns.length" class="task-log-list">
        <article v-for="run in workbench.taskRuns" :key="run.id" class="task-log-item">
          <div>
            <strong>{{ run.taskKey }}</strong>
            <p>{{ run.stderr || run.stdout || '无输出' }}</p>
          </div>
          <span class="status-pill" :class="['success', 'started'].includes(run.status) ? 'status-pill--good' : 'status-pill--warn'">
            {{ run.status }}
          </span>
        </article>
      </div>
      <p v-else class="empty-state">还没有任务记录。</p>
    </section>

    <section class="operation-log-panel">
      <header>
        <div class="service-title">
          <span class="service-icon">
            <ScrollText class="h-5 w-5" aria-hidden="true" />
          </span>
          <div>
            <h3>操作日志</h3>
            <p>记录工作台每步操作，自动保留最近 7 天。</p>
          </div>
        </div>
        <button class="icon-button" type="button" @click="workbench.refreshOperationLogs">
          <RefreshCw class="h-4 w-4" aria-hidden="true" />
          <span>刷新</span>
        </button>
      </header>
      <div v-if="workbench.operationLogs.length" class="task-log-list">
        <article v-for="log in workbench.operationLogs" :key="log.id" class="task-log-item operation-log-item">
          <div>
            <strong>{{ log.module }} / {{ log.action }}</strong>
            <p>{{ log.message }}</p>
            <small class="operation-log-meta">
              {{ formatLogTime(log.createdAt) }}<span v-if="log.detail"> · {{ log.detail }}</span>
            </small>
          </div>
          <span class="status-pill" :class="operationLogTone(log.status)">
            {{ log.status }}
          </span>
        </article>
      </div>
      <p v-else class="empty-state">还没有操作日志。</p>
    </section>

    <div v-if="workbench.activeConfig" class="drawer-backdrop" @click.self="workbench.closeConfig">
      <aside class="config-drawer" aria-label="工作台配置">
        <header class="drawer-header">
          <div>
            <h3>
              {{
                workbench.activeConfig === 'docker'
                  ? 'Docker 配置'
                  : workbench.activeConfig === 'clashParty'
                    ? 'Clash Party 配置'
                    : 'sub2api 配置'
              }}
            </h3>
            <p>配置保存后会写入本机 SQLite 数据库。</p>
          </div>
          <button class="icon-only-button" type="button" title="关闭" @click="workbench.closeConfig">
            <X class="h-4 w-4" aria-hidden="true" />
          </button>
        </header>

        <div v-if="workbench.activeConfig === 'docker'" class="drawer-body">
          <label class="field-control">
            <span class="field-label">Docker Desktop 路径</span>
            <span class="path-input-row">
              <input v-model="workbench.config.dockerDesktopPath" class="text-input" type="text" />
              <button class="secondary-button" type="button" @click="workbench.selectDockerDesktopPath">
                选择
              </button>
            </span>
          </label>
          <label class="field-control">
            <span class="field-label">docker CLI 路径</span>
            <span class="path-input-row">
              <input v-model="workbench.config.dockerCliPath" class="text-input" type="text" />
              <button class="secondary-button" type="button" @click="workbench.selectDockerCliPath">
                选择
              </button>
            </span>
          </label>
          <button class="secondary-button" type="button" @click="workbench.autoDetectDocker">
            <RefreshCw class="h-4 w-4" aria-hidden="true" />
            <span>自动侦测</span>
          </button>
        </div>

        <div v-else-if="workbench.activeConfig === 'clashParty'" class="drawer-body">
          <label class="field-control">
            <span class="field-label">Clash Party 路径</span>
            <span class="path-input-row">
              <input v-model="workbench.config.clashPartyPath" class="text-input" type="text" />
              <button class="secondary-button" type="button" @click="workbench.selectClashPartyPath">
                选择
              </button>
            </span>
          </label>
          <label class="field-control">
            <span class="field-label">数据目录</span>
            <span class="path-input-row">
              <input v-model="workbench.config.clashPartyDataDir" class="text-input" type="text" />
              <button class="secondary-button" type="button" @click="workbench.selectClashPartyDataDir">
                选择
              </button>
            </span>
            <small class="field-hint">通常是 %APPDATA%\mihomo-party，里面应包含 profile.yaml 和 profiles 目录。</small>
          </label>
          <label class="field-control">
            <span class="field-label">Mihomo API 地址</span>
            <input v-model="workbench.config.clashPartyApiUrl" class="text-input" type="text" />
            <small class="field-hint">用于切换订阅和节点；默认按 http://127.0.0.1:9090 访问。</small>
          </label>
          <label class="field-control">
            <span class="field-label">API Secret</span>
            <input v-model="workbench.config.clashPartyApiSecret" class="text-input" type="password" autocomplete="off" />
            <small class="field-hint">如果 Clash Party/Mihomo 设置了 secret，这里会以 Bearer Token 方式发送。</small>
          </label>
          <button class="secondary-button" type="button" @click="workbench.autoDetectClashParty">
            <RefreshCw class="h-4 w-4" aria-hidden="true" />
            <span>自动侦测</span>
          </button>
        </div>

        <div v-else class="drawer-body">
          <label class="field-control">
            <span class="field-label">启动脚本</span>
            <span class="path-input-row">
              <input v-model="workbench.config.sub2apiStartScript" class="text-input" type="text" />
              <button class="secondary-button" type="button" @click="workbench.selectSub2apiScript('sub2apiStartScript')">
                选择
              </button>
            </span>
          </label>
          <label class="field-control">
            <span class="field-label">停止脚本</span>
            <span class="path-input-row">
              <input v-model="workbench.config.sub2apiStopScript" class="text-input" type="text" />
              <button class="secondary-button" type="button" @click="workbench.selectSub2apiScript('sub2apiStopScript')">
                选择
              </button>
            </span>
          </label>
          <label class="field-control">
            <span class="field-label">升级脚本</span>
            <span class="path-input-row">
              <input v-model="workbench.config.sub2apiUpgradeScript" class="text-input" type="text" />
              <button class="secondary-button" type="button" @click="workbench.selectSub2apiScript('sub2apiUpgradeScript')">
                选择
              </button>
            </span>
          </label>
          <label class="field-control">
            <span class="field-label">工作目录</span>
            <span class="path-input-row">
              <input v-model="workbench.config.sub2apiWorkingDir" class="text-input" type="text" />
              <button class="secondary-button" type="button" @click="workbench.selectSub2apiWorkingDir">
                选择
              </button>
            </span>
          </label>
          <label class="field-control">
            <span class="field-label">健康检查地址</span>
            <input v-model="workbench.config.sub2apiHealthUrl" class="text-input" type="text" />
          </label>
          <label class="field-control">
            <span class="field-label">登录地址</span>
            <input v-model="workbench.config.sub2apiLoginUrl" class="text-input" type="text" />
          </label>
          <label class="field-control">
            <span class="field-label">登录账号</span>
            <input v-model="workbench.config.sub2apiUsername" class="text-input" type="text" autocomplete="username" />
          </label>
          <label class="field-control">
            <span class="field-label">登录密码</span>
            <input v-model="workbench.config.sub2apiPassword" class="text-input" type="password" autocomplete="current-password" />
          </label>
        </div>

        <footer class="drawer-footer">
          <button class="secondary-button" type="button" @click="workbench.closeConfig">取消</button>
          <button class="primary-button compact-primary" type="button" :disabled="workbench.loading === 'save-config'" @click="workbench.saveConfig">
            保存配置
          </button>
        </footer>
      </aside>
    </div>

    <div v-if="workbench.shutdownConfirmOpen" class="confirm-backdrop" @click.self="workbench.closeShutdownConfirm">
      <section class="confirm-dialog" role="dialog" aria-modal="true" aria-labelledby="shutdown-confirm-title">
        <header class="confirm-header">
          <span class="service-icon service-icon--danger">
            <Power class="h-5 w-5" aria-hidden="true" />
          </span>
          <div>
            <h3 id="shutdown-confirm-title">确认关闭 Windows？</h3>
            <p>确认后会向系统发送关机命令，当前电脑将在 10 秒后关机。</p>
          </div>
        </header>
        <p class="confirm-warning">请先保存正在编辑的文件，并确认没有正在运行的重要任务。</p>
        <footer class="confirm-footer">
          <button class="secondary-button" type="button" :disabled="workbench.loading === 'system-shutdown'" @click="workbench.closeShutdownConfirm">
            取消
          </button>
          <button class="danger-button" type="button" :disabled="workbench.loading === 'system-shutdown'" @click="workbench.confirmWindowsShutdown">
            <Power class="h-4 w-4" aria-hidden="true" />
            <span>{{ workbench.loading === 'system-shutdown' ? '发送中' : '确认关机' }}</span>
          </button>
        </footer>
      </section>
    </div>

    <Transition name="toast">
      <div v-if="workbench.toast" class="toast-message" :class="`toast-message--${workbench.toast.type}`" role="status">
        <CheckCircle2 v-if="workbench.toast.type === 'success'" class="toast-icon" aria-hidden="true" />
        <XCircle v-else class="toast-icon" aria-hidden="true" />
        <span class="toast-copy">
          <strong>{{ workbench.toast.title }}</strong>
          <small>{{ workbench.toast.detail }}</small>
        </span>
        <button class="toast-close" type="button" title="关闭提示" @click="workbench.hideToast">
          <X class="h-4 w-4" aria-hidden="true" />
        </button>
      </div>
    </Transition>
  </ToolShell>
</template>
