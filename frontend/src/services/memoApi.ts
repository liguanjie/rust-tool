type TauriWindow = Window & {
  __TAURI_INTERNALS__?: unknown
  __TAURI__?: unknown
}

type MemoCommandArgs = Record<string, unknown>

interface CommandCall {
  command: string
  args: MemoCommandArgs
}

export async function memoRequest(path: string, init: RequestInit = {}): Promise<Response> {
  if (!isTauriRuntime()) {
    return fetch(`/api/memo${path}`, init)
  }

  try {
    const { invoke } = await import('@tauri-apps/api/core')
    const call = await toCommandCall(path, init)
    const data = await invoke(call.command, call.args)
    return jsonResponse(data)
  } catch (error) {
    return errorResponse(error)
  }
}

function isTauriRuntime() {
  if (typeof window === 'undefined') {
    return false
  }
  const tauriWindow = window as TauriWindow
  return Boolean(tauriWindow.__TAURI_INTERNALS__ || tauriWindow.__TAURI__)
}

async function toCommandCall(path: string, init: RequestInit): Promise<CommandCall> {
  const method = (init.method || 'GET').toUpperCase()
  const payload = await readJsonBody(init)

  if (method === 'GET' && path === '/status') {
    return command('memo_status')
  }
  if (method === 'GET' && path === '/data-dir') {
    return command('memo_data_dir')
  }
  if (method === 'POST' && path === '/unlock') {
    return command('memo_unlock', { payload })
  }
  if (method === 'POST' && path === '/lock') {
    return command('memo_lock')
  }
  if (method === 'POST' && path === '/settings') {
    return command('memo_update_settings', { payload })
  }
  if (method === 'POST' && path === '/data-dir/migrate') {
    return command('memo_migrate_data_dir', { payload })
  }
  if (method === 'POST' && path === '/test-connection') {
    return command('memo_test_connection', { payload })
  }
  if (method === 'GET' && path === '/list') {
    return command('memo_list_documents')
  }
  if (method === 'GET' && path.startsWith('/doc/')) {
    return command('memo_get_document', { id: decodeURIComponent(path.slice('/doc/'.length)) })
  }
  if (method === 'POST' && path === '/audit/scan') {
    return command('memo_audit_scan', { payload })
  }
  if (method === 'POST' && path === '/audit/finding/status') {
    return command('memo_audit_update_finding_status', { payload })
  }
  if (method === 'POST' && path === '/audit/fix-preview') {
    return command('memo_audit_fix_preview', { payload })
  }
  if (method === 'POST' && path === '/audit/redact') {
    return command('memo_audit_redact', { payload })
  }
  if (method === 'POST' && path === '/history/doc-diff') {
    return command('memo_document_risk_diff', { payload })
  }
  if (method === 'GET' && path === '/governance/summary') {
    return command('memo_governance_summary')
  }
  if (method === 'GET' && path === '/governance/cases') {
    return command('memo_governance_cases')
  }
  if (method === 'GET' && path === '/governance/events') {
    return command('memo_governance_events')
  }
  if (method === 'POST' && path === '/governance/case/status') {
    return command('memo_governance_update_case_status', { payload })
  }
  if (method === 'POST' && path === '/governance/case/accept') {
    return command('memo_governance_accept_case', { payload })
  }
  if (method === 'GET' && path === '/assets/list') {
    return command('memo_assets_list')
  }
  if (method === 'GET' && path.startsWith('/assets/detail')) {
    const queryIndex = path.indexOf('?')
    const params = queryIndex >= 0 ? new URLSearchParams(path.slice(queryIndex + 1)) : new URLSearchParams()
    return command('memo_assets_detail', {
      payload: {
        assetId: params.get('assetId') || null,
        query: params.get('query') || null,
      },
    })
  }
  if (method === 'GET' && path.startsWith('/assets/graph')) {
    const queryIndex = path.indexOf('?')
    const params = queryIndex >= 0 ? new URLSearchParams(path.slice(queryIndex + 1)) : new URLSearchParams()
    return command('memo_assets_graph', {
      payload: {
        assetId: params.get('assetId') || null,
        query: params.get('query') || null,
      },
    })
  }
  if (method === 'POST' && path === '/reports/generate') {
    return command('memo_generate_security_report', { payload })
  }
  if (method === 'POST' && path === '/share/export') {
    return command('memo_safe_share_export', { payload })
  }
  if (method === 'GET' && path === '/standards/list') {
    return command('memo_standards_list')
  }
  if (method === 'GET' && path.startsWith('/standards/checklist')) {
    const queryIndex = path.indexOf('?')
    const params = queryIndex >= 0 ? new URLSearchParams(path.slice(queryIndex + 1)) : new URLSearchParams()
    return command('memo_standards_checklist', {
      payload: { docId: params.get('docId') || null },
    })
  }
  if (method === 'POST' && path === '/standards/checklist/status') {
    return command('memo_standards_update_checklist_status', { payload })
  }
  if (method === 'GET' && path === '/secrets') {
    return command('memo_list_secrets')
  }
  if (method === 'POST' && path === '/secrets/reveal') {
    return command('memo_reveal_secret', { payload })
  }
  if (method === 'POST' && path === '/change-master-password') {
    return command('memo_change_master_password', { payload })
  }
  if (method === 'POST' && path === '/save') {
    return command('memo_save_document', { payload })
  }
  if (method === 'POST' && path === '/draft') {
    return command('memo_draft_document', { payload })
  }
  if (method === 'POST' && path === '/delete') {
    return command('memo_delete_document', { payload })
  }
  if (method === 'POST' && path === '/query') {
    return command('memo_query', { payload })
  }
  if (method === 'POST' && path === '/chat') {
    return command('memo_chat', { payload })
  }
  if (method === 'POST' && path === '/backup') {
    return command('memo_backup', { payload })
  }
  if (method === 'POST' && path === '/restore') {
    return command('memo_restore', { payload })
  }
  if (method === 'POST' && path === '/translate-key') {
    return command('memo_translate_key', { payload })
  }

  throw new Error(`Unsupported memo API route: ${method} ${path}`)
}

