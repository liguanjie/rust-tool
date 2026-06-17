<script setup lang="ts">
import { computed, defineAsyncComponent, onMounted, ref, watch } from 'vue'
import type { Component } from 'vue'
import { RouterLink } from 'vue-router'
import {
  Box,
  Cable,
  CheckCircle2,
  Circle,
  Copy,
  FolderOpen,
  FolderSearch,
  History,
  Layers3,
  Loader2,
  Package,
  Play,
  RotateCcw,
  Search,
  Settings2,
  Sparkles,
  Terminal,
  Trash2,
  Wrench,
  XCircle,
} from '@lucide/vue'
import ToolShell from '../components/ToolShell.vue'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from '@/components/ui/dialog'

const VlessToMihomo = defineAsyncComponent(() => import('./VlessToMihomo.vue'))

interface ScriptInfo {
  name: string
  path: string
}

interface ExecutionResult {
  stdout: string
  stderr: string
  exit_code: number
  success: boolean
}

interface HistoryRecord {
  id: string
  timestamp: number
  scriptName: string
  args: string
  exit_code: number
  success: boolean
  stdout: string
  stderr: string
}

interface ScriptMeta {
  title: string
  desc: string
  icon: Component
  badge: string
}

const SCRIPT_DICT: Record<string, ScriptMeta> = {
  'bundle:install-to-project': {
    title: 'AI 技能安装向导',
    desc: '为目标项目注入团队规范与核心自动化能力。',
    icon: Package,
    badge: '批量安装',
  },
  '@tool:vless': {
    title: 'VLESS 转 Mihomo',
    desc: '将 3x-ui VLESS 链接转换为 Clash Party/Mihomo YAML。',
    icon: Cable,
    badge: '内置工具',
  },
}

const projectSkillOptions: Array<{
  value: string
  title: string
  desc: string
  icon: Component
}> = [
  {
    value: 'general',
    title: '黄金手册',
    desc: '通用团队编码规范',
    icon: CheckCircle2,
  },
  {
    value: 'scm',
    title: '供应链 SCM',
    desc: '供应链管理业务模板',
    icon: Package,
  },
  {
    value: 'scf',
    title: '金融 SCF',
    desc: '供应链金融业务模板',
    icon: Box,
  },
  {
    value: 'b2b',
    title: '供应链 B2B',
    desc: 'B2B 交易平台业务模板',
    icon: Layers3,
  },
]

function getScriptMeta(scriptName: string): ScriptMeta {
  return SCRIPT_DICT[scriptName] || {
    title: scriptName,
    desc: '系统级自动化执行脚本。',
    icon: Wrench,
    badge: '本地脚本',
  }
}

const dir = ref('/Users/ben/work/99_codex')
const scripts = ref<ScriptInfo[]>([])
const selectedScript = ref<ScriptInfo | null>(null)
const bundleSelection = ref<string[]>([])
const searchQuery = ref('')
const scriptArgs = ref('')
const projectDir = ref('')
const projectSkill = ref('general')
const isRunning = ref(false)
const errorMsg = ref('')
const executionHistory = ref<HistoryRecord[]>([])
const historySearchQuery = ref('')

watch(selectedScript, (newVal) => {
  if (newVal && newVal.name === 'bundle:install-to-project') {
    bundleSelection.value = newVal.path.split('|||')
  } else {
    bundleSelection.value = []
  }
})

const filteredScripts = computed(() => {
  if (!searchQuery.value) return scripts.value
  const lowerQ = searchQuery.value.toLowerCase()
  return scripts.value.filter((script) => {
    const meta = getScriptMeta(script.name)
    return script.name.toLowerCase().includes(lowerQ) || meta.title.toLowerCase().includes(lowerQ)
  })
})

const filteredLocalScripts = computed(() =>
  filteredScripts.value.filter((script) => script.path !== 'internal'),
)

const filteredInternalScripts = computed(() =>
  filteredScripts.value.filter((script) => script.path === 'internal'),
)

