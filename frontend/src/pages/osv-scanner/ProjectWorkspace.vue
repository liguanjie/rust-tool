<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { watchDebounced } from '@vueuse/core'
import {
  ArrowLeft,
  Shield,
  RefreshCw,
  Play,
  Copy,
  Clock3,
} from 'lucide-vue-next'

import { useOsvScannerStore } from '../../stores/osvScanner'
import { theme } from 'ant-design-vue'
import type {
  OsvCommandExecutionRecord,
  OsvDiagnosticLevel,
  OsvReportFormat,
  OsvSeverity,
  OsvVulnerabilityFinding,
} from '../../api/osvScanner'

const { token } = theme.useToken()

const activeTab = ref('config')
const route = useRoute()
const router = useRouter()
const osv = useOsvScannerStore()

const exportFormat = ref<OsvReportFormat>('json')
const exportPath = ref('')
const exportPathEdited = ref(false)
const copiedCommandId = ref('')
const now = ref(Date.now())
let operationTimer: number | undefined

const projectId = decodeURIComponent(route.params.id as string)

type ScanProfileId = 'standard' | 'audit' | 'offline'

const scanProfiles: Array<{
  id: ScanProfileId
  name: string
  label: string
  description: string
}> = [
  {
    id: 'standard',
    name: '日常扫描',
    label: '推荐',
    description: '递归扫描项目包源，保留低噪声输出。',
  },
  {
    id: 'audit',
    name: '完整审计',
    label: '覆盖更广',
    description: '纳入忽略路径、Git 根和完整包清单。',
  },
  {
    id: 'offline',
    name: '离线复核',
    label: '无网络',
    description: '使用本地漏洞库，适合受限网络环境。',
  },
]

const selectedProjectName = computed(() => osv.activeProject?.name || '未找到项目')

const canPreviewScan = computed(() =>
  Boolean(osv.activeProjectPath && !osv.loading && !osv.exporting && !osv.diagnosing),
)
const canRunScan = computed(() =>
  Boolean(osv.currentPreview && !osv.loading && !osv.exporting && !osv.diagnosing),
)

const currentRiskLabel = computed(() => {
  const summary = osv.latestResult?.summary
  if (!summary) return '等待扫描'
  if (summary.totalVulnerabilities === 0) return '暂未发现已知漏洞'
  if (summary.severityCounts.critical > 0) return '优先处理 Critical'
  if (summary.severityCounts.high > 0) return '优先处理 High'
  return '存在中低风险'
})

const scanCommandState = computed(() => {
  if (osv.loading) return '执行中'
  if (osv.diagnostic && !osv.diagnostic.canScan) return '诊断阻断'
  if (osv.currentPreview) return '已预览'
  if (osv.diagnosing) return '诊断中'
  return '待预览'
})

const activeScanProfile = computed<ScanProfileId | ''>(() => {
  const current = osv.options
  if (current.offline || current.offlineVulnerabilities) return 'offline'
  if (current.noIgnore && current.includeGitRoot && current.allVulns && current.allPackages) {
    return 'audit'
  }
  if (
    current.recursive
    && !current.noIgnore
    && !current.includeGitRoot
    && !current.allowNoLockfiles
    && !current.allPackages
    && !current.allVulns
    && !current.offline
    && !current.offlineVulnerabilities
    && !current.noResolve
  ) {
    return 'standard'
  }
  return ''
})

onMounted(async () => {
  if (!osv.settingsLoaded) {
    await osv.load()
  }
  
  if (projectId && projectId !== osv.activeProjectPath) {
    osv.selectProject(projectId)
  }

  // 默认进入时自动触发一次预览
  if (osv.activeProjectPath && !osv.currentPreview) {
    osv.previewScan()
  }

  operationTimer = window.setInterval(() => {
    now.value = Date.now()
  }, 1000)
})

watchDebounced(
  () => osv.options,
  () => {
    if (osv.activeProjectPath && !osv.loading) {
      osv.previewScan()
    }
  },
  { deep: true, debounce: 300 }
)

watch(
  () => osv.options,
  () => {
    osv.invalidateCommandPreviews()
    osv.clearDiagnostic()
  },
  { deep: true },
)

async function copyText(text: string, id = 'command') {
  await navigator.clipboard.writeText(text)
  copiedCommandId.value = id
  window.setTimeout(() => {
    if (copiedCommandId.value === id) copiedCommandId.value = ''
  }, 1400)
}

function applyScanProfile(profileId: ScanProfileId) {
  if (profileId === 'standard') {
    Object.assign(osv.options, {
      recursive: true,
      noIgnore: false,
      includeGitRoot: false,
      allowNoLockfiles: false,
      allPackages: false,
      allVulns: false,
      offline: false,
      offlineVulnerabilities: false,
      noResolve: false,
    })
  }

  if (profileId === 'audit') {
    Object.assign(osv.options, {
      recursive: true,
      noIgnore: true,
      includeGitRoot: true,
      allowNoLockfiles: false,
      allPackages: true,
      allVulns: true,
      offline: false,
      offlineVulnerabilities: false,
      noResolve: false,
    })
  }

  if (profileId === 'offline') {
    Object.assign(osv.options, {
      recursive: true,
      noIgnore: false,
      includeGitRoot: false,
      allowNoLockfiles: false,
      allPackages: false,
      allVulns: true,
      offline: true,
      offlineVulnerabilities: true,
      noResolve: false,
    })
  }
}

