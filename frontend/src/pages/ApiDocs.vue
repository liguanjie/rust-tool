<script setup lang="ts">
import {
  AlertTriangle,
  BookOpen,
  CheckCircle,
  Copy,
  Filter,
  Loader2,
  Play,
  Search,
  Send,
  Server,
  X,
} from '@lucide/vue'
import { computed, nextTick, onUnmounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import ToolShell from '../components/ToolShell.vue'
import {
  apiDocModules,
  buildCurlExample,
  findApiDocModule,
  type ApiEndpointDoc,
  type ApiMethod,
} from '../apiDocs'
import { Button } from '@/components/ui/button'

type MethodFilter = 'ALL' | ApiMethod

interface TestState {
  active: boolean
  requestBody: string
  loading: boolean
  responseStatus: string
  responseTime: number
  responseBody: string
}

const route = useRoute()
const router = useRouter()
const copiedKey = ref('')
const searchQuery = ref('')
const methodFilter = ref<MethodFilter>('ALL')
const activeEndpointId = ref('')
const testingStates = ref<Record<string, TestState>>({})
let observer: IntersectionObserver | null = null

const selectedModule = computed(() => findApiDocModule(route.query.module))
const methodOptions: MethodFilter[] = ['ALL', 'GET', 'POST', 'PUT', 'PATCH', 'DELETE']
const filteredEndpoints = computed(() => {
  const query = searchQuery.value.trim().toLowerCase()
  const filter = methodFilter.value
  return selectedModule.value.endpoints.filter((endpoint) => {
    if (filter !== 'ALL' && endpoint.method !== filter) return false
    if (!query) return true
    return (
      endpoint.path.toLowerCase().includes(query) ||
      endpoint.title.toLowerCase().includes(query) ||
      endpoint.description.toLowerCase().includes(query)
    )
  })
})
const activeMethods = computed(() =>
  Array.from(new Set(selectedModule.value.endpoints.map((endpoint) => endpoint.method))),
)
const testPanelCount = computed(() =>
  Object.values(testingStates.value).filter((state) => state.active).length,
)

watch(
  [selectedModule, filteredEndpoints],
  () => {
    setupObserver()
    activeEndpointId.value = filteredEndpoints.value[0]?.id ?? ''
  },
  { immediate: true },
)

onUnmounted(() => {
  observer?.disconnect()
})

function selectModule(moduleId: string) {
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

function setupObserver() {
  observer?.disconnect()
  observer = new IntersectionObserver(
    (entries) => {
      entries.forEach((entry) => {
        if (entry.isIntersecting) {
          activeEndpointId.value = entry.target.id.replace('endpoint-', '')
        }
      })
    },
    {
      rootMargin: '-80px 0px -60% 0px',
      threshold: 0,
    },
  )

  nextTick(() => {
    filteredEndpoints.value.forEach((endpoint) => {
      const element = document.getElementById(`endpoint-${endpoint.id}`)
      if (element) observer?.observe(element)
    })
  })
}

function scrollToEndpoint(id: string) {
  const element = document.getElementById(`endpoint-${id}`)
  if (!element) return
  element.classList.remove('api-endpoint-card--target')
  void element.offsetWidth
  element.classList.add('api-endpoint-card--target')
  element.scrollIntoView({ behavior: 'smooth', block: 'center' })
  activeEndpointId.value = id
}

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
      } catch {
        throw new Error('请求 Body 的 JSON 格式无效，请检查语法是否正确。')
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
  } catch (error) {
    const endTime = performance.now()
    state.responseTime = Math.round(endTime - startTime)
    state.responseStatus = 'Error'
    state.responseBody = error instanceof Error ? error.message : 'Failed to fetch'
  } finally {
    state.loading = false
  }
}

function methodClass(method: ApiMethod) {
  return `api-method api-method--${method.toLowerCase()}`
}

function responseStatusClass(status: string) {
  if (status.startsWith('2')) return 'status-pill status-pill--good'
  if (status.startsWith('4')) return 'status-pill status-pill--warn'
  if (status.startsWith('5') || status === 'Error') return 'status-pill status-pill--danger'
  return 'status-pill status-pill--muted'
}
</script>

