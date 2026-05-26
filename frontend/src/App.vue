<script setup lang="ts">
import { RouterView } from 'vue-router'
import AppSidebar from './components/AppSidebar.vue'
</script>

<template>
  <main class="grid min-h-screen grid-cols-[280px_minmax(0,1fr)] bg-stone-50 text-stone-950 max-[880px]:grid-cols-1">
    <AppSidebar class="max-[880px]:min-h-0 max-[880px]:border-b max-[880px]:border-r-0" />
    <section class="min-w-0 px-8 py-8 max-[880px]:px-5">
      <RouterView />
    </section>
  </main>
</template>
