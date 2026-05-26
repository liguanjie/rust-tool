<script setup lang="ts">
import ResultPanel from '../components/ResultPanel.vue'
import ToolShell from '../components/ToolShell.vue'
import { useVlessToMihomoStore } from '../stores/vlessToMihomo'

const tool = useVlessToMihomoStore()
</script>

<template>
  <ToolShell
    title="VLESS 转 Mihomo"
    description="将 3x-ui 生成的 vless:// 链接转换为 Clash Party/Mihomo YAML。"
  >
    <div class="tool-grid">
      <section class="input-panel">
        <label class="field-label" for="vless-input">VLESS 链接</label>
        <textarea
          id="vless-input"
          v-model="tool.input"
          class="vless-input"
          spellcheck="false"
          placeholder="vless://uuid@example.com:443?type=tcp&security=reality..."
        />

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
              <input v-model="tool.template" type="radio" value="minimal" />
              <span>
                <strong>最小配置</strong>
                <small>所有流量走代理</small>
              </span>
            </label>
            <label class="template-option">
              <input v-model="tool.template" type="radio" value="standard" />
              <span>
                <strong>基础分流</strong>
                <small>国内直连，其他代理</small>
              </span>
            </label>
            <label class="template-option">
              <input v-model="tool.template" type="radio" value="full_rules" />
              <span>
                <strong>多节点分流模板</strong>
                <small>适合未来多个节点/订阅源使用；单个 VLESS 节点下，各分类最终仍会走同一个代理节点。</small>
              </span>
            </label>
          </div>
        </div>

        <button class="primary-button" type="button" :disabled="!tool.canConvert" @click="tool.convert">
          {{ tool.loading ? '转换中...' : '转换' }}
        </button>

        <p v-if="tool.error" class="error-message">{{ tool.error }}</p>
      </section>

      <ResultPanel
        :yaml="tool.yaml"
        :copied="tool.copied"
        :filename="tool.downloadFilename"
        @copied="tool.markCopied"
      />
    </div>
  </ToolShell>
</template>