function command(commandName: string, args: MemoCommandArgs = {}): CommandCall {
  return {
    command: commandName,
    args,
  }
}

async function readJsonBody(init: RequestInit): Promise<unknown> {
  if (!init.body) {
    return {}
  }
  if (typeof init.body === 'string') {
    return JSON.parse(init.body)
  }
  if (init.body instanceof Blob) {
    return JSON.parse(await init.body.text())
  }
  return {}
}

function jsonResponse(data: unknown, status = 200) {
  return new Response(JSON.stringify(data), {
    status,
    headers: {
      'Content-Type': 'application/json',
    },
  })
}

function errorResponse(error: unknown) {
  const message = normalizeErrorMessage(error)
  const { code, status } = classifyError(message)
  return jsonResponse(
    {
      error: {
        code,
        message,
      },
    },
    status
  )
}

function normalizeErrorMessage(error: unknown) {
  if (typeof error === 'string') {
    return error
  }
  if (error instanceof Error) {
    return error.message
  }
  return String(error)
}

function classifyError(message: string) {
  if (message.includes('Vault is locked')) {
    return { code: 'vault_locked', status: 401 }
  }
  if (message.includes('Current master password is incorrect')) {
    return { code: 'unauthorized', status: 401 }
  }
  if (message.includes('Document not found')) {
    return { code: 'not_found', status: 404 }
  }
  if (message.includes('Secret not found')) {
    return { code: 'not_found', status: 404 }
  }
  if (message.includes('Finding not found')) {
    return { code: 'not_found', status: 404 }
  }
  if (message.includes('Security case not found')) {
    return { code: 'not_found', status: 404 }
  }
  if (message.includes('Checklist item not found')) {
    return { code: 'not_found', status: 404 }
  }
  if (message.includes('Security asset not found')) {
    return { code: 'not_found', status: 404 }
  }
  if (
    message.includes('Invalid file path') ||
    message.includes('already exists') ||
    message.includes('RustTool memo data') ||
    message.includes('Backup archive') ||
    message.includes('WebDAV backup config') ||
    message.includes('data directory') ||
    message.includes('Target directory') ||
    message.includes('Secret id cannot be empty') ||
    message.includes('master password cannot be empty') ||
    message.includes('Master password is not initialized') ||
    message.includes('New master password must be different') ||
    message.includes('Invalid finding status') ||
    message.includes('Finding id cannot be empty') ||
    message.includes('Invalid security case status') ||
    message.includes('Risk acceptance rationale cannot be empty') ||
    message.includes('Risk acceptance expiry cannot be empty') ||
    message.includes('Risk acceptance expiry must use') ||
    message.includes('Risk acceptance impact scope cannot be empty') ||
    message.includes('Risk acceptance compensating controls cannot be empty') ||
    message.includes('Risk acceptance reviewer cannot be empty') ||
    message.includes('Invalid checklist status') ||
    message.includes('Checklist item id cannot be empty') ||
    message.includes('Unsupported memo API route')
  ) {
    return { code: 'bad_request', status: 400 }
  }
  return { code: 'internal_error', status: 500 }
}
