<script setup lang="ts">
import { nextTick, onMounted, ref } from 'vue'
import { Eye, EyeOff } from '@lucide/vue'
import { Input } from '@/components/ui/input'
import { Button } from '@/components/ui/button'

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
  <div class="relative w-full">
    <Input
      ref="inputRef"
      :model-value="modelValue"
      @update:model-value="(val) => emit('update:modelValue', val as string)"
      :type="visible ? 'text' : 'password'"
      :class="['pr-10', inputClass]"
      :autocomplete="autocomplete"
      :placeholder="placeholder"
      :disabled="disabled"
    />
    <Button
      variant="ghost"
      size="icon"
      type="button"
      class="absolute right-0 top-0 h-full px-3 py-2 hover:bg-transparent text-muted-foreground hover:text-foreground"
      :title="visible ? hideTitle : showTitle"
      :disabled="disabled"
      @click="toggleVisible"
    >
      <EyeOff v-if="visible" class="h-4 w-4" />
      <Eye v-else class="h-4 w-4" />
    </Button>
  </div>
</template>


