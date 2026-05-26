<script setup lang="ts">
import { RouterLink } from 'vue-router'
import { useToolsStore } from '../stores/tools'

const toolsStore = useToolsStore()
</script>

<template>
  <aside class="flex min-h-0 flex-col border-r border-stone-200 bg-stone-100 px-5 py-7">
    <div class="flex items-center gap-3">
      <span class="grid h-11 w-11 place-items-center rounded-lg bg-emerald-800 text-sm font-bold text-white">
        RT
      </span>
      <div>
        <h1 class="text-xl font-bold leading-tight text-stone-950">RustTool</h1>
        <p class="mt-0.5 text-sm text-stone-500">本地工具站</p>
      </div>
    </div>

    <nav class="mt-8 grid gap-5" aria-label="工具导航">
      <section v-for="group in toolsStore.groups" :key="group.id" class="nav-group">
        <h2>{{ group.name }}</h2>
        <div class="grid gap-2">
          <RouterLink
            v-for="tool in group.items"
            :key="tool.id"
            :to="tool.path"
            class="group flex items-center gap-3 rounded-lg border border-stone-200 bg-white px-3 py-2.5 text-left text-sm font-medium text-stone-800 transition hover:border-emerald-800"
            active-class="border-emerald-800 shadow-[inset_3px_0_0_#065f46]"
          >
            <component :is="tool.icon" class="h-4 w-4 text-emerald-800" aria-hidden="true" />
            <span>{{ tool.name }}</span>
          </RouterLink>
        </div>
      </section>
    </nav>
  </aside>
</template>
