import { createRouter, createWebHistory } from 'vue-router'
import VlessToMihomo from '../pages/VlessToMihomo.vue'
import WindowsWorkbench from '../pages/WindowsWorkbench.vue'

export const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/',
      redirect: '/tools/windows-workbench',
    },
    {
      path: '/toolbox/vless-to-mihomo',
      name: 'vless-to-mihomo',
      component: VlessToMihomo,
    },
    {
      path: '/tools/vless-to-mihomo',
      redirect: '/toolbox/vless-to-mihomo',
    },
    {
      path: '/tools/windows-workbench',
      name: 'windows-workbench',
      component: WindowsWorkbench,
    },
  ],
})
