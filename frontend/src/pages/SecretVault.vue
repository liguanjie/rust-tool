<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, reactive, ref } from 'vue'
import {
  AlertTriangle,
  Check,
  Copy,
  Eye,
  EyeOff,
  FileText,
  KeyRound,
  Lock,
  RefreshCw,
  Search,
  ShieldCheck,
} from '@lucide/vue'
import SecurePasswordInput from '../components/SecurePasswordInput.vue'
import ToolShell from '../components/ToolShell.vue'
import { memoRequest } from '../services/memoApi'

interface SecretListItem {
  id: string
  key: string
  documentId?: string
  documentTitle?: string
  fileName?: string
  updatedAt?: number
  referenced: boolean
  hasValue: boolean
  source: string
}

const unlocked = ref(false)
const masterPassword = ref('')
const loading = ref(true)
const unlocking = ref(false)
const errorMessage = ref('')
const searchQuery = ref('')
const secrets = ref<SecretListItem[]>([])
const revealedValues = reactive<Record<string, string>>({})
const revealingIds = reactive<Record<string, boolean>>({})
const copiedId = ref('')
const noticeMessage = ref('')
const showChangePassword = ref(false)
const changingPassword = ref(false)
const currentPasswordForChange = ref('')
const newMasterPassword = ref('')
const confirmMasterPassword = ref('')
const revealTimers = new Map<string, number>()

const filteredSecrets = computed(() => {
  const query = searchQuery.value.trim().toLowerCase()
  if (!query) {
    return secrets.value
  }
  return secrets.value.filter((secret) => {
    return [secret.key, secret.documentTitle, secret.fileName, secret.source]
      .filter(Boolean)
      .some((value) => String(value).toLowerCase().includes(query))
  })
})

const referencedCount = computed(() => secrets.value.filter((secret) => secret.referenced).length)
const orphanCount = computed(() => secrets.value.filter((secret) => !secret.referenced).length)
const missingCount = computed(() => secrets.value.filter((secret) => !secret.hasValue).length)

async function readApiError(res: Response, fallback = '请求失败') {
  try {
    const data = await res.clone().json()
    if (typeof data?.error?.message === 'string' && data.error.message.trim()) {
      return data.error.message
    }
    if (typeof data?.message === 'string' && data.message.trim()) {
      return data.message
    }
  } catch {
    // Fall back to text below.
  }

  try {
    const text = await res.text()
    return text.trim() || fallback
  } catch {
    return fallback
  }
}

async function fetchStatus() {
  const res = await memoRequest('/status')
  if (!res.ok) {
    throw new Error(await readApiError(res, '读取保险箱状态失败'))
  }
  const data = await res.json()
  unlocked.value = Boolean(data.unlocked)
}

async function loadSecrets() {
  loading.value = true
  errorMessage.value = ''
  try {
    await fetchStatus()
    if (!unlocked.value) {
      secrets.value = []
      return
    }

    const res = await memoRequest('/secrets')
    if (res.ok) {
      secrets.value = await res.json()
    } else if (res.status === 401) {
      unlocked.value = false
      secrets.value = []
    } else {
      errorMessage.value = await readApiError(res, '读取密码库失败')
    }
  } catch (error) {
    errorMessage.value = String(error)
  } finally {
    loading.value = false
  }
}

async function unlockVault() {
  const password = masterPassword.value
  if (!password || unlocking.value) {
    return
  }

  unlocking.value = true
  errorMessage.value = ''
  try {
    noticeMessage.value = ''
    const res = await memoRequest('/unlock', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ password }),
    })
    if (!res.ok) {
      errorMessage.value = await readApiError(res, '解锁失败')
      return
    }
    const data = await res.json()
    if (!data.unlocked) {
      errorMessage.value = '主密码不正确。'
      return
    }
    masterPassword.value = ''
    unlocked.value = true
    await loadSecrets()
  } catch (error) {
    errorMessage.value = String(error)
  } finally {
    unlocking.value = false
  }
}

