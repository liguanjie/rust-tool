import { createRouter, createWebHistory } from 'vue-router'
import Toolbox from '../pages/Toolbox.vue'

export const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes: [
    {
      path: '/',
      redirect: '/dashboard',
    },
    {
      path: '/dashboard',
      name: 'dashboard',
      component: () => import('../pages/ToolboxDashboard.vue'),
    },
    {
      path: '/toolbox',
      name: 'toolbox',
      component: Toolbox,
      redirect: '/dashboard',
      children: [
        {
          path: 'vless-to-mihomo',
          name: 'vless-to-mihomo',
          component: () => import('../pages/VlessToMihomo.vue'),
        },
        {
          path: 'codex',
          name: 'codex',
          component: () => import('../pages/Codex.vue'),
        },
      ]
    },
    {
      path: '/osv-scanner',
      component: () => import('../pages/osv-scanner/OsvLayout.vue'),
      children: [
        {
          path: '',
          name: 'osv-scanner-dashboard',
          component: () => import('../pages/osv-scanner/Dashboard.vue'),
        },
        {
          path: 'project/:id',
          name: 'osv-scanner-project',
          component: () => import('../pages/osv-scanner/ProjectWorkspace.vue'),
        }
      ]
    },
    {
      path: '/tools/vless-to-mihomo',
      redirect: '/toolbox/vless-to-mihomo',
    },
    {
      path: '/tools/osv-scanner',
      redirect: '/osv-scanner',
    },
    {
      path: '/:catchAll(.*)',
      redirect: '/',
    },
  ],
})
