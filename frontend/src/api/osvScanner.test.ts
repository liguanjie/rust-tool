import { beforeEach, describe, expect, it, vi } from 'vitest'
import {
  checkOsvInstalled,
  defaultOsvScanOptions,
  diagnoseOsvProject,
  previewOsvScanCommand,
  saveOsvSettings,
  suggestOsvReportPath,
  type OsvCommandExecutionRecord,
} from './osvScanner'

vi.mock('@tauri-apps/api/core', () => ({
  isTauri: vi.fn(() => false),
  invoke: vi.fn(),
}))

describe('osvScanner API adapter', () => {
  const storage = new Map<string, string>()

  beforeEach(() => {
    storage.clear()
    Object.defineProperty(window, 'localStorage', {
      configurable: true,
      value: {
        getItem: vi.fn((key: string) => storage.get(key) ?? null),
        setItem: vi.fn((key: string, value: string) => {
          storage.set(key, value)
        }),
        removeItem: vi.fn((key: string) => {
          storage.delete(key)
        }),
      },
    })
  })

  it('calls the Web preview endpoint with JSON payload', async () => {
    const fetchMock = vi.fn(async () =>
      new Response(
        JSON.stringify({
          kind: 'scan',
          binary: '/usr/local/bin/osv-scanner',
          workingDir: '/tmp/project',
          argv: ['/usr/local/bin/osv-scanner', 'scan', 'source', '--format', 'json', '.'],
          displayCommand: "cd /tmp/project && /usr/local/bin/osv-scanner scan source --format json .",
          lockedArgs: ['scan', 'source', '--format json'],
          editableOptions: defaultOsvScanOptions(),
          warnings: [],
        }),
        { status: 200, headers: { 'Content-Type': 'application/json' } },
      ),
    )
    vi.stubGlobal('fetch', fetchMock)

    const result = await previewOsvScanCommand({
      projectPath: '/tmp/project',
      options: defaultOsvScanOptions(),
    })

    expect(result.kind).toBe('scan')
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/tools/osv-scanner/scan/preview',
      expect.objectContaining({
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: expect.stringContaining('"projectPath":"/tmp/project"'),
      }),
    )
  })

  it('uses the backend error message from standard error responses', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () =>
        new Response(
          JSON.stringify({
            error: {
              code: 'invalid_project_path',
              message: '项目路径无效',
            },
          }),
          { status: 400, headers: { 'Content-Type': 'application/json' } },
        ),
      ),
    )

    await expect(checkOsvInstalled()).rejects.toThrow('项目路径无效')
  })

  it('uses plain text when an error response is not JSON', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => new Response('backend unavailable', { status: 503 })),
    )

    await expect(checkOsvInstalled()).rejects.toThrow('backend unavailable')
  })

  it('does not surface HTML fallback pages as API errors', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(
        async () =>
          new Response('<!doctype html><html><body>Not found</body></html>', { status: 404 }),
      ),
    )

    await expect(checkOsvInstalled()).rejects.toThrow('OSV 操作失败')
  })

  it('calls the Web diagnose endpoint with scan options', async () => {
    const fetchMock = vi.fn(async () =>
      new Response(
        JSON.stringify({
          projectPath: '/tmp/project',
          packageSources: [{ path: 'Cargo.lock', kind: 'Cargo.lock', ecosystem: 'Rust', explicit: false }],
          messages: [],
          canScan: true,
          scannedEntries: 1,
          truncated: false,
        }),
        { status: 200, headers: { 'Content-Type': 'application/json' } },
      ),
    )
    vi.stubGlobal('fetch', fetchMock)

    const result = await diagnoseOsvProject({
      projectPath: '/tmp/project',
      options: defaultOsvScanOptions(),
    })

    expect(result.packageSources[0].path).toBe('Cargo.lock')
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/tools/osv-scanner/diagnose',
      expect.objectContaining({
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: expect.stringContaining('"projectPath":"/tmp/project"'),
      }),
    )
  })

  it('trims command history before saving Web settings', async () => {
    const history = Array.from({ length: 55 }, (_, index) => commandRecord(`record-${index}`))

    const saved = await saveOsvSettings({
      projects: [],
      autoScanSchedule: 'none',
      commandHistory: history,
    })

    expect(saved.commandHistory).toHaveLength(50)
    expect(saved.commandHistory[0].id).toBe('record-5')
    expect(saved.commandHistory[49].id).toBe('record-54')
  })

  it('suggests a Web export path under /private/tmp with the chosen extension', async () => {
    const path = await suggestOsvReportPath('/Users/ben/My Project', 'html')

    expect(path).toMatch(/^\/private\/tmp\/My-Project-osv-report-\d+\.html$/)
  })
})

function commandRecord(id: string): OsvCommandExecutionRecord {
  return {
    id,
    kind: 'scan',
    projectPath: '/tmp/project',
    workingDir: '/tmp/project',
    argv: ['osv-scanner', 'scan', 'source', '--format', 'json', '.'],
    displayCommand: 'osv-scanner scan source --format json .',
    startedAt: '1',
    finishedAt: '2',
    durationMs: 1,
    exitCode: 0,
    status: 'succeeded',
    summary: 'ok',
  }
}
