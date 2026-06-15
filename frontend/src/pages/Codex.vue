<script setup lang="ts">
import { ref, onMounted, watch, computed, defineAsyncComponent } from 'vue'
import { Folder, Play, Search, Trash2, Package, Terminal, Box, Wrench, Settings2, Sparkles, CheckCircle2, Cable, FolderSearch } from '@lucide/vue'

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

// 产品化的文案字典映射 (前端美化)
const SCRIPT_DICT: Record<string, { title: string, desc: string, icon: any }> = {
  'install-to-project.sh': {
    title: 'Codex 技能安装向导',
    desc: '一键为目标项目注入底层框架规范与核心自动化能力。',
    icon: Package
  },
  '@tool:vless': {
    title: 'VLESS 转 Mihomo',
    desc: '将 3x-ui VLESS 链接转换为 Clash Party/Mihomo YAML',
    icon: Cable
  }
}

// 默认兜底映射
function getScriptMeta(scriptName: string) {
  return SCRIPT_DICT[scriptName] || {
    title: scriptName,
    desc: '系统级自动化执行脚本。',
    icon: Wrench
  }
}

const dir = ref('/Users/ben/work/99_codex/codex')
const scripts = ref<ScriptInfo[]>([])
const selectedScript = ref<ScriptInfo | null>(null)
const searchQuery = ref('')

const filteredScripts = computed(() => {
  if (!searchQuery.value) return scripts.value
  const lowerQ = searchQuery.value.toLowerCase()
  return scripts.value.filter(s => {
    const meta = getScriptMeta(s.name)
    return s.name.toLowerCase().includes(lowerQ) || meta.title.toLowerCase().includes(lowerQ)
  })
})

const filteredLocalScripts = computed(() => {
  return filteredScripts.value.filter(s => s.path !== 'internal')
})

const filteredInternalScripts = computed(() => {
  return filteredScripts.value.filter(s => s.path === 'internal')
})

const scriptArgs = ref('')
const projectDir = ref('')
const projectSkill = ref('general')

const isRunning = ref(false)
const errorMsg = ref('')

const executionHistory = ref<HistoryRecord[]>([])
const historySearchQuery = ref('')

const filteredHistory = computed(() => {
  if (!historySearchQuery.value) return executionHistory.value
  const lowerQ = historySearchQuery.value.toLowerCase()
  return executionHistory.value.filter(record => {
    const meta = getScriptMeta(record.scriptName)
    return record.scriptName.toLowerCase().includes(lowerQ) ||
           meta.title.toLowerCase().includes(lowerQ) ||
           record.args.toLowerCase().includes(lowerQ) ||
           record.stdout.toLowerCase().includes(lowerQ) ||
           record.stderr.toLowerCase().includes(lowerQ)
  })
})

onMounted(() => {
  const savedHistory = localStorage.getItem('rusttool:codex:history')
  if (savedHistory) {
    try {
      executionHistory.value = JSON.parse(savedHistory)
    } catch (e) {
      console.error('历史解析失败', e)
    }
  }
  fetchScripts()
})

watch(executionHistory, (newHistory) => {
  localStorage.setItem('rusttool:codex:history', JSON.stringify(newHistory))
}, { deep: true })

async function pickDirectory(targetRef: 'dir' | 'projectDir') {
  try {
    const tauriCore = await import('@tauri-apps/api/core').catch(() => null)
    if (tauriCore && tauriCore.isTauri()) {
      const { open } = await import('@tauri-apps/plugin-dialog')
      const selected = await open({ directory: true })
      if (selected && typeof selected === 'string') {
        if (targetRef === 'dir') {
          dir.value = selected
          fetchScripts()
        }
        if (targetRef === 'projectDir') projectDir.value = selected
      }
    } else {
      alert('目录浏览功能仅在桌面客户端可用，请手动输入路径。')
    }
  } catch (err: any) {
    console.error(err)
    alert(`无法打开文件夹选择器: ${err.message || String(err)}`)
  }
}