<template>
  <ToolShell
    title="接口文档"
    description="查看 RustTool 本地 REST 能力，并在页面内完成 curl 复制与接口调试。"
    eyebrow="开发参考"
    fluid
  >
    <div class="api-doc-workbench">
      <section class="input-panel api-doc-status-panel">
        <div class="api-doc-status-main">
          <span class="service-icon">
            <Server class="h-5 w-5" aria-hidden="true" />
          </span>
          <div>
            <span class="field-label">当前模块</span>
            <strong>{{ selectedModule.name }}</strong>
            <small>{{ selectedModule.baseUrl }}</small>
          </div>
        </div>
        <dl class="api-doc-status-metrics">
          <div>
            <dt>{{ selectedModule.endpoints.length }}</dt>
            <dd>接口</dd>
          </div>
          <div>
            <dt>{{ activeMethods.length }}</dt>
            <dd>方法</dd>
          </div>
          <div>
            <dt>{{ testPanelCount }}</dt>
            <dd>调试面板</dd>
          </div>
          <span class="status-pill status-pill--good">本地 API</span>
        </dl>
      </section>

      <section class="api-doc-layout">
        <aside class="api-doc-nav">
          <div class="api-doc-nav-header">
            <span class="field-label">模块</span>
            <strong>接口分组</strong>
          </div>
          <button
            v-for="module in apiDocModules"
            :key="module.id"
            type="button"
            class="api-doc-nav-item"
            :class="{ 'api-doc-nav-item--active': module.id === selectedModule.id }"
            @click="selectModule(module.id)"
          >
            <BookOpen class="h-4 w-4 shrink-0" aria-hidden="true" />
            <span>
              <strong>{{ module.name }}</strong>
              <small>{{ module.endpoints.length }} 个接口</small>
            </span>
          </button>
        </aside>

        <main class="api-doc-content">
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
            <div class="api-doc-section-heading">
              <div>
                <span class="field-label">运行配置</span>
                <h3>环境变量</h3>
              </div>
            </div>
            <div class="api-env-list">
              <div v-for="item in selectedModule.environment" :key="item.name" class="api-env-item">
                <strong>{{ item.name }}</strong>
                <span>{{ item.description }}</span>
                <small v-if="item.defaultValue">默认值：{{ item.defaultValue }}</small>
              </div>
            </div>
          </section>

          <section class="api-doc-section">
            <div class="api-doc-section-heading">
              <div>
                <span class="field-label">接口列表</span>
                <h3>{{ filteredEndpoints.length }} / {{ selectedModule.endpoints.length }} 个接口</h3>
              </div>
              <span class="status-pill status-pill--muted">
                <Filter class="mr-1.5 h-3.5 w-3.5" aria-hidden="true" />
                {{ methodFilter }}
              </span>
            </div>

            <div class="api-doc-controls">
              <label class="api-doc-search" for="api-doc-search">
                <Search class="h-4 w-4 shrink-0" aria-hidden="true" />
                <input
                  id="api-doc-search"
                  v-model="searchQuery"
                  type="text"
                  placeholder="搜索接口路径、名称或描述"
                />
              </label>
              <div class="api-doc-filters" role="radiogroup" aria-label="请求方法">
                <button
                  v-for="method in methodOptions"
                  :key="method"
                  type="button"
                  class="api-filter-pill"
                  :class="{ 'api-filter-pill--active': methodFilter === method }"
                  @click="methodFilter = method"
                >
                  {{ method }}
                </button>
              </div>
            </div>

            <div v-if="filteredEndpoints.length === 0" class="api-doc-empty">
              没有找到匹配的接口，请尝试其他关键词或请求方法。
            </div>

            <article
              v-for="endpoint in filteredEndpoints"
              :id="'endpoint-' + endpoint.id"
              :key="endpoint.id"
              class="api-endpoint-card"
            >
              <header class="api-endpoint-header">
                <div>
                  <p class="api-endpoint-title">
                    <span :class="methodClass(endpoint.method)">{{ endpoint.method }}</span>
                    <code>{{ endpoint.path }}</code>
                  </p>
                  <h4>{{ endpoint.title }}</h4>
                  <p>{{ endpoint.description }}</p>
                </div>
                <div class="api-endpoint-actions">
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    @click="toggleTestMode(endpoint)"
                  >
                    <component
                      :is="testingStates[endpoint.id]?.active ? X : Play"
                      class="mr-2 h-4 w-4"
                      aria-hidden="true"
                    />
                    {{ testingStates[endpoint.id]?.active ? '关闭测试' : '测试接口' }}
                  </Button>
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    @click="copyText(`${endpoint.id}-curl`, curlFor(endpoint))"
                  >
                    <Copy class="mr-2 h-4 w-4" aria-hidden="true" />
                    {{ copiedKey === `${endpoint.id}-curl` ? '已复制' : '复制 curl' }}
                  </Button>
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

              <Transition name="slide-fade">
                <div v-if="testingStates[endpoint.id]?.active" class="api-test-panel">
                  <div class="api-test-header">
                    <div>
                      <span class="field-label">在线调试</span>
                      <strong>{{ endpoint.method }} {{ endpoint.path }}</strong>
                    </div>
                    <Button
                      type="button"
                      size="sm"
                      :disabled="testingStates[endpoint.id].loading"
                      @click="sendRequest(endpoint)"
                    >
                      <component
                        :is="testingStates[endpoint.id].loading ? Loader2 : Send"
                        class="mr-2 h-4 w-4"
                        :class="{ 'animate-spin': testingStates[endpoint.id].loading }"
                        aria-hidden="true"
                      />
                      {{ testingStates[endpoint.id].loading ? '发送中' : '发送请求' }}
                    </Button>
                  </div>

                  <label v-if="endpoint.method !== 'GET'" class="field-control" :for="`api-test-body-${endpoint.id}`">
                    <span class="api-test-body-label">请求 Body (JSON)</span>
                    <textarea
                      :id="`api-test-body-${endpoint.id}`"
                      v-model="testingStates[endpoint.id].requestBody"
                      rows="6"
                      class="api-test-textarea"
                      placeholder="请输入 JSON 请求体"
                    />
                  </label>

                  <div
                    v-if="testingStates[endpoint.id].responseStatus || testingStates[endpoint.id].loading"
                    class="api-test-response"
                  >
                    <div class="api-test-response-meta">
                      <span>调试响应结果</span>
                      <div v-if="testingStates[endpoint.id].responseStatus" class="api-test-response-status">
                        <span :class="responseStatusClass(testingStates[endpoint.id].responseStatus)">
                          {{ testingStates[endpoint.id].responseStatus }}
                        </span>
                        <small>{{ testingStates[endpoint.id].responseTime }}ms</small>
                      </div>
                    </div>

                    <div
                      v-if="testingStates[endpoint.id].responseStatus === 'Error'"
                      class="api-test-error"
                    >
                      <AlertTriangle class="h-4 w-4 shrink-0" aria-hidden="true" />
                      <div>
                        <strong>连接后端服务失败</strong>
                        <p>
                          浏览器无法与本地后端服务建立连接：{{ testingStates[endpoint.id].responseBody }}。
                          macOS/Unix 请在项目根目录运行 <code>./rt dev</code>。
                        </p>
                      </div>
                    </div>

                    <div v-else class="api-example-block">
                      <div class="api-example-header">
                        <strong>
                          <component
                            :is="testingStates[endpoint.id].responseStatus.startsWith('2') ? CheckCircle : AlertTriangle"
                            class="mr-1.5 inline h-3.5 w-3.5"
                            aria-hidden="true"
                          />
                          Response Body
                        </strong>
                        <button
                          v-if="testingStates[endpoint.id].responseBody"
                          type="button"
                          @click="copyText(`${endpoint.id}-test-resp`, testingStates[endpoint.id].responseBody)"
                        >
                          {{ copiedKey === `${endpoint.id}-test-resp` ? '已复制' : '复制' }}
                        </button>
                      </div>
                      <pre>{{ testingStates[endpoint.id].loading ? '正在发送 HTTP 请求，等待响应中...' : testingStates[endpoint.id].responseBody }}</pre>
                    </div>
                  </div>
                </div>
              </Transition>

              <ul v-if="endpoint.notes?.length" class="api-notes">
                <li v-for="note in endpoint.notes" :key="note">{{ note }}</li>
              </ul>
            </article>
          </section>
        </main>

        <aside class="api-doc-toc">
          <h3>本页目录</h3>
          <div v-if="filteredEndpoints.length > 0" class="api-doc-toc-list">
            <button
              v-for="endpoint in filteredEndpoints"
              :key="endpoint.id"
              type="button"
              class="api-doc-toc-item"
              :class="{ 'api-doc-toc-item--active': activeEndpointId === endpoint.id }"
              @click="scrollToEndpoint(endpoint.id)"
            >
              <span
                class="api-doc-toc-badge"
                :class="'api-doc-toc-badge--' + endpoint.method.toLowerCase()"
              >
                {{ endpoint.method }}
              </span>
              <span class="api-doc-toc-text" :title="endpoint.title">{{ endpoint.title }}</span>
            </button>
          </div>
          <p v-else class="field-hint">无接口</p>
        </aside>
      </section>
    </div>
  </ToolShell>
</template>