async function changeMasterPassword() {
  errorMessage.value = ''
  noticeMessage.value = ''
  if (!currentPasswordForChange.value || !newMasterPassword.value || !confirmMasterPassword.value) {
    errorMessage.value = '请完整填写当前主密码、新主密码和确认密码。'
    return
  }
  if (newMasterPassword.value !== confirmMasterPassword.value) {
    errorMessage.value = '两次输入的新主密码不一致。'
    return
  }

  changingPassword.value = true
  try {
    const res = await memoRequest('/change-master-password', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        currentPassword: currentPasswordForChange.value,
        newPassword: newMasterPassword.value,
      }),
    })
    if (!res.ok) {
      errorMessage.value = await readApiError(res, '修改主密码失败')
      return
    }

    const data = await res.json()
    showChangePassword.value = false
    resetChangePasswordForm()
    secrets.value = []
    unlocked.value = false
    Object.keys(revealedValues).forEach((id) => hideSecret(id))
    noticeMessage.value = `${data.message || '主密码已修改，请重新解锁。'} 备份：${data.backupPath || '已创建'}`
  } catch (error) {
    errorMessage.value = String(error)
  } finally {
    changingPassword.value = false
  }
}

function resetChangePasswordForm() {
  currentPasswordForChange.value = ''
  newMasterPassword.value = ''
  confirmMasterPassword.value = ''
}

function closeChangePassword() {
  if (changingPassword.value) {
    return
  }
  showChangePassword.value = false
  resetChangePasswordForm()
}

async function revealSecret(secret: SecretListItem) {
  if (!secret.hasValue || revealingIds[secret.id]) {
    return
  }
  if (revealedValues[secret.id]) {
    hideSecret(secret.id)
    return
  }

  revealingIds[secret.id] = true
  errorMessage.value = ''
  try {
    const value = await fetchSecretValue(secret.id)
    revealedValues[secret.id] = value
    scheduleHide(secret.id)
  } catch (error) {
    errorMessage.value = String(error)
  } finally {
    revealingIds[secret.id] = false
  }
}

async function copySecret(secret: SecretListItem) {
  if (!secret.hasValue) {
    return
  }

  try {
    const value = revealedValues[secret.id] || await fetchSecretValue(secret.id)
    await navigator.clipboard.writeText(value)
    copiedId.value = secret.id
    window.setTimeout(() => {
      if (copiedId.value === secret.id) {
        copiedId.value = ''
      }
    }, 1600)
  } catch (error) {
    errorMessage.value = `复制失败：${error}`
  }
}

async function fetchSecretValue(id: string) {
  const res = await memoRequest('/secrets/reveal', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ id }),
  })
  if (!res.ok) {
    throw new Error(await readApiError(res, '读取密码失败'))
  }
  const data = await res.json()
  return String(data.value || '')
}

function scheduleHide(id: string) {
  const existingTimer = revealTimers.get(id)
  if (existingTimer) {
    window.clearTimeout(existingTimer)
  }
  const timer = window.setTimeout(() => {
    hideSecret(id)
  }, 15000)
  revealTimers.set(id, timer)
}

function hideSecret(id: string) {
  delete revealedValues[id]
  const timer = revealTimers.get(id)
  if (timer) {
    window.clearTimeout(timer)
    revealTimers.delete(id)
  }
}

function formatTime(value?: number) {
  if (!value) {
    return '未知'
  }
  return new Date(value * 1000).toLocaleString()
}

function sourceLabel(source: string) {
  if (source === 'kdbx') return 'KDBX'
  if (source === 'legacy') return '兼容'
  if (source === 'kdbx+legacy') return 'KDBX / 兼容'
  return '缺失'
}

onMounted(() => {
  void loadSecrets()
})

onBeforeUnmount(() => {
  for (const timer of revealTimers.values()) {
    window.clearTimeout(timer)
  }
  revealTimers.clear()
})
</script>

