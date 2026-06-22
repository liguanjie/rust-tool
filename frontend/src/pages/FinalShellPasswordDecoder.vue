<script setup lang="ts">
import { computed, ref } from 'vue'
import { Copy, KeyRound, Loader2, ShieldCheck, Trash2 } from '@lucide/vue'
import { message, theme } from 'ant-design-vue'
import { decodeFinalShellPassword } from '../api/tools'

const { token } = theme.useToken()

const encryptedPassword = ref('')
const decodedPassword = ref('')
const error = ref('')
const loading = ref(false)
const copied = ref(false)

const canDecode = computed(() => encryptedPassword.value.trim().length > 0 && !loading.value)
const encryptedLength = computed(() => encryptedPassword.value.trim().length)
const resultStatus = computed(() => {
  if (loading.value) return '解密中'
  if (decodedPassword.value) return '已解密'
  return '待解密'
})
const resultStatusColor = computed(() => {
  if (loading.value) return 'warning'
  if (decodedPassword.value) return 'success'
  return 'default'
})
const resultSummary = computed(() => {
  if (!decodedPassword.value) return '未生成'
  return `${decodedPassword.value.length} 个字符`
})

async function decodePassword() {
  if (!canDecode.value) return

  error.value = ''
  decodedPassword.value = ''
  copied.value = false
  loading.value = true

  try {
    const result = await decodeFinalShellPassword({
      encryptedPassword: encryptedPassword.value,
    })
    decodedPassword.value = result.password
  } catch (caught) {
    error.value = caught instanceof Error ? caught.message : '解密失败'
  } finally {
    loading.value = false
  }
}

async function copyPassword() {
  if (!decodedPassword.value) return

  await navigator.clipboard.writeText(decodedPassword.value)
  copied.value = true
  message.success('已复制到剪贴板')
  window.setTimeout(() => {
    copied.value = false
  }, 1600)
}

function clearAll() {
  encryptedPassword.value = ''
  decodedPassword.value = ''
  error.value = ''
  copied.value = false
}

function clearDecodedPassword() {
  decodedPassword.value = ''
  copied.value = false
}
</script>

<template>
  <div style="padding: 24px; max-width: 1200px; margin: 0 auto;">
    <a-page-header
      title="FinalShell 密码解密"
      sub-title="本机解密 FinalShell 保存的加密密码字符串。"
      style="padding-left: 0; padding-right: 0;"
    >
      <template #tags>
        <a-tag color="purple">凭据工具</a-tag>
        <a-tag color="green">本机处理</a-tag>
      </template>
    </a-page-header>

    <a-row :gutter="16" style="margin-bottom: 24px;">
      <a-col :xs="24" :md="8">
        <a-card>
          <a-statistic title="解密状态" :value="resultStatus" />
          <div style="margin-top: 8px;">
            <a-tag :color="resultStatusColor">
              {{ decodedPassword ? '结果已生成' : '等待输入' }}
            </a-tag>
          </div>
        </a-card>
      </a-col>
      <a-col :xs="24" :md="8">
        <a-card>
          <a-statistic title="密文长度" :value="encryptedLength" />
        </a-card>
      </a-col>
      <a-col :xs="24" :md="8">
        <a-card>
          <a-statistic title="明文状态" :value="resultSummary" />
        </a-card>
      </a-col>
    </a-row>

    <a-row :gutter="24">
      <a-col :xs="24" :lg="14">
        <a-card title="解密输入" style="margin-bottom: 24px;">
          <a-alert
            type="warning"
            show-icon
            message="明文结果只保留在当前页面内存中，请在可信环境操作。"
            style="margin-bottom: 16px;"
          />

          <a-form layout="vertical">
            <a-form-item label="FinalShell 密文">
              <a-textarea
                v-model:value="encryptedPassword"
                :rows="8"
                placeholder="粘贴 FinalShell 保存的 password 字段"
                style="font-family: monospace;"
              />
            </a-form-item>
          </a-form>

          <div style="display: flex; gap: 12px; justify-content: flex-end;">
            <a-button :disabled="loading" @click="clearAll">
              <Trash2 class="h-4 w-4 mr-2 inline" />
              清空
            </a-button>
            <a-button type="primary" :disabled="!canDecode" :loading="loading" @click="decodePassword">
              <Loader2 v-if="loading" class="h-4 w-4 mr-2 inline animate-spin" />
              <KeyRound v-else class="h-4 w-4 mr-2 inline" />
              {{ loading ? '解密中' : '开始解密' }}
            </a-button>
          </div>

          <a-alert v-if="error" type="error" :message="error" show-icon style="margin-top: 16px;" />
        </a-card>
      </a-col>

      <a-col :xs="24" :lg="10">
        <a-card title="解密结果" style="margin-bottom: 24px;">
          <template #extra>
            <a-tag :color="decodedPassword ? 'success' : 'default'">
              <ShieldCheck v-if="decodedPassword" class="h-3.5 w-3.5 mr-1 inline" />
              {{ decodedPassword ? '已生成' : '待生成' }}
            </a-tag>
          </template>

          <div
            :style="{
              backgroundColor: token.colorBgLayout,
              border: `1px solid ${token.colorBorder}`,
              borderRadius: '6px',
              padding: '16px',
              marginBottom: '16px'
            }"
          >
            <a-input-password
              v-model:value="decodedPassword"
              :readonly="true"
              placeholder="解密完成后显示明文密码"
              size="large"
            />
          </div>

          <div style="display: flex; gap: 12px; justify-content: flex-end;">
            <a-button danger :disabled="!decodedPassword" @click="clearDecodedPassword">
              <Trash2 class="h-4 w-4 mr-2 inline" />
              清空明文
            </a-button>
            <a-button type="primary" :disabled="!decodedPassword" @click="copyPassword">
              <Copy class="h-4 w-4 mr-2 inline" />
              {{ copied ? '已复制' : '复制明文' }}
            </a-button>
          </div>
        </a-card>
      </a-col>
    </a-row>
  </div>
</template>
