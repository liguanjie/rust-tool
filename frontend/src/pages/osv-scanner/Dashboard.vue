<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { Shield, FolderPlus, Trash2, RefreshCw, Activity, Layers, Clock } from 'lucide-vue-next'
import { useOsvScannerStore } from '../../stores/osvScanner'

const router = useRouter()
const osv = useOsvScannerStore()

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

const globalHealthColor = computed(() => {
  const score = osv.globalHealthScore
  if (typeof score !== 'number') return 'default'
  if (score >= 90) return 'success'
  if (score >= 70) return 'warning'
  return 'error'
})

const latestScanLabel = computed(() => {
  const latest = osv.projects
    .map((project) => Number(project.lastScanned))
    .filter((value) => Number.isFinite(value) && value > 0)
    .sort((a, b) => b - a)[0]
  return latest ? new Date(latest).toLocaleString() : '未扫描'
})

function healthColor(score?: number) {
  if (typeof score !== 'number') return 'default'
  if (score >= 90) return 'success'
  if (score >= 70) return 'warning'
  return 'error'
}

function projectHealthLabel(score?: number) {
  if (typeof score !== 'number') return '未扫描'
  return `${score} 分`
}

const tableColumns = [
  { title: '项目名称', dataIndex: 'name', key: 'name' },
  { title: '项目路径', dataIndex: 'path', key: 'path', ellipsis: true },
  { title: '健康状况', dataIndex: 'healthScore', key: 'healthScore', width: 120 },
  { title: '操作', key: 'action', width: 100, align: 'center' },
]

onMounted(() => {
  void osv.load()
})

const showWebAddModal = ref(false)
const webProjectPath = ref('')

async function chooseDirectory() {
  const tauriCore = await import('@tauri-apps/api/core').catch(() => null)
  if (tauriCore && tauriCore.isTauri()) {
    const dialog = await import('@tauri-apps/plugin-dialog')
    const selected = await dialog.open({
      directory: true,
      multiple: false,
    })
    if (typeof selected === 'string') {
      await osv.addProject(selected)
    }
  } else {
    webProjectPath.value = ''
    showWebAddModal.value = true
  }
}

async function handleWebAddProject() {
  const path = webProjectPath.value.trim()
  if (!path) return
  await osv.addProject(path)
  showWebAddModal.value = false
  webProjectPath.value = ''
}

function goToProject(path: string) {
  osv.selectProject(path)
  router.push({ name: 'osv-scanner-project', params: { id: encodeURIComponent(path) } })
}
</script>

<template>
  <div style="padding: 24px; max-width: 1200px; margin: 0 auto;">
    <a-page-header
      title="OSV 漏洞大盘"
      sub-title="管理本机项目的依赖漏洞扫描、报告导出和命令审计。"
      style="padding-left: 0; padding-right: 0;"
    >
      <template #tags>
        <a-tag color="blue">项目安全</a-tag>
      </template>
    </a-page-header>

    <div class="mt-4">
      <div style="display: grid; grid-template-columns: repeat(4, 1fr); gap: 16px; margin-bottom: 24px;">
        <a-card size="small" :bordered="false">
          <a-statistic title="服务状态" :value="osv.installStatus?.installed ? '已就绪' : '等待检测'">
            <template #prefix><Shield class="w-4 h-4 mr-2 opacity-60 inline-block align-text-bottom" /></template>
          </a-statistic>
          <div class="text-xs text-gray-500 mt-2 truncate" :title="installVersionLine">
            {{ installVersionLine }}
          </div>
        </a-card>

        <a-card size="small" :bordered="false">
          <a-statistic title="全局健康分" :value="osv.globalHealthScore ?? '--'">
            <template #prefix><Activity class="w-4 h-4 mr-2 opacity-60 inline-block align-text-bottom" /></template>
          </a-statistic>
          <div class="mt-2">
            <a-tag :color="globalHealthColor">{{ globalHealthLabel }}</a-tag>
          </div>
        </a-card>

        <a-card size="small" :bordered="false">
          <a-statistic title="监控项目" :value="osv.projects.length">
            <template #prefix><Layers class="w-4 h-4 mr-2 opacity-60 inline-block align-text-bottom" /></template>
            <template #suffix><span class="text-xs">个</span></template>
          </a-statistic>
          <div class="text-xs text-gray-500 mt-2">有效路径</div>
        </a-card>

        <a-card size="small" :bordered="false">
          <a-statistic title="最近扫描" :value="latestScanLabel" :valueStyle="latestScanLabel === '未扫描' ? {} : { fontSize: '16px' }">
            <template #prefix><Clock class="w-4 h-4 mr-2 opacity-60 inline-block align-text-bottom" /></template>
          </a-statistic>
          <div class="mt-2">
            <a-button type="link" size="small" @click="osv.refreshInstallStatus" style="padding: 0;">
              <template #icon><RefreshCw class="w-3 h-3 mr-1 inline-block align-text-bottom" /></template>
              刷新状态
            </a-button>
          </div>
        </a-card>
      </div>

      <a-card class="mt-6" title="受监控的项目" :bordered="false">
        <template #extra>
          <a-button type="primary" @click="chooseDirectory">
            <template #icon><FolderPlus class="w-4 h-4 mr-1 inline-block align-text-bottom" /></template>
            添加监控项目
          </a-button>
        </template>

        <a-table 
          :dataSource="osv.projects" 
          :columns="tableColumns" 
          rowKey="path" 
          :pagination="false"
          size="middle"
          class="cursor-pointer"
          :customRow="(record: any) => ({
            onClick: () => goToProject(record.path)
          })"
        >
          <template #bodyCell="{ column, record }">
            <template v-if="column.key === 'name'">
              <span class="font-semibold">{{ record.name }}</span>
            </template>
            <template v-else-if="column.key === 'healthScore'">
              <a-tag :color="healthColor(record.healthScore)">
                {{ projectHealthLabel(record.healthScore) }}
              </a-tag>
            </template>
            <template v-else-if="column.key === 'action'">
              <a-button type="text" danger @click.stop="osv.removeProject(record.path)">
                <template #icon><Trash2 class="w-4 h-4 inline-block align-text-bottom" /></template>
              </a-button>
            </template>
          </template>
        </a-table>
      </a-card>
    </div>

    <a-modal
      v-model:open="showWebAddModal"
      title="手动添加监控项目"
      @ok="handleWebAddProject"
      okText="确认"
      cancelText="取消"
    >
      <div style="padding: 12px 0;">
        <p class="text-sm text-gray-500 mb-4">
          由于浏览器安全沙箱限制，Web 模式下无法直接打开系统目录选择器。请手动输入或粘贴项目在您电脑上的<b>绝对路径</b>（例如：/Users/username/projects/my-project）。
        </p>
        <a-input
          v-model:value="webProjectPath"
          placeholder="请输入项目的绝对路径"
          allow-clear
          @pressEnter="handleWebAddProject"
        />
      </div>
    </a-modal>
  </div>
</template>