const filteredHistory = computed(() => {
  if (!historySearchQuery.value) return executionHistory.value
  const lowerQ = historySearchQuery.value.toLowerCase()
  return executionHistory.value.filter((record) => {
    const meta = getScriptMeta(record.scriptName)
    return (
      record.scriptName.toLowerCase().includes(lowerQ) ||
      meta.title.toLowerCase().includes(lowerQ) ||
      record.args.toLowerCase().includes(lowerQ) ||
      record.stdout.toLowerCase().includes(lowerQ) ||
      record.stderr.toLowerCase().includes(lowerQ)
    )
  })
})

const selectedMeta = computed(() =>
  selectedScript.value ? getScriptMeta(selectedScript.value.name) : null,
)

const totalTaskCount = computed(() => scripts.value.length)
const successHistoryCount = computed(() => executionHistory.value.filter((record) => record.success).length)
const failedHistoryCount = computed(() => executionHistory.value.filter((record) => !record.success).length)
const latestHistory = computed(() => executionHistory.value[0] ?? null)
const selectedPathLabel = computed(() => {
  if (!selectedScript.value) return '未选择任务'
  if (selectedScript.value.path === 'internal') return '内置工具'
  if (selectedScript.value.name === 'bundle:install-to-project') return dir.value
  return selectedScript.value.path.replace(`/${selectedScript.value.name}`, '')
})
const runButtonLabel = computed(() => {
  if (isRunning.value) return '执行中'
  if (selectedScript.value?.name === 'bundle:install-to-project') return '安装 AI 技能'
  return '执行任务'
})
const runStateLabel = computed(() => {
  if (isRunning.value) return '运行中'
  if (!selectedScript.value) return '待选择'
  return '已就绪'
})
const runStateClass = computed(() => {
  if (isRunning.value) return 'status-pill status-pill--warn'
  if (selectedScript.value) return 'status-pill status-pill--good'
  return 'status-pill status-pill--muted'
})

onMounted(() => {
  const savedHistory = localStorage.getItem('rusttool:codex:history')
  if (savedHistory) {
    try {
      executionHistory.value = JSON.parse(savedHistory)
    } catch (error) {
      console.error('历史解析失败', error)
    }
  }
  void fetchScripts()
})

watch(
  executionHistory,
  (newHistory) => {
    localStorage.setItem('rusttool:codex:history', JSON.stringify(newHistory))
  },
  { deep: true },
)

async function pickDirectory(targetRef: 'dir' | 'projectDir') {
  try {
    const tauriCore = await import('@tauri-apps/api/core').catch(() => null)
    if (tauriCore && tauriCore.isTauri()) {
      const { open } = await import('@tauri-apps/plugin-dialog')
      const defaultPath = targetRef === 'dir' ? dir.value : projectDir.value
      const selected = await open({ directory: true, defaultPath: defaultPath || undefined })
      if (selected && typeof selected === 'string') {
        if (targetRef === 'dir') {
          dir.value = selected
          void fetchScripts()
        }
        if (targetRef === 'projectDir') projectDir.value = selected
      }
    } else {
      errorMsg.value = '目录浏览功能仅在桌面客户端可用，请手动输入路径。'
    }
  } catch (caught) {
    const message = caught instanceof Error ? caught.message : String(caught)
    errorMsg.value = `无法打开文件夹选择器：${message}`
  }
}

async function fetchScripts() {
  errorMsg.value = ''
  try {
    const tauriCore = await import('@tauri-apps/api/core').catch(() => null)
    let backendScripts: ScriptInfo[] = []
    if (tauriCore && tauriCore.isTauri()) {
      const { invoke } = tauriCore
      backendScripts = await invoke<ScriptInfo[]>('get_workbench_scripts', { dir: dir.value })
    } else {
      const res = await fetch(`/api/workbench/scripts?dir=${encodeURIComponent(dir.value)}`)
      const json = await res.json()
      if (json.success) {
        backendScripts = json.data
      } else {
        throw new Error(json.error || '获取脚本列表失败')
      }
    }

    const installScripts = backendScripts.filter((script) => script.name.endsWith('install-to-project.sh'))
    const normalScripts = backendScripts.filter((script) => !script.name.endsWith('install-to-project.sh'))
    const merged = [...normalScripts, { name: '@tool:vless', path: 'internal' }]

    if (installScripts.length > 0) {
      merged.push({
        name: 'bundle:install-to-project',
        path: installScripts.map((script) => script.path).join('|||'),
      })
    }

    merged.sort((a, b) => {
      if (a.name === 'bundle:install-to-project') return -1
      if (b.name === 'bundle:install-to-project') return 1
      return a.name.localeCompare(b.name)
    })
    scripts.value = merged
  } catch (caught) {
    errorMsg.value = caught instanceof Error ? caught.message : '获取脚本列表失败'
    scripts.value = [{ name: '@tool:vless', path: 'internal' }]
  }
}

