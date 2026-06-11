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
    message.includes('Unsupported memo API route')
  ) {
    return { code: 'bad_request', status: 400 }
  }
  return { code: 'internal_error', status: 500 }
}
