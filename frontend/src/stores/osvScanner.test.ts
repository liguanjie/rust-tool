import { setActivePinia, createPinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { defaultOsvScanOptions, type OsvCommandPreview, type OsvScanResult } from '../api/osvScanner'
import { useOsvScannerStore } from './osvScanner'

const api = vi.hoisted(() => ({
  checkOsvInstalled: vi.fn(),
  getOsvSettings: vi.fn(),
  saveOsvSettings: vi.fn(),
  diagnoseOsvProject: vi.fn(),
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
    diagnoseOsvProject: api.diagnoseOsvProject,
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
    api.diagnoseOsvProject.mockResolvedValue(projectDiagnostic())
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
    expect(store.operation.status).toBe('succeeded')
    expect(store.operation.command).toBe('osv-scanner scan source --format json .')
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

  it('blocks preview when diagnosis has hard errors', async () => {
    api.diagnoseOsvProject.mockResolvedValue({
      ...projectDiagnostic(),
      canScan: false,
      messages: [{
        level: 'error',
        code: 'lockfile_not_found',
        message: '指定的 lockfile 不存在。',
        suggestion: '请检查路径。',
      }],
    })
    const store = useOsvScannerStore()
    await store.load()
    await store.addProject('/tmp/example-project')

    await store.previewScan()

    expect(api.previewOsvScanCommand).not.toHaveBeenCalled()
    expect(store.currentPreview).toBeNull()
    expect(store.error).toContain('指定的 lockfile 不存在')
    expect(store.operation.status).toBe('failed')
  })

  it('keeps a visible operation record after preview and allows dismissal', async () => {
    const store = useOsvScannerStore()
    await store.load()
    await store.addProject('/tmp/example-project')
    api.previewOsvScanCommand.mockResolvedValue(scanPreview())

    await store.previewScan()

    expect(store.operation.status).toBe('succeeded')
    expect(store.operation.message).toContain('扫描命令已生成')
    expect(store.operation.command).toContain('osv-scanner scan')

    store.dismissOperation()

    expect(store.operation.status).toBe('idle')
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

function projectDiagnostic() {
  return {
    projectPath: '/tmp/example-project',
    packageSources: [{ path: 'Cargo.lock', kind: 'Cargo.lock', ecosystem: 'Rust', explicit: false }],
    messages: [],
    canScan: true,
    scannedEntries: 1,
    truncated: false,
  }
}