async function fetchScripts() {
  errorMsg.value = ''
  try {
    const tauriCore = await import('@tauri-apps/api/core').catch(() => null)
    if (tauriCore && tauriCore.isTauri()) {
      const { invoke } = tauriCore
      const backendScripts = await invoke<ScriptInfo[]>('get_workbench_scripts', { dir: dir.value })
      const merged = [...backendScripts, { name: '@tool:vless', path: 'internal' }]
      merged.sort((a, b) => {
        if (a.name === 'install-to-project.sh') return -1
        if (b.name === 'install-to-project.sh') return 1
        return a.name.localeCompare(b.name)
      })
      scripts.value = merged
    } else {
      const res = await fetch(`/api/workbench/scripts?dir=${encodeURIComponent(dir.value)}`)
      const json = await res.json()
      if (json.success) {
        const merged = [...json.data, { name: '@tool:vless', path: 'internal' }]
        merged.sort((a, b) => {
          if (a.name === 'install-to-project.sh') return -1
          if (b.name === 'install-to-project.sh') return 1
          return a.name.localeCompare(b.name)
        })
        scripts.value = merged
      } else {
        errorMsg.value = json.error || '获取脚本列表失败'
        scripts.value = []
      }
    }
  } catch (err: any) {
    errorMsg.value = err.message
    scripts.value = []
  }
}

function selectScript(script: ScriptInfo) {
  selectedScript.value = script
  scriptArgs.value = ''
}

async function rerunHistory(record: HistoryRecord) {
  const targetScript = scripts.value.find(s => s.name === record.scriptName)
  if (targetScript) {
    selectedScript.value = targetScript
    if (targetScript.name === 'install-to-project.sh') {
      const parts = record.args.split(' ')
      if (parts.length >= 1) projectDir.value = parts[0]
      if (parts.length >= 2) projectSkill.value = parts[1]
    } else {
      scriptArgs.value = record.args
    }
    
    // 平滑滚动到顶部，方便用户查看和编辑
    const mainScroll = document.querySelector('.main-content-scroll')
    if (mainScroll) {
      mainScroll.scrollTo({ top: 0, behavior: 'smooth' })
    } else {
      window.scrollTo({ top: 0, behavior: 'smooth' })
    }
  } else {
    alert('未找到对应的脚本文件，可能已被删除或目录已更改。')
  }
}

async function runScript(forceArgs?: string) {
  if (!selectedScript.value) return
  isRunning.value = true
  errorMsg.value = ''

  let finalArgs = scriptArgs.value
  if (selectedScript.value.name === 'install-to-project.sh') {
    finalArgs = `${projectDir.value} ${projectSkill.value}`.trim()
  }
  
  if (typeof forceArgs === 'string') {
    finalArgs = forceArgs
  }

  try {
    const tauriCore = await import('@tauri-apps/api/core').catch(() => null)
    let resData: ExecutionResult | null = null

    if (tauriCore && tauriCore.isTauri()) {
      const { invoke } = tauriCore
      resData = await invoke<ExecutionResult>('run_workbench_script', {
        path: selectedScript.value.path,
        args: finalArgs
      })
    } else {
      const res = await fetch('/api/workbench/scripts/execute', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json'
        },
        body: JSON.stringify({
          path: selectedScript.value.path,
          args: finalArgs
        })
      })
      const json = await res.json()
      if (json.success) {
        resData = json.data
      } else {
        errorMsg.value = json.error || '执行失败'
      }
    }

    if (resData) {
      executionHistory.value = executionHistory.value.filter(
        record => !(record.scriptName === selectedScript.value!.name && record.args === finalArgs)
      )

      executionHistory.value.unshift({
        id: crypto.randomUUID(),
        timestamp: Date.now(),
        scriptName: selectedScript.value.name,
        args: finalArgs,
        exit_code: resData.exit_code,
        success: resData.success,
        stdout: resData.stdout,
        stderr: resData.stderr
      })
      if (executionHistory.value.length > 50) {
        executionHistory.value = executionHistory.value.slice(0, 50)
      }
    }
  } catch (err: any) {
    errorMsg.value = err.message
  } finally {
    isRunning.value = false
  }
}

function clearHistory() {
  executionHistory.value = []
}

function formatTime(timestamp: number) {
  return new Date(timestamp).toLocaleString('zh-CN', {
    month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit', second: '2-digit'
  })
}
</script>

