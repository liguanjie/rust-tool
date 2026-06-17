<script setup lang="ts">
import { computed, onMounted } from 'vue'
import {
  Cable,
  CheckCircle2,
  Download,
  ExternalLink,
  FileCode2,
  Loader2,
  Network,
  Route,
} from '@lucide/vue'
import type {
  VlessOutputMode,
  VlessTemplateMode,
  VlessTransitGroupType,
} from '../api/tools'
import ResultPanel from '../components/ResultPanel.vue'
import ToolShell from '../components/ToolShell.vue'
import { useVlessToMihomoStore } from '../stores/vlessToMihomo'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Textarea } from '@/components/ui/textarea'

const tool = useVlessToMihomoStore()

const outputModes: Array<{
  value: VlessOutputMode
  title: string
  description: string
}> = [
  {
    value: 'full_config',
    title: '完整配置',
    description: '生成可直接导入的 Mihomo YAML。',
  },
  {
    value: 'proxy_only',
    title: '节点片段',
    description: '只输出 proxies 片段，适合合并到已有配置。',
  },
]

const templateModes: Array<{
  value: VlessTemplateMode
  title: string
  description: string
}> = [
  {
    value: 'full_rules',
    title: '多节点分流',
    description: '覆盖 AI、媒体、Google、Telegram 与国内直连。',
  },
  {
    value: 'standard',
    title: '基础分流',
    description: '本机、局域网、国内直连，其余走代理。',
  },
  {
    value: 'minimal',
    title: '最小配置',
    description: '仅保留代理节点和兜底规则。',
  },
]

const transitModes: Array<{
  value: VlessTransitGroupType
  title: string
  description: string
}> = [
  {
    value: 'url_test',
    title: '自动测速',
    description: '定时测速并选择更快的中转节点。',
  },
  {
    value: 'fallback',
    title: '故障切换',
    description: '按顺序使用可用节点，异常时自动切换。',
  },
  {
    value: 'select',
    title: '手动选择',
    description: '在客户端手动指定中转节点。',
  },
]

const linkCount = computed(() => splitLines(tool.input).filter((line) => line.startsWith('vless://')).length)
const directDomainCount = computed(() => splitLines(tool.directDomains).length)
const transitProviderCount = computed(() => splitLines(tool.transitProviderUrl).length)
const transitBypassCount = computed(() => splitLines(tool.transitBypassDomains).length)
const outputModeLabel = computed(() =>
  tool.mode === 'full_config' ? '完整配置' : '节点片段',
)
const resultStatus = computed(() => {
  if (tool.loading) return '转换中'
  if (tool.yaml) return '已生成'
  return '待转换'
})
const resultStatusClass = computed(() => {
  if (tool.loading) return 'status-pill status-pill--warn'
  if (tool.yaml) return 'status-pill status-pill--good'
  return 'status-pill status-pill--muted'
})
const transitStatus = computed(() => {
  if (!tool.transitEnabled) return '未启用'
  if (transitProviderCount.value > 1) return `${transitProviderCount.value} 个订阅`
  if (transitProviderCount.value === 1) return '1 个订阅'
  return '待配置'
})
const convertButtonLabel = computed(() => {
  if (tool.loading) return '转换中'
  if (tool.yaml) return '重新转换'
  return '生成 YAML'
})

onMounted(() => {
  void tool.load()
})

function splitLines(value: string) {
  return value
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean)
}

function optionClass(active: boolean) {
  return {
    'vless-choice': true,
    'vless-choice--active': active,
  }
}
</script>

