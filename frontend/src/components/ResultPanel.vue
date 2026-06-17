<script setup lang="ts">
import { computed, ref } from 'vue'
import { CheckCircle2, FileCode2, Server, Copy, Download } from '@lucide/vue'
import { message, theme } from 'ant-design-vue'
import { downloadYaml } from '../api/download'

const { token } = theme.useToken()

const props = defineProps<{
  yaml: string
  copied: boolean
  filename: string
  nodeAddress: string
}>()

const emit = defineEmits<{
  copied: []
}>()

const hasYaml = computed(() => props.yaml.trim().length > 0)
const saving = ref(false)

async function copyText() {
  if (!props.yaml) return
  await navigator.clipboard.writeText(props.yaml)
  emit('copied')
  message.success('已复制到剪贴板')
}

async function downloadText() {
  if (!props.yaml || saving.value) return
  saving.value = true
  try {
    const result = await downloadYaml(props.yaml, props.filename)
    message.success(`YAML 已保存: ${result?.path ?? props.filename}`)
  } catch (caught) {
    message.error(`保存失败: ${caught instanceof Error ? caught.message : String(caught)}`)
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <a-card size="small" :title="hasYaml ? 'YAML 生成成功' : 'YAML 结果'" style="margin-bottom: 24px;">
    <template #extra>
      <a-tag :color="hasYaml ? 'success' : 'default'">
        <CheckCircle2 v-if="hasYaml" class="mr-1.5 h-3.5 w-3.5 inline" />
        {{ hasYaml ? '已生成' : '待生成' }}
      </a-tag>
    </template>
    
    <div v-if="hasYaml" style="margin-bottom: 16px; color: gray; font-size: 13px; display: flex; gap: 16px;">
      <span><FileCode2 class="h-3 w-3 inline" /> {{ filename }}</span>
      <span><Server class="h-3 w-3 inline" /> {{ nodeAddress || '等待节点地址' }}</span>
    </div>
    
    <div :style="{ backgroundColor: token.colorBgLayout, border: `1px solid ${token.colorBorder}`, borderRadius: '6px', padding: '12px', marginBottom: '16px', overflowX: 'auto', maxHeight: '400px' }">
      <pre :style="{ margin: 0, fontFamily: 'monospace', fontSize: '13px', color: token.colorText }">{{ yaml || '转换完成后会在这里显示配置内容' }}</pre>
    </div>
    
    <div style="display: flex; gap: 12px; justify-content: flex-end;">
      <a-button type="default" :disabled="!hasYaml" @click="copyText">
        <Copy class="h-4 w-4 mr-2 inline" /> {{ copied ? '已复制' : '复制' }}
      </a-button>
      <a-button type="primary" :disabled="!hasYaml || saving" :loading="saving" @click="downloadText">
        <Download class="h-4 w-4 mr-2 inline" /> 下载
      </a-button>
    </div>
  </a-card>
</template>
