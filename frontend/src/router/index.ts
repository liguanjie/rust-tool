import { createRouter, createWebHistory } from 'vue-router'


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
      path: '/toolbox/vless-to-mihomo',
      name: 'vless-to-mihomo',
      component: () => import('../pages/VlessToMihomo.vue'),
    },
    {
      path: '/toolbox/finalshell-password',
      name: 'finalshell-password-decoder',
      component: () => import('../pages/FinalShellPasswordDecoder.vue'),
    },
    {
      path: '/agent-skills',
      name: 'agent-skills',
      component: () => import('../pages/AgentSkills.vue'),
    },
    {
      path: '/program-settings',
      name: 'program-settings',
      component: () => import('../pages/ProgramSettings.vue'),
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
      path: '/tools/finalshell-password',
      redirect: '/toolbox/finalshell-password',
    },
    {
      path: '/tools/osv-scanner',
      redirect: '/osv-scanner',
    },
    {
      path: '/settings',
      redirect: '/program-settings',
    },
    {
      path: '/:catchAll(.*)',
      redirect: '/',
    },
  ],
})
