<script setup lang="ts">
import { ChevronLeft, ChevronRight, RefreshCw, ScrollText, Search, Trash2 } from '@lucide/vue'
import { onMounted, ref } from 'vue'
import ConfirmDialog from '../components/ConfirmDialog.vue'
import ToolShell from '../components/ToolShell.vue'
import { useWindowsWorkbenchStore } from '../stores/windowsWorkbench'

const workbench = useWindowsWorkbenchStore()
const searchText = ref('')

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

function searchLogs() {
  void workbench.searchOperationLogs(searchText.value)
}

onMounted(() => {
  void (async () => {
    await workbench.ensureLoaded()
    if (workbench.desktopAvailable) {
      await workbench.refreshOperationLogs()
    }
  })()
})
</script>

<template>
  <ToolShell title="操作日志" description="记录工作台每步操作，自动保留最近 7 天。" eyebrow="工作台">
    <p v-if="!workbench.desktopAvailable" class="desktop-only-message">
      操作日志需要在 Tauri 桌面版中使用，Web 开发服务只支持页面预览。
    </p>

    <section class="operation-log-panel full-width-panel">
      <header>
        <div class="service-title">
          <span class="service-icon">
            <ScrollText class="h-5 w-5" aria-hidden="true" />
          </span>
          <div>
            <h3>系统操作日志</h3>
            <p>系统记录的所有后台任务及接口交互历史日志。</p>
          </div>
        </div>
        <div class="service-actions">
          <button class="icon-button" type="button" :disabled="workbench.loading === 'operation-logs'" @click="workbench.refreshOperationLogs">
            <RefreshCw class="h-4 w-4" aria-hidden="true" />
            <span>刷新</span>
          </button>
          <button
            class="danger-button"
            type="button"
            :disabled="!workbench.operationLogs.length || workbench.loading === 'operation-logs-clear'"
            @click="workbench.clearLogs"
          >
            <Trash2 class="h-4 w-4" aria-hidden="true" />
            <span>清理</span>
          </button>
        </div>
      </header>

      <div class="log-toolbar">
        <form class="search-field" @submit.prevent="searchLogs">
          <Search class="h-4 w-4" aria-hidden="true" />
          <input v-model="searchText" type="search" placeholder="搜索模块、动作、状态、消息或详情" @search="searchLogs" />
        </form>
        <button class="secondary-button" type="button" :disabled="workbench.loading === 'operation-logs'" @click="searchLogs">
          <Search class="h-4 w-4" aria-hidden="true" />
          <span>搜索</span>
        </button>
        <span class="log-count">
          第 {{ workbench.operationLogPage.page }} / {{ workbench.operationLogPage.totalPages }} 页 · 每页 {{ workbench.operationLogPage.pageSize }} 条 · 共 {{ workbench.operationLogPage.total }} 条
        </span>
      </div>

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
      <p v-else class="empty-state">{{ searchText.trim() ? '没有匹配的操作日志。' : '还没有操作日志。' }}</p>

      <footer class="pagination-bar">
        <button
          class="secondary-button"
          type="button"
          :disabled="workbench.loading === 'operation-logs' || workbench.operationLogPage.page <= 1"
          @click="workbench.goToOperationLogPage(workbench.operationLogPage.page - 1)"
        >
          <ChevronLeft class="h-4 w-4" aria-hidden="true" />
          <span>上一页</span>
        </button>
        <span class="pagination-summary">
          {{ workbench.operationLogPage.total ? `${(workbench.operationLogPage.page - 1) * workbench.operationLogPage.pageSize + 1}-${Math.min(workbench.operationLogPage.page * workbench.operationLogPage.pageSize, workbench.operationLogPage.total)}` : '0' }}
        </span>
        <button
          class="secondary-button"
          type="button"
          :disabled="workbench.loading === 'operation-logs' || workbench.operationLogPage.page >= workbench.operationLogPage.totalPages"
          @click="workbench.goToOperationLogPage(workbench.operationLogPage.page + 1)"
        >
          <span>下一页</span>
          <ChevronRight class="h-4 w-4" aria-hidden="true" />
        </button>
      </footer>
    </section>

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

<style scoped>
.full-width-panel {
  width: 100%;
  margin-top: 0;
}
</style>
