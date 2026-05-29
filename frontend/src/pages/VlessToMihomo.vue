<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { ExternalLink } from '@lucide/vue'
import ResultPanel from '../components/ResultPanel.vue'
import ToolShell from '../components/ToolShell.vue'
import { useVlessToMihomoStore } from '../stores/vlessToMihomo'

const tool = useVlessToMihomoStore()
const templateHints = {
  full_rules: '规则集分流，覆盖广告、AI、媒体、Google、Telegram 与国内直连。',
  standard: '本机、局域网、国内直连；其他走代理。',
  minimal: '仅保留代理节点和兜底规则。',
}
const transitGroupHints = {
  url_test: '刷新节奏：订阅节点 1 小时 > 节点测速 5 分钟 > 自动选择最优中转。',
  fallback: '按顺序使用可用节点，当前节点不可用时自动切到下一个。',
  select: '在 Clash Party 里手动指定中转节点，不自动切换。',
}
const selectedTemplateHint = computed(() => templateHints[tool.template])
const selectedTransitGroupHint = computed(() => transitGroupHints[tool.transitGroupType])

onMounted(() => {
  void tool.load()
})
</script>

<template>
  <ToolShell
    title="VLESS 转 Mihomo"
    description="将 3x-ui 生成的 vless:// 链接转换为 Clash Party/Mihomo YAML。"
    :breadcrumbs="[
      { label: '工具箱', to: '/toolbox' },
      { label: 'VLESS 转 Mihomo' },
    ]"
  >
    <div class="tool-grid">
      <section class="input-panel vless-config-panel">
        <label class="field-label" for="vless-input">VLESS 链接</label>
        <textarea
          id="vless-input"
          v-model="tool.input"
          class="vless-input"
          spellcheck="false"
          placeholder="每行一个 vless:// 链接&#10;vless://uuid@example.com:443?type=tcp&security=reality..."
        />
        <small class="field-hint">支持单条或多条 VLESS 链接；多条时会生成多个节点并加入同一份 Mihomo 配置。</small>

        <label class="field-control file-name-field" for="download-name">
          <span class="field-label">下载文件名</span>
          <input
            id="download-name"
            :value="tool.downloadName"
            class="text-input"
            type="text"
            placeholder="自动读取链接 # 后面的名称"
            @input="tool.updateDownloadName(($event.target as HTMLInputElement).value)"
          />
        </label>

        <div class="mode-row" role="radiogroup" aria-label="输出格式">
          <label class="mode-option">
            <input v-model="tool.mode" type="radio" value="full_config" />
            <span>完整配置</span>
          </label>
          <label class="mode-option">
            <input v-model="tool.mode" type="radio" value="proxy_only" />
            <span>仅节点片段</span>
          </label>
        </div>

        <div v-if="tool.mode === 'full_config'" class="template-section">
          <p class="field-label">配置模板</p>
          <div class="template-options" role="radiogroup" aria-label="配置模板">
            <label class="template-option">
              <input v-model="tool.template" type="radio" value="full_rules" />
              <span>
                <strong>多节点分流模板</strong>
              </span>
            </label>
            <label class="template-option">
              <input v-model="tool.template" type="radio" value="standard" />
              <span>
                <strong>基础分流</strong>
              </span>
            </label>
            <label class="template-option">
              <input v-model="tool.template" type="radio" value="minimal" />
              <span>
                <strong>最小配置</strong>
              </span>
            </label>
          </div>
          <div class="template-hint">
            <strong>当前模板</strong>
            <span>{{ selectedTemplateHint }}</span>
          </div>
        </div>

        <label v-if="tool.mode === 'full_config'" class="field-control direct-domain-field" for="direct-domains">
          <span class="field-label">特殊直连域名</span>
          <textarea
            id="direct-domains"
            v-model="tool.directDomains"
            class="direct-domain-input"
            spellcheck="false"
            placeholder="github.com&#10;example.com"
          />
          <small class="field-hint">每行一条；会生成 DOMAIN-SUFFIX 直连规则，并优先于代理兜底匹配。</small>
        </label>

        <section class="transit-section" aria-labelledby="transit-provider-title">
          <div class="section-divider">
            <span>中转配置</span>
          </div>

          <label class="toggle-option">
            <input v-model="tool.transitEnabled" type="checkbox" />
            <span>
              <strong id="transit-provider-title">启用 Proxy Provider 中转</strong>
              <small>将任意 Clash/Mihomo 订阅作为中转组，3x-ui 节点会通过 dialer-proxy 先走该组。</small>
            </span>
          </label>

          <div v-if="tool.transitEnabled" class="transit-fields">
            <div class="transit-chain" aria-label="中转链路说明">
              <strong>流量链路</strong>
              <span>设备/浏览器</span>
              <span>中转节点 (VPN)</span>
              <span>终端节点 (3x-ui)</span>
              <span>最终目标 (google.com)</span>
            </div>

            <label class="field-control" for="transit-provider-url">
              <span class="field-label">中转订阅地址</span>
              <input
                id="transit-provider-url"
                v-model="tool.transitProviderUrl"
                class="text-input"
                type="url"
                placeholder="https://example.com/sub.yaml"
              />
              <small class="field-hint">需要是 Clash/Mihomo 可解析订阅；生成到 proxy-providers。</small>
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

            <div class="mode-row" role="radiogroup" aria-label="中转组类型">
              <label class="mode-option">
                <input v-model="tool.transitGroupType" type="radio" value="url_test" />
                <span>自动测速</span>
              </label>
              <label class="mode-option">
                <input v-model="tool.transitGroupType" type="radio" value="fallback" />
                <span>故障切换</span>
              </label>
              <label class="mode-option">
                <input v-model="tool.transitGroupType" type="radio" value="select" />
                <span>手动选择</span>
              </label>
            </div>

            <div class="transit-mode-hint">
              <strong>中转模式</strong>
              <span>{{ selectedTransitGroupHint }}</span>
            </div>
          </div>
        </section>

        <button class="primary-button" type="button" :disabled="!tool.canConvert" @click="tool.convert">
          {{ tool.loading ? '转换中...' : '转换' }}
        </button>

        <p v-if="tool.error" class="error-message">{{ tool.error }}</p>
      </section>

      <div class="result-column">
        <section class="guide-panel" aria-labelledby="clash-party-guide">
          <h3 id="clash-party-guide">导入到 Clash Party</h3>
          <ol>
            <li>粘贴 3x-ui 生成的 vless:// 链接</li>
            <li>转换并下载 YAML 文件</li>
            <li>打开 Clash Party，将 YAML 作为本地配置导入</li>
          </ol>
          <p>这里生成的是本地配置文件，不是订阅服务。</p>
          <div class="guide-links">
            <a href="https://clashparty.org/" target="_blank" rel="noreferrer">
              <span>Clash Party 官网</span>
              <ExternalLink class="h-3.5 w-3.5" aria-hidden="true" />
            </a>
            <a href="https://github.com/mihomo-party-org/clash-party" target="_blank" rel="noreferrer">
              <span>GitHub 项目</span>
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
    </div>
  </ToolShell>
</template>
