<script setup lang="ts">
import { Copy } from '@lucide/vue'
import { Button } from '@/components/ui/button'

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
  <Button variant="secondary" size="sm" type="button" title="复制结果" @click="copyText">
    <Copy class="h-4 w-4 mr-2" aria-hidden="true" />
    复制
  </Button>
</template>
