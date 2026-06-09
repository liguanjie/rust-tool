<script setup lang="ts">
import { BookOpen, Copy, Server, Search, Play, X, Send, Loader2, AlertTriangle, CheckCircle } from '@lucide/vue'
import { computed, ref, watch, nextTick, onUnmounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { apiDocModules, buildCurlExample, findApiDocModule, type ApiEndpointDoc } from '../apiDocs'
import ToolShell from '../components/ToolShell.vue'

const route = useRoute()
const router = useRouter()
const copiedKey = ref('')

const selectedModule = computed(() => findApiDocModule(route.query.module))

const breadcrumbs = computed(() => {
  if (selectedModule.value.id === 'clash-party') {
    return [
      { label: 'API 管理', to: { name: 'api-management' } },
      { label: 'Clash Party / Mihomo', to: { name: 'api-management', query: { api: 'clash-party' } } },
      { label: '接口文档' },
    ]
  }
  return [
    { label: 'API 管理', to: { name: 'api-management' } },
    { label: '接口文档' },
  ]
})

// 过滤与搜索状态
const searchQuery = ref('')
const methodFilter = ref<'ALL' | 'GET' | 'POST' | 'PUT' | 'DELETE'>('ALL')

const filteredEndpoints = computed(() => {
  const query = searchQuery.value.trim().toLowerCase()
  const filter = methodFilter.value
  return selectedModule.value.endpoints.filter((endpoint) => {
    // 1. 请求方法过滤
    if (filter !== 'ALL' && endpoint.method !== filter) {
      return false
    }
    // 2. 文本模糊搜索过滤
    if (query) {
      const matchesPath = endpoint.path.toLowerCase().includes(query)
      const matchesTitle = endpoint.title.toLowerCase().includes(query)
      const matchesDesc = endpoint.description.toLowerCase().includes(query)
      return matchesPath || matchesTitle || matchesDesc
    }
    return true
  })
})

function selectModule(moduleId: string) {
  // 切换模块时重置过滤状态
  searchQuery.value = ''
  methodFilter.value = 'ALL'
  void router.push({
    name: 'api-docs',
    query: { module: moduleId },
  })
}

async function copyText(key: string, text: string) {
  await navigator.clipboard.writeText(text)
  copiedKey.value = key
  window.setTimeout(() => {
    if (copiedKey.value === key) copiedKey.value = ''
  }, 1400)
}

function curlFor(endpoint: ApiEndpointDoc) {
  return buildCurlExample(selectedModule.value, endpoint)
}

// 目录滚动高亮 (Scrollspy) 逻辑
const activeEndpointId = ref('')
let observer: IntersectionObserver | null = null

function setupObserver() {
  if (observer) {
    observer.disconnect()
  }

  observer = new IntersectionObserver(
    (entries) => {
      entries.forEach((entry) => {
        if (entry.isIntersecting) {
          activeEndpointId.value = entry.target.id.replace('endpoint-', '')
        }
      })
    },
    {
      // 触发高亮的黄金比例视口区间（以顶部下方 -80px 到中部为判定范围）
      rootMargin: '-80px 0px -60% 0px',
      threshold: 0,
    }
  )

  nextTick(() => {
    filteredEndpoints.value.forEach((ep) => {
      const el = document.getElementById(`endpoint-${ep.id}`)
      if (el) {
        observer?.observe(el)
      }
    })
  })
}

// 监听模块或过滤后的列表，重新绑定监听
watch(
  [selectedModule, filteredEndpoints],
  () => {
    setupObserver()
    if (filteredEndpoints.value.length > 0) {
      activeEndpointId.value = filteredEndpoints.value[0].id
    } else {
      activeEndpointId.value = ''
    }
  },
  { immediate: true }
)

onUnmounted(() => {
  if (observer) {
    observer.disconnect()
  }
})

// 点击 TOC 节点，平滑滚动至对应卡片并触发高亮闪烁动画
function scrollToEndpoint(id: string) {
  const el = document.getElementById(`endpoint-${id}`)
  if (el) {
    // 移除已有的动画类并重新添加，以便多次点击能重复触发动画
    el.classList.remove('api-endpoint-card--target')
    // 强制重绘，使动画生效
    void el.offsetWidth
    el.classList.add('api-endpoint-card--target')

    el.scrollIntoView({ behavior: 'smooth', block: 'center' })
    activeEndpointId.value = id
  }
}

// API 在线调试状态管理
interface TestState {
  active: boolean
  requestBody: string
  loading: boolean
  responseStatus: string
  responseTime: number
  responseBody: string
}

const testingStates = ref<Record<string, TestState>>({})

function toggleTestMode(endpoint: ApiEndpointDoc) {
  if (!testingStates.value[endpoint.id]) {
    testingStates.value[endpoint.id] = {
      active: true,
      requestBody: endpoint.request || '',
      loading: false,
      responseStatus: '',
      responseTime: 0,
      responseBody: '',
    }
  } else {
    testingStates.value[endpoint.id].active = !testingStates.value[endpoint.id].active
  }
}

async function sendRequest(endpoint: ApiEndpointDoc) {
  const state = testingStates.value[endpoint.id]
  if (!state) return

  state.loading = true
  state.responseStatus = ''
  state.responseBody = ''
  state.responseTime = 0

  const startTime = performance.now()
  const url = `${selectedModule.value.baseUrl}${endpoint.path}`

  try {
    const options: RequestInit = {
      method: endpoint.method,
      headers: {
        'Content-Type': 'application/json',
      },
    }

    if (endpoint.method !== 'GET' && state.requestBody) {
      try {
        JSON.parse(state.requestBody)
      } catch (err) {
        throw new Error('请求 Body 的 JSON 格式无效，请检查语法语法是否正确。')
      }
      options.body = state.requestBody
    }

    const response = await fetch(url, options)
    const endTime = performance.now()
    state.responseTime = Math.round(endTime - startTime)
    state.responseStatus = `${response.status} ${response.statusText}`

    const text = await response.text()
    try {
      const parsed = JSON.parse(text)
      state.responseBody = JSON.stringify(parsed, null, 2)
    } catch {
      state.responseBody = text
    }
  } catch (error: any) {
    const endTime = performance.now()
    state.responseTime = Math.round(endTime - startTime)
    state.responseStatus = 'Error'
    state.responseBody = error.message || 'Failed to fetch'
  } finally {
    state.loading = false
  }
}
</script>

<template>
  <ToolShell
    title="接口文档"
    description="查看 RustTool 对外提供的 REST API，包含请求参数、响应示例和调用示例。"
    eyebrow="工作台"
    :breadcrumbs="breadcrumbs"
  >
    <section class="api-doc-layout">
      <!-- 左侧模块导航 -->
      <aside class="api-doc-nav">
        <h3>模块</h3>
        <button
          v-for="module in apiDocModules"
          :key="module.id"
          type="button"
          :class="{ 'api-doc-nav-item--active': module.id === selectedModule.id }"
          class="api-doc-nav-item"
          @click="selectModule(module.id)"
        >
          <BookOpen class="h-4 w-4" aria-hidden="true" />
          <span>
            <strong>{{ module.name }}</strong>
            <small>{{ module.endpoints.length }} 个接口</small>
          </span>
        </button>
      </aside>

      <!-- 中间主内容区 -->
      <div class="api-doc-content">
        <section class="api-doc-summary">
          <div class="service-title">
            <span class="service-icon">
              <Server class="h-5 w-5" aria-hidden="true" />
            </span>
            <div>
              <h3>{{ selectedModule.name }}</h3>
              <p>{{ selectedModule.summary }}</p>
            </div>
          </div>

          <dl class="service-facts api-doc-facts">
            <div>
              <dt>Base URL</dt>
              <dd>{{ selectedModule.baseUrl }}</dd>
            </div>
          </dl>
        </section>

        <section class="api-doc-section">
          <h3>运行配置</h3>
          <div class="api-env-list">
            <div v-for="item in selectedModule.environment" :key="item.name" class="api-env-item">
              <strong>{{ item.name }}</strong>
              <span>{{ item.description }}</span>
              <small v-if="item.defaultValue">默认值：{{ item.defaultValue }}</small>
            </div>
          </div>
        </section>

        <section class="api-doc-section">
          <div class="flex items-center justify-between mb-3 flex-wrap gap-2">
            <h3 class="!m-0">接口列表</h3>
            <span class="api-filter-count">
              共 {{ selectedModule.endpoints.length }} 个，已筛选 {{ filteredEndpoints.length }} 个
            </span>
          </div>

          <!-- 顶部搜索与过滤控制栏 (方案 C) -->
          <div class="api-doc-controls">
            <div class="api-doc-search">
              <Search class="h-4 w-4 shrink-0" aria-hidden="true" />
              <input
                v-model="searchQuery"
                type="text"
                placeholder="搜索接口路径、名称或描述..."
              />
            </div>
            <div class="api-doc-filters">
              <button
                v-for="method in (['ALL', 'GET', 'POST', 'PUT', 'DELETE'] as const)"
                :key="method"
                type="button"
                :class="{ 'api-filter-pill--active': methodFilter === method }"
                class="api-filter-pill"
                @click="methodFilter = method"
              >
                {{ method }}
              </button>
            </div>
          </div>

          <!-- 空白状态 -->
          <div v-if="filteredEndpoints.length === 0" class="empty-state py-8 text-center bg-stone-50 rounded-lg border border-dashed border-stone-200">
            <p class="text-stone-400 text-sm m-0">没有找到匹配的接口，请尝试其他关键词或请求方法</p>
          </div>

          <!-- 接口列表 -->
          <article
            v-for="endpoint in filteredEndpoints"
            :id="'endpoint-' + endpoint.id"
            :key="endpoint.id"
            class="api-endpoint-card"
          >
            <header class="api-endpoint-header">
              <div>
                <p class="api-endpoint-title">
                  <span class="api-method" :class="'api-method--' + endpoint.method.toLowerCase()">{{ endpoint.method }}</span>
                  <code>{{ endpoint.path }}</code>
                </p>
                <h4>{{ endpoint.title }}</h4>
                <p>{{ endpoint.description }}</p>
              </div>
              <div class="flex items-center gap-2 flex-wrap">
                <button
                  class="icon-button"
                  type="button"
                  :class="{ 'border-emerald-800 bg-emerald-50 text-emerald-800': testingStates[endpoint.id]?.active }"
                  @click="toggleTestMode(endpoint)"
                >
                  <component
                    :is="testingStates[endpoint.id]?.active ? X : Play"
                    class="h-4 w-4"
                    :class="{ 'text-emerald-800': testingStates[endpoint.id]?.active, 'fill-emerald-800/10': !testingStates[endpoint.id]?.active }"
                    aria-hidden="true"
                  />
                  <span>{{ testingStates[endpoint.id]?.active ? '关闭测试' : '测试接口' }}</span>
                </button>
                <button class="icon-button" type="button" @click="copyText(`${endpoint.id}-curl`, curlFor(endpoint))">
                  <Copy class="h-4 w-4" aria-hidden="true" />
                  <span>{{ copiedKey === `${endpoint.id}-curl` ? '已复制' : '复制 curl' }}</span>
                </button>
              </div>
            </header>

            <div class="api-example-grid" :class="{ 'api-example-grid--single': !endpoint.request }">
              <div v-if="endpoint.request" class="api-example-block">
                <div class="api-example-header">
                  <strong>请求示例</strong>
                  <button type="button" @click="copyText(`${endpoint.id}-request`, endpoint.request)">复制</button>
                </div>
                <pre>{{ endpoint.request }}</pre>
              </div>

              <div class="api-example-block">
                <div class="api-example-header">
                  <strong>响应示例</strong>
                  <button type="button" @click="copyText(`${endpoint.id}-response`, endpoint.response)">复制</button>
                </div>
                <pre>{{ endpoint.response }}</pre>
              </div>
            </div>

            <!-- 在线测试面板 -->
            <Transition name="slide-fade">
              <div v-if="testingStates[endpoint.id]?.active" class="api-test-panel">
                <div class="api-test-header">
                  <div class="flex items-center gap-2">
                    <span class="inline-block h-2 w-2 rounded-full bg-emerald-500 animate-ping"></span>
                    <span class="text-xs font-bold text-stone-900">本地接口在线调试</span>
                  </div>
                  <button
                    class="compact-primary primary-button inline-flex items-center gap-1.5 px-3 py-1.5 h-8 min-h-0 mt-0 w-auto text-xs font-semibold"
                    :disabled="testingStates[endpoint.id].loading"
                    @click="sendRequest(endpoint)"
                  >
                    <component
                      :is="testingStates[endpoint.id].loading ? Loader2 : Send"
                      class="h-3.5 w-3.5"
                      :class="{ 'animate-spin': testingStates[endpoint.id].loading }"
                    />
                    <span>{{ testingStates[endpoint.id].loading ? '发送中...' : '发送请求' }}</span>
                  </button>
                </div>

                <!-- GET 以外的请求 Body 编辑 -->
                <div v-if="endpoint.method !== 'GET'" class="mb-4">
                  <div class="flex items-center justify-between mb-1">
                    <label class="api-test-body-label !mb-0">请求 Body (JSON)</label>
                    <span class="text-[10px] text-stone-400 font-mono">application/json</span>
                  </div>
                  <textarea
                    v-model="testingStates[endpoint.id].requestBody"
                    rows="6"
                    class="api-test-textarea"
                    placeholder="请输入 JSON 请求体..."
                  ></textarea>
                </div>

                <!-- 接口实际响应展现 -->
                <div v-if="testingStates[endpoint.id].responseStatus || testingStates[endpoint.id].loading" class="api-test-response">
                  <div class="api-test-response-meta">
                    <span class="text-xs font-bold text-stone-600">调试响应结果</span>
                    <div v-if="testingStates[endpoint.id].responseStatus" class="flex items-center gap-2">
                      <span
                        class="status-pill text-[10px] py-0.5 px-2 min-h-0 font-mono font-bold"
                        :class="[
                          testingStates[endpoint.id].responseStatus.startsWith('2') ? 'status-pill--good' : '',
                          testingStates[endpoint.id].responseStatus.startsWith('4') ? 'status-pill--warn' : '',
                          testingStates[endpoint.id].responseStatus.startsWith('5') || testingStates[endpoint.id].responseStatus === 'Error' ? 'bg-red-50 text-red-800 border border-red-100' : ''
                        ]"
                      >
                        {{ testingStates[endpoint.id].responseStatus }}
                      </span>
                      <span class="text-[10px] text-stone-400 font-mono font-semibold">{{ testingStates[endpoint.id].responseTime }}ms</span>
                    </div>
                  </div>

                  <!-- 友好错误提示框 -->
                  <div
                    v-if="testingStates[endpoint.id].responseStatus === 'Error'"
                    class="mt-2 rounded-lg border border-red-200 bg-red-50/50 p-3.5 text-xs text-red-800 leading-relaxed"
                  >
                    <div class="flex items-center gap-2 font-bold mb-1.5">
                      <AlertTriangle class="h-4 w-4 shrink-0 text-red-600" />
                      <span>连接后端服务失败</span>
                    </div>
                    <p class="m-0">
                      浏览器无法与本地后端服务建立连接（报错：<code>{{ testingStates[endpoint.id].responseBody }}</code>）。<br/>
                      这通常是因为本地后端服务未启动。macOS/Unix 请在项目根目录运行 <code>./rt dev</code>；Windows 可双击 <code>start.bat</code> 或运行 <code>rt dev</code>。
                    </p>
                  </div>

                  <!-- 正常响应体展示 (包含成功的 2xx 和失败的 4xx/5xx) -->
                  <div
                    v-else
                    class="api-example-block mt-2 border transition-colors"
                    :class="[
                      testingStates[endpoint.id].responseStatus.startsWith('2') ? 'border-stone-200' : '',
                      testingStates[endpoint.id].responseStatus.startsWith('4') ? 'border-amber-200' : '',
                      testingStates[endpoint.id].responseStatus.startsWith('5') ? 'border-red-200' : ''
                    ]"
                  >
                    <div
                      class="api-example-header py-1.5 transition-colors"
                      :class="[
                        testingStates[endpoint.id].responseStatus.startsWith('2') ? 'bg-stone-50/50 border-stone-100' : '',
                        testingStates[endpoint.id].responseStatus.startsWith('4') ? 'bg-amber-50/50 border-amber-100/50 text-amber-900' : '',
                        testingStates[endpoint.id].responseStatus.startsWith('5') ? 'bg-red-50/50 border-red-100/50 text-red-900' : ''
                      ]"
                    >
                      <strong class="text-[10px] flex items-center gap-1">
                        <component
                          :is="testingStates[endpoint.id].responseStatus.startsWith('2') ? CheckCircle : AlertTriangle"
                          class="h-3 w-3"
                          :class="testingStates[endpoint.id].responseStatus.startsWith('2') ? 'text-emerald-700' : (testingStates[endpoint.id].responseStatus.startsWith('4') ? 'text-amber-700' : 'text-red-700')"
                        />
                        <span>Response Body</span>
                      </strong>
                      <button
                        v-if="testingStates[endpoint.id].responseBody"
                        type="button"
                        class="text-[10px]"
                        :class="[
                          testingStates[endpoint.id].responseStatus.startsWith('2') ? 'text-emerald-800 hover:text-emerald-950' : '',
                          testingStates[endpoint.id].responseStatus.startsWith('4') ? 'text-amber-800 hover:text-amber-950' : '',
                          testingStates[endpoint.id].responseStatus.startsWith('5') ? 'text-red-800 hover:text-red-950' : ''
                        ]"
                        @click="copyText(`${endpoint.id}-test-resp`, testingStates[endpoint.id].responseBody)"
                      >
                        {{ copiedKey === `${endpoint.id}-test-resp` ? '已复制' : '复制' }}
                      </button>
                    </div>
                    <pre
                      class="max-h-[320px] overflow-auto transition-colors"
                      :class="[
                        testingStates[endpoint.id].responseStatus.startsWith('2') ? 'bg-white text-stone-800' : '',
                        testingStates[endpoint.id].responseStatus.startsWith('4') ? 'bg-amber-50/10 text-stone-700' : '',
                        testingStates[endpoint.id].responseStatus.startsWith('5') ? 'bg-red-50/10 text-stone-700' : ''
                      ]"
                    >{{ testingStates[endpoint.id].loading ? '正在发送 HTTP 请求，等待响应中...' : testingStates[endpoint.id].responseBody }}</pre>
                  </div>
                </div>
              </div>
            </Transition>

            <ul v-if="endpoint.notes?.length" class="api-notes">
              <li v-for="note in endpoint.notes" :key="note">{{ note }}</li>
            </ul>
          </article>
        </section>
      </div>

      <!-- 右侧悬浮目录 (方案 B) -->
      <aside class="api-doc-toc">
        <h3>本页目录</h3>
        <div v-if="filteredEndpoints.length > 0" class="api-doc-toc-list">
          <button
            v-for="endpoint in filteredEndpoints"
            :key="endpoint.id"
            type="button"
            :class="{ 'api-doc-toc-item--active': activeEndpointId === endpoint.id }"
            class="api-doc-toc-item"
            @click="scrollToEndpoint(endpoint.id)"
          >
            <span
              class="api-doc-toc-badge"
              :class="'api-doc-toc-badge--' + endpoint.method.toLowerCase()"
            >
              {{ endpoint.method }}
            </span>
            <span class="api-doc-toc-text" :title="endpoint.title">
              {{ endpoint.title }}
            </span>
          </button>
        </div>
        <p v-else class="text-xs text-stone-400 px-1 m-0">无接口</p>
      </aside>
    </section>
  </ToolShell>
</template>