<template>
  <div class="saas-layout">
    <!-- 左侧：任务中心 -->
    <aside class="saas-sidebar">
      <div class="sidebar-header">
        <h1 class="app-title">工作台</h1>
        <div class="search-bar">
          <Search class="h-4 w-4 icon-subtle" />
          <input v-model="searchQuery" type="text" placeholder="寻找任务或功能..." />
        </div>
      </div>

      <div class="task-list">
        <div class="task-lists">
          <div class="list-group">
            <div class="list-header" style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.75rem;">
              <div style="display: flex; align-items: center; gap: 0.5rem;">
                <h3 style="font-size: 0.75rem; font-weight: 600; color: var(--color-text-sub); margin: 0;">AI 技能</h3>
                <span class="badge">{{ filteredLocalScripts.length }}</span>
              </div>
              <button class="btn-icon-small" @click="pickDirectory('dir')" title="配置本地技能目录">
                <FolderSearch class="h-4 w-4 icon-subtle" />
              </button>
            </div>

            <div class="script-list">
              <div 
                v-for="s in filteredLocalScripts" 
                :key="s.name"
                class="task-card"
                :class="{ active: selectedScript?.name === s.name }"
                @click="selectScript(s)"
              >
                <div class="task-icon-wrapper" :class="{ active: selectedScript?.name === s.name }">
                  <component :is="getScriptMeta(s.name).icon" class="h-5 w-5" />
                </div>
                <div class="task-info">
                  <h4>{{ getScriptMeta(s.name).title }}</h4>
                  <p>{{ getScriptMeta(s.name).desc }}</p>
                </div>
              </div>
            </div>
          </div>

          <div class="list-group" style="margin-top: 1.5rem;" v-if="filteredInternalScripts.length > 0">
            <div class="list-header" style="display: flex; align-items: center; gap: 0.5rem; margin-bottom: 0.75rem;">
              <h3 style="font-size: 0.75rem; font-weight: 600; color: var(--color-text-sub); margin: 0;">内置工具</h3>
              <span class="badge">{{ filteredInternalScripts.length }}</span>
            </div>

            <div class="script-list">
              <div 
                v-for="s in filteredInternalScripts" 
                :key="s.name"
                class="task-card"
                :class="{ active: selectedScript?.name === s.name }"
                @click="selectScript(s)"
              >
                <div class="task-icon-wrapper" :class="{ active: selectedScript?.name === s.name }">
                  <component :is="getScriptMeta(s.name).icon" class="h-5 w-5" />
                </div>
                <div class="task-info">
                  <h4>{{ getScriptMeta(s.name).title }}</h4>
                  <p>{{ getScriptMeta(s.name).desc }}</p>
                </div>
              </div>
            </div>
          </div>
        </div>

        <div v-if="filteredLocalScripts.length === 0 && filteredInternalScripts.length === 0" class="empty-list">
          没有找到匹配的任务
        </div>
      </div>
    </aside>

    <!-- 右侧：操作区 -->
    <main class="saas-main">
      <div v-if="selectedScript && selectedScript.name === '@tool:vless'" style="flex: 1; display: flex; flex-direction: column; overflow-y: auto;">
        <VlessToMihomo />
      </div>

      <div v-else class="main-content-scroll">
        
        <!-- 空状态 (Onboarding) -->
        <div v-if="!selectedScript" class="onboarding-state">
          <div class="hero-icon-new">
            <div class="icon-glow"></div>
            <Sparkles class="h-10 w-10 text-emerald-400 relative z-10" />
          </div>
          <h2 class="bg-gradient-to-r from-white to-white/60 bg-clip-text text-transparent">智能工作台</h2>
          <p class="onboarding-desc">
            选择左侧任务卡片，即可使用 AI 技能自动化执行任务，或使用内置工具进行管理配置。
          </p>
          <div class="features-grid-new">
            <div class="feature-card-new">
              <div class="feature-icon-new"><Terminal class="h-5 w-5 text-emerald-400" /></div>
              <div class="feature-text-new">
                <h3>AI 技能驱动</h3>
                <p>挂载本地脚本，扩展核心自动化能力</p>
              </div>
            </div>
            <div class="feature-card-new">
              <div class="feature-icon-new"><Wrench class="h-5 w-5 text-emerald-400" /></div>
              <div class="feature-text-new">
                <h3>开箱即用工具</h3>
                <p>集成常用网络代理与系统配置模块</p>
              </div>
            </div>
          </div>
        </div>

        <!-- 任务操作台 -->
        <div v-else class="task-workspace">
          
          <!-- 左半区：配置表单 -->
          <div class="workspace-left">
            <div class="workspace-header">
              <h2>{{ getScriptMeta(selectedScript.name).title }}</h2>
              <div class="script-badges" style="display: flex; gap: 0.5rem; align-items: center;">
                <span 
                  v-if="selectedScript.path !== 'internal'" 
                  class="script-badge" 
                  style="opacity: 0.7; font-family: monospace; cursor: pointer; transition: all 0.2s;"
                  title="点击重新选择技能目录"
                  @click="pickDirectory('dir')"
                  onmouseover="this.style.opacity='1'; this.style.borderColor='var(--color-primary)'"
                  onmouseout="this.style.opacity='0.7'; this.style.borderColor='var(--color-border)'"
                >
                  <FolderSearch class="h-3 w-3 inline-block mr-1" style="vertical-align: text-bottom;" />
                  {{ selectedScript.path.replace('/' + selectedScript.name, '') }}
                </span>
                <span class="script-badge">{{ selectedScript.name }}</span>
              </div>
            </div>

            <div class="form-container">
            <template v-if="selectedScript.name === 'install-to-project.sh'">
              <div class="form-field">
                <label>第一步：配置目标工作空间</label>
                <div class="input-with-button">
                  <input v-model="projectDir" type="text" placeholder="例如：/Users/ben/my-new-project" />
                  <button class="btn-outline" @click="pickDirectory('projectDir')">选择目录</button>
                </div>
                <span class="field-hint">指定一个本地文件夹作为目标注入的工作空间位置</span>
              </div>
              
              <div class="form-field">
                <label>第二步：项目/目录类型</label>
                <div class="skill-cards">
                  <div 
                    class="skill-card" 
                    :class="{ active: projectSkill === 'general' }"
                    @click="projectSkill = 'general'"
                  >
                    <span class="skill-icon">🟢</span>
                    <div class="skill-info">
                      <div class="skill-title">黄金手册</div>
                      <div class="skill-desc">通用团队编码规范</div>
                    </div>
                  </div>

                  <div 
                    class="skill-card" 
                    :class="{ active: projectSkill === 'scm' }"
                    @click="projectSkill = 'scm'"
                  >
                    <span class="skill-icon">📦</span>
                    <div class="skill-info">
                      <div class="skill-title">供应链 (SCM)</div>
                      <div class="skill-desc">供应链管理业务模板</div>
                    </div>
                  </div>

                  <div 
                    class="skill-card" 
                    :class="{ active: projectSkill === 'scf' }"
                    @click="projectSkill = 'scf'"
                  >
                    <span class="skill-icon">💰</span>
                    <div class="skill-info">
                      <div class="skill-title">金融 (SCF)</div>
                      <div class="skill-desc">供应链金融业务模板</div>
                    </div>
                  </div>

                  <div 
                    class="skill-card" 
                    :class="{ active: projectSkill === 'b2b' }"
                    @click="projectSkill = 'b2b'"
                  >
                    <span class="skill-icon">🤝</span>
                    <div class="skill-info">
                      <div class="skill-title">交易平台</div>
                      <div class="skill-desc">B2B 交易平台业务模板</div>
                    </div>
                  </div>
                </div>
              </div>
            </template>
            
            <template v-else>
              <div class="form-field">
                <label>任务执行参数（可选）</label>
                <input v-model="scriptArgs" type="text" placeholder="输入参数，以空格分隔..." class="full-width" />
              </div>
            </template>

            <div class="action-row">
              <button class="btn-primary-action" :disabled="isRunning" @click="() => runScript()">
                <Play class="h-4 w-4" :class="{'spin-pulse': isRunning}" />
                <span>{{ isRunning ? '正在处理中...' : (selectedScript.name === 'install-to-project.sh' ? '立刻安装技能' : '立刻开始执行') }}</span>
              </button>
            </div>
            </div>
          </div>

          <!-- 右半区：历史记录 -->
          <div class="workspace-right">
            <div class="workspace-header" style="display: flex; justify-content: space-between; align-items: flex-start;">
              <div>
                <h2>执行记录</h2>
                <p class="script-badge">History / Logs</p>
              </div>
              <button class="btn-text text-red" @click="clearHistory" v-if="executionHistory.length > 0" style="margin-top: 0.5rem;">清空</button>
            </div>

            <div class="form-container history-container">
              <template v-if="executionHistory.length > 0">

              <!-- 历史搜索框 -->
              <div class="history-search">
                <Search class="h-4 w-4 icon-subtle" />
                <input v-model="historySearchQuery" type="text" placeholder="搜索参数、输出或脚本..." />
              </div>

              <div class="feed-list">
                <div v-if="filteredHistory.length === 0" class="empty-list">
                  没有找到匹配的记录
                </div>
                
                <div v-for="record in filteredHistory" :key="record.id" class="feed-item">
                  <div class="feed-item-top">
                    <div class="feed-info">
                      <span class="status-dot" :class="record.success ? 'bg-green' : 'bg-red'"></span>
                      <span class="script-tag">{{ getScriptMeta(record.scriptName).title }}</span>
                      <span class="time">{{ formatTime(record.timestamp) }}</span>
                    </div>
                    <button class="btn-outline-small" @click="rerunHistory(record)" :disabled="isRunning">
                      载入配置
                    </button>
                  </div>
                  
                  <div class="feed-args" v-if="record.args">
                    参数：<code>{{ record.args }}</code>
                  </div>

                  <!-- 终端输出块 (折叠感) -->
                  <div class="terminal-block" v-if="record.stdout || record.stderr">
                    <pre v-if="record.stdout" class="stdout">{{ record.stdout }}</pre>
                    <pre v-if="record.stderr" class="stderr">{{ record.stderr }}</pre>
                  </div>
                </div>
              </div>
            </template>
            <template v-else>
                <div class="empty-list" style="padding: 4rem 0; text-align: center; color: var(--color-text-muted);">
                  <Box class="h-8 w-8 opacity-50" style="margin: 0 auto 0.5rem auto;" />
                  <p>暂无执行记录</p>
                </div>
              </template>
            </div>
          </div>
          
        </div>
      </div>
    </main>
  </div>