<template>
  <ToolShell
    title="密码库"
    description="查看 AI 安全文档中保存的 secret。明文只在单项查看或复制时临时解密。"
    :breadcrumbs="[
      { label: '工具箱', to: '/toolbox' },
      { label: '密码库' },
    ]"
    fluid
  >
    <section v-if="loading" class="vault-panel vault-empty">
      <RefreshCw class="h-6 w-6 animate-spin text-emerald-400" />
      <span>正在读取密码库...</span>
    </section>

    <section v-else-if="!unlocked" class="vault-panel unlock-panel">
      <div class="unlock-icon">
        <Lock class="h-8 w-8" />
      </div>
      <div>
        <h3>保险箱已锁定</h3>
        <p>输入 Master Password 后才能查看密码库索引。首次使用输入的密码会设为主密码。</p>
      </div>
      <form class="unlock-form" @submit.prevent="unlockVault">
        <SecurePasswordInput
          v-model="masterPassword"
          input-class="vault-input"
          autocomplete="current-password"
          placeholder="Master Password"
          show-title="显示主密码"
          hide-title="隐藏主密码"
        />
        <button type="submit" class="primary-btn" :disabled="unlocking || !masterPassword">
          <RefreshCw v-if="unlocking" class="h-4 w-4 animate-spin" />
          <Lock v-else class="h-4 w-4" />
          解锁
        </button>
      </form>
      <p v-if="noticeMessage" class="notice-line">{{ noticeMessage }}</p>
      <p v-if="errorMessage" class="error-line">{{ errorMessage }}</p>
    </section>

    <section v-else class="vault-panel">
      <header class="vault-toolbar">
        <div class="status-group">
          <span class="status-pill good">
            <ShieldCheck class="h-4 w-4" />
            已解锁
          </span>
          <span class="status-pill">{{ secrets.length }} 个 secret</span>
          <span class="status-pill">{{ referencedCount }} 个被文档引用</span>
          <span v-if="orphanCount" class="status-pill warn">{{ orphanCount }} 个孤儿项</span>
          <span v-if="missingCount" class="status-pill warn">{{ missingCount }} 个缺失值</span>
        </div>
        <button type="button" class="ghost-btn" @click="loadSecrets">
          <RefreshCw class="h-4 w-4" />
          刷新
        </button>
        <button type="button" class="ghost-btn" @click="showChangePassword = true">
          <Lock class="h-4 w-4" />
          修改主密码
        </button>
      </header>

      <div class="search-row">
        <Search class="h-4 w-4 text-slate-500" />
        <input
          v-model="searchQuery"
          type="search"
          class="search-input"
          placeholder="搜索 key、文档标题或文件名"
        />
      </div>

      <p v-if="errorMessage" class="error-line">
        <AlertTriangle class="h-4 w-4" />
        {{ errorMessage }}
      </p>

      <div v-if="filteredSecrets.length" class="secret-table">
        <div class="secret-row secret-row-head">
          <span>Secret</span>
          <span>归属文档</span>
          <span>状态</span>
          <span>值</span>
          <span>操作</span>
        </div>

        <div v-for="secret in filteredSecrets" :key="secret.id" class="secret-row">
          <div class="secret-main">
            <KeyRound class="h-4 w-4 text-emerald-400" />
            <div class="min-w-0">
              <div class="secret-key">{{ secret.key }}</div>
              <div class="secret-id">{{ secret.id }}</div>
            </div>
          </div>

          <div class="doc-cell">
            <FileText class="h-4 w-4 text-slate-500" />
            <div class="min-w-0">
              <div class="doc-title">{{ secret.documentTitle || '未知文档' }}</div>
              <div class="doc-path">{{ secret.fileName || '无文件路径' }}</div>
              <div class="doc-time">{{ formatTime(secret.updatedAt) }}</div>
            </div>
          </div>

          <div class="status-stack">
            <span class="mini-pill" :class="secret.referenced ? 'good' : 'warn'">
              {{ secret.referenced ? '已引用' : '孤儿项' }}
            </span>
            <span class="mini-pill" :class="secret.hasValue ? 'good' : 'warn'">
              {{ secret.hasValue ? sourceLabel(secret.source) : '缺失值' }}
            </span>
          </div>

          <div class="secret-value" :class="{ revealed: revealedValues[secret.id] }">
            {{ revealedValues[secret.id] || '••••••••••••' }}
          </div>

          <div class="actions-cell">
            <button
              type="button"
              class="icon-btn"
              :disabled="!secret.hasValue || revealingIds[secret.id]"
              :title="revealedValues[secret.id] ? '隐藏' : '临时查看'"
              @click="revealSecret(secret)"
            >
              <RefreshCw v-if="revealingIds[secret.id]" class="h-4 w-4 animate-spin" />
              <EyeOff v-else-if="revealedValues[secret.id]" class="h-4 w-4" />
              <Eye v-else class="h-4 w-4" />
            </button>
            <button
              type="button"
              class="icon-btn"
              :disabled="!secret.hasValue"
              title="复制"
              @click="copySecret(secret)"
            >
              <Check v-if="copiedId === secret.id" class="h-4 w-4 text-emerald-400" />
              <Copy v-else class="h-4 w-4" />
            </button>
          </div>
        </div>
      </div>

      <div v-else class="vault-empty">
        <KeyRound class="h-8 w-8 text-slate-600" />
        <div>
          <h3>暂无 secret</h3>
          <p>在 AI 安全文档中保存包含 &#123;&#123;secret:key&#125;&#125; 占位符的文档后，这里会显示对应条目。</p>
        </div>
      </div>
    </section>

    <div v-if="showChangePassword" class="modal-overlay" @click.self="closeChangePassword">
      <section class="password-modal" aria-label="修改主密码">
        <header class="password-modal-header">
          <div>
            <h3>修改主密码</h3>
            <p>系统会先自动备份，再更换 KDBX 与本地兼容 secret 的加密密码。</p>
          </div>
        </header>

        <form class="password-form" @submit.prevent="changeMasterPassword">
          <label>
            <span>当前主密码</span>
            <SecurePasswordInput
              v-model="currentPasswordForChange"
              input-class="vault-input"
              autocomplete="current-password"
              show-title="显示当前主密码"
              hide-title="隐藏当前主密码"
            />
          </label>
          <label>
            <span>新主密码</span>
            <SecurePasswordInput
              v-model="newMasterPassword"
              input-class="vault-input"
              autocomplete="new-password"
              show-title="显示新主密码"
              hide-title="隐藏新主密码"
            />
          </label>
          <label>
            <span>确认新主密码</span>
            <SecurePasswordInput
              v-model="confirmMasterPassword"
              input-class="vault-input"
              autocomplete="new-password"
              show-title="显示确认主密码"
              hide-title="隐藏确认主密码"
            />
          </label>

          <p class="warning-line">
            修改成功后保险箱会立即锁定，需要使用新主密码重新解锁。旧主密码将不能再打开当前资料库。
          </p>

          <div class="modal-actions">
            <button type="button" class="ghost-btn" :disabled="changingPassword" @click="closeChangePassword">
              取消
            </button>
            <button type="submit" class="primary-btn" :disabled="changingPassword">
              <RefreshCw v-if="changingPassword" class="h-4 w-4 animate-spin" />
              <Lock v-else class="h-4 w-4" />
              确认修改
            </button>
          </div>
        </form>
      </section>
    </div>
  </ToolShell>
