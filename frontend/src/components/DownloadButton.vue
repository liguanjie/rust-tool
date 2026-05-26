<script setup lang="ts">
import { Download } from '@lucide/vue'
import { ref } from 'vue'
import { downloadYaml } from '../api/download'

const props = defineProps<{
  text: string
  filename: string
}>()

const saving = ref(false)
const message = ref('')

async function downloadText() {
  if (!props.text || saving.value) return

  message.value = ''
  saving.value = true
  try {
    const result = await downloadYaml(props.text, props.filename)
    message.value = result ? `已保存到 ${result.path}` : ''
  } catch (caught) {
    message.value = caught instanceof Error ? caught.message : String(caught)
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <span class="download-action">
    <button class="icon-button" type="button" title="下载 YAML" :disabled="saving || !text" @click="downloadText">
      <Download class="h-4 w-4" aria-hidden="true" />
      <span>{{ saving ? '保存中' : '下载' }}</span>
    </button>
    <span v-if="message" class="download-message">{{ message }}</span>
  </span>
</template>