function goBack() {
  router.push({ name: 'osv-scanner-dashboard' })
}

const activeCollapse = ref(['scope'])
</script>

<template>
  <div 
    class="flex flex-col w-full h-full p-6 overflow-auto" 
    :style="{ 
      background: token.colorBgLayout,
      '--primary-color': token.colorPrimary,
      '--primary-hover-color': token.colorPrimaryHover,
      '--primary-bg-color': token.colorPrimaryBg,
      '--border-color': token.colorBorder,
      '--border-secondary-color': token.colorBorderSecondary,
      '--text-color': token.colorText,
      '--text-disabled-color': token.colorTextDisabled,
      '--bg-disabled-color': token.colorBgContainerDisabled,
      '--bg-container-color': token.colorBgContainer
    }"
  >
    <div class="w-full max-w-7xl mx-auto flex flex-col gap-6">
      <!-- Header -->
      <a-page-header
        :style="{ background: token.colorBgContainer, borderRadius: '8px', border: `1px solid ${token.colorBorderSecondary}` }"
        :title="selectedProjectName"
        :sub-title="projectId"
        @back="goBack"
      >
        <template #tags>
          <a-tag color="blue"><Shield class="w-3 h-3 inline-block" style="margin-right: 4px;" />OSV Scanner</a-tag>
        </template>
      </a-page-header>

      <!-- Terminal Box -->
      <a-card size="small" :style="{ borderRadius: '8px', overflow: 'hidden', border: `1px solid ${token.colorBorderSecondary}` }" :bodyStyle="{ padding: 0 }">
        <div :style="{ padding: '12px 16px', background: token.colorFillAlter, borderBottom: `1px solid ${token.colorBorderSecondary}`, display: 'flex', justifyContent: 'space-between', alignItems: 'center' }">
          <div class="flex items-center gap-2">
            <span class="font-mono text-sm font-semibold opacity-70">osv-scanner</span>
            <a-tag :color="scanCommandState === '执行中' ? 'processing' : 'default'">{{ scanCommandState }}</a-tag>
          </div>
          <div class="flex gap-3">
            <a-button 
              class="btn-refresh" 
              :disabled="!canPreviewScan" 
              @click="osv.previewScan()"
            >
              <template #icon><RefreshCw class="w-3.5 h-3.5 inline-block align-text-bottom mr-1.5 icon-spin" /></template>
              手动刷新
            </a-button>
            <a-button 
              type="primary" 
              class="btn-scan" 
              :disabled="!canRunScan" 
              :loading="osv.loading" 
              @click="() => { activeTab = 'scan'; osv.runScan(); }"
            >
              <template #icon><Play v-if="!osv.loading" class="w-3.5 h-3.5 inline-block align-text-bottom mr-1.5" /></template>
              执行扫描
            </a-button>
          </div>
        </div>
        <div style="padding: 16px; background: #1e1e1e; color: #d4d4d4; font-family: monospace; position: relative;" class="group">
          <div style="display: flex;">
            <span style="color: #569cd6; margin-right: 8px; user-select: none;">$</span>
            <span style="flex: 1; word-break: break-all; white-space: pre-wrap;">
              <template v-if="osv.currentPreview">
                {{ osv.currentPreview.displayCommand }}
                <a-button
                  type="text"
                  class="opacity-0 group-hover:opacity-100 transition-opacity"
                  style="position: absolute; top: 8px; right: 8px; color: #fff;"
                  @click="copyText(osv.currentPreview.displayCommand)"
                >
                  <template #icon><Copy class="w-4 h-4" /></template>
                </a-button>
              </template>
              <span v-else style="color: #6a9955; font-style: italic;">
                # 正在生成对应配置的终端指令...
              </span>
            </span>
          </div>
        </div>
      </a-card>

      <!-- Tabs Content -->
      <!-- Tabs Content -->
      <a-tabs v-model:activeKey="activeTab" type="card">
        <a-tab-pane key="config" tab="扫描配置">
          <div class="flex flex-col gap-6">
            <a-card title="扫描预设" size="small" :bordered="true" style="border-radius: 8px;">
              <a-row :gutter="[16, 16]">
                <a-col :xs="24" :sm="8" v-for="profile in scanProfiles" :key="profile.id">
                  <a-card
                    hoverable
                    size="small"
                    @click="applyScanProfile(profile.id)"
                    :style="{ borderColor: activeScanProfile === profile.id ? token.colorPrimary : undefined, background: activeScanProfile === profile.id ? token.colorPrimaryBg : undefined }"
                  >
                    <template #title>
                      <span :style="{ color: activeScanProfile === profile.id ? token.colorPrimary : undefined }">{{ profile.name }}</span>
                    </template>
                    <template #extra>
                      <a-tag color="blue" v-if="activeScanProfile === profile.id">{{ profile.label }}</a-tag>
                      <a-tag v-else>{{ profile.label }}</a-tag>
                    </template>
                    <p class="text-xs text-gray-500 m-0">{{ profile.description }}</p>
                  </a-card>
                </a-col>
              </a-row>
            </a-card>

            <a-card title="高级设置" size="small" :bordered="true" style="border-radius: 8px;">
              <a-collapse v-model:activeKey="activeCollapse" ghost>
                <a-collapse-panel key="scope" header="扫描范围">
                  <div class="flex items-start gap-2 mb-2">
                    <a-checkbox v-model:checked="osv.options.recursive" />
                    <div>
                      <div class="font-medium">扫描子目录</div>
                      <div class="text-xs text-gray-500">递归扫描项目包源，适合 monorepo 与多语言项目。</div>
                    </div>
                  </div>
                </a-collapse-panel>
                <a-collapse-panel key="resolving" header="包解析">
                  <div class="flex items-start gap-2">
                    <a-checkbox v-model:checked="osv.options.allowNoLockfiles" />
                    <div>
                      <div class="font-medium">允许无锁文件项目</div>
                      <div class="text-xs text-gray-500">允许在没有 lockfile 的情况下进行扫描，但结果可信度可能降低。</div>
                    </div>
                  </div>
                </a-collapse-panel>
              </a-collapse>
            </a-card>
          </div>
        </a-tab-pane>

        <a-tab-pane key="scan" tab="执行与结果">
          <a-card :bordered="true" style="min-height: 300px; border-radius: 8px;">
            <div class="flex items-center justify-between mb-4">
              <h3 class="text-lg font-medium m-0">当前风险</h3>
              <a-tag :color="osv.latestResult ? 'error' : 'default'">{{ currentRiskLabel }}</a-tag>
            </div>
            
            <template v-if="osv.latestResult">
              <p>发现 {{ osv.latestResult.summary.totalVulnerabilities }} 个漏洞。</p>
            </template>
            <div v-else class="text-center py-12 text-gray-400">
              <Shield class="h-10 w-10 mx-auto mb-4 opacity-30" />
              <p>暂无风险数据，请在上方执行扫描</p>
            </div>
          </a-card>
        </a-tab-pane>

        <a-tab-pane key="history" tab="历史记录">
          <a-card :bordered="true" style="min-height: 300px; border-radius: 8px;">
            <div class="text-center py-12 text-gray-400">
              <Clock3 class="h-10 w-10 mx-auto mb-4 opacity-30" />
              <p>历史记录为空</p>
            </div>
          </a-card>
        </a-tab-pane>
      </a-tabs>
    </div>
  </div>
