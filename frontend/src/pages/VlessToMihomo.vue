<script setup lang="ts">
import { onMounted } from 'vue'
import { ExternalLink } from '@lucide/vue'
import ResultPanel from '../components/ResultPanel.vue'
import ToolShell from '../components/ToolShell.vue'
import { useVlessToMihomoStore } from '../stores/vlessToMihomo'

const tool = useVlessToMihomoStore()

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
      <section class="input-panel">
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
                <small>引用社区规则集，覆盖广告、AI、媒体、Google、Telegram、国内直连；单节点下各分类最终仍会走同一个代理。</small>
              </span>
            </label>
            <label class="template-option">
              <input v-model="tool.template" type="radio" value="standard" />
              <span>
                <strong>基础分流</strong>
                <small>本机/局域网/国内直连，其他代理</small>
              </span>
            </label>
            <label class="template-option">
              <input v-model="tool.template" type="radio" value="minimal" />
              <span>
                <strong>最小配置</strong>
                <small>所有流量走代理</small>
              </span>
            </label>
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
