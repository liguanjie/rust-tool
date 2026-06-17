<script setup lang="ts">
import { computed } from 'vue'
import { CheckCircle2, FileCode2, Server } from '@lucide/vue'
import CopyButton from './CopyButton.vue'
import DownloadButton from './DownloadButton.vue'

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
</script>

<template>
  <section class="result-panel yaml-result-panel">
    <header class="yaml-result-header">
      <div class="service-title">
        <span class="service-icon">
          <FileCode2 class="h-5 w-5" aria-hidden="true" />
        </span>
        <div>
          <h3>YAML 结果</h3>
          <p>{{ hasYaml ? filename : '转换完成后会在这里显示配置内容' }}</p>
        </div>
      </div>
      <span :class="hasYaml ? 'status-pill status-pill--good' : 'status-pill status-pill--muted'">
        <CheckCircle2 v-if="hasYaml" class="mr-1.5 h-3.5 w-3.5" aria-hidden="true" />
        {{ hasYaml ? '已生成' : '待生成' }}
      </span>
    </header>

    <div class="yaml-result-meta">
      <span>
        <FileCode2 class="h-4 w-4" aria-hidden="true" />
        {{ filename }}
      </span>
      <span>
        <Server class="h-4 w-4" aria-hidden="true" />
        {{ nodeAddress || '等待节点地址' }}
      </span>
    </div>

    <pre class="yaml-output">{{ yaml || '转换结果会显示在这里' }}</pre>

    <footer class="yaml-result-actions">
      <span v-if="copied" class="copy-status">已复制</span>
      <CopyButton :text="yaml" @copied="emit('copied')" />
      <DownloadButton :text="yaml" :filename="filename" />
    </footer>
  </section>
</template>