</template>

<style scoped>
@reference "tailwindcss";

.vault-panel {
  border: 1px solid var(--border-card);
  background: var(--bg-card);
  box-shadow: var(--shadow-card);
  backdrop-filter: var(--backdrop-card);
  @apply mt-6 overflow-hidden rounded-2xl;
}

.vault-empty {
  @apply flex min-h-[260px] flex-col items-center justify-center gap-3 px-6 py-12 text-center text-sm text-slate-500;
}

.vault-empty h3 {
  @apply text-base font-bold text-slate-200;
}

.unlock-panel {
  @apply mx-auto flex max-w-xl flex-col items-center gap-5 px-8 py-10 text-center;
}

.unlock-panel h3 {
  @apply text-xl font-bold text-slate-100;
}

.unlock-panel p {
  @apply mt-2 text-sm leading-6 text-slate-400;
}

.unlock-icon {
  @apply flex h-20 w-20 items-center justify-center rounded-2xl border border-emerald-500/20 bg-emerald-500/10 text-emerald-400;
}

.unlock-form {
  @apply grid w-full gap-3;
}

.vault-input,
.search-input {
  background: var(--bg-input);
  border: 1px solid var(--border-input);
  color: var(--text-input-color);
  @apply h-11 w-full rounded-xl px-4 text-sm outline-none transition focus:border-emerald-500 focus:ring-1 focus:ring-emerald-500;
}

.primary-btn,
.ghost-btn,
.icon-btn {
  @apply inline-flex items-center justify-center gap-2 rounded-xl font-bold transition;
}

.primary-btn {
  @apply h-11 bg-emerald-500 px-5 text-sm text-white hover:bg-emerald-400;
}

.ghost-btn {
  border: 1px solid var(--border-card);
  @apply h-10 px-4 text-sm text-slate-300 hover:border-emerald-500/40 hover:text-emerald-300;
}

