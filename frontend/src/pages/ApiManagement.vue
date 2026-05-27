<script setup lang="ts">
import {
  CheckCircle2,
  ChevronRight,
  Cog,
  GitBranch,
  RefreshCw,
  X,
  XCircle,
} from '@lucide/vue'
import { computed, onMounted, ref } from 'vue'
import ConfirmDialog from '../components/ConfirmDialog.vue'
import SmartSelect, { type SmartSelectItem } from '../components/SmartSelect.vue'
import ToolShell from '../components/ToolShell.vue'
import { useWindowsWorkbenchStore } from '../stores/windowsWorkbench'

const workbench = useWindowsWorkbenchStore()
const selectedApi = ref<'clashParty' | ''>('')

const activeSubscription = computed(() =>
  workbench.clashPartyManager?.subscriptions.find((item) => item.active),
)

const selectedSubscription = computed(() =>
  workbench.clashPartyManager?.subscriptions.find(
    (item) => item.id === workbench.selectedClashPartySubscriptionId,
  ),
)

const selectedGroupNodes = computed(() => workbench.selectedClashPartyGroup?.nodes ?? [])

const subscriptionItems = computed<SmartSelectItem[]>(() =>
  (workbench.clashPartyManager?.subscriptions ?? []).map((subscription) => ({
    value: subscription.id,
    label: subscription.name,
    description: `${subscription.nodeCount} 节点 / ${subscription.groupCount} 组 · ${formatBytes(subscription.usedBytes)} / ${formatBytes(subscription.totalBytes)}`,
    badge: subscription.active ? '当前' : subscription.profileType,
    active: subscription.active,
  })),
)

const proxyGroupItems = computed<SmartSelectItem[]>(() =>
  (workbench.clashPartyManager?.groups ?? []).map((group) => ({
    value: group.name,
    label: group.displayName || group.name,
    description: group.selectedDisplayName ? `当前节点: ${group.selectedDisplayName}` : group.groupType,
    badge: group.groupType,
  })),
)

const nodeItems = computed<SmartSelectItem[]>(() =>
  selectedGroupNodes.value.map((node) => ({
    value: node.name,
    label: node.displayName || node.name,
    description: node.server ? `${node.nodeType} · ${node.server}${node.port ? `:${node.port}` : ''}` : node.nodeType,
    badge: node.delay !== null && node.delay !== undefined ? `${node.delay}ms` : node.nodeType,
    active: node.active,
  })),
)

const clashPartyApiState = computed(() => {
  if (workbench.clashPartyManager?.apiAvailable) return { label: 'API 已连接', tone: 'good' }
  if (workbench.clashPartyManager) return { label: '仅读取文件', tone: 'warn' }
  return { label: '待刷新', tone: 'muted' }
})

const breadcrumbs = computed(() =>
  selectedApi.value === 'clashParty'
    ? [
        { label: 'API 管理', onClick: closeApiDetail },
        { label: 'Clash Party / Mihomo' },
      ]
    : undefined,
)

function openApiDetail(api: 'clashParty') {
  selectedApi.value = api
}

function closeApiDetail() {
  selectedApi.value = ''
}


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



onMounted(() => {
  void (async () => {
    await workbench.ensureLoaded()
    if (workbench.desktopAvailable) {
      await workbench.refreshClashPartyManager()
    }
  })()
})
</script>

<template>
  <ToolShell
    title="API 管理"
    description="通过 Mihomo API 管理 Clash Party / Mihomo Party 的订阅与节点。"
    eyebrow="工作台"
    :breadcrumbs="breadcrumbs"
  >
    <p v-if="!workbench.desktopAvailable" class="desktop-only-message">
      API 管理需要在 Tauri 桌面版中使用，Web 开发服务只支持页面预览。
    </p>

    <section v-if="!selectedApi" class="api-card-grid">
      <button class="api-entry-card" type="button" @click="openApiDetail('clashParty')">
        <span class="service-icon">
          <GitBranch class="h-5 w-5" aria-hidden="true" />
        </span>
        <span class="api-entry-copy">
          <strong>Clash Party / Mihomo</strong>
          <small>订阅、代理组与节点切换</small>
        </span>
        <span class="api-entry-meta">
          <span class="status-pill" :class="`status-pill--${clashPartyApiState.tone}`">{{ clashPartyApiState.label }}</span>
          <small>{{ workbench.clashPartyManager?.subscriptions.length ?? 0 }} 个订阅 · {{ workbench.clashPartyManager?.groups.length ?? 0 }} 组</small>
        </span>
        <ChevronRight class="h-5 w-5 api-entry-arrow" aria-hidden="true" />
      </button>
    </section>

    <section v-else-if="selectedApi === 'clashParty'" class="clash-manager-panel">
      <header class="service-card-header">
        <div class="service-title">
          <span class="service-icon">
            <GitBranch class="h-5 w-5" aria-hidden="true" />
          </span>
          <div>
            <h3>Clash Party / Mihomo</h3>
            <p>读取订阅配置，查看运行时代理组，并通过 Mihomo API 切换订阅和节点。</p>
          </div>
        </div>
        <div class="service-actions">
          <span class="status-pill" :class="`status-pill--${clashPartyApiState.tone}`">{{ clashPartyApiState.label }}</span>
          <button
            class="icon-only-button"
            type="button"
            title="Clash Party 配置"
            @click="workbench.openConfig('clashParty')"
          >
            <Cog class="h-4 w-4" aria-hidden="true" />
          </button>
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

      <p class="manager-status-note" :class="{ 'manager-status-note--warn': workbench.clashPartyManager && !workbench.clashPartyManager.apiAvailable }">
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
            <SmartSelect
              v-model="workbench.selectedClashPartySubscriptionId"
              :items="subscriptionItems"
              placeholder="请选择订阅"
              search-placeholder="搜索订阅、类型或流量"
              empty-text="没有匹配的订阅"
            />
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
            <SmartSelect
              v-model="workbench.selectedClashPartyGroupName"
              :items="proxyGroupItems"
              placeholder="请选择代理组"
              search-placeholder="搜索代理组或当前节点"
              empty-text="没有匹配的代理组"
            />
          </label>

          <label class="field-control">
            <span class="field-label">目标节点</span>
            <SmartSelect
              v-model="workbench.selectedClashPartyNodeName"
              :items="nodeItems"
              placeholder="请选择节点"
              search-placeholder="搜索节点、类型或地址"
              empty-text="没有匹配的节点"
            />
          </label>

          <p v-if="!selectedGroupNodes.length" class="empty-state">未读取到可切换节点。请确认 API 地址可用后刷新。</p>

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



    <div v-if="workbench.activeConfig === 'clashParty'" class="drawer-backdrop" @click.self="workbench.closeConfig">
      <aside class="config-drawer" aria-label="工作台配置">
        <header class="drawer-header">
          <div>
            <h3>Clash Party 配置</h3>
            <p>配置保存后会写入本机 SQLite 数据库。</p>
          </div>
          <button class="icon-only-button" type="button" title="关闭" @click="workbench.closeConfig">
            <X class="h-4 w-4" aria-hidden="true" />
          </button>
        </header>

        <div class="drawer-body">
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

        <footer class="drawer-footer">
          <button class="secondary-button" type="button" @click="workbench.closeConfig">取消</button>
          <button class="primary-button compact-primary" type="button" :disabled="workbench.loading === 'save-config'" @click="workbench.saveConfig">
            保存配置
          </button>
        </footer>
      </aside>
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
  </ToolShell>
</template>
