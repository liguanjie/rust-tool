<script setup lang="ts">
import { RouterLink } from 'vue-router'
import type { RouteLocationRaw } from 'vue-router'

defineProps<{
  title: string
  description: string
  eyebrow?: string
  breadcrumbs?: Array<{
    label: string
    to?: RouteLocationRaw
    onClick?: () => void
  }>
  fluid?: boolean
}>()
</script>

<template>
  <article class="tool-shell" :class="{ 'tool-shell--fluid': fluid }">
    <header class="tool-header">
      <div>
        <nav v-if="breadcrumbs?.length" class="breadcrumb-nav" aria-label="面包屑">
          <template v-for="(item, index) in breadcrumbs" :key="`${item.label}-${index}`">
            <RouterLink v-if="item.to" :to="item.to">{{ item.label }}</RouterLink>
            <button v-else-if="item.onClick" type="button" @click="item.onClick">{{ item.label }}</button>
            <span v-else>{{ item.label }}</span>
            <span v-if="index < breadcrumbs.length - 1" aria-hidden="true">›</span>
          </template>
        </nav>
        <p v-else class="eyebrow">{{ eyebrow || '工具箱' }}</p>
        <h2>{{ title }}</h2>
        <p>{{ description }}</p>
      </div>
    </header>

    <slot />
  </article>
</template>