function selectScript(script: ScriptInfo) {
  selectedScript.value = script
  scriptArgs.value = ''
  errorMsg.value = ''
}

async function rerunHistory(record: HistoryRecord) {
  const targetScript = scripts.value.find((script) => script.name === record.scriptName)
  if (targetScript) {
    selectedScript.value = targetScript
    if (targetScript.name === 'bundle:install-to-project') {
      const parts = record.args.split(' ')
      if (parts.length >= 1) projectDir.value = parts[0]
      if (parts.length >= 2) projectSkill.value = parts[1]
    } else {
      scriptArgs.value = record.args
    }

    const mainScroll = document.querySelector('.codex-main-panel')
    if (mainScroll) {
      mainScroll.scrollTo({ top: 0, behavior: 'smooth' })
    } else {
      window.scrollTo({ top: 0, behavior: 'smooth' })
    }
  } else {
    errorMsg.value = '未找到对应的脚本文件，可能已被删除或目录已更改。'
  }
}

async function runScript(forceArgs?: string) {
  if (!selectedScript.value) return
  isRunning.value = true
  errorMsg.value = ''

  let finalArgs = scriptArgs.value
  if (selectedScript.value.name === 'bundle:install-to-project') {
    finalArgs = `${projectDir.value} ${projectSkill.value}`.trim()
  }

  if (typeof forceArgs === 'string') {
    finalArgs = forceArgs
  }

  try {
    const tauriCore = await import('@tauri-apps/api/core').catch(() => null)
    const resData: ExecutionResult = { stdout: '', stderr: '', exit_code: 0, success: true }
    const isBundle = selectedScript.value.name === 'bundle:install-to-project'
    const pathsToRun = isBundle ? bundleSelection.value : [selectedScript.value.path]

    if (pathsToRun.length === 0) {
      errorMsg.value = '请至少选择一个技能执行。'
      return
    }

    for (const path of pathsToRun) {
      let currentRes: ExecutionResult | null = null

      if (isBundle) {
        let engineName = '系统模块'
        if (path.includes('antigravity/')) engineName = 'Antigravity 核心引擎'
        else if (path.includes('codex/')) engineName = 'Codex 底层规范'
        resData.stdout += `\n\n[阶段] 开始注入：${engineName}\n--------------------------------------------------\n`
      }

      if (tauriCore && tauriCore.isTauri()) {
        const { invoke } = tauriCore
        currentRes = await invoke<ExecutionResult>('run_workbench_script', {
          path,
          args: finalArgs,
        })
      } else {
        const res = await fetch('/api/workbench/scripts/execute', {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
          },
          body: JSON.stringify({
            path,
            args: finalArgs,
          }),
        })
        const json = await res.json()
        if (json.success) {
          currentRes = json.data
        } else {
          errorMsg.value = json.error || '执行失败'
          break
        }
      }

      if (currentRes) {
        resData.stdout += currentRes.stdout
        resData.stderr += currentRes.stderr
        resData.exit_code = currentRes.exit_code
        resData.success = currentRes.success
        if (!currentRes.success) break
      }
    }

    executionHistory.value = executionHistory.value.filter(
      (record) => !(record.scriptName === selectedScript.value!.name && record.args === finalArgs),
    )
    executionHistory.value.unshift({
      id: crypto.randomUUID(),
      timestamp: Date.now(),
      scriptName: selectedScript.value.name,
      args: finalArgs,
      exit_code: resData.exit_code,
      success: resData.success,
      stdout: resData.stdout,
      stderr: resData.stderr,
    })
    if (executionHistory.value.length > 50) {
      executionHistory.value = executionHistory.value.slice(0, 50)
    }
  } catch (caught) {
    errorMsg.value = caught instanceof Error ? caught.message : '执行失败'
  } finally {
    isRunning.value = false
  }
}

function clearHistory() {
  executionHistory.value = []
}

