import { nextTick } from 'vue'
import { setActivePinia, createPinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { convertVlessToMihomo } from '../api/tools'
import { useVlessToMihomoStore } from './vlessToMihomo'

vi.mock('../api/tools', () => ({
  convertVlessToMihomo: vi.fn(async () => ({ yaml: 'type: vless\n' })),
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

  it('defaults to standard template', () => {
    const store = useVlessToMihomoStore()

    expect(store.template).toBe('standard')
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
})
