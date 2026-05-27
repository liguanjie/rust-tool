<script setup lang="ts">
import { AlertTriangle } from '@lucide/vue'

defineProps<{
  title: string
  message: string
  warning?: string
  confirmText: string
  loading?: boolean
  tone?: 'default' | 'danger'
}>()

defineEmits<{
  cancel: []
  confirm: []
}>()
</script>

<template>
  <div class="confirm-backdrop" @click.self="$emit('cancel')">
    <section
      class="confirm-dialog"
      :class="{ 'confirm-dialog--danger': tone === 'danger' }"
      role="dialog"
      aria-modal="true"
      aria-labelledby="confirm-dialog-title"
    >
      <header class="confirm-header">
        <span class="confirm-icon" :class="{ 'confirm-icon--danger': tone === 'danger' }">
          <AlertTriangle class="h-5 w-5" aria-hidden="true" />
        </span>
        <div>
          <h3 id="confirm-dialog-title">{{ title }}</h3>
          <p>{{ message }}</p>
        </div>
      </header>
      <p v-if="warning" class="confirm-warning">{{ warning }}</p>
      <footer class="confirm-footer">
        <button class="secondary-button" type="button" :disabled="loading" @click="$emit('cancel')">
          取消
        </button>
        <button
          :class="tone === 'danger' ? 'danger-button' : 'primary-button compact-primary'"
          type="button"
          :disabled="loading"
          @click="$emit('confirm')"
        >
          <AlertTriangle v-if="tone === 'danger'" class="h-4 w-4" aria-hidden="true" />
          <span>{{ loading ? '执行中' : confirmText }}</span>
        </button>
      </footer>
    </section>
  </div>
</template>
