<script setup lang="ts">
import { Download } from '@lucide/vue'

const props = defineProps<{
  text: string
  filename: string
}>()

function downloadText() {
  if (!props.text) return
  const blob = new Blob([props.text], { type: 'text/yaml;charset=utf-8' })
  const url = URL.createObjectURL(blob)
  const link = document.createElement('a')
  link.href = url
  link.download = props.filename
  link.click()
  URL.revokeObjectURL(url)
}
</script>

<template>
  <button class="icon-button" type="button" title="下载 YAML" @click="downloadText">
    <Download class="h-4 w-4" aria-hidden="true" />
    <span>下载</span>
  </button>
</template>