<template>
  <ToolShell
    title="VLESS 转 Mihomo"
    description="把 3x-ui 节点转换成可导入、可审计、可复用的本地 Mihomo 配置。"
    eyebrow="网络配置"
    fluid
  >
    <div class="vless-workbench">
      <section class="input-panel vless-status-panel">
        <div class="vless-status-main">
          <span class="service-icon">
            <Cable class="h-5 w-5" aria-hidden="true" />
          </span>
          <div>
            <span class="field-label">转换状态</span>
            <strong>{{ resultStatus }}</strong>
            <small>{{ tool.nodeAddress || '等待 VLESS 链接' }}</small>
          </div>
        </div>
        <dl class="vless-status-metrics">
          <div>
            <dt>{{ linkCount }}</dt>
            <dd>链接</dd>
          </div>
          <div>
            <dt>{{ outputModeLabel }}</dt>
            <dd>输出</dd>
          </div>
          <div>
            <dt>{{ transitStatus }}</dt>
            <dd>中转</dd>
          </div>
          <span :class="resultStatusClass">{{ resultStatus }}</span>
        </dl>
      </section>

      <section class="tool-grid vless-grid">
        <div class="input-panel vless-config-panel">
          <section class="config-section">
            <div class="vless-section-heading">
              <div>
                <span class="field-label">源链接</span>
                <strong>VLESS 输入</strong>
              </div>
              <span class="status-pill status-pill--muted">{{ linkCount }} 个链接</span>
            </div>
            <Textarea
              id="vless-input"
              v-model="tool.input"
              class="vless-link-input"
              spellcheck="false"
              placeholder="每行一个 vless:// 链接"
            />
            <div class="vless-mini-facts">
              <span>
                <CheckCircle2 class="h-4 w-4" aria-hidden="true" />
                {{ tool.nodeAddress || '等待解析节点地址' }}
              </span>
              <span>
                <FileCode2 class="h-4 w-4" aria-hidden="true" />
                {{ tool.downloadFilename }}
              </span>
            </div>
          </section>

          <section class="config-section">
            <div class="vless-section-heading">
              <div>
                <span class="field-label">输出策略</span>
                <strong>配置形态</strong>
              </div>
              <span class="status-pill status-pill--muted">{{ outputModeLabel }}</span>
            </div>
            <label class="field-control" for="download-name">
              <span class="field-label">文件名 / 节点名</span>
              <Input
                id="download-name"
                :model-value="tool.downloadName"
                type="text"
                placeholder="mihomo"
                @update:model-value="(val) => tool.updateDownloadName(val as string)"
              />
            </label>

            <div class="vless-choice-grid" role="radiogroup" aria-label="输出格式">
              <label
                v-for="mode in outputModes"
                :key="mode.value"
                :class="optionClass(tool.mode === mode.value)"
              >
                <input v-model="tool.mode" type="radio" :value="mode.value" />
                <span>
                  <strong>{{ mode.title }}</strong>
                  <small>{{ mode.description }}</small>
                </span>
              </label>
            </div>
          </section>

          <section v-if="tool.mode === 'full_config'" class="config-section">
            <div class="vless-section-heading">
              <div>
                <span class="field-label">规则模板</span>
                <strong>路由基线</strong>
              </div>
              <Route class="vless-heading-icon h-4 w-4" aria-hidden="true" />
            </div>
            <div class="vless-choice-grid vless-choice-grid--three" role="radiogroup" aria-label="配置模板">
              <label
                v-for="template in templateModes"
                :key="template.value"
                :class="optionClass(tool.template === template.value)"
              >
                <input v-model="tool.template" type="radio" :value="template.value" />
                <span>
                  <strong>{{ template.title }}</strong>
                  <small>{{ template.description }}</small>
                </span>
              </label>
            </div>
          </section>

          <section v-if="tool.mode === 'full_config'" class="config-section">
            <div class="vless-section-heading">
              <div>
                <span class="field-label">直连域名</span>
                <strong>绕过代理</strong>
              </div>
              <span class="status-pill status-pill--muted">{{ directDomainCount }} 条</span>
            </div>
            <textarea
              id="direct-domains"
              v-model="tool.directDomains"
              class="direct-domain-input"
              spellcheck="false"
              placeholder="github.com&#10;example.com"
            />
          </section>

          <section class="transit-section" aria-labelledby="transit-provider-title">
            <div class="vless-section-heading">
              <div>
                <span class="field-label">中转链路</span>
                <strong id="transit-provider-title">Proxy Provider</strong>
              </div>
              <span :class="tool.transitEnabled ? 'status-pill status-pill--good' : 'status-pill status-pill--muted'">
                {{ transitStatus }}
              </span>
            </div>

            <label class="toggle-option toggle-option--card">
              <input v-model="tool.transitEnabled" type="checkbox" />
              <span>
                <strong>启用中转</strong>
                <small>通过 dialer-proxy 将终端节点接入指定中转组。</small>
              </span>
            </label>

            <div v-if="tool.transitEnabled" class="transit-fields">
              <div class="transit-chain" aria-label="中转链路">
                <strong>链路</strong>
                <span>设备</span>
                <span>中转节点</span>
                <span>3x-ui 节点</span>
                <span>目标站点</span>
              </div>

              <label class="field-control" for="transit-provider-url">
                <span class="field-label">中转订阅地址</span>
                <textarea
                  id="transit-provider-url"
                  v-model="tool.transitProviderUrl"
                  class="direct-domain-input transit-provider-url-input"
                  spellcheck="false"
                  placeholder="https://example.com/sub-1.yaml&#10;https://example.com/sub-2.yaml"
                />
              </label>

              <div class="transit-two-col">
                <label class="field-control" for="transit-provider-name">
                  <span class="field-label">Provider 名称</span>
                  <input
                    id="transit-provider-name"
                    v-model="tool.transitProviderName"
                    class="text-input"
                    type="text"
                    placeholder="transit"
                  />
                </label>

                <label class="field-control" for="transit-group-name">
                  <span class="field-label">中转组名</span>
                  <input
                    id="transit-group-name"
                    v-model="tool.transitGroupName"
                    class="text-input"
                    type="text"
                    placeholder="中转节点组"
                  />
                </label>
              </div>

              <label class="field-control" for="transit-provider-path">
                <span class="field-label">Provider 缓存路径</span>
                <input
                  id="transit-provider-path"
                  v-model="tool.transitProviderPath"
                  class="text-input"
                  type="text"
                  placeholder="./proxy_providers/transit.yaml"
                />
              </label>

              <label class="field-control" for="transit-bypass-domains">
                <span class="field-label">仅走中转域名</span>
                <textarea
                  id="transit-bypass-domains"
                  v-model="tool.transitBypassDomains"
                  class="direct-domain-input"
                  spellcheck="false"
                  placeholder="youtube.com&#10;netflix.com"
                />
                <small class="field-hint">{{ transitBypassCount }} 条中转直达规则</small>
              </label>

              <div class="vless-choice-grid vless-choice-grid--three" role="radiogroup" aria-label="中转组类型">
                <label
                  v-for="mode in transitModes"
                  :key="mode.value"
                  :class="optionClass(tool.transitGroupType === mode.value)"
                >
                  <input v-model="tool.transitGroupType" type="radio" :value="mode.value" />
                  <span>
                    <strong>{{ mode.title }}</strong>
                    <small>{{ mode.description }}</small>
                  </span>
                </label>
              </div>
            </div>
          </section>

          <div class="vless-primary-action">
            <Button class="w-full" size="lg" type="button" :disabled="!tool.canConvert" @click="tool.convert">
              <Loader2 v-if="tool.loading" class="mr-2 h-4 w-4 animate-spin" aria-hidden="true" />
              <Download v-else class="mr-2 h-4 w-4" aria-hidden="true" />
              {{ convertButtonLabel }}
            </Button>
            <small v-if="tool.savingSettings">正在保存配置</small>
          </div>

          <p v-if="tool.error" class="error-message">{{ tool.error }}</p>
        </div>

        <div class="result-column vless-result-column">
          <section class="guide-panel vless-import-panel" aria-labelledby="clash-party-resources">
            <div>
              <span class="field-label">导入目标</span>
              <h3 id="clash-party-resources">Clash Party / Mihomo</h3>
            </div>
            <div class="vless-import-facts">
              <span>
                <Network class="h-4 w-4" aria-hidden="true" />
                本地 YAML
              </span>
              <span>
                <FileCode2 class="h-4 w-4" aria-hidden="true" />
                {{ tool.downloadFilename }}
              </span>
            </div>
            <div class="guide-links">
              <a href="https://clashparty.org/" target="_blank" rel="noreferrer">
                <span>Clash Party</span>
                <ExternalLink class="h-3.5 w-3.5" aria-hidden="true" />
              </a>
              <a href="https://github.com/mihomo-party-org/clash-party" target="_blank" rel="noreferrer">
                <span>GitHub</span>
                <ExternalLink class="h-3.5 w-3.5" aria-hidden="true" />
              </a>
            </div>
          </section>

          <ResultPanel
            :yaml="tool.yaml"
            :copied="tool.copied"
            :filename="tool.downloadFilename"
            :node-address="tool.nodeAddress"
            @copied="tool.markCopied"
          />
        </div>
      </section>
    </div>
  </ToolShell>
</template>
