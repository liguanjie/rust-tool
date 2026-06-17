<script setup lang="ts">
import { computed, defineAsyncComponent, onMounted, ref, watch } from 'vue'
import type { Component } from 'vue'
import { RouterLink } from 'vue-router'
import {
  Box,
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
import { useRoute, useRouter } from 'vue-router'
import { message, theme } from 'ant-design-vue'

const { token } = theme.useToken()

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
    const merged = [...normalScripts]

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
    scripts.value = []
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
  <div style="padding: 24px; max-width: 1200px; margin: 0 auto;">
    <a-page-header
      title="AI 技能"
      sub-title="集中管理本地脚本、执行参数和可追溯历史。"
      style="padding-left: 0; padding-right: 0;"
    >
      <template #tags>
        <a-tag color="blue">自动化</a-tag>
      </template>
    </a-page-header>

    <a-row :gutter="16" style="margin-bottom: 24px;">
      <a-col :span="6">
        <a-card>
          <a-statistic title="可用任务" :value="totalTaskCount" />
        </a-card>
      </a-col>
      <a-col :span="6">
        <a-card>
          <a-statistic title="执行记录" :value="executionHistory.length" />
        </a-card>
      </a-col>
      <a-col :span="6">
        <a-card>
          <a-statistic title="失败记录" :value="failedHistoryCount" />
        </a-card>
      </a-col>
      <a-col :span="6">
        <a-card>
          <a-statistic title="执行状态" :value="runStateLabel" />
        </a-card>
      </a-col>
    </a-row>

    <a-row :gutter="24">
      <a-col :span="8">
        <a-card title="任务目录" size="small" style="margin-bottom: 24px;">
          <template #extra>
            <a-button type="link" size="small" @click="pickDirectory('dir')">选择目录</a-button>
          </template>
          <div style="margin-bottom: 16px;">
            <a-input v-model:value="searchQuery" placeholder="搜索任务或脚本">
              <template #prefix><Search class="h-4 w-4" :style="{ color: token.colorTextTertiary }"/></template>
            </a-input>
          </div>
          <div style="margin-bottom: 8px; color: gray; font-size: 12px; word-break: break-all;">
            <FolderOpen class="h-4 w-4" style="display:inline-block; vertical-align:text-bottom; margin-right:4px;" />
            {{ dir }}
          </div>
          
          <div v-if="filteredLocalScripts.length" style="margin-top: 16px;">
            <div style="font-weight: bold; margin-bottom: 8px;">AI 技能</div>
            <a-list :data-source="filteredLocalScripts" size="small" bordered>
              <template #renderItem="{ item }">
                <a-list-item @click="selectScript(item)" :style="{ cursor: 'pointer', background: selectedScript?.name === item.name ? token.controlItemBgActive : 'transparent' }">
                  <a-list-item-meta :description="getScriptMeta(item.name).desc">
                    <template #title>
                      {{ getScriptMeta(item.name).title }}
                    </template>
                    <template #avatar>
                      <component :is="getScriptMeta(item.name).icon" class="h-5 w-5" />
                    </template>
                  </a-list-item-meta>
                </a-list-item>
              </template>
            </a-list>
          </div>


        </a-card>
      </a-col>

      <a-col :span="16">
        <template v-if="!selectedScript">
          <a-card>
            <a-empty description="先选任务，再调参数，最后执行" />
          </a-card>
        </template>
        <template v-else>
          <a-card title="任务配置" style="margin-bottom: 24px;">
            <template #extra>
              <a-tag :color="isRunning ? 'orange' : 'green'">{{ runStateLabel }}</a-tag>
            </template>
            <div style="margin-bottom: 16px;">
              <a-tag color="purple">{{ selectedMeta?.badge }}</a-tag>
              <span style="color: gray; font-size: 13px;">{{ selectedPathLabel }}</span>
            </div>

            <template v-if="selectedScript.name === 'bundle:install-to-project'">
               <a-form layout="vertical">
                 <a-form-item label="目标工作空间" extra="技能会写入这个项目或目录，请先确认路径。">
                   <a-input-search v-model:value="projectDir" placeholder="/Users/ben/my-new-project" @search="pickDirectory('projectDir')" enter-button="选择目录" />
                 </a-form-item>
                 <a-form-item label="项目类型">
                    <a-row :gutter="16">
                      <a-col :span="12" v-for="option in projectSkillOptions" :key="option.value" style="margin-bottom: 16px;">
                        <a-card size="small" hoverable @click="projectSkill = option.value" :style="{ borderColor: projectSkill === option.value ? token.colorPrimary : undefined }">
                          <a-card-meta :title="option.title" :description="option.desc">
                            <template #avatar><component :is="option.icon" class="h-5 w-5" /></template>
                          </a-card-meta>
                        </a-card>
                      </a-col>
                    </a-row>
                 </a-form-item>
                 <a-form-item label="安装模块">
                    <a-checkbox-group v-model:value="bundleSelection" style="width: 100%">
                      <a-row>
                        <a-col :span="24" v-for="path in selectedScript.path.split('|||')" :key="path" style="margin-bottom: 8px;">
                          <a-checkbox :value="path">{{ path.replace(`${dir}/`, '') }}</a-checkbox>
                        </a-col>
                      </a-row>
                    </a-checkbox-group>
                 </a-form-item>
               </a-form>
            </template>
            <template v-else>
               <a-form layout="vertical">
                 <a-form-item label="执行参数" extra="执行前会保留参数到历史记录，方便复用。">
                   <a-input v-model:value="scriptArgs" placeholder="按脚本约定输入参数，以空格分隔" />
                 </a-form-item>
               </a-form>
            </template>
            
            <a-button type="primary" size="large" :loading="isRunning" @click="runScript()">
              {{ runButtonLabel }}
            </a-button>
            <div v-if="errorMsg" style="color: red; margin-top: 16px;">{{ errorMsg }}</div>
          </a-card>

          <a-card title="执行记录">
            <template #extra>
              <a-popconfirm title="确认清空执行记录？" @confirm="clearHistory" ok-text="确认清空" ok-type="danger">
                 <a-button type="link" danger size="small">清空</a-button>
              </a-popconfirm>
            </template>
            <div style="margin-bottom: 16px;">
              <a-input v-model:value="historySearchQuery" placeholder="搜索参数、输出或脚本">
                 <template #prefix><Search class="h-4 w-4" :style="{ color: token.colorTextTertiary }"/></template>
              </a-input>
            </div>
            <a-list :data-source="filteredHistory" size="small" item-layout="vertical" bordered>
              <template #renderItem="{ item: record }">
                <a-list-item>
                  <template #extra>
                    <a-button type="link" size="small" @click="rerunHistory(record)">复用参数</a-button>
                  </template>
                  <a-list-item-meta>
                    <template #title>
                       <a-tag :color="record.success ? 'success' : 'error'">{{ record.success ? '成功' : '失败' }}</a-tag>
                       {{ getScriptMeta(record.scriptName).title }}
                    </template>
                    <template #description>
                      {{ formatTime(record.timestamp) }} · exit {{ record.exit_code }}
                    </template>
                  </a-list-item-meta>
                  <div v-if="record.args" style="margin-bottom: 8px;">
                    <a-typography-text type="secondary">参数: </a-typography-text>
                    <a-typography-text code>{{ record.args }}</a-typography-text>
                  </div>
                  <div v-if="record.stdout || record.stderr" style="background: #1e1e1e; color: #d4d4d4; padding: 12px; border-radius: 6px; overflow-x: auto; font-size: 12px; max-height: 400px; overflow-y: auto;">
                    <pre v-if="record.stdout" style="margin: 0; font-family: monospace; white-space: pre-wrap;">{{ record.stdout }}</pre>
                    <pre v-if="record.stderr" style="margin: 0; font-family: monospace; color: #f48771; white-space: pre-wrap;">{{ record.stderr }}</pre>
                  </div>
                </a-list-item>
              </template>
              <template #empty>
                <a-empty description="暂无执行记录" />
              </template>
            </a-list>
          </a-card>
        </template>
      </a-col>
    </a-row>
  </div>
</template>
