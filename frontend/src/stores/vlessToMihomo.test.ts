import { nextTick } from 'vue'
import { setActivePinia, createPinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { convertVlessToMihomo } from '../api/tools'
import { useVlessToMihomoStore } from './vlessToMihomo'

vi.mock('../api/tools', () => ({
  convertVlessToMihomo: vi.fn(async () => ({ yaml: 'type: vless\n' })),
  getVlessToolSettings: vi.fn(async () => ({
    input: '',
    mode: 'full_config',
    template: 'full_rules',
    downloadName: 'mihomo',
    directDomains: '',
    transitEnabled: false,
    transitProviderUrl: '',
    transitProviderName: 'transit',
    transitProviderPath: '',
    transitGroupName: '中转节点组',
    transitGroupType: 'url_test',
    transitBypassDomains: '',
  })),
  saveVlessToolSettings: vi.fn(async (settings) => settings),
}))

describe('useVlessToMihomoStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('disables conversion while input is empty', () => {
    const store = useVlessToMihomoStore()

    expect(store.canConvert).toBe(false)
  })

  it('stores YAML after conversion', async () => {
    const store = useVlessToMihomoStore()
    store.input = 'vless://example'

    await store.convert()

    expect(store.yaml).toBe('type: vless\n')
    expect(store.error).toBe('')
  })

  it('defaults to full rules template', () => {
    const store = useVlessToMihomoStore()

    expect(store.template).toBe('full_rules')
  })

  it('normalizes download filename with yaml extension', () => {
    const store = useVlessToMihomoStore()
    store.updateDownloadName('my-config')

    expect(store.downloadFilename).toBe('my-config.yaml')
  })

  it('uses vless fragment as the default download and proxy name', async () => {
    const store = useVlessToMihomoStore()

    store.input = 'vless://11111111-1111-1111-1111-111111111111@example.com:443#My-Reality'
    await nextTick()

    expect(store.downloadName).toBe('My-Reality')
    expect(store.downloadFilename).toBe('My-Reality.yaml')
    expect(store.proxyName).toBe('My-Reality')
    expect(store.nodeAddress).toBe('example.com:443')
  })

  it('defaults node address port by security type', () => {
    const store = useVlessToMihomoStore()

    store.input = 'vless://11111111-1111-1111-1111-111111111111@example.com?security=reality'

    expect(store.nodeAddress).toBe('example.com:443')
  })

  it('sends edited download name as proxy name', async () => {
    const store = useVlessToMihomoStore()
    store.input = 'vless://11111111-1111-1111-1111-111111111111@example.com:443#Original'
    store.updateDownloadName('Edited.yaml')

    await store.convert()

    expect(convertVlessToMihomo).toHaveBeenCalledWith(
      expect.objectContaining({
        proxy_name: 'Edited',
      }),
    )
  })

  it('sends transit proxy provider settings when enabled', async () => {
    const store = useVlessToMihomoStore()
    store.input = 'vless://11111111-1111-1111-1111-111111111111@example.com:443#Original'
    store.transitEnabled = true
    store.transitProviderUrl = 'https://example.com/sushi.yaml'
    store.transitProviderName = 'sushi'
    store.transitGroupName = '寿司云中转'
    store.transitGroupType = 'fallback'
    store.transitBypassDomains = 'youtube.com\nhttps://netflix.com/watch'

    await store.convert()

    expect(convertVlessToMihomo).toHaveBeenCalledWith(
      expect.objectContaining({
        transit_proxy: {
          provider_name: 'sushi',
          provider_url: 'https://example.com/sushi.yaml',
          provider_path: undefined,
          group_name: '寿司云中转',
          group_type: 'fallback',
          bypass_domains: ['youtube.com', 'https://netflix.com/watch'],
          providers: undefined,
        },
      }),
    )
  })

  it('splits multiple transit subscription URLs into provider entries', async () => {
    const store = useVlessToMihomoStore()
    store.input = 'vless://11111111-1111-1111-1111-111111111111@example.com:443#Original'
    store.transitEnabled = true
    store.transitProviderUrl = 'https://example.com/sushi.yaml\nhttps://example.com/fast.yaml'
    store.transitProviderName = 'transit'
    store.transitGroupName = '中转总组'

    await store.convert()

    expect(convertVlessToMihomo).toHaveBeenCalledWith(
      expect.objectContaining({
        transit_proxy: expect.objectContaining({
          provider_name: 'transit',
          provider_url: undefined,
          provider_path: undefined,
          group_name: '中转总组',
          providers: [
            {
              provider_name: 'transit-1',
              provider_url: 'https://example.com/sushi.yaml',
              provider_path: undefined,
              group_name: '中转总组-1',
            },
            {
              provider_name: 'transit-2',
              provider_url: 'https://example.com/fast.yaml',
              provider_path: undefined,
              group_name: '中转总组-2',
            },
          ],
        }),
      }),
    )
  })
})
