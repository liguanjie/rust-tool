<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import {
  Archive,
  BarChart3,
  Database,
  FolderOpen,
  HardDrive,
  RefreshCw,
  RotateCcw,
  Save,
  Trash2,
  Upload,
  Wrench,
} from '@lucide/vue'
import { message, Modal, theme } from 'ant-design-vue'
import {
  backupProgramDatabase,
  clearProgramDatabaseHistory,
  compactProgramDatabase,
  type DatabaseHealth,
  type DatabaseFileStats,
  type DatabaseStorageDiagnostics,
  deleteLegacyProgramDatabase,
  getProgramSettings,
  type LegacyDatabaseInfo,
  openProgramDatabaseDirectory,
  restoreProgramDatabase,
  saveProgramSettings,
  type ProgramSettingsState,
} from '../api/programSettings'

const { token } = theme.useToken()
const BYTE_SCALE = 1024
const BYTE_PRECISION = 1

const loading = ref(false)
const saving = ref(false)
const maintenanceAction = ref('')
const isDesktop = ref(false)
const error = ref('')
const databasePath = ref('')
const defaultDatabasePath = ref('')
const effectiveDatabasePath = ref('')
const lastSavedDatabasePath = ref('')
const databaseHealth = ref<DatabaseHealth>(defaultDatabaseHealth())
const databaseStats = ref<DatabaseFileStats>(defaultDatabaseFileStats())
const databaseDiagnostics = ref<DatabaseStorageDiagnostics>(defaultDatabaseStorageDiagnostics())
const legacyDatabase = ref<LegacyDatabaseInfo>(defaultLegacyDatabaseInfo())

const usesDefaultDatabasePath = computed(() => databasePath.value.trim().length === 0)
const hasChanges = computed(() => databasePath.value !== lastSavedDatabasePath.value)
const isMaintenanceDisabled = computed(
  () => !isDesktop.value || loading.value || saving.value || maintenanceAction.value.length > 0,
)
const databasePathStatus = computed(() =>
  usesDefaultDatabasePath.value ? '使用默认位置' : '使用自定义位置',
)
const databasePathStatusColor = computed(() =>
  usesDefaultDatabasePath.value ? 'default' : 'processing',
)
const databaseHealthLabel = computed(() => {
  if (databaseHealth.value.status === 'ready') return '已就绪'
  if (databaseHealth.value.status === 'error') return '异常'
  return '未检查'
})
const databaseHealthColor = computed(() => {
  if (databaseHealth.value.status === 'ready') return 'success'
  if (databaseHealth.value.status === 'error') return 'error'
  return 'default'
})
const schemaVersionLabel = computed(() =>
  databaseHealth.value.schemaVersion === null ? '-' : databaseHealth.value.schemaVersion,
)
const databaseSizeLabel = computed(() => formatBytes(databaseStats.value.totalSizeBytes))
const mainDatabaseSizeLabel = computed(() => formatBytes(databaseStats.value.mainFileSizeBytes))
const walDatabaseSizeLabel = computed(() => formatBytes(databaseStats.value.walFileSizeBytes))
const shmDatabaseSizeLabel = computed(() => formatBytes(databaseStats.value.shmFileSizeBytes))
const legacyDatabaseSizeLabel = computed(() => formatBytes(legacyDatabase.value.totalSizeBytes))
const hasLegacyDatabase = computed(() => legacyDatabase.value.exists)

onMounted(() => {
  void loadSettings()
})

async function loadSettings() {
  loading.value = true
  error.value = ''
  try {
    const tauriCore = await import('@tauri-apps/api/core')
    isDesktop.value = tauriCore.isTauri()
    applySettingsState(await getProgramSettings())
  } catch (caught) {
    error.value = caught instanceof Error ? caught.message : '读取程序配置失败'
  } finally {
    loading.value = false
  }
}

async function backupDatabase() {
  if (!isDesktop.value) {
    message.info('Web 模式下不支持备份本地 SQLite 数据库')
    return
  }

  maintenanceAction.value = 'backup'
  error.value = ''
  try {
    const result = await backupProgramDatabase()
    databaseStats.value = result.databaseStats
    message.success(`数据库已备份到 ${result.backupPath}`)
  } catch (caught) {
    error.value = caught instanceof Error ? caught.message : '备份数据库失败'
  } finally {
    maintenanceAction.value = ''
  }
}

