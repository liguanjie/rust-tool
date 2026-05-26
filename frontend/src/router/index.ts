import { createRouter, createWebHistory } from 'vue-router'
import VlessToMihomo from '../pages/VlessToMihomo.vue'

export const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/',
      redirect: '/tools/vless-to-mihomo',
    },
    {
      path: '/tools/vless-to-mihomo',
      name: 'vless-to-mihomo',
      component: VlessToMihomo,
    },
  ],
})
