<script setup lang="ts">
import { AlertTriangle } from '@lucide/vue'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'

defineProps<{
  title: string
  message: string
  warning?: string
  confirmText: string
  loading?: boolean
  tone?: 'default' | 'danger'
}>()

const emit = defineEmits<{
  cancel: []
  confirm: []
}>()

const open = true
</script>

<template>
  <Dialog :open="open" @update:open="(val) => { if(!val) emit('cancel') }">
    <DialogContent>
      <DialogHeader>
        <DialogTitle class="flex items-center gap-2" :class="{ 'text-destructive': tone === 'danger' }">
          <AlertTriangle class="h-5 w-5" />
          {{ title }}
        </DialogTitle>
        <DialogDescription>
          {{ message }}
        </DialogDescription>
      </DialogHeader>
      
      <p v-if="warning" class="text-sm font-medium text-destructive bg-destructive/10 p-3 rounded-md border border-destructive/20 mt-2">
        {{ warning }}
      </p>

      <DialogFooter>
        <Button variant="outline" :disabled="loading" @click="emit('cancel')">
          取消
        </Button>
        <Button 
          :variant="tone === 'danger' ? 'destructive' : 'default'" 
          :disabled="loading" 
          @click="emit('confirm')"
        >
          <AlertTriangle v-if="tone === 'danger'" class="h-4 w-4 mr-2" />
          {{ loading ? '执行中' : confirmText }}
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
