<script setup lang="ts">
import CopyButton from './CopyButton.vue'
import DownloadButton from './DownloadButton.vue'
import { Card, CardHeader, CardTitle, CardDescription, CardContent } from '@/components/ui/card'

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
  <Card class="mt-6">
    <CardHeader class="flex flex-row items-center justify-between space-y-0 pb-4">
      <div>
        <CardTitle class="text-xl">YAML 结果</CardTitle>
        <CardDescription v-if="nodeAddress" class="mt-1">
          节点地址：{{ nodeAddress }}
        </CardDescription>
      </div>
      <div class="flex items-center gap-2">
        <span v-if="copied" class="text-sm text-muted-foreground mr-2">已复制</span>
        <CopyButton :text="yaml" @copied="emit('copied')" />
        <DownloadButton :text="yaml" :filename="filename" />
      </div>
    </CardHeader>
    <CardContent>
      <pre class="bg-muted p-4 rounded-lg overflow-x-auto text-sm font-mono text-muted-foreground">{{ yaml || '转换结果会显示在这里' }}</pre>
    </CardContent>
  </Card>
</template>
