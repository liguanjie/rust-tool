<script setup lang="ts">
import CopyButton from './CopyButton.vue'
import DownloadButton from './DownloadButton.vue'

defineProps<{
  yaml: string
  copied: boolean
  filename: string
  nodeAddress: string
}>()

const emit = defineEmits<{
  copied: []
}>()
</script>

<template>
  <section class="result-panel">
    <header class="result-header">
      <div class="result-title">
        <h3>YAML 结果</h3>
        <p v-if="nodeAddress" class="node-address">节点地址：{{ nodeAddress }}</p>
      </div>
      <div class="result-actions">
        <span v-if="copied" class="copy-status">已复制</span>
        <CopyButton :text="yaml" @copied="emit('copied')" />
        <DownloadButton :text="yaml" :filename="filename" />
      </div>
    </header>
    <pre class="yaml-output">{{ yaml || '转换结果会显示在这里' }}</pre>
  </section>
</template>
