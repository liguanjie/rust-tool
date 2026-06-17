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
import { useVlessToMihomoStore } from '../stores/vlessToMihomo'

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
  <div style="padding: 24px; max-width: 1200px; margin: 0 auto;">
    <a-page-header
      title="VLESS 转 Mihomo"
      sub-title="把 3x-ui 节点转换成可导入、可审计、可复用的本地 Mihomo 配置。"
      style="padding-left: 0; padding-right: 0;"
    >
      <template #tags>
        <a-tag color="blue">网络配置</a-tag>
      </template>
    </a-page-header>

    <a-row :gutter="16" style="margin-bottom: 24px;">
      <a-col :span="6">
        <a-card>
          <a-statistic title="转换状态" :value="resultStatus" />
          <div style="margin-top: 8px;">
            <a-tag :color="tool.yaml ? 'success' : tool.loading ? 'warning' : 'default'">
               {{ tool.nodeAddress || '等待 VLESS 链接' }}
            </a-tag>
          </div>
        </a-card>
      </a-col>
      <a-col :span="6">
        <a-card>
          <a-statistic title="源链接数" :value="linkCount" />
        </a-card>
      </a-col>
      <a-col :span="6">
        <a-card>
          <a-statistic title="输出模式" :value="outputModeLabel" />
        </a-card>
      </a-col>
      <a-col :span="6">
        <a-card>
          <a-statistic title="中转状态" :value="transitStatus" />
        </a-card>
      </a-col>
    </a-row>

    <a-row :gutter="24">
      <a-col :span="14">
        <a-card title="配置参数" style="margin-bottom: 24px;">
          <a-form layout="vertical">
            <a-form-item label="源链接 (VLESS 输入)">
              <template #extra>
                 <div style="display: flex; justify-content: space-between; margin-top: 4px;">
                    <span style="color: gray;">{{ linkCount }} 个链接</span>
                    <span>
                      <CheckCircle2 class="h-3 w-3 inline" /> {{ tool.nodeAddress || '等待解析节点地址' }}
                      <a-divider type="vertical" />
                      <FileCode2 class="h-3 w-3 inline" /> {{ tool.downloadFilename }}
                    </span>
                 </div>
              </template>
              <a-textarea
                v-model:value="tool.input"
                :rows="4"
                placeholder="每行一个 vless:// 链接"
                style="font-family: monospace;"
              />
            </a-form-item>

            <a-form-item label="输出策略 (配置形态)">
              <a-radio-group v-model:value="tool.mode" button-style="solid">
                <a-radio-button v-for="mode in outputModes" :key="mode.value" :value="mode.value">
                  {{ mode.title }}
                </a-radio-button>
              </a-radio-group>
              <div style="margin-top: 8px; color: gray; font-size: 12px;">
                 {{ outputModes.find(m => m.value === tool.mode)?.description }}
              </div>
            </a-form-item>

            <a-form-item label="文件名 / 节点名">
              <a-input
                :value="tool.downloadName"
                @update:value="(val: string) => tool.updateDownloadName(val)"
                placeholder="mihomo"
              />
            </a-form-item>

            <template v-if="tool.mode === 'full_config'">
              <a-form-item label="规则模板 (路由基线)">
                <a-radio-group v-model:value="tool.template" button-style="solid">
                  <a-radio-button v-for="template in templateModes" :key="template.value" :value="template.value">
                    {{ template.title }}
                  </a-radio-button>
                </a-radio-group>
                <div style="margin-top: 8px; color: gray; font-size: 12px;">
                   {{ templateModes.find(t => t.value === tool.template)?.description }}
                </div>
              </a-form-item>

              <a-form-item label="直连域名 (绕过代理)">
                <template #extra>
                  <span style="color: gray;">{{ directDomainCount }} 条</span>
                </template>
                <a-textarea
                  v-model:value="tool.directDomains"
                  :rows="3"
                  placeholder="github.com&#10;example.com"
                  style="font-family: monospace;"
                />
              </a-form-item>
            </template>
          </a-form>
        </a-card>

        <a-card title="中转链路 (Proxy Provider)" style="margin-bottom: 24px;">
           <template #extra>
              <a-switch v-model:checked="tool.transitEnabled" />
           </template>

           <div v-if="tool.transitEnabled">
              <p style="color: gray; margin-bottom: 16px;">通过 dialer-proxy 将终端节点接入指定中转组。</p>
              <a-form layout="vertical">
                 <a-form-item label="中转订阅地址">
                   <a-textarea
                     v-model:value="tool.transitProviderUrl"
                     :rows="2"
                     placeholder="https://example.com/sub-1.yaml&#10;https://example.com/sub-2.yaml"
                     style="font-family: monospace;"
                   />
                 </a-form-item>

                 <a-row :gutter="16">
                    <a-col :span="12">
                      <a-form-item label="Provider 名称">
                        <a-input v-model:value="tool.transitProviderName" placeholder="transit" />
                      </a-form-item>
                    </a-col>
                    <a-col :span="12">
                      <a-form-item label="中转组名">
                        <a-input v-model:value="tool.transitGroupName" placeholder="中转节点组" />
                      </a-form-item>
                    </a-col>
                 </a-row>

                 <a-form-item label="Provider 缓存路径">
                   <a-input v-model:value="tool.transitProviderPath" placeholder="./proxy_providers/transit.yaml" />
                 </a-form-item>

                 <a-form-item label="仅走中转域名">
                   <template #extra>
                     <span style="color: gray;">{{ transitBypassCount }} 条中转直达规则</span>
                   </template>
                   <a-textarea
                     v-model:value="tool.transitBypassDomains"
                     :rows="2"
                     placeholder="youtube.com&#10;netflix.com"
                     style="font-family: monospace;"
                   />
                 </a-form-item>

                 <a-form-item label="中转组类型">
                   <a-radio-group v-model:value="tool.transitGroupType" button-style="solid">
                     <a-radio-button v-for="mode in transitModes" :key="mode.value" :value="mode.value">
                       {{ mode.title }}
                     </a-radio-button>
                   </a-radio-group>
                   <div style="margin-top: 8px; color: gray; font-size: 12px;">
                      {{ transitModes.find(m => m.value === tool.transitGroupType)?.description }}
                   </div>
                 </a-form-item>
              </a-form>
           </div>
           <div v-else style="color: gray; text-align: center; padding: 24px;">
              中转功能未启用
           </div>
        </a-card>

        <div style="margin-bottom: 24px;">
           <a-button type="primary" block size="large" :disabled="!tool.canConvert" @click="tool.convert">
             <Loader2 v-if="tool.loading" class="mr-2 h-4 w-4 animate-spin" style="display: inline-block" />
             <Download v-else class="mr-2 h-4 w-4" style="display: inline-block" />
             {{ convertButtonLabel }}
           </a-button>
           <div v-if="tool.savingSettings" style="text-align: center; margin-top: 8px; color: gray;">
             <small>正在保存配置</small>
           </div>
           <a-alert v-if="tool.error" type="error" :message="tool.error" show-icon style="margin-top: 16px;" />
        </div>
      </a-col>

      <a-col :span="10">
        <a-card title="导入目标: Clash Party / Mihomo" size="small" style="margin-bottom: 24px;">
          <div style="display: flex; gap: 16px; margin-bottom: 16px;">
            <a-tag color="blue">
              <Network class="h-3 w-3 inline" /> 本地 YAML
            </a-tag>
            <a-tag>
              <FileCode2 class="h-3 w-3 inline" /> {{ tool.downloadFilename }}
            </a-tag>
          </div>
          <div style="display: flex; gap: 16px;">
            <a href="https://clashparty.org/" target="_blank" style="color: #1677ff; text-decoration: none;">
              Clash Party <ExternalLink class="h-3 w-3 inline" />
            </a>
            <a href="https://github.com/mihomo-party-org/clash-party" target="_blank" style="color: #1677ff; text-decoration: none;">
              GitHub <ExternalLink class="h-3 w-3 inline" />
            </a>
          </div>
        </a-card>

        <ResultPanel
          :yaml="tool.yaml"
          :copied="tool.copied"
          :filename="tool.downloadFilename"
          :node-address="tool.nodeAddress"
          @copied="tool.markCopied"
        />
      </a-col>
    </a-row>
  </div>
</template>
