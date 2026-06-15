import { createRouter, createWebHistory } from 'vue-router'
import ApiDocs from '../pages/ApiDocs.vue'
import Toolbox from '../pages/Toolbox.vue'

export const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/',
      redirect: '/toolbox/codex',
    },
    {
      path: '/toolbox',
      name: 'toolbox',
      component: Toolbox,
    },
    {
      path: '/toolbox/vless-to-mihomo',
      name: 'vless-to-mihomo',
      component: () => import('../pages/VlessToMihomo.vue'),
    },
    {
      path: '/toolbox/codex',
      name: 'codex',
      component: () => import('../pages/Codex.vue'),
    },
    {
      path: '/tools/vless-to-mihomo',
      redirect: '/toolbox/vless-to-mihomo',
    },
    {
      path: '/tools/api-docs',
      name: 'api-docs',
      component: ApiDocs,
    },
  ],
})
