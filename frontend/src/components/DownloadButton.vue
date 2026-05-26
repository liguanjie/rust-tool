<script setup lang="ts">
import { CheckCircle2, Download, X, XCircle } from '@lucide/vue'
import { ref } from 'vue'
import { downloadYaml } from '../api/download'

const props = defineProps<{
  text: string
  filename: string
}>()

const saving = ref(false)
const toast = ref<{ type: 'success' | 'error'; title: string; detail: string } | null>(null)
let toastTimer: number | undefined

async function downloadText() {
  if (!props.text || saving.value) return

  hideToast()
  saving.value = true
  try {
    const result = await downloadYaml(props.text, props.filename)
    showToast({
      type: 'success',
      title: 'YAML 已保存',
      detail: result?.path ?? props.filename,
    })
  } catch (caught) {
    showToast({
      type: 'error',
      title: '保存失败',
      detail: caught instanceof Error ? caught.message : String(caught),
    })
  } finally {
    saving.value = false
  }
}

function showToast(nextToast: { type: 'success' | 'error'; title: string; detail: string }) {
  toast.value = nextToast
  window.clearTimeout(toastTimer)
  toastTimer = window.setTimeout(hideToast, 4200)
}

function hideToast() {
  window.clearTimeout(toastTimer)
  toast.value = null
}
</script>

<template>
  <span class="download-action">
    <button class="icon-button" type="button" title="下载 YAML" :disabled="saving || !text" @click="downloadText">
      <Download class="h-4 w-4" aria-hidden="true" />
      <span>{{ saving ? '保存中' : '下载' }}</span>
    </button>
    <Transition name="toast">
      <div v-if="toast" class="toast-message" :class="`toast-message--${toast.type}`" role="status">
        <CheckCircle2 v-if="toast.type === 'success'" class="toast-icon" aria-hidden="true" />
        <XCircle v-else class="toast-icon" aria-hidden="true" />
        <span class="toast-copy">
          <strong>{{ toast.title }}</strong>
          <small>{{ toast.detail }}</small>
        </span>
        <button class="toast-close" type="button" title="关闭提示" @click="hideToast">
          <X class="h-4 w-4" aria-hidden="true" />
        </button>
      </div>
    </Transition>
  </span>
</template>
