<script setup lang="ts">
defineProps<{
  title: string
  description: string
}>()
</script>

<template>
  <article class="tool-shell">
    <header class="tool-header">
      <div>
        <p class="eyebrow">转换工具</p>
        <h2>{{ title }}</h2>
        <p>{{ description }}</p>
      </div>
    </header>

    <slot />
  </article>
</template>