.icon-btn {
  border: 1px solid var(--border-card);
  @apply h-9 w-9 text-slate-400 hover:border-emerald-500/40 hover:text-emerald-300;
}

.vault-toolbar {
  border-bottom: 1px solid var(--border-card);
  @apply flex flex-wrap items-center justify-between gap-3 px-5 py-4;
}

.status-group {
  @apply flex flex-wrap items-center gap-2;
}

.status-pill,
.mini-pill {
  border: 1px solid var(--border-card);
  @apply inline-flex items-center gap-1.5 rounded-full px-2.5 py-1 text-xs font-bold text-slate-400;
}

.status-pill.good,
.mini-pill.good {
  @apply border-emerald-500/25 bg-emerald-500/10 text-emerald-300;
}

.status-pill.warn,
.mini-pill.warn {
  @apply border-amber-500/25 bg-amber-500/10 text-amber-300;
}

.search-row {
  border-bottom: 1px solid var(--border-card);
  @apply flex items-center gap-3 px-5 py-4;
}

.error-line {
  @apply mx-5 mt-4 flex items-center gap-2 rounded-xl border border-red-500/25 bg-red-500/10 px-4 py-3 text-sm text-red-300;
}

.notice-line {
  @apply mt-4 rounded-xl border border-emerald-500/25 bg-emerald-500/10 px-4 py-3 text-sm leading-6 text-emerald-200;
}

.warning-line {
  @apply rounded-xl border border-amber-500/25 bg-amber-500/10 px-4 py-3 text-xs leading-6 text-amber-200;
}

.modal-overlay {
  @apply fixed inset-0 z-50 flex items-center justify-center bg-black/75 p-4 backdrop-blur-md;
}

.password-modal {
  border: 1px solid var(--border-card);
  background: var(--confirm-bg);
  @apply w-full max-w-lg overflow-hidden rounded-2xl shadow-2xl;
}

.password-modal-header {
  border-bottom: 1px solid var(--border-card);
  @apply px-5 py-4;
}

.password-modal-header h3 {
  @apply text-base font-bold text-slate-100;
}

.password-modal-header p {
  @apply mt-1 text-xs leading-5 text-slate-400;
}

.password-form {
  @apply grid gap-4 p-5;
}

.password-form label {
  @apply grid gap-1.5 text-xs font-bold text-slate-400;
}

.modal-actions {
  @apply flex justify-end gap-3 pt-2;
}

.secret-table {
  @apply grid;
}

.secret-row {
  border-bottom: 1px solid var(--border-card);
  @apply grid min-h-[84px] grid-cols-[minmax(220px,1.1fr)_minmax(220px,1.1fr)_140px_minmax(180px,0.8fr)_100px] items-center gap-4 px-5 py-3;
}

.secret-row:last-child {
  border-bottom: 0;
}

.secret-row-head {
  @apply min-h-0 py-3 text-xs font-bold uppercase tracking-wide text-slate-500;
}

.secret-main,
.doc-cell,
.actions-cell {
  @apply flex min-w-0 items-center gap-3;
}

.actions-cell {
  @apply justify-end;
}

.secret-key,
.doc-title {
  @apply truncate text-sm font-bold text-slate-100;
}

.secret-id,
.doc-path,
.doc-time {
  @apply mt-1 truncate font-mono text-[11px] text-slate-500;
}

.status-stack {
  @apply flex flex-col items-start gap-1.5;
}

.secret-value {
  @apply truncate rounded-lg border border-slate-800 bg-slate-950/60 px-3 py-2 font-mono text-xs text-slate-500;
}

.secret-value.revealed {
  @apply border-emerald-500/20 bg-emerald-500/10 text-emerald-200;
}

@media (max-width: 1180px) {
  .secret-row {
    @apply grid-cols-[minmax(220px,1fr)_minmax(220px,1fr)_120px_90px];
  }

  .secret-row-head span:nth-child(4),
  .secret-value {
    @apply hidden;
  }
}

@media (max-width: 860px) {
  .vault-toolbar,
  .search-row {
    @apply px-4;
  }

  .secret-row,
  .secret-row-head {
    @apply grid-cols-1 items-start gap-3 px-4;
  }

  .secret-row-head {
    @apply hidden;
  }

  .status-stack {
    @apply flex-row flex-wrap;
  }

  .actions-cell {
    @apply justify-start;
  }
}
</style>