</template>

<style scoped>
.btn-refresh {
  height: 32px;
  padding: 4px 16px;
  border-radius: 6px;
  font-weight: 500;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  transition: all 0.25s cubic-bezier(0.4, 0, 0.2, 1);
  border: 1px solid var(--border-secondary-color);
  background: var(--bg-container-color);
  color: var(--text-color);
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.03);
}
.btn-refresh:hover:not(:disabled) {
  border-color: var(--primary-color);
  color: var(--primary-color);
  background: var(--primary-bg-color);
  transform: translateY(-1px);
}
.btn-refresh:active:not(:disabled) {
  transform: translateY(0);
}
.btn-refresh:hover:not(:disabled) .icon-spin {
  transform: rotate(180deg);
}
.icon-spin {
  transition: transform 0.5s cubic-bezier(0.4, 0, 0.2, 1);
}

.btn-scan {
  height: 32px;
  padding: 4px 16px;
  border-radius: 6px;
  font-weight: 600;
  background: linear-gradient(135deg, var(--primary-color) 0%, #36cfc9 100%) !important;
  border: none !important;
  color: #fff !important;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  transition: all 0.25s cubic-bezier(0.4, 0, 0.2, 1);
  box-shadow: 0 2px 6px rgba(24, 144, 255, 0.25);
  cursor: pointer;
}
.btn-scan:hover:not(:disabled) {
  transform: translateY(-1px) scale(1.02);
  box-shadow: 0 4px 12px rgba(24, 144, 255, 0.4);
  filter: brightness(1.06);
}
.btn-scan:active:not(:disabled) {
  transform: translateY(0) scale(1);
}
.btn-scan:disabled,
.btn-scan[disabled],
.btn-scan.ant-btn-disabled {
  background: var(--bg-disabled-color) !important;
  color: var(--text-disabled-color) !important;
  border: 1px solid var(--border-color) !important;
  box-shadow: none !important;
  cursor: not-allowed !important;
  transform: none !important;
  filter: none !important;
}
.btn-scan:disabled *,
.btn-scan[disabled] *,
.btn-scan.ant-btn-disabled * {
  color: var(--text-disabled-color) !important;
}
</style>
