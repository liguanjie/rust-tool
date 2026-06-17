<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import {
  AlertTriangle,
  ChevronDown,
  CheckCircle2,
  Clock3,
  Copy,
  Download,
  FileJson,
  FileText,
  FolderPlus,
  Loader2,
  Play,
  RefreshCw,
  SearchCheck,
  Shield,
  SlidersHorizontal,
  Trash2,
  X,
} from '@lucide/vue'
import ToolShell from '../components/ToolShell.vue'
import { useOsvScannerStore } from '../stores/osvScanner'
import type {
  OsvCommandExecutionRecord,
  OsvDiagnosticLevel,
  OsvReportFormat,
  OsvSeverity,
  OsvVulnerabilityFinding,
} from '../api/osvScanner'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'

const osv = useOsvScannerStore()
const newProjectPath = ref('')
const exportFormat = ref<OsvReportFormat>('json')
const exportPath = ref('')
const exportPathEdited = ref(false)
const ignoreReasons = ref<Record<string, string>>({})
const copiedCommandId = ref('')
const advancedOpen = ref(false)
const now = ref(Date.now())
let operationTimer: number | undefined

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

const selectedProjectName = computed(() => osv.activeProject?.name || '未选择项目')
const canPreviewScan = computed(() =>
  Boolean(osv.activeProjectPath && !osv.loading && !osv.exporting && !osv.diagnosing),
)
const canRunScan = computed(() =>
  Boolean(osv.currentPreview && !osv.loading && !osv.exporting && !osv.diagnosing),
)
const canPreviewExport = computed(() =>
  Boolean(osv.hasCurrentScanResult && exportPath.value && !osv.loading && !osv.exporting),
)
const canRunExport = computed(() =>
  Boolean(osv.currentExportPreview && osv.hasCurrentScanResult && !osv.loading && !osv.exporting),
)
const installVersionLine = computed(() => {
  const version = osv.installStatus?.version || osv.installStatus?.message || '未读取安装状态'
  return version.split('\n')[0]
})
const globalHealthLabel = computed(() => {
  if (typeof osv.globalHealthScore !== 'number') return '未扫描'
  if (osv.globalHealthScore >= 90) return '健康'
  if (osv.globalHealthScore >= 70) return '可关注'
  if (osv.globalHealthScore >= 40) return '风险较高'
  return '高风险'
})
const globalHealthClass = computed(() => healthPillClass(osv.globalHealthScore))
const prioritizedVulnerabilities = computed(() =>
  [...osv.vulnerabilities].sort(
    (left, right) => severityRank(left.severity) - severityRank(right.severity),
  ),
)
const currentRiskLabel = computed(() => {
  const summary = osv.latestResult?.summary
  if (!summary) return '等待扫描'
  if (summary.totalVulnerabilities === 0) return '暂未发现已知漏洞'
  if (summary.severityCounts.critical > 0) return '优先处理 Critical'
  if (summary.severityCounts.high > 0) return '优先处理 High'
  return '存在中低风险'
})
const currentRiskClass = computed(() => healthPillClass(osv.latestResult?.summary.healthScore))
const diagnosticSummary = computed(() => {
  if (!osv.activeProjectPath) return '未选择项目'
  if (osv.diagnosing) return '诊断中'
  if (!osv.diagnostic) return '等待诊断'
  if (osv.diagnostic.messages.some((message) => message.level === 'error')) return '需要处理'
  if (osv.diagnostic.messages.some((message) => message.level === 'warning')) return '有提醒'
  if (osv.diagnostic.packageSources.length) return `${osv.diagnostic.packageSources.length} 个包源`
  return '未发现包源'
})
const diagnosticSummaryClass = computed(() => {
  if (!osv.diagnostic) return 'status-pill status-pill--muted'
  if (osv.diagnostic.messages.some((message) => message.level === 'error')) {
    return 'status-pill status-pill--danger'
  }
  if (osv.diagnostic.messages.some((message) => message.level === 'warning')) {
    return 'status-pill status-pill--warn'
  }
  return 'status-pill status-pill--good'
})
const scanCommandState = computed(() => {
  if (osv.loading) return '执行中'
  if (osv.diagnostic && !osv.diagnostic.canScan) return '诊断阻断'
  if (osv.currentPreview) return '已预览'
  if (osv.diagnosing) return '诊断中'
  return '待预览'
})
const scanCommandStateClass = computed(() => {
  if (osv.loading) return 'status-pill status-pill--warn'
  if (osv.diagnostic && !osv.diagnostic.canScan) return 'status-pill status-pill--danger'
  if (osv.currentPreview) return 'status-pill status-pill--good'
  return 'status-pill status-pill--muted'
})
const operationStatusLabel = computed(() => {
  if (osv.operation.status === 'running') return '进行中'
  if (osv.operation.status === 'succeeded') return '已完成'
  if (osv.operation.status === 'failed') return '失败'
  return '空闲'
})
const operationStatusClass = computed(() => {
  if (osv.operation.status === 'running') return 'status-pill status-pill--warn'
  if (osv.operation.status === 'succeeded') return 'status-pill status-pill--good'
  if (osv.operation.status === 'failed') return 'status-pill status-pill--danger'
  return 'status-pill status-pill--muted'
})
const operationElapsedLabel = computed(() => {
  if (!osv.operation.startedAt) return ''
  const finishedAt = osv.operation.finishedAt || now.value
  const seconds = Math.max(0, Math.round((finishedAt - osv.operation.startedAt) / 1000))
  if (seconds < 60) return `${seconds}s`
  return `${Math.floor(seconds / 60)}m ${seconds % 60}s`
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
const lockfilesText = computed({
  get: () => osv.options.lockfiles.join('\n'),
  set: (value) => {
    osv.options.lockfiles = splitLines(String(value))
  },
})
const excludesText = computed({
  get: () => osv.options.experimentalExcludes.join('\n'),
  set: (value) => {
    osv.options.experimentalExcludes = splitLines(String(value))
  },
})
const advancedArgsText = computed({
  get: () => osv.options.advancedArgs.join('\n'),
  set: (value) => {
    osv.options.advancedArgs = splitLines(String(value))
  },
})
const latestScanLabel = computed(() => {
  const latest = osv.projects
    .map((project) => Number(project.lastScanned))
    .filter((value) => Number.isFinite(value) && value > 0)
    .sort((a, b) => b - a)[0]
  return latest ? new Date(latest).toLocaleString() : '未扫描'
})

onMounted(() => {
  void osv.load()
  operationTimer = window.setInterval(() => {
    now.value = Date.now()
  }, 1000)
})

onUnmounted(() => {
  if (operationTimer) window.clearInterval(operationTimer)
})

watch(
  () => [osv.activeProjectPath, exportFormat.value] as const,
  async () => {
    osv.invalidateExportPreview()
    if (exportPathEdited.value) return
    exportPath.value = await osv.suggestedReportPath(exportFormat.value)
  },
  { immediate: true },
)

watch(
  () => osv.options,
  () => {
    osv.invalidateCommandPreviews()
    osv.clearDiagnostic()
  },
  { deep: true },
)

async function addProject() {
  await osv.addProject(newProjectPath.value)
  newProjectPath.value = ''
}

async function chooseDirectory() {
  const tauriCore = await import('@tauri-apps/api/core').catch(() => null)
  if (!tauriCore?.isTauri()) return

  const dialog = await import('@tauri-apps/plugin-dialog')
  const selected = await dialog.open({
    directory: true,
    multiple: false,
  })
  if (typeof selected === 'string') {
    newProjectPath.value = selected
    await addProject()
  }
}

async function copyText(text: string, id = 'command') {
  await navigator.clipboard.writeText(text)
  copiedCommandId.value = id
  window.setTimeout(() => {
    if (copiedCommandId.value === id) copiedCommandId.value = ''
  }, 1400)
}

function updateExportPath(value: string | number) {
  exportPathEdited.value = true
  exportPath.value = String(value)
  osv.invalidateExportPreview()
}

async function resetExportPath() {
  exportPathEdited.value = false
  exportPath.value = await osv.suggestedReportPath(exportFormat.value)
  osv.invalidateExportPreview()
}

function severityLabel(severity: OsvSeverity) {
  const labels: Record<OsvSeverity, string> = {
    critical: 'Critical',
    high: 'High',
    medium: 'Medium',
    low: 'Low',
    unknown: 'Unknown',
  }
  return labels[severity]
}

function severityClass(severity: OsvSeverity) {
  return `osv-severity osv-severity--${severity}`
}

function severityRank(severity: OsvSeverity) {
  const ranks: Record<OsvSeverity, number> = {
    critical: 0,
    high: 1,
    medium: 2,
    low: 3,
    unknown: 4,
  }
  return ranks[severity]
}

function healthPillClass(score?: number) {
  if (typeof score !== 'number') return 'status-pill status-pill--muted'
  if (score >= 90) return 'status-pill status-pill--good'
  if (score >= 70) return 'status-pill status-pill--warn'
  return 'status-pill status-pill--danger'
}

function projectHealthLabel(score?: number) {
  if (typeof score !== 'number') return '未扫描'
  return `${score}`
}

function findingPackageLabel(finding: OsvVulnerabilityFinding) {
  const version = finding.package.version ? ` ${finding.package.version}` : ''
  return `${finding.package.name}${version}`
}

function findingFixLabel(finding: OsvVulnerabilityFinding) {
  return finding.fixedVersions.length ? finding.fixedVersions.join(', ') : '暂无直接修复版本'
}

function diagnosticLevelLabel(level: OsvDiagnosticLevel) {
  const labels: Record<OsvDiagnosticLevel, string> = {
    info: '提示',
    warning: '提醒',
    error: '阻断',
  }
  return labels[level]
}

function diagnosticLevelClass(level: OsvDiagnosticLevel) {
  return `osv-diagnostic-item osv-diagnostic-item--${level}`
}

function splitLines(value: string) {
  return value
    .split('\n')
    .map((line) => line.trim())
    .filter(Boolean)
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

function scanProfileClass(profileId: ScanProfileId) {
  return {
    'osv-profile-option': true,
    'osv-profile-option--active': activeScanProfile.value === profileId,
  }
}

function formatRecordKind(kind: OsvCommandExecutionRecord['kind']) {
  const labels: Record<OsvCommandExecutionRecord['kind'], string> = {
    scan: '扫描',
    export: '导出',
    fix: '修复',
  }
  return labels[kind]
}

function formatRecordStatus(record: OsvCommandExecutionRecord) {
  if (record.status === 'failed') return '失败'
  if (record.exitCode && record.exitCode !== 0) return '已完成 · 有发现'
  return '已完成'
}

function recordStatusClass(record: OsvCommandExecutionRecord) {
  if (record.status === 'failed') return 'status-pill status-pill--danger'
  if (record.exitCode && record.exitCode !== 0) return 'status-pill status-pill--warn'
  return 'status-pill status-pill--good'
}

function formatCommandTime(record: OsvCommandExecutionRecord) {
  const millis = Number(record.finishedAt || record.startedAt)
  if (!Number.isFinite(millis) || millis <= 0) return '未知时间'
  return new Date(millis).toLocaleString()
}
</script>

<template>
  <ToolShell
    title="OSV 漏洞管理"
    description="管理本机项目的依赖漏洞扫描、报告导出和命令审计。"
    eyebrow="供应链安全"
    fluid
  >
    <div class="osv-layout">
      <section class="input-panel osv-status-panel">
        <div class="osv-status-main">
          <span class="service-icon">
            <Shield class="h-5 w-5" aria-hidden="true" />
          </span>
          <div>
            <strong>{{ osv.installStatus?.installed ? 'osv-scanner 已就绪' : '等待检测' }}</strong>
            <small>{{ installVersionLine }}</small>
          </div>
        </div>
        <div class="osv-status-metrics">
          <div class="osv-score">
            <span>{{ osv.globalHealthScore ?? '--' }}</span>
            <small>全局健康分</small>
          </div>
          <div class="osv-status-fact">
            <strong>{{ osv.projects.length }}</strong>
            <small>监控项目</small>
          </div>
          <div class="osv-status-fact">
            <strong>{{ latestScanLabel }}</strong>
            <small>最近扫描</small>
          </div>
          <span :class="globalHealthClass">{{ globalHealthLabel }}</span>
        </div>
        <Button type="button" variant="outline" size="sm" @click="osv.refreshInstallStatus">
          <RefreshCw class="mr-2 h-4 w-4" aria-hidden="true" />
          刷新
        </Button>
      </section>

      <section
        v-if="osv.operation.status !== 'idle'"
        class="input-panel osv-task-panel"
        :class="`osv-task-panel--${osv.operation.status}`"
        role="status"
        aria-live="polite"
      >
        <div class="osv-task-icon" aria-hidden="true">
          <Loader2 v-if="osv.operation.status === 'running'" class="h-5 w-5 animate-spin" />
          <CheckCircle2 v-else-if="osv.operation.status === 'succeeded'" class="h-5 w-5" />
          <AlertTriangle v-else class="h-5 w-5" />
        </div>
        <div class="osv-task-copy">
          <span class="field-label">当前任务</span>
          <strong>{{ osv.operation.title }}</strong>
          <small>{{ osv.operation.message }}</small>
          <code v-if="osv.operation.command">{{ osv.operation.command }}</code>
        </div>
        <div class="osv-task-meta">
          <span :class="operationStatusClass">{{ operationStatusLabel }}</span>
          <small v-if="operationElapsedLabel">
            <Clock3 class="h-3.5 w-3.5" aria-hidden="true" />
            {{ operationElapsedLabel }}
          </small>
          <Button
            v-if="osv.operation.status !== 'running'"
            type="button"
            variant="ghost"
            size="sm"
            aria-label="关闭任务反馈"
            @click="osv.dismissOperation()"
          >
            <X class="h-4 w-4" aria-hidden="true" />
          </Button>
        </div>
        <div v-if="osv.operation.status === 'running'" class="osv-task-progress" aria-hidden="true" />
      </section>

      <section class="tool-grid">
        <div class="input-panel">
          <section class="config-section">
            <label class="field-label" for="osv-project-path">项目路径</label>
            <div class="osv-inline-controls">
              <Input
                id="osv-project-path"
                v-model="newProjectPath"
                type="text"
                placeholder="/Users/ben/project"
                @keyup.enter="addProject"
              />
              <Button type="button" variant="outline" @click="chooseDirectory">
                <FolderPlus class="mr-2 h-4 w-4" aria-hidden="true" />
                选择
              </Button>
              <Button type="button" @click="addProject">添加</Button>
            </div>
          </section>

          <section class="config-section">
            <div class="section-divider">
              <span>监控项目</span>
            </div>
            <div v-if="osv.projects.length" class="osv-project-list">
              <button
                v-for="project in osv.projects"
                :key="project.path"
                type="button"
                class="osv-project-row"
                :class="{ 'osv-project-row--active': project.path === osv.activeProjectPath }"
                @click="osv.selectProject(project.path)"
              >
                <span>
                  <strong>{{ project.name }}</strong>
                  <small>{{ project.path }}</small>
                </span>
                <span :class="healthPillClass(project.healthScore)">
                  {{ projectHealthLabel(project.healthScore) }}
                </span>
                <Trash2
                  class="h-4 w-4"
                  aria-label="移除项目"
                  @click.stop="osv.removeProject(project.path)"
                />
              </button>
            </div>
            <p v-else class="field-hint">暂无项目。</p>
          </section>

          <section class="config-section">
            <div class="osv-panel-heading">
              <div>
                <span class="field-label">扫描诊断</span>
                <strong>{{ selectedProjectName }}</strong>
              </div>
              <div class="osv-inline-controls">
                <span :class="diagnosticSummaryClass">{{ diagnosticSummary }}</span>
                <Button
                  type="button"
                  variant="outline"
                  :disabled="!osv.activeProjectPath || osv.diagnosing"
                  @click="osv.diagnoseActiveProject()"
                >
                  <SearchCheck class="mr-2 h-4 w-4" aria-hidden="true" />
                  {{ osv.diagnosing ? '诊断中' : '诊断' }}
                </Button>
              </div>
            </div>

            <template v-if="osv.diagnostic">
              <div v-if="osv.diagnostic.packageSources.length" class="osv-source-list">
                <span
                  v-for="source in osv.diagnostic.packageSources"
                  :key="`${source.path}:${source.explicit}`"
                  class="osv-source-chip"
                >
                  <strong>{{ source.ecosystem }}</strong>
                  <small>{{ source.path }}</small>
                </span>
              </div>
              <div v-if="osv.diagnostic.messages.length" class="osv-diagnostic-list">
                <article
                  v-for="(message, index) in osv.diagnostic.messages"
                  :key="`${message.code}:${index}`"
                  :class="diagnosticLevelClass(message.level)"
                >
                  <span>{{ diagnosticLevelLabel(message.level) }}</span>
                  <div>
                    <strong>{{ message.message }}</strong>
                    <small v-if="message.suggestion">{{ message.suggestion }}</small>
                  </div>
                </article>
              </div>
              <p v-else class="field-hint">诊断通过。</p>
            </template>
            <p v-else class="field-hint">等待诊断。</p>
          </section>

          <section class="config-section">
            <div class="section-divider">
              <span>扫描参数</span>
            </div>

            <div class="osv-profile-grid" role="group" aria-label="扫描预设">
              <button
                v-for="profile in scanProfiles"
                :key="profile.id"
                type="button"
                :class="scanProfileClass(profile.id)"
                @click="applyScanProfile(profile.id)"
              >
                <span>
                  <strong>{{ profile.name }}</strong>
                  <small>{{ profile.description }}</small>
                </span>
                <em>{{ profile.label }}</em>
              </button>
            </div>

            <div class="osv-option-grid">
              <label class="toggle-option toggle-option--card">
                <input v-model="osv.options.recursive" type="checkbox" />
                <span>
                  <strong>扫描子目录</strong>
                  <small><code>--recursive</code> 覆盖 monorepo 与多语言项目。</small>
                </span>
              </label>
              <label class="toggle-option toggle-option--card">
                <input v-model="osv.options.allVulns" type="checkbox" />
                <span>
                  <strong>显示完整漏洞列表</strong>
                  <small><code>--all-vulns</code> 适合审计，噪声会更高。</small>
                </span>
              </label>
            </div>

            <label class="field-control" for="osv-config-path">
              <span class="field-label">配置文件</span>
              <Input
                id="osv-config-path"
                v-model="osv.options.configPath"
                type="text"
                placeholder="/path/to/osv-scanner.toml"
              />
              <small class="field-hint">留空时使用项目内默认配置；填入路径后会在扫描前诊断是否存在。</small>
            </label>
            <button
              type="button"
              class="osv-advanced-toggle"
              :class="{ 'osv-advanced-toggle--open': advancedOpen }"
              :aria-expanded="advancedOpen"
              @click="advancedOpen = !advancedOpen"
            >
              <span>
                <SlidersHorizontal class="h-4 w-4" aria-hidden="true" />
                高级参数
              </span>
              <ChevronDown class="h-4 w-4" aria-hidden="true" />
            </button>
            <div v-if="advancedOpen" class="osv-advanced-groups">
              <div class="osv-advanced-block">
                <header>
                  <strong>扫描范围</strong>
                  <small>决定哪些目录会进入扫描。</small>
                </header>
                <div class="osv-option-grid">
                  <label class="toggle-option toggle-option--card">
                    <input v-model="osv.options.noIgnore" type="checkbox" />
                    <span>
                      <strong>纳入忽略路径</strong>
                      <small><code>--no-ignore</code> 包含 .gitignore 中的路径。</small>
                    </span>
                  </label>
                  <label class="toggle-option toggle-option--card">
                    <input v-model="osv.options.includeGitRoot" type="checkbox" />
                    <span>
                      <strong>包含 Git 根目录</strong>
                      <small><code>--include-git-root</code> 适合从子目录发起扫描。</small>
                    </span>
                  </label>
                </div>
                <label class="field-control" for="osv-excludes">
                  <span class="field-label">排除路径</span>
                  <textarea
                    id="osv-excludes"
                    v-model="excludesText"
                    class="text-input osv-textarea"
                    spellcheck="false"
                    placeholder="node_modules&#10;dist"
                  />
                  <small class="field-hint">每行一个相对路径，适合跳过构建产物或大型缓存目录。</small>
                </label>
              </div>

              <div class="osv-advanced-block">
                <header>
                  <strong>包解析</strong>
                  <small>影响锁文件、依赖图和结果完整度。</small>
                </header>
                <div class="osv-option-grid">
                  <label class="toggle-option toggle-option--card">
                    <input v-model="osv.options.allowNoLockfiles" type="checkbox" />
                    <span>
                      <strong>允许无锁文件项目</strong>
                      <small><code>--allow-no-lockfiles</code> 结果可信度可能降低。</small>
                    </span>
                  </label>
                  <label class="toggle-option toggle-option--card">
                    <input v-model="osv.options.allPackages" type="checkbox" />
                    <span>
                      <strong>输出全部包</strong>
                      <small><code>--all-packages</code> 用于包清单盘点。</small>
                    </span>
                  </label>
                  <label class="toggle-option toggle-option--card">
                    <input v-model="osv.options.noResolve" type="checkbox" />
                    <span>
                      <strong>跳过传递依赖解析</strong>
                      <small><code>--no-resolve</code> 更快，但覆盖更窄。</small>
                    </span>
                  </label>
                </div>
                <label class="field-control" for="osv-lockfiles">
                  <span class="field-label">指定 lockfile</span>
                  <textarea
                    id="osv-lockfiles"
                    v-model="lockfilesText"
                    class="text-input osv-textarea"
                    spellcheck="false"
                    placeholder="Cargo.lock&#10;package-lock.json"
                  />
                  <small class="field-hint">每行一个相对路径；填写后诊断会检查路径是否存在。</small>
                </label>
              </div>

              <div class="osv-advanced-block">
                <header>
                  <strong>网络与数据源</strong>
                  <small>用于网络受限或离线复核。</small>
                </header>
                <div class="osv-option-grid">
                  <label class="toggle-option toggle-option--card">
                    <input v-model="osv.options.offline" type="checkbox" />
                    <span>
                      <strong>离线执行</strong>
                      <small><code>--offline</code> 不访问外部网络。</small>
                    </span>
                  </label>
                  <label class="toggle-option toggle-option--card">
                    <input v-model="osv.options.offlineVulnerabilities" type="checkbox" />
                    <span>
                      <strong>使用本地漏洞库</strong>
                      <small><code>--offline-vulnerabilities</code> 需要预置 OSV 数据库。</small>
                    </span>
                  </label>
                </div>
              </div>

              <div class="osv-advanced-block">
                <header>
                  <strong>扩展参数</strong>
                  <small>仅接受后端 allowlist 中的安全参数。</small>
                </header>
                <label class="field-control" for="osv-advanced-args">
                  <span class="field-label">额外 allowlist 参数</span>
                  <textarea
                    id="osv-advanced-args"
                    v-model="advancedArgsText"
                    class="text-input osv-textarea"
                    spellcheck="false"
                    placeholder="--offline-vulnerabilities"
                  />
                  <small class="field-hint">每行一个参数，生成命令前会由核心层校验。</small>
                </label>
              </div>
            </div>
          </section>
        </div>

        <div class="input-panel osv-workspace-panel">
          <section class="config-section">
            <div class="osv-panel-heading">
              <div>
                <span class="field-label">扫描命令</span>
                <strong>{{ selectedProjectName }}</strong>
              </div>
              <div class="osv-inline-controls">
                <span :class="scanCommandStateClass">{{ scanCommandState }}</span>
                <Button
                  type="button"
                  variant="outline"
                  :disabled="!canPreviewScan"
                  @click="osv.previewScan()"
                >
                  <RefreshCw class="mr-2 h-4 w-4" aria-hidden="true" />
                  预览命令
                </Button>
                <Button type="button" :disabled="!canRunScan" @click="osv.runScan">
                  <Loader2 v-if="osv.loading" class="mr-2 h-4 w-4 animate-spin" aria-hidden="true" />
                  <Play v-else class="mr-2 h-4 w-4" aria-hidden="true" />
                  {{ osv.loading ? '扫描中' : '执行扫描' }}
                </Button>
              </div>
            </div>

            <div v-if="osv.currentPreview" class="osv-command-box">
              <code>{{ osv.currentPreview.displayCommand }}</code>
              <Button
                type="button"
                variant="ghost"
                size="sm"
                aria-label="复制扫描命令"
                @click="copyText(osv.currentPreview.displayCommand)"
              >
                <Copy class="h-4 w-4" aria-hidden="true" />
              </Button>
            </div>
            <div v-else class="osv-command-empty">
              <strong>命令尚未生成</strong>
              <small>配置变更后需要重新预览，执行按钮才会启用。</small>
            </div>
            <div v-if="osv.currentPreview?.warnings.length" class="osv-command-warnings">
              <AlertTriangle class="h-4 w-4" aria-hidden="true" />
              <span>{{ osv.currentPreview.warnings.join(' ') }}</span>
            </div>
          </section>

          <section class="config-section osv-result-workspace">
            <div class="osv-panel-heading">
              <div>
                <span class="field-label">当前风险</span>
                <strong>{{ currentRiskLabel }}</strong>
              </div>
              <span :class="currentRiskClass">
                {{ osv.latestResult ? `Health ${osv.latestResult.summary.healthScore}` : '未扫描' }}
              </span>
            </div>

            <template v-if="osv.latestResult">
              <div class="osv-result-summary-grid">
                <div>
                  <strong>{{ osv.latestResult.summary.totalVulnerabilities }}</strong>
                  <small>漏洞总数</small>
                </div>
                <span class="osv-severity osv-severity--critical">
                  Critical {{ osv.latestResult.summary.severityCounts.critical }}
                </span>
                <span class="osv-severity osv-severity--high">
                  High {{ osv.latestResult.summary.severityCounts.high }}
                </span>
                <span class="osv-severity osv-severity--medium">
                  Medium {{ osv.latestResult.summary.severityCounts.medium }}
                </span>
                <span class="osv-severity osv-severity--low">
                  Low {{ osv.latestResult.summary.severityCounts.low }}
                </span>
              </div>

              <div class="osv-result-actions">
                <div class="mode-row" role="radiogroup" aria-label="报告格式">
                  <label class="mode-option">
                    <input v-model="exportFormat" type="radio" value="json" />
                    <span><FileJson class="h-4 w-4" aria-hidden="true" />JSON</span>
                  </label>
                  <label class="mode-option">
                    <input v-model="exportFormat" type="radio" value="html" />
                    <span><FileText class="h-4 w-4" aria-hidden="true" />HTML</span>
                  </label>
                </div>
                <label class="field-control" for="osv-export-path">
                  <span class="field-label">导出路径</span>
                  <Input
                    id="osv-export-path"
                    :model-value="exportPath"
                    type="text"
                    placeholder="/private/tmp/rusttool-osv-report.json"
                    @update:model-value="updateExportPath"
                  />
                </label>
                <div class="osv-inline-controls">
                  <Button type="button" variant="outline" @click="resetExportPath">
                    重置路径
                  </Button>
                  <Button
                    type="button"
                    variant="outline"
                    :disabled="!canPreviewExport"
                    @click="osv.previewExport(exportFormat, exportPath)"
                  >
                    <RefreshCw class="mr-2 h-4 w-4" aria-hidden="true" />
                    预览导出
                  </Button>
                  <Button
                    type="button"
                    :disabled="!canRunExport"
                    @click="osv.runExport(exportFormat, exportPath)"
                  >
                    <Loader2 v-if="osv.exporting" class="mr-2 h-4 w-4 animate-spin" aria-hidden="true" />
                    <Download v-else class="mr-2 h-4 w-4" aria-hidden="true" />
                    {{ osv.exporting ? '导出中' : '导出' }}
                  </Button>
                </div>
                <div v-if="osv.currentExportPreview" class="osv-command-box">
                  <code>{{ osv.currentExportPreview.displayCommand }}</code>
                  <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    aria-label="复制导出命令"
                    @click="copyText(osv.currentExportPreview.displayCommand, 'export')"
                  >
                    <Copy class="h-4 w-4" aria-hidden="true" />
                  </Button>
                </div>
              </div>

              <div v-if="prioritizedVulnerabilities.length" class="osv-vulnerability-list">
                <article
                  v-for="finding in prioritizedVulnerabilities"
                  :key="finding.id"
                  class="osv-vulnerability-row"
                >
                  <header>
                    <span :class="severityClass(finding.severity)">{{ severityLabel(finding.severity) }}</span>
                    <strong>{{ finding.id }}</strong>
                    <small>{{ findingPackageLabel(finding) }}</small>
                  </header>
                  <p>{{ finding.summary || finding.details || '暂无摘要' }}</p>
                  <small>Fixed: {{ findingFixLabel(finding) }}</small>
                  <div class="osv-ignore-row">
                    <Input
                      v-model="ignoreReasons[finding.id]"
                      type="text"
                      placeholder="忽略原因"
                    />
                    <Button
                      type="button"
                      variant="outline"
                      aria-label="复制漏洞 ID"
                      @click="copyText(finding.id, finding.id)"
                    >
                      <Copy class="mr-2 h-4 w-4" aria-hidden="true" />
                      复制 ID
                    </Button>
                    <Button
                      type="button"
                      variant="outline"
                      @click="osv.ignoreFinding(finding, ignoreReasons[finding.id] || '')"
                    >
                      忽略
                    </Button>
                  </div>
                </article>
              </div>
              <p v-else class="osv-empty-result">未发现已知漏洞。</p>
            </template>

            <div v-else class="osv-empty-result">
              <strong>等待扫描结果</strong>
              <small>当前项目还没有完成扫描。</small>
            </div>
          </section>
        </div>
      </section>

      <p v-if="osv.error" class="status-banner status-banner--error">{{ osv.error }}</p>
      <p v-else-if="osv.notice" class="status-banner">{{ osv.notice }}</p>

      <section class="input-panel">
        <div class="section-divider">
          <span>命令历史</span>
        </div>
        <div v-if="osv.commandHistory.length" class="osv-history-list">
          <article v-for="record in osv.commandHistory.slice().reverse()" :key="record.id" class="osv-history-row">
            <div>
              <div class="osv-history-title">
                <strong>{{ formatRecordKind(record.kind) }}</strong>
                <span :class="recordStatusClass(record)">{{ formatRecordStatus(record) }}</span>
              </div>
              <small>{{ formatCommandTime(record) }} · exit {{ record.exitCode ?? 'n/a' }}</small>
              <code>{{ record.displayCommand }}</code>
            </div>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              aria-label="复制历史命令"
              @click="copyText(record.displayCommand, record.id)"
            >
              <Copy class="h-4 w-4" aria-hidden="true" />
            </Button>
          </article>
        </div>
        <p v-else class="field-hint">暂无命令历史。</p>
      </section>

      <div v-if="osv.loading || osv.exporting" class="osv-run-overlay" role="status" aria-live="assertive">
        <div class="osv-run-dialog">
          <Loader2 class="h-6 w-6 animate-spin" aria-hidden="true" />
          <div>
            <strong>{{ osv.operation.title }}</strong>
            <small>{{ osv.operation.message }}</small>
          </div>
          <code v-if="osv.operation.command">{{ osv.operation.command }}</code>
        </div>
      </div>
    </div>
  </ToolShell>
</template>