</template>

<style scoped>
/* 
  Modern SaaS Aesthetic integrated with Global Cyberpunk Dark Theme
*/

* {
  --color-bg-sidebar: transparent;
  --color-bg-main: transparent;
  --color-border: var(--border-card, rgba(16, 185, 129, 0.15));
  --color-border-hover: rgba(16, 185, 129, 0.4);
  --color-text-main: var(--text-title, #ffffff);
  --color-text-sub: var(--text-base, #94a3b8);
  --color-text-muted: var(--text-muted, #64748b);
  --color-primary: #10b981; /* Global Emerald */
  --color-primary-hover: #059669;
  --color-surface: var(--bg-card, rgba(15, 23, 30, 0.6));
  --color-surface-hover: rgba(16, 185, 129, 0.05);
  --color-terminal-bg: rgba(0, 0, 0, 0.5);
  --color-terminal-text: #f8fafc;
  
  --shadow-card: var(--shadow-card, 0 8px 32px 0 rgba(0, 0, 0, 0.6));
  --shadow-input: inset 0 2px 4px rgba(0, 0, 0, 0.2);
  
  --radius-lg: 16px;
  --radius-md: 12px;
  --radius-sm: 8px;
}

.saas-layout {
  display: flex;
  height: calc(100vh - 3rem);
  background: var(--color-bg-main);
  color: var(--color-text-main);
  overflow: hidden;
  transition: all 0.3s ease;
}

/* --- Sidebar --- */
.saas-sidebar {
  width: 320px;
  background: var(--color-bg-sidebar);
  border-right: 1px solid var(--color-border);
  display: flex;
  flex-direction: column;
  flex-shrink: 0;
}

.sidebar-header {
  padding: 1.5rem 1.5rem 1rem 1.5rem;
}

.app-title {
  font-size: 1.25rem;
  font-weight: 700;
  margin: 0 0 1rem 0;
  color: var(--color-text-main);
  letter-spacing: -0.02em;
}

.search-bar {
  display: flex;
  align-items: center;
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  padding: 0.5rem 0.75rem;
  box-shadow: var(--shadow-input);
  transition: border-color 0.2s;
}

.search-bar:focus-within {
  border-color: var(--color-primary);
  box-shadow: 0 0 0 3px rgba(16, 185, 129, 0.15);
}

.search-bar input {
  flex: 1;
  border: none;
  background: transparent;
  outline: none;
  margin-left: 0.5rem;
  font-size: 0.875rem;
  color: var(--color-text-main);
}

.search-bar input::placeholder {
  color: var(--color-text-muted);
  opacity: 0.8;
}

.btn-icon-small {
  background: transparent;
  border: none;
  cursor: pointer;
  padding: 4px;
  border-radius: 4px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-secondary);
  transition: all 0.2s ease;
}

.btn-icon-small:hover {
  background: rgba(255, 255, 255, 0.05);
  color: var(--text-primary);
}

.icon-subtle {
  color: var(--color-text-muted);
}

.task-list {
  flex: 1;
  overflow-y: auto;
  padding: 0 1rem 1rem 1rem;
}

.list-label {
  font-size: 0.75rem;
  font-weight: 600;
  color: var(--color-text-sub);
  margin: 0.5rem 0.5rem 0.75rem 0.5rem;
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.badge {
  background: var(--color-border);
  padding: 0.1rem 0.5rem;
  border-radius: 12px;
  font-size: 0.7rem;
  color: var(--color-text-sub);
}

.task-card {
  display: flex;
  align-items: flex-start;
  padding: 1rem;
  margin-bottom: 0.5rem;
  border-radius: var(--radius-md);
  border: 1px solid transparent;
  cursor: pointer;
  transition: all 0.2s ease;
}

.task-card:hover {
  background: var(--color-surface-hover);
  border-color: var(--color-border);
}

.task-card.active {
  background: var(--color-surface);
  border-color: var(--color-primary);
  box-shadow: 0 0 15px rgba(16, 185, 129, 0.15);
}

.task-icon-wrapper {
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  padding: 0.5rem;
  border-radius: 10px;
  color: var(--color-text-sub);
  margin-right: 1rem;
  transition: all 0.2s;
}

.task-icon-wrapper.active {
  background: rgba(16, 185, 129, 0.05);
  border-color: rgba(16, 185, 129, 0.2);
  color: var(--color-primary);
}

.task-info h4 {
  margin: 0 0 0.25rem 0;
  font-size: 0.9375rem;
  font-weight: 600;
  color: var(--color-text-main);
}

.task-info p {
  margin: 0;
  font-size: 0.8125rem;
  color: var(--color-text-sub);
  line-height: 1.4;
}

.empty-list {
  padding: 2rem;
  text-align: center;
  color: var(--color-text-muted);
  font-size: 0.875rem;
}

.sidebar-footer {
  padding: 1rem;
  border-top: 1px solid var(--color-border);
}

.settings-btn {
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.5rem;
  padding: 0.625rem;
  background: transparent;
  border: 1px dashed var(--color-border-hover);
  border-radius: var(--radius-sm);
  color: var(--color-text-sub);
  font-size: 0.8125rem;
  cursor: pointer;
  transition: all 0.2s;
}

.settings-btn:hover {
  background: var(--color-surface-hover);
  color: var(--color-text-main);
  border-style: solid;
}

/* --- Main Content --- */
.saas-main {
  flex: 1;
  display: flex;
  flex-direction: column;
  position: relative;
}

.main-content-scroll {
  flex: 1;
  overflow-y: auto;
  padding: 3rem 4rem;
  display: flex;
  justify-content: center;
}

/* Onboarding State */
.onboarding-state {
  max-width: 560px;
  margin-top: 6rem;
  text-align: center;
}

.hero-icon-new {
  position: relative;
  width: 96px;
  height: 96px;
  margin: 0 auto 2rem auto;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: 32px;
  box-shadow: 0 12px 32px -8px rgba(0, 0, 0, 0.5);
}

.icon-glow {
  position: absolute;
  inset: -1px;
  border-radius: 32px;
  background: linear-gradient(135deg, rgba(16, 185, 129, 0.4), transparent);
  opacity: 0.5;
  filter: blur(8px);
  z-index: 0;
}

.onboarding-state h2 {
  font-size: 2.25rem;
  font-weight: 800;
  margin-bottom: 1rem;
  letter-spacing: -0.03em;
}

.onboarding-desc {
  font-size: 1.0625rem;
  color: var(--color-text-sub);
  line-height: 1.6;
  margin-bottom: 3rem;
  max-width: 420px;
  margin-inline: auto;
}

.features-grid-new {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 1.25rem;
}

.feature-card-new {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 1rem;
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  padding: 1.5rem;
  border-radius: var(--radius-lg);
  text-align: left;
  transition: all 0.2s ease;
}

.feature-card-new:hover {
  border-color: var(--color-primary);
  transform: translateY(-2px);
  box-shadow: 0 8px 24px -8px rgba(16, 185, 129, 0.15);
}

.feature-icon-new {
  width: 40px;
  height: 40px;
  border-radius: 12px;
  background: rgba(16, 185, 129, 0.1);
  display: flex;
  align-items: center;
  justify-content: center;
}

.feature-text-new h3 {
  font-size: 1rem;
  font-weight: 600;
  color: var(--color-text-main);
  margin-bottom: 0.25rem;
}

.feature-text-new p {
  font-size: 0.8125rem;
  color: var(--color-text-muted);
  line-height: 1.4;
}

/* Task Workspace (Form Area) */
.task-workspace {
  width: 100%;
  max-width: 1400px;
  display: grid;
  grid-template-columns: minmax(400px, 3fr) minmax(300px, 2fr);
  gap: 3rem;
  align-items: start;
}

@media (max-width: 1100px) {
  .task-workspace {
    grid-template-columns: 1fr;
    gap: 2rem;
  }
}

.workspace-left {
  display: flex;
  flex-direction: column;
}

.workspace-right {
  display: flex;
  flex-direction: column;
}

.workspace-header {
  margin-bottom: 2.5rem;
}

.workspace-header h2 {
  font-size: 2rem;
  font-weight: 700;
  margin: 0 0 0.5rem 0;
  letter-spacing: -0.02em;
}

.script-badge {
  display: inline-block;
  background: var(--color-surface-hover);
  border: 1px solid var(--color-border);
  padding: 0.25rem 0.75rem;
  border-radius: 16px;
  font-family: ui-monospace, monospace;
  font-size: 0.75rem;
  color: var(--color-text-sub);
  margin: 0;
}

.form-container {
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  padding: 2rem;
  box-shadow: var(--shadow-card);
  margin-bottom: 3rem;
}

.feed-item {
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  padding: 1.25rem;
  background: var(--color-bg-main);
  transition: all 0.2s;
}

.history-container {
  padding: 1.5rem;
}

.history-container .feed-list {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.form-field {
  margin-bottom: 1.75rem;
}

.form-field label {
  display: block;
  font-size: 0.9375rem;
  font-weight: 600;
  margin-bottom: 0.75rem;
  color: var(--color-text-main);
}

.field-hint {
  display: block;
  font-size: 0.8125rem;
  color: var(--color-text-muted);
  margin-top: 0.5rem;
}

.input-with-button {
  display: flex;
  gap: 0.5rem;
}

input {
  background: var(--color-bg-main);
  border: 1px solid var(--color-border);
  color: var(--color-text-main);
  padding: 0.75rem 1rem;
  border-radius: var(--radius-sm);
  font-size: 0.9375rem;
  outline: none;
  transition: all 0.2s;
  box-shadow: var(--shadow-input);
}

.input-with-button input {
  flex: 1;
}

.full-width {
  width: 100%;
}

input:focus, select:focus {
  border-color: var(--color-primary);
  box-shadow: 0 0 0 3px rgba(16, 185, 129, 0.15);
}

.btn-outline {
  background: var(--color-surface);
  border: 1px solid var(--color-border-hover);
  color: var(--color-text-main);
  padding: 0 1rem;
  border-radius: var(--radius-sm);
  font-size: 0.875rem;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s;
  white-space: nowrap;
}

.btn-outline:hover {
  background: var(--color-surface-hover);
}

.skill-cards {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 1rem;
}

.skill-card {
  background: var(--color-bg-main);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  padding: 1rem;
  display: flex;
  align-items: flex-start;
  gap: 0.75rem;
  cursor: pointer;
  transition: all 0.2s;
}

.skill-card:hover {
  background: var(--color-surface-hover);
  border-color: var(--color-border-hover);
}

.skill-card.active {
  background: rgba(16, 185, 129, 0.08);
  border-color: var(--color-primary);
  box-shadow: 0 0 0 1px var(--color-primary);
}

.skill-icon {
  font-size: 1.25rem;
  line-height: 1;
}

.skill-info {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.skill-title {
  color: var(--color-text-main);
  font-weight: 600;
  font-size: 0.9375rem;
}

.skill-desc {
  color: var(--color-text-sub);
  font-size: 0.75rem;
  line-height: 1.4;
}

.action-row {
  display: flex;
  justify-content: flex-start;
  margin-top: 1.5rem;
}

.btn-primary-action {
  background: var(--color-primary);
  color: #050a0f;
  border: none;
  padding: 0.625rem 1.5rem;
  border-radius: var(--radius-sm);
  font-size: 0.9375rem;
  font-weight: 600;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 0.5rem;
  cursor: pointer;
  transition: all 0.2s;
  box-shadow: 0 4px 12px rgba(16, 185, 129, 0.2);
}

.btn-primary-action:hover:not(:disabled) {
  background: var(--color-primary-hover);
  transform: translateY(-1px);
}

.btn-primary-action:disabled {
  opacity: 0.6;
  cursor: not-allowed;
  transform: none;
}

.history-search {
  display: flex;
  align-items: center;
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  padding: 0.5rem 0.75rem;
  box-shadow: var(--shadow-input);
  margin-bottom: 1.25rem;
  transition: border-color 0.2s;
}

.history-search:focus-within {
  border-color: var(--color-primary);
  box-shadow: 0 0 0 3px rgba(16, 185, 129, 0.15);
}

.history-search input {
  flex: 1;
  border: none;
  background: transparent;
  outline: none;
  margin-left: 0.5rem;
  font-size: 0.8125rem;
  color: var(--color-text-main);
  padding: 0;
  box-shadow: none;
}

.history-search input::placeholder {
  color: var(--color-text-muted);
}

.spin-pulse {
  animation: pulse 1.5s cubic-bezier(0.4, 0, 0.6, 1) infinite;
}

/* --- History Feed --- */
.history-feed {
  margin-top: 2rem;
}

.feed-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 1rem;
}

.feed-header h3 {
  font-size: 1.125rem;
  font-weight: 600;
  margin: 0;
}

.btn-text {
  background: transparent;
  border: none;
  font-size: 0.875rem;
  cursor: pointer;
  padding: 0.25rem 0.5rem;
  border-radius: 4px;
}

.text-red { color: #ef4444; }
.text-red:hover { background: rgba(239, 68, 68, 0.1); }

.feed-list {
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
}

.feed-item {
  border-left: 2px solid var(--color-border);
  padding-left: 1rem;
  position: relative;
}

.feed-item::before {
  content: '';
  position: absolute;
  left: -5px;
  top: 8px;
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--color-border-hover);
}

.feed-item-top {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 0.5rem;
}

.feed-info {
  display: flex;
  align-items: center;
  gap: 0.75rem;
}

.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
}

.bg-green { background: #10b981; box-shadow: 0 0 6px rgba(16, 185, 129, 0.4); }
.bg-red { background: #ef4444; box-shadow: 0 0 6px rgba(239, 68, 68, 0.4); }

.script-tag {
  font-weight: 600;
  font-size: 0.875rem;
}

.time {
  font-size: 0.75rem;
  color: var(--color-text-muted);
}

.btn-outline-small {
  background: transparent;
  border: 1px solid var(--color-border);
  color: var(--color-text-sub);
  padding: 0.25rem 0.75rem;
  border-radius: 99px;
  font-size: 0.75rem;
  cursor: pointer;
  transition: all 0.2s;
}

.btn-outline-small:hover:not(:disabled) {
  border-color: var(--color-primary);
  color: var(--color-primary);
  background: rgba(16, 185, 129, 0.1);
}

.feed-args {
  font-size: 0.8125rem;
  color: var(--color-text-sub);
  margin-bottom: 0.75rem;
}

.feed-args code {
  background: var(--color-surface-hover);
  padding: 0.125rem 0.375rem;
  border-radius: 4px;
  font-family: ui-monospace, monospace;
}

.terminal-block {
  background: var(--color-terminal-bg);
  color: var(--color-terminal-text);
  padding: 1rem;
  border-radius: var(--radius-sm);
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  font-size: 0.8125rem;
  line-height: 1.6;
  max-height: 250px;
  overflow-y: auto;
}

.terminal-block pre {
  margin: 0;
  white-space: pre-wrap;
  word-wrap: break-word;
}

.stdout { color: #e2e8f0; }
.stderr { color: #fca5a5; margin-top: 0.5rem; }

</style>