async function compactDatabase() {
  if (!isDesktop.value) {
    message.info('Web 模式下不支持压缩本地 SQLite 数据库')
    return
  }

  maintenanceAction.value = 'compact'
  error.value = ''
  try {
    applySettingsState(await compactProgramDatabase())
    message.success('数据库已压缩')
  } catch (caught) {
    error.value = caught instanceof Error ? caught.message : '压缩数据库失败'
  } finally {
    maintenanceAction.value = ''
  }
}

async function chooseBackupAndRestore() {
  if (!isDesktop.value) {
    message.info('Web 模式下不支持恢复本地 SQLite 数据库')
    return
  }

  const dialog = await import('@tauri-apps/plugin-dialog')
  const selected = await dialog.open({
    directory: false,
    filters: [
      {
        name: 'SQLite 数据库',
        extensions: ['db', 'sqlite'],
      },
    ],
    multiple: false,
    title: '选择要恢复的数据库备份',
  })

  if (typeof selected !== 'string') return
  Modal.confirm({
    title: '从备份恢复数据库',
    content: `恢复前会自动备份当前数据库。恢复来源：${selected}`,
    okText: '恢复',
    okType: 'danger',
    cancelText: '取消',
    onOk: () => restoreDatabase(selected),
  })
}

async function restoreDatabase(backupPath: string) {
  maintenanceAction.value = 'restore'
  error.value = ''
  try {
    const result = await restoreProgramDatabase(backupPath)
    applySettingsState(result.state)
    message.success(`数据库已恢复，恢复前备份保存在 ${result.safetyBackupPath}`)
  } catch (caught) {
    error.value = caught instanceof Error ? caught.message : '恢复数据库失败'
    throw caught
  } finally {
    maintenanceAction.value = ''
  }
}

async function openDatabaseDirectory() {
  if (!isDesktop.value) {
    message.info('Web 模式下不支持打开本地目录')
    return
  }

  maintenanceAction.value = 'open'
  error.value = ''
  try {
    await openProgramDatabaseDirectory()
  } catch (caught) {
    error.value = caught instanceof Error ? caught.message : '打开数据库目录失败'
  } finally {
    maintenanceAction.value = ''
  }
}

function confirmDeleteLegacyDatabase() {
  if (!isDesktop.value) {
    message.info('Web 模式下没有本地旧数据库可清理')
    return
  }
  if (!hasLegacyDatabase.value) {
    message.info('未发现旧数据库文件')
    return
  }

  Modal.confirm({
    title: '清理旧数据库',
    content: `将删除旧库文件及 sidecar 文件，不做迁移。旧库路径：${legacyDatabase.value.path}`,
    okText: '删除旧库',
    okType: 'danger',
    cancelText: '取消',
    onOk: deleteLegacyDatabase,
  })
}

async function deleteLegacyDatabase() {
  maintenanceAction.value = 'legacy'
  error.value = ''
  try {
    applySettingsState(await deleteLegacyProgramDatabase())
    message.success('旧数据库已清理')
  } catch (caught) {
    error.value = caught instanceof Error ? caught.message : '清理旧数据库失败'
    throw caught
  } finally {
    maintenanceAction.value = ''
  }
}

function confirmClearDatabaseHistory() {
  if (!isDesktop.value) {
    message.info('Web 模式下没有本地 SQLite 历史可清理')
    return
  }

  Modal.confirm({
    title: '清理历史数据',
    content: '将清理 Agent 执行历史和 OSV 命令历史，保留项目清单和最后一次完整扫描结果。',
    okText: '清理',
    okType: 'danger',
    cancelText: '取消',
    onOk: clearDatabaseHistory,
  })
}

async function clearDatabaseHistory() {
  maintenanceAction.value = 'clear'
  error.value = ''
  try {
    applySettingsState(await clearProgramDatabaseHistory())
    message.success('历史数据已清理')
  } catch (caught) {
    error.value = caught instanceof Error ? caught.message : '清理历史数据失败'
    throw caught
  } finally {
    maintenanceAction.value = ''
  }
}

