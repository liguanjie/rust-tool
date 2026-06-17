import { setActivePinia, createPinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { defaultOsvScanOptions, type OsvCommandPreview, type OsvScanResult } from '../api/osvScanner'
import { useOsvScannerStore } from './osvScanner'

const api = vi.hoisted(() => ({
  checkOsvInstalled: vi.fn(),
  getOsvSettings: vi.fn(),
  saveOsvSettings: vi.fn(),
  previewOsvScanCommand: vi.fn(),
  scanOsvProject: vi.fn(),
  previewOsvReportExportCommand: vi.fn(),
  exportOsvReport: vi.fn(),
  ignoreOsvVulnerability: vi.fn(),
}))

vi.mock('../api/osvScanner', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../api/osvScanner')>()
  return {
    ...actual,
    checkOsvInstalled: api.checkOsvInstalled,
    getOsvSettings: api.getOsvSettings,
    saveOsvSettings: api.saveOsvSettings,
    previewOsvScanCommand: api.previewOsvScanCommand,
    scanOsvProject: api.scanOsvProject,
    previewOsvReportExportCommand: api.previewOsvReportExportCommand,
    exportOsvReport: api.exportOsvReport,
    ignoreOsvVulnerability: api.ignoreOsvVulnerability,
  }
})

describe('useOsvScannerStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
    api.getOsvSettings.mockResolvedValue({
      projects: [],
      autoScanSchedule: 'none',
      commandHistory: [],
    })
    api.saveOsvSettings.mockImplementation(async (settings) => settings)
    api.checkOsvInstalled.mockResolvedValue({
      installed: true,
      binaryPath: '/usr/local/bin/osv-scanner',
      version: 'osv-scanner version: 2.3.8',
      message: '已检测到 osv-scanner。',
    })
  })

  it('loads settings and install status', async () => {
    const store = useOsvScannerStore()

    await store.load()

    expect(store.installStatus?.installed).toBe(true)
    expect(store.projects).toEqual([])
  })

  it('selects the first saved project after loading settings', async () => {
    api.getOsvSettings.mockResolvedValue({
      projects: [{ name: 'saved-project', path: '/tmp/saved-project' }],
      autoScanSchedule: 'none',
      commandHistory: [],
    })
    const store = useOsvScannerStore()

    await store.load()

    expect(store.activeProjectPath).toBe('/tmp/saved-project')
    expect(store.activeProject?.name).toBe('saved-project')
  })

  it('adds a project and persists settings', async () => {
    const store = useOsvScannerStore()
    await store.load()

    await store.addProject('/tmp/example-project')

    expect(store.projects).toEqual([
      {
        name: 'example-project',
        path: '/tmp/example-project',
      },
    ])
    expect(api.saveOsvSettings).toHaveBeenCalledWith(
      expect.objectContaining({
        projects: expect.arrayContaining([
          expect.objectContaining({ path: '/tmp/example-project' }),
        ]),
      }),
    )
  })

  it('runs a confirmed scan and appends command history', async () => {
    const store = useOsvScannerStore()
    await store.load()
    await store.addProject('/tmp/example-project')
    api.previewOsvScanCommand.mockResolvedValue(scanPreview())
    api.scanOsvProject.mockResolvedValue(scanResult())

    await store.previewScan()
    await store.runScan()

    expect(store.latestResult?.summary.healthScore).toBe(90)
    expect(store.projects[0].healthScore).toBe(90)
    expect(store.commandHistory).toHaveLength(1)
    expect(store.commandHistory[0].id).toBe('scan-1')
  })

  it('invalidates existing command previews', async () => {
    const store = useOsvScannerStore()
    await store.load()
    await store.addProject('/tmp/example-project')
    api.previewOsvScanCommand.mockResolvedValue(scanPreview())
    api.previewOsvReportExportCommand.mockResolvedValue({
      ...scanPreview(),
      kind: 'export',
    })

    await store.previewScan()
    await store.previewExport('json', '/private/tmp/report.json')
    store.invalidateCommandPreviews()

    expect(store.currentPreview).toBeNull()
    expect(store.currentExportPreview).toBeNull()
  })

  it('clears stale result and previews when selecting another project', async () => {
    const store = useOsvScannerStore()
    await store.load()
    await store.addProject('/tmp/example-project')
    await store.addProject('/tmp/other-project')
    store.selectProject('/tmp/example-project')
    api.previewOsvScanCommand.mockResolvedValue(scanPreview())
    api.scanOsvProject.mockResolvedValue(scanResult())

    await store.previewScan()
    await store.runScan()
    store.selectProject('/tmp/other-project')

    expect(store.latestResult).toBeNull()
    expect(store.currentPreview).toBeNull()
    expect(store.currentExportPreview).toBeNull()
  })

  it('requires a current scan result before export preview', async () => {
    const store = useOsvScannerStore()
    await store.load()
    await store.addProject('/tmp/example-project')

    await store.previewExport('json', '/private/tmp/report.json')

    expect(api.previewOsvReportExportCommand).not.toHaveBeenCalled()
    expect(store.error).toBe('请先完成当前项目扫描后再导出报告')
  })
})

function scanPreview(): OsvCommandPreview {
  return {
    kind: 'scan',
    binary: '/usr/local/bin/osv-scanner',
    workingDir: '/tmp/example-project',
    argv: ['/usr/local/bin/osv-scanner', 'scan', 'source', '--format', 'json', '.'],
    displayCommand: 'osv-scanner scan source --format json .',
    lockedArgs: ['scan', 'source', '--format json'],
    editableOptions: defaultOsvScanOptions(),
    warnings: [],
  }
}

function scanResult(): OsvScanResult {
  return {
    projectPath: '/tmp/example-project',
    vulnerabilities: [],
    summary: {
      totalVulnerabilities: 0,
      severityCounts: {
        critical: 0,
        high: 0,
        medium: 0,
        low: 0,
        unknown: 0,
      },
      highestSeverity: 'unknown',
      healthScore: 90,
      message: '未发现已知漏洞。',
    },
    command: {
      id: 'scan-1',
      kind: 'scan',
      projectPath: '/tmp/example-project',
      workingDir: '/tmp/example-project',
      argv: ['/usr/local/bin/osv-scanner', 'scan', 'source', '--format', 'json', '.'],
      displayCommand: 'osv-scanner scan source --format json .',
      startedAt: '1',
      finishedAt: '2',
      durationMs: 10,
      exitCode: 0,
      status: 'succeeded',
      summary: '未发现已知漏洞。',
    },
  }
}
