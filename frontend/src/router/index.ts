import { createRouter, createWebHistory } from 'vue-router'
import VlessToMihomo from '../pages/VlessToMihomo.vue'
import WindowsWorkbench from '../pages/WindowsWorkbench.vue'
import ApiManagement from '../pages/ApiManagement.vue'
import ApiDocs from '../pages/ApiDocs.vue'
import OperationLogs from '../pages/OperationLogs.vue'
import Toolbox from '../pages/Toolbox.vue'

export const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/',
      redirect: '/tools/windows-workbench',
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
      path: '/tools/windows-workbench',
      name: 'windows-workbench',
      component: WindowsWorkbench,
    },
    {
      path: '/tools/api-management',
      name: 'api-management',
      component: ApiManagement,
    },
    {
      path: '/tools/api-docs',
      name: 'api-docs',
      component: ApiDocs,
    },
    {
      path: '/tools/operation-logs',
      name: 'operation-logs',
      component: OperationLogs,
    },
  ],
})
