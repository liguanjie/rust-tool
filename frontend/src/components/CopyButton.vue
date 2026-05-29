<script setup lang="ts">
import { Copy } from '@lucide/vue'

const props = defineProps<{
  text: string
}>()

const emit = defineEmits<{
  copied: []
}>()

async function copyText() {
  if (!props.text) return
  await navigator.clipboard.writeText(props.text)
  emit('copied')
}
</script>

<template>
  <button class="icon-button" type="button" title="复制结果" @click="copyText">
    <Copy class="h-4 w-4" aria-hidden="true" />
    <span class="button-label">复制</span>
  </button>
</template>
