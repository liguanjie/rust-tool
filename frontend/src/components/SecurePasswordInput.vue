<script setup lang="ts">
import { nextTick, onMounted, ref } from 'vue'
import { Eye, EyeOff } from '@lucide/vue'

const props = withDefaults(defineProps<{
  modelValue: string
  placeholder?: string
  autocomplete?: string
  inputClass?: string
  disabled?: boolean
  autofocus?: boolean
  showTitle?: string
  hideTitle?: string
}>(), {
  placeholder: '',
  autocomplete: 'current-password',
  inputClass: '',
  disabled: false,
  autofocus: false,
  showTitle: '显示密码',
  hideTitle: '隐藏密码',
})

const emit = defineEmits<{
  'update:modelValue': [value: string]
}>()

const inputRef = ref<HTMLInputElement | null>(null)
const visible = ref(false)

onMounted(() => {
  if (props.autofocus) {
    window.requestAnimationFrame(() => inputRef.value?.focus())
  }
})

function updateValue(event: Event) {
  emit('update:modelValue', (event.target as HTMLInputElement).value)
}

function toggleVisible() {
  visible.value = !visible.value
  void nextTick(() => inputRef.value?.focus())
}
</script>

<template>
  <div class="secure-password-input">
    <input
      ref="inputRef"
      :value="modelValue"
      :type="visible ? 'text' : 'password'"
      :class="['secure-password-input__input', inputClass]"
      :autocomplete="autocomplete"
      :placeholder="placeholder"
      :disabled="disabled"
      @input="updateValue"
    />
    <button
      type="button"
      class="secure-password-input__toggle"
      :title="visible ? hideTitle : showTitle"
      :disabled="disabled"
      @click="toggleVisible"
    >
      <EyeOff v-if="visible" class="h-4 w-4" />
      <Eye v-else class="h-4 w-4" />
    </button>
  </div>
</template>

<style scoped>
@reference "tailwindcss";

.secure-password-input {
  @apply relative w-full;
}

.secure-password-input__input {
  @apply w-full rounded-xl border border-gray-800 bg-gray-950 px-4 pr-12 py-2.5 text-sm text-gray-200 outline-none transition focus:border-emerald-500 focus:ring-1 focus:ring-emerald-500;
}

.secure-password-input__input.vault-input {
  background: var(--bg-input);
  border: 1px solid var(--border-input);
  color: var(--text-input-color);
  @apply h-11 px-4 pr-12 py-0;
}

.secure-password-input__input.lock-input {
  @apply border-gray-800 bg-gray-950 py-2.5 text-center font-mono text-gray-200;
}

.secure-password-input__input.m-input {
  @apply px-3 pr-12 py-2 text-xs text-gray-300;
}

.secure-password-input__toggle {
  @apply absolute right-2 top-1/2 flex h-8 w-8 -translate-y-1/2 items-center justify-center rounded-lg text-slate-500 transition hover:bg-emerald-500/10 hover:text-emerald-300 disabled:cursor-not-allowed disabled:opacity-50;
}
</style>
