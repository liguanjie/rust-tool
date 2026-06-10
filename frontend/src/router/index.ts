import { createRouter, createWebHistory } from 'vue-router'
import VlessToMihomo from '../pages/VlessToMihomo.vue'
import ApiDocs from '../pages/ApiDocs.vue'
import Toolbox from '../pages/Toolbox.vue'
import AiMemo from '../pages/AiMemo.vue'

export const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/',
      redirect: '/toolbox',
    },
    {
      path: '/toolbox',
      name: 'toolbox',
      component: Toolbox,
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
      path: '/tools/api-docs',
      name: 'api-docs',
      component: ApiDocs,
    },
    {
      path: '/tools/ai-memo',
      name: 'ai-memo',
      component: AiMemo,
    },
  ],
})
