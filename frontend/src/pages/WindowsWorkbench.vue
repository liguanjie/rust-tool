<script setup lang="ts">
import {
  Activity,
  CheckCircle2,
  Cog,
  Container,
  MonitorCog,
  Play,
  Power,
  RefreshCw,
  Rocket,
  Square,
  UploadCloud,
  X,
  XCircle,
} from '@lucide/vue'
import { computed, onMounted } from 'vue'
import ConfirmDialog from '../components/ConfirmDialog.vue'
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



onMounted(() => {
  void workbench.refreshDashboard()
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
            <dd class="service-status-text">{{ workbench.sub2apiHealth?.message || '待检测' }}</dd>
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
            <small class="field-hint">用于切换订阅和节点；默认按 http://127.0.0.1:9998 访问。</small>
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
            <small class="field-hint">选择脚本后会自动填入脚本所在目录；也可以手动覆盖。</small>
          </label>
          <label class="field-control">
            <span class="field-label">健康检查地址</span>
            <input v-model="workbench.config.sub2apiHealthUrl" class="text-input" type="text" />
            <small class="field-hint">/v1 接口会使用下面的 API Key 进行鉴权。</small>
          </label>
          <label class="field-control">
            <span class="field-label">API Key</span>
            <input v-model="workbench.config.sub2apiApiKey" class="text-input" type="password" autocomplete="off" />
            <small class="field-hint">用于 /v1/models 等 OpenAI 兼容接口，通常在 sub2api 后台的 API 密钥页面创建。</small>
          </label>
          <label class="field-control">
            <span class="field-label">登录地址</span>
            <input v-model="workbench.config.sub2apiLoginUrl" class="text-input" type="text" />
            <small class="field-hint">填写邮箱和密码后，检测时会额外校验后台登录。</small>
          </label>
          <label class="field-control">
            <span class="field-label">登录邮箱</span>
            <input v-model="workbench.config.sub2apiUsername" class="text-input" type="email" autocomplete="email" />
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

    <ConfirmDialog
      v-if="workbench.pendingConfirm"
      :title="workbench.pendingConfirm.title"
      :message="workbench.pendingConfirm.message"
      :warning="workbench.pendingConfirm.warning"
      :confirm-text="workbench.pendingConfirm.confirmText"
      :tone="workbench.pendingConfirm.tone"
      :loading="workbench.loading === workbench.pendingConfirm.loadingKey"
      @cancel="workbench.closeConfirm"
      @confirm="workbench.confirmPendingAction"
    />

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
