<script setup lang="ts">
import { Check, ChevronDown, Search } from '@lucide/vue'
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'

export interface SmartSelectItem {
  value: string
  label: string
  description?: string
  badge?: string
  active?: boolean
  disabled?: boolean
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
  const item = props.items.find((next) => next.value === value)
  if (item?.disabled) return
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
  <div ref="root" class="relative" :class="{ 'z-50': open }">
    <button
      class="flex h-10 w-full items-center justify-between rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50"
      type="button"
      @click="open = !open"
    >
      <span class="flex items-center gap-2 truncate text-left">
        <span class="truncate font-medium">{{ selectedItem?.label || placeholder }}</span>
        <small v-if="selectedItem?.description" class="text-muted-foreground truncate">{{ selectedItem.description }}</small>
      </span>
      <div class="flex items-center gap-2 shrink-0">
        <span v-if="selectedItem?.badge" class="inline-flex items-center rounded-md border px-2 py-0.5 text-xs font-semibold transition-colors border-transparent bg-secondary text-secondary-foreground">{{ selectedItem.badge }}</span>
        <ChevronDown class="h-4 w-4 opacity-50" aria-hidden="true" />
      </div>
    </button>

    <div
      v-if="open"
      class="absolute top-full z-50 mt-1 max-h-96 w-full min-w-[8rem] overflow-hidden rounded-md border bg-popover text-popover-foreground shadow-md animate-in fade-in-0 zoom-in-95"
    >
      <div class="flex items-center border-b px-3">
        <Search class="mr-2 h-4 w-4 shrink-0 opacity-50" />
        <input
          v-model="query"
          class="flex h-11 w-full rounded-md bg-transparent py-3 text-sm outline-none placeholder:text-muted-foreground disabled:cursor-not-allowed disabled:opacity-50"
          type="text"
          :placeholder="searchPlaceholder"
          autocomplete="off"
        />
      </div>

      <div class="max-h-[300px] overflow-y-auto p-1">
        <button
          v-for="item in filteredItems"
          :key="item.value"
          class="relative flex w-full cursor-default select-none items-center rounded-sm py-1.5 pl-8 pr-2 text-sm outline-none hover:bg-accent hover:text-accent-foreground"
          :class="{
            'bg-accent text-accent-foreground': item.value === modelValue,
            'opacity-50 pointer-events-none': item.disabled,
          }"
          type="button"
          :disabled="item.disabled"
          @click="selectItem(item.value)"
        >
          <span class="absolute left-2 flex h-3.5 w-3.5 items-center justify-center">
            <Check v-if="item.value === modelValue" class="h-4 w-4" />
          </span>
          <span class="flex items-center gap-2 truncate text-left">
            <span class="font-medium">{{ item.label }}</span>
            <small v-if="item.description" class="text-muted-foreground truncate">{{ item.description }}</small>
          </span>
          <span v-if="item.badge" class="ml-auto inline-flex items-center rounded-md border px-2 py-0.5 text-xs font-semibold bg-secondary text-secondary-foreground">{{ item.badge }}</span>
        </button>

        <p v-if="filteredItems.length === 0" class="py-6 text-center text-sm text-muted-foreground">{{ emptyText }}</p>
      </div>
    </div>
  </div>
</template>
