<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import {
  Copy,
  Download,
  FileJson,
  FileText,
  FolderPlus,
  Play,
  RefreshCw,
  Shield,
  Trash2,
} from '@lucide/vue'
import ToolShell from '../components/ToolShell.vue'
import { useOsvScannerStore } from '../stores/osvScanner'
import type { OsvCommandExecutionRecord, OsvReportFormat, OsvSeverity } from '../api/osvScanner'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'

const osv = useOsvScannerStore()
const newProjectPath = ref('')
const exportFormat = ref<OsvReportFormat>('json')
const exportPath = ref('')
const exportPathEdited = ref(false)
const ignoreReasons = ref<Record<string, string>>({})
const copiedCommandId = ref('')

const selectedProjectName = computed(() => osv.activeProject?.name || '未选择项目')
const canRunScan = computed(() => Boolean(osv.currentPreview && !osv.loading))
const canRunExport = computed(() => Boolean(osv.currentExportPreview && !osv.exporting))

onMounted(() => {
  void osv.load()
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
            <small>{{ osv.installStatus?.version || osv.installStatus?.message || '未读取安装状态' }}</small>
          </div>
        </div>
        <div class="osv-score">
          <span>{{ osv.globalHealthScore ?? '--' }}</span>
          <small>全局健康分</small>
        </div>
        <Button type="button" variant="outline" size="sm" @click="osv.refreshInstallStatus">
          <RefreshCw class="mr-2 h-4 w-4" aria-hidden="true" />
          刷新
        </Button>
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
                <span class="status-pill status-pill--muted">{{ project.healthScore ?? '--' }}</span>
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
            <div class="section-divider">
              <span>扫描参数</span>
            </div>
            <label class="toggle-option">
              <input v-model="osv.options.recursive" type="checkbox" />
              <span>
                <strong>递归扫描</strong>
                <small>--recursive</small>
              </span>
            </label>
            <label class="toggle-option">
              <input v-model="osv.options.allVulns" type="checkbox" />
              <span>
                <strong>显示全部漏洞</strong>
                <small>--all-vulns</small>
              </span>
            </label>
            <label class="field-control" for="osv-config-path">
              <span class="field-label">配置文件</span>
              <Input
                id="osv-config-path"
                v-model="osv.options.configPath"
                type="text"
                placeholder="/path/to/osv-scanner.toml"
              />
            </label>
          </section>
        </div>

        <div class="input-panel">
          <section class="config-section">
            <div class="osv-panel-heading">
              <div>
                <span class="field-label">扫描命令</span>
                <strong>{{ selectedProjectName }}</strong>
              </div>
              <div class="osv-inline-controls">
                <Button type="button" variant="outline" @click="osv.previewScan()">
                  <RefreshCw class="mr-2 h-4 w-4" aria-hidden="true" />
                  预览
                </Button>
                <Button type="button" :disabled="!canRunScan" @click="osv.runScan">
                  <Play class="mr-2 h-4 w-4" aria-hidden="true" />
                  {{ osv.loading ? '扫描中' : '执行' }}
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
          </section>

          <section class="config-section">
            <div class="section-divider">
              <span>报告导出</span>
            </div>
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
              <Button
                type="button"
                variant="outline"
                @click="resetExportPath"
              >
                重置路径
              </Button>
              <Button
                type="button"
                variant="outline"
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
                <Download class="mr-2 h-4 w-4" aria-hidden="true" />
                导出
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
          </section>
        </div>
      </section>

      <p v-if="osv.error" class="status-banner status-banner--error">{{ osv.error }}</p>
      <p v-else-if="osv.notice" class="status-banner">{{ osv.notice }}</p>

      <section v-if="osv.latestResult" class="input-panel">
        <div class="osv-panel-heading">
          <div>
            <span class="field-label">扫描结果</span>
            <strong>{{ osv.latestResult.summary.message }}</strong>
          </div>
          <span class="status-pill status-pill--muted">
            Health {{ osv.latestResult.summary.healthScore }}
          </span>
        </div>

        <div v-if="osv.vulnerabilities.length" class="osv-vulnerability-list">
          <article v-for="finding in osv.vulnerabilities" :key="finding.id" class="osv-vulnerability-row">
            <header>
              <span :class="severityClass(finding.severity)">{{ severityLabel(finding.severity) }}</span>
              <strong>{{ finding.id }}</strong>
              <small>{{ finding.package.name }} {{ finding.package.version || '' }}</small>
            </header>
            <p>{{ finding.summary || finding.details || '暂无摘要' }}</p>
            <small v-if="finding.fixedVersions.length">
              Fixed: {{ finding.fixedVersions.join(', ') }}
            </small>
            <div class="osv-ignore-row">
              <Input
                v-model="ignoreReasons[finding.id]"
                type="text"
                placeholder="忽略原因"
              />
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
        <p v-else class="field-hint">未发现漏洞。</p>
      </section>

      <section class="input-panel">
        <div class="section-divider">
          <span>命令历史</span>
        </div>
        <div v-if="osv.commandHistory.length" class="osv-history-list">
          <article v-for="record in osv.commandHistory.slice().reverse()" :key="record.id" class="osv-history-row">
            <div>
              <strong>{{ record.kind }} · {{ record.status }}</strong>
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
    </div>
  </ToolShell>
</template>