function formatTime(timestamp: number) {
  return new Date(timestamp).toLocaleString('zh-CN', {
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  })
}

function taskCardClass(script: ScriptInfo) {
  return {
    'codex-task-card': true,
    'codex-task-card--active': selectedScript.value?.name === script.name,
  }
}
</script>

<template>
  <ToolShell
    title="Codex 工作台"
    description="集中管理本地脚本、内置工具、执行参数和可追溯历史。"
    eyebrow="自动化"
    fluid
  >
    <div class="codex-workbench">
      <section class="input-panel codex-status-panel">
        <div class="codex-status-main">
          <span class="service-icon">
            <Terminal class="h-5 w-5" aria-hidden="true" />
          </span>
          <div>
            <span class="field-label">执行中心</span>
            <strong>{{ selectedMeta?.title || '选择一个任务开始' }}</strong>
            <small>{{ selectedPathLabel }}</small>
          </div>
        </div>
        <dl class="codex-status-metrics">
          <div>
            <dt>{{ totalTaskCount }}</dt>
            <dd>可用任务</dd>
          </div>
          <div>
            <dt>{{ executionHistory.length }}</dt>
            <dd>执行记录</dd>
          </div>
          <div>
            <dt>{{ failedHistoryCount }}</dt>
            <dd>失败记录</dd>
          </div>
          <span :class="runStateClass">{{ runStateLabel }}</span>
        </dl>
      </section>

      <section class="codex-layout">
        <aside class="input-panel codex-task-panel">
          <div class="codex-panel-heading">
            <div>
              <span class="field-label">任务目录</span>
              <strong>本地脚本与内置工具</strong>
            </div>
            <Button type="button" variant="outline" size="sm" @click="pickDirectory('dir')">
              <FolderSearch class="mr-2 h-4 w-4" aria-hidden="true" />
              目录
            </Button>
          </div>

          <label class="codex-search-field" for="codex-task-search">
            <Search class="h-4 w-4" aria-hidden="true" />
            <Input id="codex-task-search" v-model="searchQuery" type="text" placeholder="搜索任务或脚本" />
          </label>

          <div class="codex-source-path">
            <FolderOpen class="h-4 w-4" aria-hidden="true" />
            <span>{{ dir }}</span>
          </div>

          <section class="codex-task-group">
            <div class="codex-task-group-header">
              <span>AI 技能</span>
              <span class="status-pill status-pill--muted">{{ filteredLocalScripts.length }}</span>
            </div>
            <div v-if="filteredLocalScripts.length" class="codex-task-list">
              <button
                v-for="script in filteredLocalScripts"
                :key="script.name"
                type="button"
                :class="taskCardClass(script)"
                @click="selectScript(script)"
              >
                <span class="codex-task-icon">
                  <component :is="getScriptMeta(script.name).icon" class="h-4 w-4" aria-hidden="true" />
                </span>
                <span>
                  <strong>{{ getScriptMeta(script.name).title }}</strong>
                  <small>{{ getScriptMeta(script.name).desc }}</small>
                </span>
              </button>
            </div>
            <p v-else class="field-hint">没有找到匹配的 AI 技能。</p>
          </section>

          <section v-if="filteredInternalScripts.length" class="codex-task-group">
            <div class="codex-task-group-header">
              <span>内置工具</span>
              <span class="status-pill status-pill--muted">{{ filteredInternalScripts.length }}</span>
            </div>
            <div class="codex-task-list">
              <button
                v-for="script in filteredInternalScripts"
                :key="script.name"
                type="button"
                :class="taskCardClass(script)"
                @click="selectScript(script)"
              >
                <span class="codex-task-icon">
                  <component :is="getScriptMeta(script.name).icon" class="h-4 w-4" aria-hidden="true" />
                </span>
                <span>
                  <strong>{{ getScriptMeta(script.name).title }}</strong>
                  <small>{{ getScriptMeta(script.name).desc }}</small>
                </span>
              </button>
            </div>
          </section>
        </aside>

        <main class="codex-main-panel">
          <template v-if="selectedScript && selectedScript.name === '@tool:vless'">
            <section class="input-panel codex-embedded-tool-note">
              <div>
                <span class="field-label">内置工具</span>
                <strong>VLESS 转 Mihomo 已有独立工作台</strong>
                <small>你可以在这里继续使用，也可以打开独立工具页获得完整页面空间。</small>
              </div>
              <RouterLink class="secondary-button" to="/toolbox/vless-to-mihomo">打开独立工具页</RouterLink>
            </section>
            <VlessToMihomo />
          </template>

          <template v-else-if="!selectedScript">
            <section class="input-panel codex-empty-state">
              <span class="service-icon">
                <Sparkles class="h-5 w-5" aria-hidden="true" />
              </span>
              <div>
                <span class="field-label">待选择</span>
                <h3>先选任务，再调参数，最后执行</h3>
                <p>左侧是可执行能力目录；右侧会根据任务类型切换参数表单，并保留每一次执行记录。</p>
              </div>
              <div class="codex-empty-grid">
                <span>
                  <Terminal class="h-4 w-4" aria-hidden="true" />
                  本地脚本
                </span>
                <span>
                  <History class="h-4 w-4" aria-hidden="true" />
                  历史追溯
                </span>
                <span>
                  <Settings2 class="h-4 w-4" aria-hidden="true" />
                  参数化执行
                </span>
              </div>
            </section>
          </template>

          <template v-else>
            <section class="input-panel codex-run-panel">
              <div class="codex-panel-heading">
                <div>
                  <span class="field-label">任务配置</span>
                  <strong>{{ selectedMeta?.title }}</strong>
                  <small>{{ selectedMeta?.desc }}</small>
                </div>
                <span :class="runStateClass">{{ runStateLabel }}</span>
              </div>

              <div class="codex-script-meta">
                <span>
                  <Settings2 class="h-4 w-4" aria-hidden="true" />
                  {{ selectedMeta?.badge }}
                </span>
                <button v-if="selectedScript.path !== 'internal'" type="button" @click="pickDirectory('dir')">
                  <FolderSearch class="h-4 w-4" aria-hidden="true" />
                  {{ selectedPathLabel }}
                </button>
              </div>

              <template v-if="selectedScript.name === 'bundle:install-to-project'">
                <section class="config-section">
                  <label class="field-control" for="codex-project-dir">
                    <span class="field-label">目标工作空间</span>
                    <div class="codex-inline-control">
                      <Input
                        id="codex-project-dir"
                        v-model="projectDir"
                        type="text"
                        placeholder="/Users/ben/my-new-project"
                      />
                      <Button type="button" variant="outline" @click="pickDirectory('projectDir')">
                        选择目录
                      </Button>
                    </div>
                    <small class="field-hint">技能会写入这个项目或目录，请先确认路径。</small>
                  </label>
                </section>

                <section class="config-section">
                  <div class="codex-panel-heading">
                    <div>
                      <span class="field-label">项目类型</span>
                      <strong>选择注入的规范模板</strong>
                    </div>
                  </div>
                  <div class="codex-skill-grid">
                    <button
                      v-for="option in projectSkillOptions"
                      :key="option.value"
                      type="button"
                      class="codex-skill-card"
                      :class="{ 'codex-skill-card--active': projectSkill === option.value }"
                      @click="projectSkill = option.value"
                    >
                      <component :is="option.icon" class="h-4 w-4" aria-hidden="true" />
                      <span>
                        <strong>{{ option.title }}</strong>
                        <small>{{ option.desc }}</small>
                      </span>
                    </button>
                  </div>
                </section>

                <section class="config-section">
                  <div class="codex-panel-heading">
                    <div>
                      <span class="field-label">安装模块</span>
                      <strong>将要执行的技能脚本</strong>
                    </div>
                    <span class="status-pill status-pill--muted">{{ bundleSelection.length }}</span>
                  </div>
                  <div class="codex-bundle-list">
                    <label
                      v-for="path in selectedScript.path.split('|||')"
                      :key="path"
                      class="codex-bundle-option"
                    >
                      <input v-model="bundleSelection" type="checkbox" :value="path" />
                      <span>{{ path.replace(`${dir}/`, '') }}</span>
                    </label>
                  </div>
                </section>
              </template>

              <template v-else>
                <section class="config-section">
                  <label class="field-control" for="codex-script-args">
                    <span class="field-label">执行参数</span>
                    <Input
                      id="codex-script-args"
                      v-model="scriptArgs"
                      type="text"
                      placeholder="按脚本约定输入参数，以空格分隔"
                    />
                    <small class="field-hint">执行前会保留参数到历史记录，方便复用。</small>
                  </label>
                </section>
              </template>

              <div class="codex-run-actions">
                <Button size="lg" type="button" :disabled="isRunning" @click="() => runScript()">
                  <Loader2 v-if="isRunning" class="mr-2 h-5 w-5 animate-spin" aria-hidden="true" />
                  <Play v-else class="mr-2 h-5 w-5" aria-hidden="true" />
                  {{ runButtonLabel }}
                </Button>
              </div>

              <p v-if="errorMsg" class="error-message">{{ errorMsg }}</p>
            </section>

            <section class="input-panel codex-history-panel">
              <div class="codex-panel-heading">
                <div>
                  <span class="field-label">执行记录</span>
                  <strong>{{ latestHistory ? getScriptMeta(latestHistory.scriptName).title : '暂无记录' }}</strong>
                  <small>{{ successHistoryCount }} 成功 / {{ failedHistoryCount }} 失败</small>
                </div>
                <Dialog v-if="executionHistory.length > 0">
                  <DialogTrigger as-child>
                    <Button type="button" variant="ghost" size="sm">
                      <Trash2 class="mr-2 h-4 w-4" aria-hidden="true" />
                      清空
                    </Button>
                  </DialogTrigger>
                  <DialogContent class="sm:max-w-md shadow-2xl shadow-black">
                    <DialogHeader>
                      <DialogTitle>确认清空执行记录？</DialogTitle>
                      <DialogDescription>此操作将永久删除所有执行历史记录，无法撤销。</DialogDescription>
                    </DialogHeader>
                    <DialogFooter class="sm:justify-end gap-2 sm:space-x-2 mt-4">
                      <DialogClose as-child>
                        <Button variant="outline">取消</Button>
                      </DialogClose>
                      <DialogClose as-child>
                        <Button variant="destructive" @click="clearHistory">确认清空</Button>
                      </DialogClose>
                    </DialogFooter>
                  </DialogContent>
                </Dialog>
              </div>

              <label v-if="executionHistory.length" class="codex-search-field" for="codex-history-search">
                <Search class="h-4 w-4" aria-hidden="true" />
                <Input
                  id="codex-history-search"
                  v-model="historySearchQuery"
                  type="text"
                  placeholder="搜索参数、输出或脚本"
                />
              </label>

              <div v-if="executionHistory.length" class="codex-history-list">
                <article v-if="filteredHistory.length === 0" class="codex-history-empty">
                  没有找到匹配的记录
                </article>
                <article v-for="record in filteredHistory" :key="record.id" class="codex-history-row">
                  <header>
                    <span :class="record.success ? 'status-pill status-pill--good' : 'status-pill status-pill--danger'">
                      <CheckCircle2 v-if="record.success" class="mr-1.5 h-3.5 w-3.5" aria-hidden="true" />
                      <XCircle v-else class="mr-1.5 h-3.5 w-3.5" aria-hidden="true" />
                      {{ record.success ? '成功' : '失败' }}
                    </span>
                    <strong>{{ getScriptMeta(record.scriptName).title }}</strong>
                    <small>{{ formatTime(record.timestamp) }} · exit {{ record.exit_code }}</small>
                    <Button type="button" variant="outline" size="sm" :disabled="isRunning" @click="rerunHistory(record)">
                      <Copy class="mr-2 h-3.5 w-3.5" aria-hidden="true" />
                      复用
                    </Button>
                  </header>
                  <div v-if="record.args" class="codex-history-args">
                    <span>参数</span>
                    <code>{{ record.args }}</code>
                  </div>
                  <div v-if="record.stdout || record.stderr" class="codex-terminal-block">
                    <pre v-if="record.stdout">{{ record.stdout }}</pre>
                    <pre v-if="record.stderr" class="codex-terminal-error">{{ record.stderr }}</pre>
                  </div>
                </article>
              </div>
              <div v-else class="codex-history-empty">
                <Circle class="h-6 w-6" aria-hidden="true" />
                <span>暂无执行记录</span>
              </div>
            </section>
          </template>
        </main>
      </section>
    </div>
  </ToolShell>
</template>