async function saveSettings() {
  saving.value = true
  error.value = ''
  try {
    applySettingsState(
      await saveProgramSettings({
        databasePath: databasePath.value,
      }),
    )
    message.success('程序配置已保存')
  } catch (caught) {
    error.value = caught instanceof Error ? caught.message : '保存程序配置失败'
  } finally {
    saving.value = false
  }
}

async function chooseDatabaseDirectory() {
  if (!isDesktop.value) {
    message.info('Web 模式下请直接输入数据库文件路径')
    return
  }

  const dialog = await import('@tauri-apps/plugin-dialog')
  const pathApi = await import('@tauri-apps/api/path')
  const selected = await dialog.open({
    directory: true,
    multiple: false,
    title: '选择数据库存放目录',
  })

  if (typeof selected !== 'string') return
  databasePath.value = await pathApi.join(selected, 'rusttool.db')
}

function restoreDefaultPath() {
  databasePath.value = ''
}

function applySettingsState(state: ProgramSettingsState) {
  databasePath.value = state.settings.databasePath
  defaultDatabasePath.value = state.defaultDatabasePath
  effectiveDatabasePath.value = state.effectiveDatabasePath
  databaseHealth.value = state.databaseHealth
  databaseStats.value = state.databaseStats
  databaseDiagnostics.value = state.databaseDiagnostics
  legacyDatabase.value = state.legacyDatabase
  lastSavedDatabasePath.value = state.settings.databasePath
}

function defaultDatabaseHealth(): DatabaseHealth {
  return {
    databasePath: '',
    status: 'unavailable',
    databaseExists: false,
    parentDirectoryExists: false,
    schemaVersion: null,
    appliedMigrations: 0,
    message: '尚未检查数据库状态。',
  }
}

function defaultDatabaseFileStats(): DatabaseFileStats {
  return {
    databasePath: '',
    mainFileSizeBytes: 0,
    walFileSizeBytes: 0,
    shmFileSizeBytes: 0,
    totalSizeBytes: 0,
  }
}

function defaultDatabaseStorageDiagnostics(): DatabaseStorageDiagnostics {
  return {
    totalRecords: 0,
    recordCounts: [],
  }
}

function defaultLegacyDatabaseInfo(): LegacyDatabaseInfo {
  return {
    path: '',
    exists: false,
    mainFileSizeBytes: 0,
    walFileSizeBytes: 0,
    shmFileSizeBytes: 0,
    totalSizeBytes: 0,
  }
}

function formatBytes(bytes: number) {
  if (!Number.isFinite(bytes) || bytes <= 0) return '0 B'

  const units = ['B', 'KB', 'MB', 'GB'] as const
  let value = bytes
  let unitIndex = 0
  while (value >= BYTE_SCALE && unitIndex < units.length - 1) {
    value /= BYTE_SCALE
    unitIndex += 1
  }

  return `${value.toFixed(unitIndex === 0 ? 0 : BYTE_PRECISION)} ${units[unitIndex]}`
}
</script>

