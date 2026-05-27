<script setup lang="ts">
import { Check, ChevronDown, Search } from '@lucide/vue'
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'

export interface SmartSelectItem {
  value: string
  label: string
  description?: string
  badge?: string
  active?: boolean
}

const props = withDefaults(
  defineProps<{
    modelValue: string
    items: SmartSelectItem[]
    placeholder?: string
    searchPlaceholder?: string
    emptyText?: string
  }>(),
  {
    placeholder: '请选择',
    searchPlaceholder: '搜索...',
    emptyText: '没有匹配项',
  },
)

const emit = defineEmits<{
  'update:modelValue': [value: string]
}>()

const root = ref<HTMLElement | null>(null)
const open = ref(false)
const query = ref('')

const selectedItem = computed(() => props.items.find((item) => item.value === props.modelValue))

const filteredItems = computed(() => {
  const keyword = query.value.trim().toLowerCase()
  if (!keyword) return props.items
  return props.items.filter((item) =>
    [item.label, item.description, item.badge]
      .filter(Boolean)
      .some((value) => value!.toLowerCase().includes(keyword)),
  )
})

function selectItem(value: string) {
  emit('update:modelValue', value)
  open.value = false
  query.value = ''
}

function handleDocumentPointerDown(event: PointerEvent) {
  if (!root.value?.contains(event.target as Node)) {
    open.value = false
  }
}

onMounted(() => {
  document.addEventListener('pointerdown', handleDocumentPointerDown)
})

onBeforeUnmount(() => {
  document.removeEventListener('pointerdown', handleDocumentPointerDown)
})
</script>

<template>
  <div ref="root" class="smart-select" :class="{ 'smart-select--open': open }">
    <button class="smart-select-trigger" type="button" @click="open = !open">
      <span class="smart-select-value">
        <strong>{{ selectedItem?.label || placeholder }}</strong>
        <small v-if="selectedItem?.description">{{ selectedItem.description }}</small>
      </span>
      <span v-if="selectedItem?.badge" class="smart-select-badge">{{ selectedItem.badge }}</span>
      <ChevronDown class="h-4 w-4 smart-select-chevron" aria-hidden="true" />
    </button>

    <div v-if="open" class="smart-select-menu">
      <label class="smart-select-search">
        <Search class="h-4 w-4" aria-hidden="true" />
        <input v-model="query" type="text" :placeholder="searchPlaceholder" autocomplete="off" />
      </label>

      <div class="smart-select-options">
        <button
          v-for="item in filteredItems"
          :key="item.value"
          class="smart-select-option"
          :class="{ 'smart-select-option--selected': item.value === modelValue, 'smart-select-option--active': item.active }"
          type="button"
          @click="selectItem(item.value)"
        >
          <span class="smart-select-option-copy">
            <strong>{{ item.label }}</strong>
            <small v-if="item.description">{{ item.description }}</small>
          </span>
          <span v-if="item.badge" class="smart-select-badge">{{ item.badge }}</span>
          <Check v-if="item.value === modelValue" class="h-4 w-4 smart-select-check" aria-hidden="true" />
        </button>

        <p v-if="filteredItems.length === 0" class="smart-select-empty">{{ emptyText }}</p>
      </div>
    </div>
  </div>
</template>