<template>
  <div style="padding: 24px; max-width: 1040px; margin: 0 auto;">
    <a-page-header
      title="程序配置"
      sub-title="管理 RustTool 的本机运行配置。"
      style="padding-left: 0; padding-right: 0;"
    >
      <template #tags>
        <a-tag color="blue">系统设置</a-tag>
      </template>
      <template #extra>
        <a-button :loading="loading" @click="loadSettings">
          <span class="settings-button-content">
            <RefreshCw :size="16" />
            刷新
          </span>
        </a-button>
      </template>
    </a-page-header>

    <a-alert
      v-if="error"
      type="error"
      show-icon
      :message="error"
      style="margin-bottom: 16px;"
    />

    <a-row :gutter="[16, 16]" style="margin-bottom: 24px;">
      <a-col :xs="24" :md="12" :lg="6">
        <a-card>
          <a-statistic title="数据库位置" :value="databasePathStatus" />
          <div style="margin-top: 8px;">
            <a-tag :color="databasePathStatusColor">
              SQLite
            </a-tag>
          </div>
        </a-card>
      </a-col>
      <a-col :xs="24" :md="12" :lg="6">
        <a-card>
          <a-statistic title="数据库状态" :value="databaseHealthLabel" />
          <div style="margin-top: 8px;">
            <a-tag :color="databaseHealthColor">
              schema {{ schemaVersionLabel }}
            </a-tag>
          </div>
        </a-card>
      </a-col>
      <a-col :xs="24" :md="12" :lg="6">
        <a-card>
          <a-statistic title="数据库大小" :value="databaseSizeLabel" />
          <div style="margin-top: 8px;">
            <a-tag color="blue">
              SQLite 文件
            </a-tag>
          </div>
        </a-card>
      </a-col>
      <a-col :xs="24" :md="12" :lg="6">
        <a-card>
          <a-statistic title="待保存改动" :value="hasChanges ? '有' : '无'" />
          <div style="margin-top: 8px;">
            <a-tag :color="hasChanges ? 'warning' : 'success'">
              {{ hasChanges ? '未保存' : '已同步' }}
            </a-tag>
          </div>
        </a-card>
      </a-col>
    </a-row>

    <a-card>
      <template #title>
        <span style="display: inline-flex; align-items: center; gap: 8px;">
          <Database :size="18" />
          数据库
        </span>
      </template>
      <template #extra>
        <a-tag :color="databasePathStatusColor">
          {{ databasePathStatus }}
        </a-tag>
      </template>

      <a-form layout="vertical">
        <a-form-item label="SQLite 数据库文件路径">
          <a-input
            v-model:value="databasePath"
            :placeholder="defaultDatabasePath || './data/rusttool.db'"
            allow-clear
          />
          <div class="settings-actions">
            <a-button @click="chooseDatabaseDirectory">
              <span class="settings-button-content">
                <FolderOpen :size="16" />
                选择目录
              </span>
            </a-button>
            <a-button @click="restoreDefaultPath">
              <span class="settings-button-content">
                <RotateCcw :size="16" />
                恢复默认
              </span>
            </a-button>
            <a-button
              type="primary"
              :loading="saving"
              :disabled="!hasChanges"
              @click="saveSettings"
            >
              <span class="settings-button-content">
                <Save :size="16" />
                保存
              </span>
            </a-button>
          </div>
        </a-form-item>
      </a-form>

      <a-divider />

      <a-descriptions :column="1" size="small" bordered>
        <a-descriptions-item label="当前生效路径">
          <span class="settings-path">
            {{ effectiveDatabasePath || defaultDatabasePath || './data/rusttool.db' }}
          </span>
        </a-descriptions-item>
        <a-descriptions-item label="默认路径">
          <span class="settings-path">
            {{ defaultDatabasePath || './data/rusttool.db' }}
          </span>
        </a-descriptions-item>
        <a-descriptions-item label="健康状态">
          <a-tag :color="databaseHealthColor">
            {{ databaseHealthLabel }}
          </a-tag>
          <span style="margin-left: 8px;">
            {{ databaseHealth.message }}
          </span>
        </a-descriptions-item>
        <a-descriptions-item label="Schema 版本">
          {{ schemaVersionLabel }}
        </a-descriptions-item>
        <a-descriptions-item label="已应用迁移">
          {{ databaseHealth.appliedMigrations }}
        </a-descriptions-item>
        <a-descriptions-item label="文件状态">
          数据库文件：{{ databaseHealth.databaseExists ? '存在' : '不存在' }}
          <a-divider type="vertical" />
          父目录：{{ databaseHealth.parentDirectoryExists ? '存在' : '不存在' }}
        </a-descriptions-item>
        <a-descriptions-item label="文件大小">
          合计：{{ databaseSizeLabel }}
          <a-divider type="vertical" />
          主库：{{ mainDatabaseSizeLabel }}
          <a-divider type="vertical" />
          WAL：{{ walDatabaseSizeLabel }}
          <a-divider type="vertical" />
          SHM：{{ shmDatabaseSizeLabel }}
        </a-descriptions-item>
        <a-descriptions-item label="旧数据库">
          <a-tag :color="hasLegacyDatabase ? 'warning' : 'success'">
            {{ hasLegacyDatabase ? '发现旧库' : '未发现' }}
          </a-tag>
          <span v-if="hasLegacyDatabase" class="settings-path" style="margin-left: 8px;">
            {{ legacyDatabase.path }}（{{ legacyDatabaseSizeLabel }}）
          </span>
        </a-descriptions-item>
        <a-descriptions-item label="运行模式">
          <a-tag :color="isDesktop ? 'success' : 'default'">
            {{ isDesktop ? 'Tauri' : 'Browser' }}
          </a-tag>
        </a-descriptions-item>
      </a-descriptions>

      <a-divider />

      <div class="maintenance-actions">
        <a-button :loading="loading" @click="loadSettings">
          <span class="settings-button-content">
            <RefreshCw :size="16" />
            刷新状态
          </span>
        </a-button>
        <a-button
          :disabled="isMaintenanceDisabled"
          :loading="maintenanceAction === 'open'"
          @click="openDatabaseDirectory"
        >
          <span class="settings-button-content">
            <FolderOpen :size="16" />
            打开目录
          </span>
        </a-button>
        <a-button
          :disabled="isMaintenanceDisabled"
          :loading="maintenanceAction === 'backup'"
          @click="backupDatabase"
        >
          <span class="settings-button-content">
            <Archive :size="16" />
            备份数据库
          </span>
        </a-button>
        <a-button
          :disabled="isMaintenanceDisabled"
          :loading="maintenanceAction === 'compact'"
          @click="compactDatabase"
        >
          <span class="settings-button-content">
            <Wrench :size="16" />
            压缩数据库
          </span>
        </a-button>
        <a-button
          danger
          :disabled="isMaintenanceDisabled"
          :loading="maintenanceAction === 'restore'"
          @click="chooseBackupAndRestore"
        >
          <span class="settings-button-content">
            <Upload :size="16" />
            从备份恢复
          </span>
        </a-button>
        <a-button
          danger
          :disabled="isMaintenanceDisabled"
          :loading="maintenanceAction === 'clear'"
          @click="confirmClearDatabaseHistory"
        >
          <span class="settings-button-content">
            <Trash2 :size="16" />
            清理历史
          </span>
        </a-button>
        <a-button
          danger
          :disabled="isMaintenanceDisabled || !hasLegacyDatabase"
          :loading="maintenanceAction === 'legacy'"
          @click="confirmDeleteLegacyDatabase"
        >
          <span class="settings-button-content">
            <Trash2 :size="16" />
            清理旧库
          </span>
        </a-button>
      </div>
    </a-card>

    <a-card style="margin-top: 16px;">
      <template #title>
        <span style="display: inline-flex; align-items: center; gap: 8px;">
          <BarChart3 :size="18" />
          存储诊断
        </span>
      </template>
      <template #extra>
        <a-tag color="blue">
          总记录 {{ databaseDiagnostics.totalRecords }}
        </a-tag>
      </template>

      <a-list
        v-if="databaseDiagnostics.recordCounts.length > 0"
        size="small"
        :data-source="databaseDiagnostics.recordCounts"
      >
        <template #renderItem="{ item }">
          <a-list-item>
            <span>{{ item.label }}</span>
            <template #extra>
              <a-tag>{{ item.count }}</a-tag>
            </template>
          </a-list-item>
        </template>
      </a-list>
      <a-empty v-else description="暂无诊断数据" />
    </a-card>

    <a-card style="margin-top: 16px;">
      <template #title>
        <span style="display: inline-flex; align-items: center; gap: 8px;">
          <HardDrive :size="18" />
          配置边界
        </span>
      </template>

      <a-list size="small">
        <a-list-item>
          SQLite 只保存结构化配置、历史索引和扫描摘要。
        </a-list-item>
        <a-list-item>
          YAML、HTML、JSON 报告继续保存为文件，数据库只记录路径。
        </a-list-item>
        <a-list-item>
          第三方工具自身配置继续保留在原项目或原工具目录中。
        </a-list-item>
      </a-list>
    </a-card>
  </div>
</template>

<style scoped>
.settings-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-top: 12px;
}

.maintenance-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.settings-button-content {
  align-items: center;
  display: inline-flex;
  gap: 6px;
  justify-content: center;
  white-space: nowrap;
}

.settings-path {
  color: v-bind('token.colorText');
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", monospace;
  overflow-wrap: anywhere;
}
</style>
