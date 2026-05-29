<script setup lang="ts">
import { ref, onMounted, computed, watch, nextTick } from 'vue'
import {
  Brain,
  Lock,
  Unlock,
  Key,
  FileText,
  Send,
  Plus,
  Settings,
  Trash2,
  Eye,
  EyeOff,
  Save,
  X,
  Database,
  RefreshCw,
  PenTool,
  MessageSquare,
  ChevronRight,
  ChevronLeft,
  FolderOpen,
  Folder,
  Sparkles,
  AlertTriangle,
  CheckCircle2,
  AlertCircle,
  Info,
} from '@lucide/vue'
import ToolShell from '../components/ToolShell.vue'

interface DraftResponse {
  title: string
  fileName: string
  markdown: string
  secrets: Record<string, string>
  summary: string
}

interface SearchSourceDoc {
  id: string
  title: string
  fileName: string
  score: number
}

interface SearchAnswerResponse {
  answer: string
  sources: SearchSourceDoc[]
}

// --- Dialog state and helper functions ---
interface DialogState {
  show: boolean
  title: string
  message: string
  type: 'info' | 'warning' | 'error' | 'success' | 'confirm'
  confirmText: string
  cancelText: string
  resolve: ((value: boolean) => void) | null
}

const dialogState = ref<DialogState>({
  show: false,
  title: '',
  message: '',
  type: 'info',
  confirmText: '确定',
  cancelText: '取消',
  resolve: null,
})

function customAlert(message: string, title = '提示', type: DialogState['type'] = 'info') {
  return new Promise<boolean>((resolve) => {
    dialogState.value = {
      show: true,
      title,
      message,
      type,
      confirmText: '确定',
      cancelText: '取消',
      resolve: resolve as (value: boolean) => void,
    }
  })
}

function customConfirm(message: string, title = '确认操作', type: DialogState['type'] = 'confirm') {
  return new Promise<boolean>((resolve) => {
    dialogState.value = {
      show: true,
      title,
      message,
      type,
      confirmText: '确定',
      cancelText: '取消',
      resolve: resolve as (value: boolean) => void,
    }
  })
}

function handleDialogConfirm() {
  if (dialogState.value.resolve) {
    dialogState.value.resolve(true)
  }
  dialogState.value.show = false
}

function handleDialogCancel() {
  if (dialogState.value.resolve) {
    dialogState.value.resolve(false)
  }
  dialogState.value.show = false
}

// --- API host resolution ---
const apiBase = ''

// --- Core state ---
const unlocked = ref(false)
const masterPassword = ref('')
const isFirstTime = ref(false) // Whether verification token is not set
const statusLoading = ref(true)

// Configuration state
const ollamaUrl = ref('https://api.openai.com/v1')
const apiKey = ref('')
const hasApiKey = ref(false)
const chatModel = ref('gpt-5.5')
const embeddingModel = ref('text-embedding-3-small')
const reasoningEffort = ref('xhigh')
const disableResponseStorage = ref(true)
const allowAiSecrets = ref(false)
const customDataDir = ref('')
const showSettings = ref(false)
const testingConnection = ref(false)
const connectionMessage = ref('')
const connectionOk = ref<boolean | null>(null)

// Backup settings
const localBackupDir = ref('')
const webdavUrl = ref('')
const webdavUser = ref('')
const webdavPass = ref('')
const backupMessage = ref('')
const backupLoading = ref(false)
const restorePath = ref('')
const restoreLoading = ref(false)

// Documents list
const documents = ref<any[]>([])
const searchFilter = ref('')
const listLoading = ref(false)

// Selection & editing
const showEditorSecrets = ref(true)
const showDocSidebar = ref(true)
const selectedDocId = ref<string | null>(null)
const editingDoc = ref<any>({
  id: '',
  title: '',
  fileName: '',
  markdown: '',
  secrets: {} as Record<string, string>,
  summary: '',
})
const editorSecretsList = ref<Array<{ key: string; value: string; masked: boolean; aiLoading?: boolean }>>([])
const editorAiInstruction = ref('')
const editorAiLoading = ref(false)

// Chat state
const chatInput = ref('')
const chatMessages = ref<any[]>([
  {
    role: 'assistant',
    content: '你好！我是您的离线安全 AI 备忘大管家。\n您可以随时在这里打下杂乱的文字，让我帮您整理并加密归档到本地；或者随时向我提问，我会快速从本地文档中为您检索。',
  },
])
const chatLoading = ref(false)

// Textarea DOM ref for auto-growing
const textareaRef = ref<HTMLTextAreaElement | null>(null)
const editorTextareaRef = ref<HTMLTextAreaElement | null>(null)

async function insertSecretPlaceholder(key: string) {
  if (!key.trim()) {
    await customAlert('密码Key不能为空！', '提示', 'error')
    return
  }
  const textarea = editorTextareaRef.value
  if (!textarea) {
    editingDoc.value.markdown += ` {{secret:${key}}}`
    return
  }

  const startPos = textarea.selectionStart
  const endPos = textarea.selectionEnd
  const text = editingDoc.value.markdown
  const placeholder = `{{secret:${key}}}`

  editingDoc.value.markdown =
    text.substring(0, startPos) +
    placeholder +
    text.substring(endPos)

  // Put focus back and place cursor after the placeholder
  nextTick(() => {
    textarea.focus()
    const newCursorPos = startPos + placeholder.length
    textarea.setSelectionRange(newCursorPos, newCursorPos)
  })
}

// --- Computed properties ---
const filteredDocs = computed(() => {
  if (!searchFilter.value.trim()) {
    return documents.value
  }
  const query = searchFilter.value.toLowerCase()
  return documents.value.filter(
    (doc) =>
      doc.title.toLowerCase().includes(query) ||
      doc.summary.toLowerCase().includes(query) ||
      doc.fileName.toLowerCase().includes(query)
  )
})

interface TreeNode {
  name: string
  type: 'folder' | 'file'
  path: string
  children?: TreeNode[]
  doc?: any
}

interface FlatNode {
  name: string
  type: 'folder' | 'file'
  path: string
  depth: number
  doc?: any
  hasChildren: boolean
}

const expandedFolders = ref<Record<string, boolean>>({})

const docTree = computed(() => {
  const root: TreeNode = { name: 'Root', type: 'folder', path: '', children: [] }

  for (const doc of filteredDocs.value) {
    const standardFileName = doc.fileName.replace(/\\/g, '/')
    const parts = standardFileName.split('/')
    let current = root

    for (let i = 0; i < parts.length; i++) {
      const part = parts[i]
      const isLast = i === parts.length - 1
      const currentPath = parts.slice(0, i + 1).join('/')

      if (isLast) {
        current.children!.push({
          name: doc.title || part,
          type: 'file',
          path: currentPath,
          doc,
        })
      } else {
        let folder = current.children!.find(c => c.type === 'folder' && c.name === part)
        if (!folder) {
          folder = {
            name: part,
            type: 'folder',
            path: currentPath,
            children: [],
          }
          current.children!.push(folder)
        }
        current = folder
      }
    }
  }

  const sortNodes = (nodes: TreeNode[]) => {
    nodes.sort((a, b) => {
      if (a.type !== b.type) {
        return a.type === 'folder' ? -1 : 1
      }
      return a.name.localeCompare(b.name)
    })
    for (const node of nodes) {
      if (node.children) {
        sortNodes(node.children)
      }
    }
  }

  if (root.children) {
    sortNodes(root.children)
  }

  return root.children || []
})

const visibleFlatNodes = computed(() => {
  const result: FlatNode[] = []

  const traverse = (nodes: TreeNode[], depth: number) => {
    for (const node of nodes) {
      const isExpanded = expandedFolders.value[node.path] !== false
      
      result.push({
        name: node.name,
        type: node.type,
        path: node.path,
        depth,
        doc: node.doc,
        hasChildren: !!(node.type === 'folder' && node.children && node.children.length > 0),
      })

      if (node.type === 'folder' && isExpanded && node.children) {
        traverse(node.children, depth + 1)
      }
    }
  }

  traverse(docTree.value, 0)
  return result
})

function escapeHtml(value: string) {
  return value
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;')
}

function renderInlineMarkdown(value: string) {
  return escapeHtml(value)
    .replace(/`([^`]+)`/g, '<code>$1</code>')
    .replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>')
    .replace(/\*([^*]+)\*/g, '<em>$1</em>')
    .replace(/{{secret:([^}]+)}}/g, '<span class="md-secret">{{secret:$1}}</span>')
}

function renderMarkdown(markdown: string) {
  const lines = markdown.split(/\r?\n/)
  const html: string[] = []
  let inCodeBlock = false
  let listType: 'ul' | 'ol' | null = null
  let codeLines: string[] = []

  const closeList = () => {
    if (listType) {
      html.push(`</${listType}>`)
      listType = null
    }
  }

  const openList = (type: 'ul' | 'ol') => {
    if (listType !== type) {
      closeList()
      html.push(`<${type}>`)
      listType = type
    }
  }

  const closeCodeBlock = () => {
    html.push(`<pre><code>${escapeHtml(codeLines.join('\n'))}</code></pre>`)
    codeLines = []
    inCodeBlock = false
  }

  for (const line of lines) {
    const trimmed = line.trim()

    if (trimmed.startsWith('```')) {
      closeList()
      if (inCodeBlock) {
        closeCodeBlock()
      } else {
        inCodeBlock = true
        codeLines = []
      }
      continue
    }

    if (inCodeBlock) {
      codeLines.push(line)
      continue
    }

    if (!trimmed) {
      closeList()
      html.push('<p class="md-spacer"></p>')
      continue
    }

    const heading = trimmed.match(/^(#{1,6})\s+(.+)$/)
    if (heading) {
      closeList()
      const level = heading[1].length
      html.push(`<h${level}>${renderInlineMarkdown(heading[2])}</h${level}>`)
      continue
    }

    const unordered = line.match(/^\s*[-*+]\s+(.+)$/)
    if (unordered) {
      openList('ul')
      html.push(`<li>${renderInlineMarkdown(unordered[1])}</li>`)
      continue
    }

    const ordered = line.match(/^\s*\d+\.\s+(.+)$/)
    if (ordered) {
      openList('ol')
      html.push(`<li>${renderInlineMarkdown(ordered[1])}</li>`)
      continue
    }

    if (trimmed.startsWith('>')) {
      closeList()
      html.push(`<blockquote>${renderInlineMarkdown(trimmed.replace(/^>\s?/, ''))}</blockquote>`)
      continue
    }

    closeList()
    html.push(`<p>${renderInlineMarkdown(line)}</p>`)
  }

  closeList()
  if (inCodeBlock) {
    closeCodeBlock()
  }

  return html.join('')
}

const previewHtml = computed(() => {
  const markdown = editingDoc.value.markdown?.trim()
  if (!markdown) {
    return '<p class="preview-empty">当前文档还没有内容。</p>'
  }
  return renderMarkdown(editingDoc.value.markdown)
})

function handleNodeClick(node: FlatNode) {
  if (node.type === 'folder') {
    const current = expandedFolders.value[node.path] !== false
    expandedFolders.value[node.path] = !current
  } else if (node.type === 'file' && node.doc) {
    selectDocument(node.doc.id)
  }
}

// Auto-grow textarea handler
function adjustTextareaHeight() {
  nextTick(() => {
    const el = textareaRef.value
    if (el) {
      el.style.height = 'auto'
      // Restrict max-height through CSS, but set height dynamically to scrollHeight
      el.style.height = `${Math.min(el.scrollHeight, 96)}px`
    }
  })
}

// Watch chat input to trigger auto-resize
watch(chatInput, () => {
  adjustTextareaHeight()
})

// --- API Calls ---

async function readApiError(res: Response, fallback = '请求失败') {
  try {
    const data = await res.clone().json()
    if (typeof data?.error?.message === 'string' && data.error.message.trim()) {
      return data.error.message
    }
    if (typeof data?.message === 'string' && data.message.trim()) {
      return data.message
    }
  } catch {
    // Fall back to plain text below for non-JSON errors.
  }

  try {
    const text = await res.text()
    return text.trim() || fallback
  } catch {
    return fallback
  }
}

async function fetchStatus() {
  try {
    statusLoading.value = true
    const res = await fetch(`${apiBase}/api/memo/status`)
    if (res.ok) {
      const data = await res.json()
      unlocked.value = data.unlocked
      ollamaUrl.value = data.ollamaUrl
      hasApiKey.value = data.hasApiKey
      apiKey.value = ''
      chatModel.value = data.chatModel
      embeddingModel.value = data.embeddingModel
      reasoningEffort.value = data.reasoningEffort || 'xhigh'
      disableResponseStorage.value = data.disableResponseStorage ?? true
      allowAiSecrets.value = data.allowAiSecrets
      customDataDir.value = data.customDataDir || ''
    }
  } catch (e) {
    console.error('Failed to fetch memo status:', e)
  } finally {
    statusLoading.value = false
  }
}

async function handleUnlock() {
  if (!masterPassword.value) return
  try {
    const res = await fetch(`${apiBase}/api/memo/unlock`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ password: masterPassword.value }),
    })
    if (res.ok) {
      const data = await res.json()
      if (data.unlocked) {
        unlocked.value = true
        masterPassword.value = ''
        await loadDocuments()
        addSystemChatMessage('保密库已成功解锁，您可以开始使用了！')
      } else {
        await customAlert('主密码错误，请重新输入！', '解锁失败', 'error')
      }
    } else {
      const errMsg = await readApiError(res, '解锁失败')
      await customAlert('解锁失败: ' + errMsg, '解锁失败', 'error')
    }
  } catch (e) {
    await customAlert('请求失败，请确认后端服务器正在运行。', '连接错误', 'error')
  }
}

async function handleLock() {
  try {
    const res = await fetch(`${apiBase}/api/memo/lock`, { method: 'POST' })
    if (res.ok) {
      unlocked.value = false
      documents.value = []
      selectedDocId.value = null
      addSystemChatMessage('保密库已锁定，内存密钥已销毁。')
    }
  } catch (e) {
    console.error('Failed to lock:', e)
  }
}

async function loadDocuments() {
  if (!unlocked.value) return
  try {
    listLoading.value = true
    const res = await fetch(`${apiBase}/api/memo/list`)
    if (res.ok) {
      documents.value = await res.json()
    }
  } catch (e) {
    console.error('Failed to load documents:', e)
  } finally {
    listLoading.value = false
  }
}

async function saveSettings() {
  const confirmed = await customConfirm(
    '确定要保存当前的 AI 服务与存储路径配置吗？\n\n' +
    '提示：若修改了存储路径，您需要手动将原目录下的文件复制到新路径下并重启本软件生效。',
    '保存配置确认',
    'confirm'
  )
  if (!confirmed) {
    return
  }
  try {
    const res = await fetch(`${apiBase}/api/memo/settings`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        ollamaUrl: ollamaUrl.value,
        apiKey: apiKey.value.trim() ? apiKey.value : null,
        chatModel: chatModel.value,
        embeddingModel: embeddingModel.value,
        reasoningEffort: reasoningEffort.value,
        disableResponseStorage: disableResponseStorage.value,
        allowAiSecrets: allowAiSecrets.value,
        customDataDir: customDataDir.value || null,
      }),
    })
    if (res.ok) {
      await fetchStatus()
      await customAlert('配置已成功保存！若修改了存储路径，请手动将原目录下的文件复制到新路径下并重启本软件生效。', '配置保存成功', 'success')
      showSettings.value = false
    } else {
      const text = await readApiError(res, '保存配置失败')
      await customAlert('保存配置失败: ' + text, '配置保存失败', 'error')
    }
  } catch (e) {
    await customAlert('请求失败: ' + e, '连接错误', 'error')
  }
}

async function testConnection() {
  try {
    testingConnection.value = true
    connectionMessage.value = '正在测试模型服务...'
    connectionOk.value = null
    const res = await fetch(`${apiBase}/api/memo/test-connection`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        ollamaUrl: ollamaUrl.value,
        apiKey: apiKey.value.trim() ? apiKey.value : null,
        chatModel: chatModel.value,
        embeddingModel: embeddingModel.value,
        reasoningEffort: reasoningEffort.value,
        disableResponseStorage: disableResponseStorage.value,
      }),
    })

    if (res.ok) {
      const data = await res.json()
      connectionOk.value = true
      connectionMessage.value = data.message || '连接成功'
    } else {
      connectionOk.value = false
      connectionMessage.value = await readApiError(res, '连接失败')
    }
  } catch (e) {
    connectionOk.value = false
    connectionMessage.value = `连接失败: ${e}`
  } finally {
    testingConnection.value = false
  }
}

async function handleAllowSecretsChange(e: Event) {
  const target = e.target as HTMLInputElement
  const checked = target.checked

  if (checked) {
    const confirmed = await customConfirm(
      '您正在开启“允许 AI 检索并读取解密后的密码”功能。\n\n' +
      '由于您当前连接的是外部云端大模型，开启此项将使您的明文密码随文档上下文发送至云端 API，存在不可控的泄露风险！\n\n' +
      '您确定要开启此项高风险设置吗？',
      '重要安全警告',
      'warning'
    )
    if (confirmed) {
      allowAiSecrets.value = true
    } else {
      target.checked = false
      allowAiSecrets.value = false
    }
  } else {
    allowAiSecrets.value = false
  }
}

async function triggerBackup() {
  const confirmed = await customConfirm(
    '确定要立即执行安全备份吗？\n\n' +
    '系统将打包本地所有的加密数据库及文档，并同步上传至配置的本地及云端 WebDAV 备份路径。',
    '安全备份确认',
    'confirm'
  )
  if (!confirmed) {
    return
  }
  try {
    backupLoading.value = true
    backupMessage.value = '备份运行中...'
    const res = await fetch(`${apiBase}/api/memo/backup`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        localBackupDir: localBackupDir.value || null,
        webdavUrl: webdavUrl.value || null,
        webdavUser: webdavUser.value || null,
        webdavPass: webdavPass.value || null,
      }),
    })
    if (res.ok) {
      const data = await res.json()
      backupMessage.value = data.message || '备份成功！'
    } else {
      backupMessage.value = '备份失败: ' + await readApiError(res, '备份失败')
    }
  } catch (e) {
    backupMessage.value = '备份失败: ' + e
  } finally {
    backupLoading.value = false
  }
}

async function triggerRestore() {
  if (!restorePath.value) {
    await customAlert('请输入备份 ZIP 压缩包路径！', '提示', 'warning')
    return
  }
  const confirmed = await customConfirm('还原将覆盖当前的本地文档和数据库。是否确定继续？', '数据还原确认', 'warning')
  if (!confirmed) {
    return
  }
  try {
    restoreLoading.value = true
    const res = await fetch(`${apiBase}/api/memo/restore`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ zipPath: restorePath.value }),
    })
    if (res.ok) {
      await customAlert('还原成功！页面将重新加载。', '还原成功', 'success')
      location.reload()
    } else {
      const txt = await readApiError(res, '还原失败')
      await customAlert('还原失败: ' + txt, '还原失败', 'error')
    }
  } catch (e) {
    await customAlert('还原出错: ' + e, '错误', 'error')
  } finally {
    restoreLoading.value = false
  }
}

// --- Document loading and editing ---

async function selectDocument(id: string) {
  try {
    const res = await fetch(`${apiBase}/api/memo/doc/${id}`)
    if (res.ok) {
      const detail = await res.json()
      selectedDocId.value = id
      editingDoc.value = {
        id: detail.metadata.id,
        title: detail.metadata.title,
        fileName: detail.metadata.fileName,
        markdown: detail.markdown,
        secrets: detail.secrets,
        summary: detail.metadata.summary,
      }
      
      // Parse secrets into lists for rendering
      editorSecretsList.value = Object.entries(detail.secrets).map(([k, v]) => ({
        key: k,
        value: v as string,
          masked: true,
          aiLoading: false,
        }))
    } else {
      await customAlert('加载文档失败: ' + await readApiError(res, '加载文档失败'), '错误', 'error')
    }
  } catch (e) {
    await customAlert('加载文档出错: ' + e, '错误', 'error')
  }
}

function createNewDocumentManual() {
  selectedDocId.value = 'new'
  editingDoc.value = {
    id: '',
    title: '新建文档',
    fileName: 'new_doc.md',
    markdown: '# 新建文档\n\n在这里写下你的技术内容。',
    secrets: {},
    summary: '手动新建的文档',
  }
  editorSecretsList.value = []
  nextTick(() => {
    editorTextareaRef.value?.focus()
  })
}

function applyDraftToEditor(draft: DraftResponse, options: { asNew?: boolean } = {}) {
  const asNew = options.asNew ?? true
  const currentSelectedId = selectedDocId.value
  const keepCurrentDoc = !asNew && currentSelectedId !== null && currentSelectedId !== 'new'
  const currentDoc = editingDoc.value
  const existingSecrets = new Map<string, string>()

  if (keepCurrentDoc) {
    for (const secret of editorSecretsList.value) {
      const key = secret.key.trim()
      if (key) {
        existingSecrets.set(key, secret.value)
      }
    }
  }

  const nextSecrets = new Map<string, string>()
  if (keepCurrentDoc) {
    for (const [key, value] of existingSecrets.entries()) {
      nextSecrets.set(key, value)
    }
  }
  for (const [key, value] of Object.entries(draft.secrets)) {
    nextSecrets.set(key, existingSecrets.get(key) ?? value)
  }
  const mergedSecrets = Object.fromEntries(nextSecrets)

  selectedDocId.value = keepCurrentDoc ? currentSelectedId : 'new'
  editingDoc.value = {
    id: keepCurrentDoc ? currentDoc.id : '',
    title: draft.title,
    fileName: keepCurrentDoc ? currentDoc.fileName : draft.fileName,
    markdown: draft.markdown,
    secrets: mergedSecrets,
    summary: draft.summary,
  }
  editorSecretsList.value = Object.entries(mergedSecrets).map(([key, value]) => ({
    key,
    value: value as string,
    masked: true,
    aiLoading: false,
  }))
  nextTick(() => {
    editorTextareaRef.value?.focus()
  })
}

function addSecretToEditor() {
  editorSecretsList.value.push({
    key: '',
    value: '',
    masked: false,
    aiLoading: false,
  })
}

async function generateAiKey(sec: any, idx: number) {
  if (!sec.key.trim()) {
    await customAlert('请先输入一些中文描述作为密码占位符（例如：“数据库密码”）', '提示', 'info')
    return
  }
  try {
    sec.aiLoading = true
    const res = await fetch(`${apiBase}/api/memo/translate-key`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ text: sec.key }),
    })
    if (res.ok) {
      const data = await res.json()
      if (data.key) {
        // Deduplicate against other keys
        let candidate = data.key
        let counter = 2
        while (editorSecretsList.value.some((s, sIdx) => sIdx !== idx && s.key === candidate)) {
          candidate = `${data.key}${counter}`
          counter++
        }
        sec.key = candidate
      }
    } else {
      const text = await readApiError(res, 'AI 转换失败')
      await customAlert('AI 转换失败: ' + text, '错误', 'error')
    }
  } catch (e) {
    await customAlert('AI 转换出错: ' + e, '错误', 'error')
  } finally {
    sec.aiLoading = false
  }
}

function removeSecretFromEditor(index: number) {
  editorSecretsList.value.splice(index, 1)
}

async function saveDocumentChanges() {
  const secretsMap: Record<string, string> = {}
  for (const s of editorSecretsList.value) {
    if (s.key.trim()) {
      secretsMap[s.key.trim()] = s.value
    }
  }

  try {
    const isNew = selectedDocId.value === 'new'
    const res = await fetch(`${apiBase}/api/memo/save`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        id: isNew ? null : editingDoc.value.id,
        fileName: editingDoc.value.fileName,
        title: editingDoc.value.title,
        markdown: editingDoc.value.markdown,
        secrets: secretsMap,
        summary: editingDoc.value.summary || '手动编辑保存的文档',
      }),
    })
    
    if (res.ok) {
      const meta = await res.json()
      await customAlert('文档已成功保存！', '保存成功', 'success')
      selectedDocId.value = meta.id
      await loadDocuments()
      await selectDocument(meta.id)
    } else {
      const txt = await readApiError(res, '保存失败')
      await customAlert('保存失败: ' + txt, '错误', 'error')
    }
  } catch (e) {
    await customAlert('保存文档时出错: ' + e, '错误', 'error')
  }
}

async function deleteDocument(id: string, title: string) {
  const confirmed = await customConfirm(`确定要彻底删除文档《${title}》及其包含的全部加密密码吗？`, '删除文档确认', 'warning')
  if (!confirmed) {
    return
  }
  try {
    const res = await fetch(`${apiBase}/api/memo/delete`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ id }),
    })
    if (res.ok) {
      if (selectedDocId.value === id) {
        selectedDocId.value = null
      }
      await loadDocuments()
    } else {
      await customAlert('删除失败: ' + await readApiError(res, '删除失败'), '错误', 'error')
    }
  } catch (e) {
    await customAlert('删除文档出错: ' + e, '错误', 'error')
  }
}

// --- Chat & Assistant Actions ---

function addSystemChatMessage(text: string) {
  chatMessages.value.push({
    role: 'assistant',
    content: text,
  })
}

async function sendChatMessage() {
  const query = chatInput.value.trim()
  if (!query) return

  chatMessages.value.push({
    role: 'user',
    content: query,
  })
  chatInput.value = ''
  adjustTextareaHeight() // Reset input height

  // Analyze intent (Write vs Search)
  const isWriteCommand = 
    query.startsWith('记下') || 
    query.startsWith('帮我记') || 
    query.startsWith('保存文档') || 
    query.startsWith('新建文档') || 
    query.startsWith('写个文档') || 
    query.includes('帮我保存');
  const hasActiveDoc = !!selectedDocId.value
  const wantsCurrentDocEdit =
    hasActiveDoc && (
      query.includes('修改') ||
      query.includes('改一下') ||
      query.includes('优化') ||
      query.includes('整理') ||
      query.includes('补充') ||
      query.includes('润色') ||
      query.includes('重写') ||
      query.includes('摘要') ||
      query.includes('标题') ||
      query.includes('当前文档') ||
      query.includes('这篇文档') ||
      query.includes('这个文档')
    )
  const wantsLocalSearch =
    query.includes('查找') ||
    query.includes('检索') ||
    query.includes('搜索') ||
    query.includes('查询') ||
    query.includes('本地文档') ||
    query.includes('文档里') ||
    query.includes('知识库') ||
    query.includes('密码是什么') ||
    query.includes('密码是多少') ||
    query.includes('服务器密码') ||
    query.includes('api key') ||
    query.includes('API Key') ||
    query.includes('密钥')

  chatLoading.value = true
  try {
    if (wantsCurrentDocEdit) {
      const res = await fetch(`${apiBase}/api/memo/draft`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          rawInput: `请根据用户要求修改当前 Markdown 文档，保持事实准确，并在需要时识别敏感信息为 secret 占位符。\n\n用户要求：${query}\n\n当前文档标题：${editingDoc.value.title}\n当前文档摘要：${editingDoc.value.summary}\n当前 Markdown：\n${editingDoc.value.markdown}`,
        }),
      })
      if (res.ok) {
        const draft: DraftResponse = await res.json()
        applyDraftToEditor(draft, { asNew: selectedDocId.value === 'new' })
        chatMessages.value.push({
          role: 'assistant',
          content: '我已按你的要求更新当前文档，右侧预览已同步刷新。请检查后保存。',
        })
      } else {
        const txt = await readApiError(res, 'AI 修改文档失败')
        chatMessages.value.push({
          role: 'assistant',
          content: `AI 修改文档失败: ${txt}。`,
        })
      }
    } else if (isWriteCommand) {
      const res = await fetch(`${apiBase}/api/memo/draft`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ rawInput: query }),
      })
      if (res.ok) {
        const draft: DraftResponse = await res.json()
        applyDraftToEditor(draft, { asNew: true })
        chatMessages.value.push({
          role: 'assistant',
          content: '我已经生成文档草稿，并填入当前编辑区。右侧预览会实时同步，确认后保存即可。',
        })
      } else {
        const txt = await readApiError(res, 'AI 整理草稿失败')
        chatMessages.value.push({
          role: 'assistant',
          content: `AI 整理草稿失败: ${txt}。请检查 OpenAI 兼容接口、API Key 和模型 ${chatModel.value} 是否可用。`,
        })
      }
    } else if (wantsLocalSearch) {
      const res = await fetch(`${apiBase}/api/memo/query`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ query }),
      })
      if (res.ok) {
        const data: SearchAnswerResponse = await res.json()
        chatMessages.value.push({
          role: 'assistant',
          content: data.answer,
          sources: data.sources,
        })
      } else {
        const txt = await readApiError(res, '检索失败')
        chatMessages.value.push({
          role: 'assistant',
          content: `检索失败: ${txt}。`,
        })
      }
    } else {
      const res = await fetch(`${apiBase}/api/memo/chat`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ query }),
      })
      if (res.ok) {
        const data = await res.json()
        chatMessages.value.push({
          role: 'assistant',
          content: data.answer,
        })
      } else {
        const txt = await readApiError(res, '对话失败')
        chatMessages.value.push({
          role: 'assistant',
          content: `对话失败: ${txt}。`,
        })
      }
    }
  } catch (e) {
    chatMessages.value.push({
      role: 'assistant',
      content: `发生错误，无法连接到本地服务: ${e}`,
    })
  } finally {
    chatLoading.value = false
  }
}

async function runEditorAi(action: 'organize' | 'summary' | 'custom') {
  const source = editingDoc.value.markdown?.trim()
  if (!source) {
    await customAlert('当前文档内容为空，先写一点内容再让 AI 处理。', '提示', 'warning')
    return
  }

  const instruction =
    action === 'organize'
      ? '请整理当前 Markdown 文档，保持原有事实不变，优化标题层级、列表、段落和可读性，并识别敏感信息为 secret 占位符。'
      : action === 'summary'
        ? '请根据当前 Markdown 文档生成更准确的标题、文件名、摘要，并保留正文内容。'
        : editorAiInstruction.value.trim()

  if (!instruction) {
    await customAlert('请输入希望 AI 如何处理当前文档。', '提示', 'warning')
    return
  }

  try {
    editorAiLoading.value = true
    const res = await fetch(`${apiBase}/api/memo/draft`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        rawInput: `${instruction}\n\n当前文档标题：${editingDoc.value.title}\n当前文档摘要：${editingDoc.value.summary}\n当前 Markdown：\n${editingDoc.value.markdown}`,
      }),
    })
    if (res.ok) {
      const draft: DraftResponse = await res.json()
      applyDraftToEditor(draft, { asNew: selectedDocId.value === 'new' })
      editorAiInstruction.value = ''
      await customAlert('AI 已生成新版草稿，已填入编辑器。保存前请检查内容。', 'AI 草稿已应用', 'success')
    } else {
      await customAlert('AI 处理失败: ' + await readApiError(res, 'AI 处理失败'), '错误', 'error')
    }
  } catch (e) {
    await customAlert('AI 处理出错: ' + e, '错误', 'error')
  } finally {
    editorAiLoading.value = false
  }
}

// Set WebDAV default URL if empty but other fields are filled
watch(webdavUrl, (newVal) => {
  if (newVal && !newVal.startsWith('http')) {
    webdavUrl.value = 'http://' + newVal
  }
})

onMounted(() => {
  void fetchStatus()
})
</script>

<template>
  <ToolShell
    title="AI 备忘大管家"
    description="100% 离线运行的个人机密知识库。支持 AI 辅助书写、安全字段加密以及语义化 RAG 快速查阅。"
    :breadcrumbs="[
      { label: '工具箱', to: '/toolbox' },
      { label: 'AI 备忘大管家' },
    ]"
    fluid
  >
    <!-- Background glowing ambient lights for high-end feel -->
    <div class="pointer-events-none absolute inset-0 z-0 overflow-hidden">
      <div class="absolute -top-[10%] -left-[10%] h-[350px] w-[350px] rounded-full bg-emerald-500/5 blur-[80px]"></div>
      <div class="absolute top-[40%] right-[5%] h-[400px] w-[400px] rounded-full bg-teal-500/5 blur-[90px]"></div>
    </div>

    <!-- Lock Screen overlay -->
    <div v-if="!unlocked && !statusLoading" class="lock-overlay">
      <div class="lock-card">
        <div class="lock-icon-container">
          <Lock class="h-10 w-10 text-emerald-400" />
        </div>
        <h3 class="lock-title">保密知识库已锁定</h3>
        <p class="lock-subtitle">
          请输入您的 Master Password 锁匙。首次使用输入的密码将自动设定为终身主密钥。
        </p>

        <form @submit.prevent="handleUnlock" class="lock-form">
          <input
            v-model="masterPassword"
            type="password"
            class="lock-input"
            placeholder="请输入主密码"
            autofocus
          />
          <button type="submit" class="lock-submit-btn">
            <Unlock class="h-4 w-4 mr-2" />
            解 锁
          </button>
        </form>
        <p class="lock-warn">
          ⚠️ 注意：此密钥直接用于在本地对您的数据进行 AES 加密，服务器绝不保存明文，一旦遗失将无法恢复！
        </p>
      </div>
    </div>

    <!-- Main Application UI when Unlocked -->
    <div v-else-if="unlocked" class="memo-layout relative z-10">
      <!-- Top Action Bar with sleek border -->
      <div class="action-bar">
        <div class="flex items-center gap-3">
          <div class="h-2 w-2 rounded-full bg-emerald-500 animate-pulse"></div>
          <span class="text-xs font-bold tracking-wider text-emerald-400">SECURE VAULT ACTIVE</span>
        </div>
        <div class="flex items-center gap-3">
          <button @click="showSettings = true" class="action-btn">
            <Settings class="h-3.5 w-3.5" />
            <span>设置 / 备份</span>
          </button>
          <button @click="handleLock" class="lock-btn">
            <Lock class="h-3.5 w-3.5 mr-1.5" />
            锁定保险箱
          </button>
        </div>
      </div>

      <div class="memo-grid" :class="{ 'memo-grid--collapsed': !showDocSidebar }">
        <!-- Sidebar - Left Column: Document List -->
        <aside v-show="showDocSidebar" class="doc-sidebar">
          <div class="p-3 border-b border-gray-800 flex justify-between items-center gap-2 bg-gray-950/20">
            <button
              @click="showDocSidebar = false"
              class="p-1 hover:bg-gray-800 rounded text-gray-500 hover:text-gray-300 transition flex items-center justify-center"
              title="收起文档列表"
            >
              <ChevronLeft class="h-3.5 w-3.5" />
            </button>
            <input
              v-model="searchFilter"
              type="text"
              class="search-input"
              placeholder="搜索本地..."
            />
            <button @click="createNewDocumentManual" class="new-doc-btn" title="新建文档">
              <Plus class="h-3.5 w-3.5" />
            </button>
          </div>

          <div class="doc-list-scroll">
            <div v-if="listLoading" class="p-6 text-center text-gray-500">
              <RefreshCw class="animate-spin h-5 w-5 mx-auto mb-2 text-emerald-500" />
              <span class="text-xs">载入文档中...</span>
            </div>
            <div v-else-if="filteredDocs.length === 0" class="empty-docs-container">
              <FolderOpen class="h-8 w-8 text-gray-800 mb-2" />
              <p class="text-xs text-gray-600">暂无文档文件</p>
              <p class="text-[10px] text-gray-700 mt-1 max-w-[150px] mx-auto leading-relaxed">
                在右侧聊天框打入“记下：...”让 AI 帮您生成文档吧！
              </p>
            </div>
            <div v-else class="divide-y divide-gray-800/10 font-sans">
              <div
                v-for="node in visibleFlatNodes"
                :key="node.path"
                class="group flex items-center justify-between py-2 px-3 cursor-pointer hover:bg-gray-800/25 transition-colors relative"
                :class="{ 'bg-emerald-500/5': node.type === 'file' && selectedDocId === node.doc?.id }"
                :style="{ paddingLeft: `${node.depth * 14 + 12}px` }"
                @click="handleNodeClick(node)"
              >
                <div class="flex items-center gap-2 min-w-0 flex-1">
                  <!-- Folder / File Icon -->
                  <component 
                    :is="node.type === 'folder' ? (expandedFolders[node.path] !== false ? FolderOpen : Folder) : FileText" 
                    class="h-3.5 w-3.5 flex-shrink-0 transition-transform duration-200"
                    :class="node.type === 'folder' ? 'text-amber-500/90' : 'text-emerald-400/80'"
                  />
                  <div class="min-w-0 flex-1">
                    <h4 class="text-xs text-gray-200 truncate" :class="node.type === 'folder' ? 'font-bold text-gray-300' : 'font-medium'">
                      {{ node.name }}
                    </h4>
                    <p v-if="node.type === 'file'" class="text-[9px] text-gray-500 truncate mt-0.5">
                      {{ node.doc.summary }}
                    </p>
                  </div>
                </div>

                <!-- Delete button for files -->
                <button
                  v-if="node.type === 'file'"
                  @click.stop="deleteDocument(node.doc.id, node.doc.title)"
                  class="p-1 text-gray-700 hover:text-red-400 transition flex-shrink-0 opacity-0 group-hover:opacity-100"
                  title="删除文档"
                >
                  <Trash2 class="h-3 w-3" />
                </button>
              </div>
            </div>
          </div>
        </aside>

        <!-- Right Side: collaborative editor workspace -->
        <div class="main-workspace">
          <div class="workspace-toolbar">
            <button
              v-if="!showDocSidebar"
              @click="showDocSidebar = true"
              class="toolbar-icon-btn"
              title="展开文档列表"
            >
              <ChevronRight class="h-3.5 w-3.5" />
            </button>
            <div class="min-w-0 flex-1">
              <div class="text-xs font-bold text-gray-200 truncate">
                {{ selectedDocId ? editingDoc.title : '选择或新建文档后开始协作' }}
              </div>
              <div class="text-[10px] text-gray-600 font-mono truncate">
                {{ selectedDocId ? editingDoc.fileName : 'AI 聊天、Markdown 编辑和实时预览会同时显示' }}
              </div>
            </div>
            <div class="flex items-center gap-2">
              <button
                v-if="selectedDocId"
                @click="showEditorSecrets = !showEditorSecrets"
                type="button"
                class="toolbar-secondary-btn"
              >
                <Key class="h-3.5 w-3.5" />
                {{ showEditorSecrets ? '隐藏密码箱' : '密码箱' }}
              </button>
              <button
                v-if="selectedDocId"
                @click="saveDocumentChanges"
                class="toolbar-save-btn"
              >
                <Save class="h-3.5 w-3.5" />
                保存
              </button>
              <button @click="createNewDocumentManual" class="mode-new-btn" type="button">
                <Plus class="h-3.5 w-3.5" />
                新建文档
              </button>
            </div>
          </div>

          <div class="collab-workspace">
            <section class="chat-panel">
              <div class="panel-heading">
                <MessageSquare class="h-3.5 w-3.5 text-emerald-400" />
                <span>AI 协作</span>
              </div>
              <div class="messages-area">
                <div
                  v-for="(msg, idx) in chatMessages"
                  :key="idx"
                  class="message-row"
                  :class="msg.role === 'user' ? 'message-row--user' : 'message-row--assistant'"
                >
                  <div class="message-bubble shadow-xl">
                    <div class="message-sender">
                      <component :is="msg.role === 'user' ? Key : Brain" class="h-3 w-3" />
                      <span>{{ msg.role === 'user' ? '您' : 'AI 大管家' }}</span>
                    </div>
                    <div class="message-text">
                      {{ msg.content }}
                    </div>

                    <div v-if="msg.sources && msg.sources.length > 0" class="mt-3 border-t border-gray-800/80 pt-2">
                      <p class="text-[10px] text-gray-500 font-bold mb-1.5">参考本地文档：</p>
                      <div class="flex flex-wrap gap-1.5">
                        <button
                          v-for="src in msg.sources"
                          :key="src.id"
                          @click="selectDocument(src.id)"
                          class="source-badge"
                        >
                          <FileText class="h-2.5 w-2.5 text-emerald-400" />
                          <span>{{ src.title }}</span>
                          <span class="text-[8px] text-emerald-500/60 font-semibold">{{ Math.round(src.score * 100) }}%</span>
                        </button>
                      </div>
                    </div>
                  </div>
                </div>

                <div v-if="chatLoading" class="chat-loading-bubble">
                  <RefreshCw class="animate-spin h-3.5 w-3.5 mr-2 text-emerald-400" />
                  <span>AI 正在处理...</span>
                </div>
              </div>
              <div class="chat-input-bar-wrapper">
                <div class="chat-input-bar">
                  <textarea
                    ref="textareaRef"
                    v-model="chatInput"
                    @keydown.enter.prevent="sendChatMessage"
                    class="c-textarea"
                    placeholder="边聊边改：例如“把当前文档整理成部署步骤”"
                    rows="1"
                  ></textarea>
                  <button
                    @click="sendChatMessage"
                    :disabled="chatLoading || !chatInput.trim()"
                    class="c-send-btn"
                  >
                    <Send class="h-3.5 w-3.5" />
                  </button>
                </div>
              </div>
            </section>

            <section class="document-panel">
              <div v-if="!selectedDocId" class="no-doc-placeholder">
                <FileText class="h-12 w-12 text-gray-800 mb-3" />
                <p class="text-sm text-gray-500">请选择一个文档，或者立即创建新的本地草稿。</p>
                <button @click="createNewDocumentManual" class="mt-4 d-save-btn text-xs">
                  <Plus class="h-4 w-4 mr-1" />
                  手动创建新文档
                </button>
              </div>

              <div v-else class="document-editor">
                <div class="editor-main-form">
                  <div class="panel-heading mb-4">
                    <PenTool class="h-3.5 w-3.5 text-emerald-400" />
                    <span>Markdown 编辑</span>
                  </div>
                  <div class="grid grid-cols-2 gap-4 mb-4">
                    <div>
                      <label class="d-label">文档标题</label>
                      <input v-model="editingDoc.title" type="text" class="d-input" />
                    </div>
                    <div>
                      <label class="d-label">文件名</label>
                      <input v-model="editingDoc.fileName" type="text" class="d-input" />
                    </div>
                  </div>

                  <div class="mb-4">
                    <label class="d-label">文档摘要</label>
                    <input v-model="editingDoc.summary" type="text" class="d-input" />
                  </div>

                  <div class="editor-ai-strip">
                    <div class="flex items-center gap-2 min-w-0">
                      <Sparkles class="h-4 w-4 text-emerald-400 flex-shrink-0" />
                      <input
                        v-model="editorAiInstruction"
                        type="text"
                        class="editor-ai-input"
                        placeholder="让 AI 帮我：例如整理成部署文档、补充排查步骤..."
                        @keydown.enter.prevent="runEditorAi('custom')"
                      />
                    </div>
                    <div class="flex items-center gap-2 flex-shrink-0">
                      <button
                        @click="runEditorAi('organize')"
                        :disabled="editorAiLoading"
                        type="button"
                        class="editor-ai-btn"
                      >
                        <RefreshCw v-if="editorAiLoading" class="h-3.5 w-3.5 animate-spin" />
                        <Sparkles v-else class="h-3.5 w-3.5" />
                        整理当前内容
                      </button>
                      <button
                        @click="runEditorAi('summary')"
                        :disabled="editorAiLoading"
                        type="button"
                        class="editor-ai-btn"
                      >
                        生成摘要
                      </button>
                      <button
                        @click="runEditorAi('custom')"
                        :disabled="editorAiLoading || !editorAiInstruction.trim()"
                        type="button"
                        class="editor-ai-run-btn"
                      >
                        应用到当前文档
                      </button>
                    </div>
                  </div>

                  <div class="flex-1 flex flex-col min-h-[300px]">
                    <label class="d-label">Markdown 内容</label>
                    <textarea
                      ref="editorTextareaRef"
                      v-model="editingDoc.markdown"
                      class="editor-textarea"
                      placeholder="在这里输入 Markdown 文档..."
                    ></textarea>
                  </div>

                  <details v-if="showEditorSecrets" class="secrets-drawer" open>
                    <summary>
                      <span class="flex items-center gap-1.5">
                        <Key class="h-3.5 w-3.5" />
                        关联的加密密码箱
                      </span>
                      <button @click.stop="addSecretToEditor" class="add-sec-btn" type="button">
                        <Plus class="h-3 w-3 mr-0.5" />
                        新增
                      </button>
                    </summary>
                    <div class="secrets-list-inline">
                      <div v-if="editorSecretsList.length === 0" class="text-center py-5 text-xs text-gray-600">
                        该文档暂无关联密码
                      </div>
                      <div v-else class="grid grid-cols-1 2xl:grid-cols-2 gap-3">
                        <div
                          v-for="(sec, idx) in editorSecretsList"
                          :key="idx"
                          class="p-3 bg-gray-950/60 border border-gray-800 rounded-xl relative group transition hover:border-emerald-500/50 space-y-2"
                        >
                          <div>
                            <label class="block text-[9px] text-gray-500 mb-1 font-semibold">密码占位符 (Key)</label>
                            <div class="flex items-center gap-1.5">
                              <input
                                v-model="sec.key"
                                type="text"
                                class="bg-gray-900 border border-gray-800 rounded-lg px-2.5 py-1 text-[11px] text-gray-200 focus:outline-none focus:border-emerald-500 flex-1 font-mono w-0 min-w-0"
                                placeholder="例如：数据库密码"
                              />
                              <button
                                @click="generateAiKey(sec, idx)"
                                :disabled="sec.aiLoading"
                                class="p-1.5 bg-emerald-500/10 border border-emerald-500/20 hover:bg-emerald-500/20 text-emerald-400 rounded-lg transition disabled:opacity-50"
                                title="AI 智能转换成英文去重代号"
                              >
                                <RefreshCw v-if="sec.aiLoading" class="animate-spin h-3.5 w-3.5" />
                                <Sparkles v-else class="h-3.5 w-3.5" />
                              </button>
                            </div>
                          </div>
                          <div>
                            <label class="block text-[9px] text-gray-500 mb-1 font-semibold">真实密码</label>
                            <div class="flex items-center gap-1 bg-gray-900 border border-gray-800 rounded-lg px-2 py-1">
                              <input
                                v-model="sec.value"
                                :type="sec.masked ? 'password' : 'text'"
                                class="bg-transparent text-xs text-gray-200 border-none focus:ring-0 p-0 focus:outline-none flex-1 font-mono"
                                placeholder="密码值"
                              />
                              <button @click="sec.masked = !sec.masked" class="text-gray-500 hover:text-gray-300 transition">
                                <component :is="sec.masked ? Eye : EyeOff" class="h-3.5 w-3.5" />
                              </button>
                            </div>
                          </div>
                          <div class="flex justify-between items-center pt-1.5 border-t border-gray-900">
                            <button
                              @click="insertSecretPlaceholder(sec.key)"
                              class="px-2.5 py-1 bg-emerald-500/10 hover:bg-emerald-500/20 border border-emerald-500/20 text-emerald-400 rounded-lg text-[10px] flex items-center gap-1 font-semibold transition"
                              title="在文档光标处插入占位符"
                            >
                              <Sparkles class="h-3 w-3" />
                              插入占位符
                            </button>
                            <button
                              @click="removeSecretFromEditor(idx)"
                              class="p-1 text-gray-600 hover:text-red-400 transition"
                              title="删除密码"
                            >
                              <Trash2 class="h-3.5 w-3.5" />
                            </button>
                          </div>
                        </div>
                      </div>
                    </div>
                  </details>
                </div>
              </div>
            </section>

            <section class="preview-panel">
              <div class="panel-heading">
                <FileText class="h-3.5 w-3.5 text-emerald-400" />
                <span>实时预览</span>
              </div>
              <div class="preview-surface" v-html="previewHtml"></div>
            </section>
          </div>
        </div>
      </div>
    </div>

    <!-- Settings & Backup Modal -->
    <div v-if="showSettings" class="modal-overlay" @click.self="showSettings = false">
      <div class="modal-card shadow-2xl">
        <div class="modal-header">
          <h3 class="modal-title font-bold text-gray-100">AI 服务配置与安全备份</h3>
          <button @click="showSettings = false" class="text-gray-600 hover:text-gray-400 transition">
            <X class="h-5 w-5" />
          </button>
        </div>

        <div class="modal-body divide-y divide-gray-800/60">
          <!-- AI Config -->
          <div class="py-4 first:pt-0">
            <h4 class="section-title">LLM 服务配置 (OpenAI 兼容)</h4>
            <div class="grid grid-cols-2 gap-4 mt-3">
              <div class="col-span-2">
                <label class="m-label">API 基础路径 (默认 OpenAI: https://api.openai.com/v1)</label>
                <input v-model="ollamaUrl" type="text" class="m-input" />
              </div>
              <div class="col-span-2">
                <label class="m-label">API Key</label>
                <input
                  v-model="apiKey"
                  type="password"
                  class="m-input"
                  :placeholder="hasApiKey ? '已保存 API Key，留空则保留不变' : '输入您的 API Key'"
                />
                <p v-if="hasApiKey" class="mt-1 text-[10px] text-gray-500">
                  已保存 API Key。出于安全考虑，当前页面不会回显明文。
                </p>
              </div>
              <div>
                <label class="m-label">文本/对话模型 (Chat Model)</label>
                <input v-model="chatModel" type="text" class="m-input" />
              </div>
              <div>
                <label class="m-label">向量模型 (Embedding Model)</label>
                <input v-model="embeddingModel" type="text" class="m-input" />
              </div>
              <div>
                <label class="m-label">推理强度 (reasoning_effort)</label>
                <input v-model="reasoningEffort" type="text" class="m-input" />
              </div>
              <label class="flex items-center gap-2 self-end text-xs text-gray-300 select-none cursor-pointer">
                <input v-model="disableResponseStorage" type="checkbox" class="h-4 w-4 rounded border-gray-800 bg-gray-950 text-emerald-500 focus:ring-emerald-500 focus:ring-offset-gray-905" />
                禁用响应存储 (store: false)
              </label>
              <div class="col-span-2 flex items-center gap-3">
                <button
                  @click="testConnection"
                  :disabled="testingConnection || !ollamaUrl.trim() || !chatModel.trim()"
                  type="button"
                  class="d-cancel-btn text-xs flex items-center gap-1.5"
                >
                  <RefreshCw v-if="testingConnection" class="h-3.5 w-3.5 animate-spin" />
                  <CheckCircle2 v-else class="h-3.5 w-3.5" />
                  测试连通性
                </button>
                <span
                  v-if="connectionMessage"
                  class="text-[10px] leading-relaxed"
                  :class="connectionOk ? 'text-emerald-400' : connectionOk === false ? 'text-rose-400' : 'text-gray-500'"
                >
                  {{ connectionMessage }}
                </span>
              </div>
              <div class="col-span-2 flex items-center gap-2 mt-2">
                <input id="allowAiSecrets" :checked="allowAiSecrets" @change="handleAllowSecretsChange" type="checkbox" class="h-4 w-4 rounded border-gray-800 bg-gray-950 text-emerald-500 focus:ring-emerald-500 focus:ring-offset-gray-905" />
                <label for="allowAiSecrets" class="text-xs text-gray-300 select-none cursor-pointer">
                  允许 AI 检索并读取解密后的密码 (使用云端大模型时建议关闭)
                </label>
              </div>
            </div>
            <!-- Storage Location Config -->
            <div class="mt-4 pt-3.5 border-t border-gray-800/80">
              <h4 class="section-title text-emerald-400">数据存储路径配置</h4>
              <p class="text-[10px] text-gray-500 mb-2">
                自定义本地文档和加密数据库的实际物理存储目录。
              </p>
              <div>
                <label class="m-label">本地数据存储目录路径</label>
                <input v-model="customDataDir" type="text" class="m-input font-mono" placeholder="默认位置: C:\Users\用户名\AppData\Roaming\rust-tool" />
              </div>
            </div>

            <div class="mt-4 flex items-center gap-2">
              <button @click="saveSettings" class="d-save-btn text-xs">
                保存设置与配置
              </button>
              <span class="text-[10px] text-gray-600">
                保存后会重新读取后端状态确认 API Key 是否已保存。
              </span>
            </div>
          </div>

          <!-- Backup Config -->
          <div class="py-4">
            <h4 class="section-title flex items-center gap-1 text-emerald-400">
              <Database class="h-4 w-4" />
              异地备份与容灾
            </h4>
            <p class="text-xs text-gray-500 mb-3">
              支持一键将本地所有的文档、加密密码和数据库压缩备份。
            </p>

            <div class="space-y-3">
              <div>
                <label class="m-label">本地备份目标目录（例如 D:\RustToolBackups）</label>
                <input v-model="localBackupDir" type="text" class="m-input" placeholder="留空则仅在临时目录打包" />
              </div>
              
              <div class="grid grid-cols-3 gap-3">
                <div class="col-span-3">
                  <label class="m-label">WebDAV 备份 URL（例如 https://dav.jianguoyun.com/dav/Backup）</label>
                  <input v-model="webdavUrl" type="text" class="m-input" placeholder="留空则不进行云端同步" />
                </div>
                <div>
                  <label class="m-label">WebDAV 账号</label>
                  <input v-model="webdavUser" type="text" class="m-input" />
                </div>
                <div class="col-span-2">
                  <label class="m-label">WebDAV 密码</label>
                  <input v-model="webdavPass" type="password" class="m-input" />
                </div>
              </div>
            </div>

            <div class="flex items-center justify-between gap-4 mt-4">
              <button
                @click="triggerBackup"
                :disabled="backupLoading"
                class="d-save-btn text-xs bg-gradient-to-r from-emerald-600 to-teal-500 hover:from-emerald-500 hover:to-teal-400"
              >
                <RefreshCw v-if="backupLoading" class="animate-spin h-3.5 w-3.5 mr-1" />
                立即执行安全备份
              </button>
              <span v-if="backupMessage" class="text-xs text-emerald-400 font-mono">{{ backupMessage }}</span>
            </div>
          </div>

          <!-- Restore Config -->
          <div class="py-4 last:pb-0">
            <h4 class="section-title flex items-center gap-1 text-orange-400">
              数据恢复 (Restore)
            </h4>
            <div class="mt-3">
              <label class="m-label">备份 ZIP 压缩包的绝对路径</label>
              <div class="flex gap-2">
                <input v-model="restorePath" type="text" class="m-input flex-1" placeholder="D:\RustToolBackups\rust_tool_memo_backup_xxx.zip" />
                <button
                  @click="triggerRestore"
                  :disabled="restoreLoading"
                  class="px-4 py-2 bg-orange-600 hover:bg-orange-500 rounded text-xs font-semibold text-white transition flex items-center"
                >
                  <RefreshCw v-if="restoreLoading" class="animate-spin h-3.5 w-3.5 mr-1" />
                  还原
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Custom Dialog Modal (Alert/Confirm) -->
    <div v-if="dialogState.show" class="dialog-overlay" @click.self="handleDialogCancel">
      <div class="dialog-card shadow-2xl">
        <!-- Icon and Visual Indicator -->
        <div class="dialog-icon-wrapper" :class="'dialog-icon-wrapper--' + dialogState.type">
          <AlertTriangle v-if="dialogState.type === 'warning' || dialogState.type === 'confirm'" class="h-6 w-6 text-amber-400 animate-pulse" />
          <AlertCircle v-else-if="dialogState.type === 'error'" class="h-6 w-6 text-rose-400" />
          <CheckCircle2 v-else-if="dialogState.type === 'success'" class="h-6 w-6 text-emerald-400 animate-bounce" />
          <Info v-else class="h-6 w-6 text-blue-400" />
        </div>

        <!-- Title -->
        <h3 class="dialog-title text-sm font-bold text-gray-100">{{ dialogState.title }}</h3>

        <!-- Message -->
        <div class="dialog-message-container py-1">
          <p class="dialog-message text-xs text-gray-400 leading-relaxed font-sans">{{ dialogState.message }}</p>
        </div>

        <!-- Footer Actions -->
        <div class="dialog-actions flex gap-3 w-full justify-center mt-2">
          <button
            v-if="dialogState.type === 'confirm' || dialogState.type === 'warning'"
            @click="handleDialogCancel"
            class="dialog-btn dialog-btn--cancel"
          >
            {{ dialogState.cancelText }}
          </button>
          <button
            @click="handleDialogConfirm"
            class="dialog-btn"
            :class="dialogState.type === 'error' ? 'dialog-btn--error' : (dialogState.type === 'warning' ? 'dialog-btn--warning' : 'dialog-btn--primary')"
          >
            {{ dialogState.confirmText }}
          </button>
        </div>
      </div>
    </div>
  </ToolShell>
</template>

<style scoped>
@reference "tailwindcss";
/* Glassmorphism details & modern layout styling */
.lock-overlay {
  @apply fixed inset-0 z-50 flex items-center justify-center bg-gray-950/80 backdrop-blur-md p-4;
}
.lock-card {
  @apply w-full max-w-md bg-gray-900/80 border border-white/5 rounded-2xl p-6 text-center shadow-2xl backdrop-blur-xl;
  box-shadow: 0 8px 32px 0 rgba(0, 0, 0, 0.4);
}
.lock-icon-container {
  @apply w-16 h-16 mx-auto flex items-center justify-center bg-emerald-500/10 rounded-full mb-4 border border-emerald-500/20;
}
.lock-title {
  @apply text-lg font-bold tracking-tight text-gray-100;
}
.lock-subtitle {
  @apply text-xs text-gray-400 mt-2 leading-relaxed;
}
.lock-form {
  @apply mt-6 flex flex-col gap-3;
}
.lock-input {
  @apply w-full px-4 py-2.5 bg-gray-950 border border-gray-800 rounded-xl text-sm text-gray-200 focus:outline-none focus:border-emerald-500 focus:ring-1 focus:ring-emerald-500 transition-colors text-center font-mono;
}
.lock-submit-btn {
  @apply w-full py-2.5 bg-gradient-to-r from-emerald-600 to-teal-500 hover:from-emerald-500 hover:to-teal-400 text-white rounded-xl text-xs font-semibold transition shadow-lg shadow-emerald-950/20 flex items-center justify-center;
}
.lock-warn {
  @apply text-[10px] text-gray-500 mt-4 leading-relaxed;
}

/* Main Memo Layout */
.memo-layout {
  @apply flex flex-col h-[75vh] min-h-[550px] border border-white/5 rounded-2xl bg-gray-900/30 backdrop-blur-xl overflow-hidden shadow-2xl;
}
.action-bar {
  @apply px-4 py-2.5 bg-gray-950/40 border-b border-white/5 flex justify-between items-center;
}
.action-btn {
  @apply px-3 py-1.5 bg-gray-950/40 border border-white/5 hover:bg-gray-800/40 rounded-lg text-xs font-medium text-gray-300 transition flex items-center gap-1.5;
}
.lock-btn {
  @apply px-3 py-1.5 bg-red-950/10 border border-red-900/20 hover:bg-red-950/30 rounded-lg text-xs font-semibold text-red-400 transition flex items-center;
}

.memo-grid {
  @apply flex-1 grid grid-cols-[230px_minmax(0,1fr)] min-h-0 transition-all duration-300;
}
.memo-grid--collapsed {
  @apply grid-cols-[0px_minmax(0,1fr)];
}

/* Sidebar Document List */
.doc-sidebar {
  @apply border-r border-white/5 flex flex-col bg-gray-950/10 min-h-0;
}
.search-input {
  @apply flex-1 px-3 py-1.5 bg-gray-950/80 border border-gray-800 rounded-lg text-xs text-gray-300 focus:outline-none focus:border-emerald-500 transition-colors;
}
.new-doc-btn {
  @apply p-2 bg-emerald-900/10 border border-emerald-800/20 hover:bg-emerald-900/20 rounded-lg text-emerald-400 transition-colors;
}
.doc-list-scroll {
  @apply flex-1 overflow-y-auto min-h-0;
}
.empty-docs-container {
  @apply p-6 text-center text-gray-500 text-sm flex flex-col items-center justify-center h-[200px];
}
.doc-item {
  @apply p-3 cursor-pointer hover:bg-gray-800/20 transition-colors border-l-2 border-transparent relative;
}
.doc-item--active {
  @apply bg-emerald-500/5 border-emerald-500/80;
}
.doc-item-title {
  @apply text-xs font-bold text-gray-200 truncate;
}
.doc-item-summary {
  @apply text-[10px] text-gray-500 mt-1 line-clamp-2 leading-relaxed;
}
.doc-delete-btn {
  @apply p-1 text-gray-700 hover:text-red-400 transition flex-shrink-0 opacity-0 group-hover:opacity-100;
}
.doc-item:hover .doc-delete-btn {
  @apply opacity-100;
}

/* Workspace main content */
.main-workspace {
  @apply flex flex-col min-h-0 bg-gray-950/5;
}
.workspace-toolbar {
  @apply px-3 py-2 border-b border-white/5 bg-gray-950/20 flex items-center justify-between gap-3;
}
.toolbar-icon-btn {
  @apply h-8 w-8 rounded-lg border border-gray-800/70 bg-gray-950/60 text-emerald-400 hover:bg-gray-900 transition flex items-center justify-center;
}
.toolbar-secondary-btn {
  @apply h-8 px-3 rounded-lg border border-gray-800/70 bg-gray-950/60 text-xs font-semibold text-gray-400 hover:text-emerald-300 hover:border-emerald-500/30 transition flex items-center gap-1.5;
}
.toolbar-save-btn {
  @apply h-8 px-3 rounded-lg bg-emerald-600 hover:bg-emerald-500 text-white text-xs font-semibold transition flex items-center gap-1.5 shadow-md shadow-emerald-950/20;
}
.mode-new-btn {
  @apply h-8 px-3 rounded-lg bg-emerald-600 hover:bg-emerald-500 text-white text-xs font-semibold transition flex items-center gap-1.5 shadow-md shadow-emerald-950/20;
}
.collab-workspace {
  @apply flex-1 min-h-0 grid grid-cols-[minmax(230px,0.8fr)_minmax(360px,1.45fr)_minmax(300px,1fr)];
}
.chat-panel,
.document-panel,
.preview-panel {
  @apply min-h-0 flex flex-col border-r border-white/5;
}
.preview-panel {
  @apply border-r-0 bg-gray-950/10;
}
.panel-heading {
  @apply h-10 px-4 border-b border-white/5 bg-gray-950/20 flex items-center gap-1.5 text-xs font-bold text-gray-400;
}
.messages-area {
  @apply flex-1 overflow-y-auto p-4 space-y-4 min-h-0;
}
.message-row {
  @apply flex;
}
.message-row--user {
  @apply justify-end;
}
.message-bubble {
  @apply max-w-[92%] rounded-2xl px-4 py-2.5 text-xs shadow-md leading-relaxed border;
}
.message-row--user .message-bubble {
  @apply bg-emerald-600/90 text-white rounded-br-none border-emerald-500/30;
}
.message-row--assistant .message-bubble {
  @apply bg-gray-900/90 border-white/5 text-gray-200 rounded-bl-none;
}
.message-sender {
  @apply flex items-center gap-1 text-[9px] text-gray-500 mb-1.5 font-bold uppercase tracking-wider;
}
.message-row--user .message-sender {
  @apply text-emerald-200/80 justify-end;
}
.message-text {
  @apply whitespace-pre-wrap;
}

.chat-input-bar-wrapper {
  @apply p-3 border-t border-white/5 bg-gray-950/30;
}
.chat-input-bar {
  @apply flex gap-2 items-center bg-gray-950 border border-gray-800 rounded-2xl px-3 py-2 focus-within:border-emerald-500/80 transition-colors shadow-inner;
}
.c-textarea {
  @apply flex-1 min-h-[32px] max-h-[96px] overflow-y-auto bg-transparent border-none text-xs text-gray-300 placeholder:text-gray-600 focus:outline-none focus:ring-0 transition resize-none leading-relaxed;
  scrollbar-width: none;
}
.c-textarea::-webkit-scrollbar {
  display: none;
}
.c-send-btn {
  @apply h-9 w-9 bg-emerald-600 hover:bg-emerald-500 disabled:bg-gray-900 disabled:text-gray-700 rounded-xl text-white transition flex-shrink-0 shadow-md shadow-emerald-950/20 flex items-center justify-center;
}

.chat-loading-bubble {
  @apply flex items-center text-xs text-emerald-400 bg-emerald-500/5 border border-emerald-500/10 rounded-xl px-4 py-2 w-max shadow-md;
}

.source-badge {
  @apply flex items-center gap-1.5 px-2 py-0.5 bg-gray-950 border border-gray-800 hover:bg-gray-900 rounded-lg text-[10px] text-gray-400 font-mono transition shadow-sm;
}

.d-label {
  @apply block text-[10px] font-bold text-gray-500 uppercase tracking-wider mb-1.5;
}
.d-input {
  @apply w-full px-3 py-2 bg-gray-950 border border-gray-800 rounded-xl text-xs text-gray-200 focus:outline-none focus:border-emerald-500 focus:ring-1 focus:ring-emerald-500 transition;
}
.d-textarea {
  @apply w-full px-3 py-2 bg-gray-950 border border-gray-800 rounded-xl text-xs text-gray-200 focus:outline-none focus:border-emerald-500 focus:ring-1 focus:ring-emerald-500 transition font-mono resize-y;
}
.d-cancel-btn {
  @apply px-4 py-2 border border-gray-800 hover:bg-gray-800 text-gray-400 rounded-xl text-xs font-semibold transition-colors;
}
.d-save-btn {
  @apply px-4 py-2 bg-gradient-to-r from-emerald-600 to-teal-500 hover:from-emerald-500 hover:to-teal-400 text-white rounded-xl text-xs font-semibold transition shadow-md shadow-emerald-950/20 flex items-center;
}

.no-doc-placeholder {
  @apply flex-1 flex flex-col items-center justify-center p-8 text-center text-gray-500 text-sm;
}
.document-editor {
  @apply flex-1 min-h-0 flex flex-col;
}
.editor-main-form {
  @apply p-4 flex flex-col min-h-0 overflow-y-auto;
}
.editor-ai-strip {
  @apply mb-4 rounded-xl border border-emerald-500/15 bg-emerald-500/5 px-3 py-2 flex items-center justify-between gap-3;
}
.editor-ai-input {
  @apply min-w-0 flex-1 bg-transparent border-none text-xs text-gray-300 placeholder:text-gray-600 focus:outline-none focus:ring-0;
}
.editor-ai-btn {
  @apply h-8 px-2.5 rounded-lg border border-gray-800 bg-gray-950/60 text-xs font-semibold text-gray-400 hover:text-emerald-300 hover:border-emerald-500/30 hover:bg-emerald-500/10 disabled:opacity-50 transition flex items-center gap-1.5;
}
.editor-ai-run-btn {
  @apply h-8 px-3 rounded-lg bg-emerald-600 hover:bg-emerald-500 disabled:bg-gray-900 disabled:text-gray-700 text-xs font-semibold text-white transition;
}
.editor-textarea {
  @apply flex-1 w-full px-4 py-3 bg-gray-950 border border-gray-800 rounded-xl text-xs text-gray-200 focus:outline-none focus:border-emerald-500 focus:ring-1 focus:ring-emerald-500 transition font-mono resize-none mt-1 min-h-[300px] leading-relaxed;
}
.add-sec-btn {
  @apply px-2 py-0.5 border border-emerald-800/40 hover:bg-emerald-900/10 text-emerald-400 rounded-lg text-[10px] font-semibold transition flex items-center;
}
.secrets-drawer {
  @apply mt-4 rounded-xl border border-gray-800 bg-gray-950/35 overflow-hidden;
}
.secrets-drawer summary {
  @apply px-3 py-2 cursor-pointer list-none flex items-center justify-between text-[11px] font-bold text-emerald-400 border-b border-gray-800/70;
}
.secrets-drawer summary::-webkit-details-marker {
  display: none;
}
.secrets-list-inline {
  @apply max-h-72 overflow-y-auto p-3;
}
.preview-surface {
  @apply flex-1 min-h-0 overflow-y-auto p-6 text-sm text-gray-300 leading-relaxed;
}
.preview-surface :deep(h1) {
  @apply text-2xl font-bold text-gray-100 mb-4 pb-3 border-b border-gray-800;
}
.preview-surface :deep(h2) {
  @apply text-lg font-bold text-gray-100 mt-5 mb-3;
}
.preview-surface :deep(h3) {
  @apply text-base font-semibold text-gray-200 mt-4 mb-2;
}
.preview-surface :deep(h4),
.preview-surface :deep(h5),
.preview-surface :deep(h6) {
  @apply text-sm font-semibold text-gray-200 mt-3 mb-2;
}
.preview-surface :deep(p) {
  @apply mb-3;
}
.preview-surface :deep(.md-spacer) {
  @apply mb-2;
}
.preview-surface :deep(.preview-empty) {
  @apply text-gray-600 text-center mt-20;
}
.preview-surface :deep(ul),
.preview-surface :deep(ol) {
  @apply mb-3 pl-5 space-y-1;
}
.preview-surface :deep(ul) {
  @apply list-disc;
}
.preview-surface :deep(ol) {
  @apply list-decimal;
}
.preview-surface :deep(blockquote) {
  @apply my-3 border-l-2 border-emerald-500/40 pl-3 text-gray-400 bg-emerald-500/5 py-2 rounded-r-lg;
}
.preview-surface :deep(pre) {
  @apply my-3 overflow-x-auto rounded-xl border border-gray-800 bg-gray-950 p-3 text-xs;
}
.preview-surface :deep(code) {
  @apply rounded-md border border-gray-800 bg-gray-950 px-1.5 py-0.5 text-[0.85em] text-emerald-200;
}
.preview-surface :deep(pre code) {
  @apply border-0 bg-transparent p-0 text-gray-300;
}
.preview-surface :deep(strong) {
  @apply text-gray-100 font-semibold;
}
.preview-surface :deep(.md-secret) {
  @apply inline-flex rounded-md border border-emerald-500/25 bg-emerald-500/10 px-1.5 py-0.5 text-[0.85em] font-mono text-emerald-300;
}

@media (max-width: 1400px) {
  .collab-workspace {
    @apply grid-cols-[minmax(220px,0.75fr)_minmax(320px,1.2fr)_minmax(280px,0.95fr)];
  }
}

@media (max-width: 1180px) {
  .collab-workspace {
    @apply grid-cols-[260px_minmax(0,1fr)] grid-rows-[minmax(0,1fr)_minmax(260px,0.75fr)];
  }
  .chat-panel {
    @apply row-span-2;
  }
  .document-panel {
    @apply border-r-0 border-b border-white/5;
  }
  .preview-panel {
    @apply col-start-2;
  }
}

/* Modal configuration */
.modal-overlay {
  @apply fixed inset-0 z-50 flex items-center justify-center bg-black/85 backdrop-blur-md p-4;
}
.modal-card {
  @apply w-full max-w-xl bg-gray-900/90 border border-white/5 rounded-2xl shadow-2xl overflow-hidden flex flex-col max-h-[85vh] backdrop-blur-2xl;
}
.modal-header {
  @apply px-5 py-3.5 border-b border-gray-800/80 flex justify-between items-center bg-gray-950/40;
}
.modal-title {
  @apply text-xs font-bold text-gray-200 uppercase tracking-wide;
}
.modal-body {
  @apply p-5 overflow-y-auto flex-1 min-h-0;
}
.section-title {
  @apply text-xs font-bold text-gray-300 uppercase tracking-wider mb-1;
}
.m-label {
  @apply block text-[10px] font-semibold text-gray-500 mb-1.5;
}
.m-input {
  @apply w-full px-3 py-2 bg-gray-950 border border-gray-800 rounded-xl text-xs text-gray-300 focus:outline-none focus:border-emerald-500 focus:ring-1 focus:ring-emerald-500 transition-colors;
}

/* Custom Thin Premium Scrollbars */
.messages-area::-webkit-scrollbar,
.doc-list-scroll::-webkit-scrollbar,
.editor-main-form::-webkit-scrollbar,
.secrets-list-inline::-webkit-scrollbar,
.preview-surface::-webkit-scrollbar,
.modal-body::-webkit-scrollbar {
  width: 4px;
  height: 4px;
}
.messages-area::-webkit-scrollbar-track,
.doc-list-scroll::-webkit-scrollbar-track,
.editor-main-form::-webkit-scrollbar-track,
.secrets-list-inline::-webkit-scrollbar-track,
.preview-surface::-webkit-scrollbar-track,
.modal-body::-webkit-scrollbar-track {
  background: transparent;
}
.messages-area::-webkit-scrollbar-thumb,
.doc-list-scroll::-webkit-scrollbar-thumb,
.editor-main-form::-webkit-scrollbar-thumb,
.secrets-list-inline::-webkit-scrollbar-thumb,
.preview-surface::-webkit-scrollbar-thumb,
.modal-body::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.05);
  border-radius: 4px;
}
.messages-area::-webkit-scrollbar-thumb:hover,
.doc-list-scroll::-webkit-scrollbar-thumb:hover,
.editor-main-form::-webkit-scrollbar-thumb:hover,
.secrets-list-inline::-webkit-scrollbar-thumb:hover,
.preview-surface::-webkit-scrollbar-thumb:hover,
.modal-body::-webkit-scrollbar-thumb:hover {
  background: rgba(255, 255, 255, 0.15);
}

/* Custom Dialog Styles */
.dialog-overlay {
  @apply fixed inset-0 z-[100] flex items-center justify-center bg-black/60 backdrop-blur-sm p-4;
  animation: dialog-fade-in 0.2s ease-out;
}
.dialog-card {
  @apply w-full max-w-sm bg-gray-900/90 border border-white/10 rounded-2xl p-6 text-center shadow-2xl backdrop-blur-xl flex flex-col items-center;
  box-shadow: 0 20px 50px rgba(0, 0, 0, 0.5), inset 0 1px 0 rgba(255, 255, 255, 0.05);
  animation: dialog-slide-up 0.25s cubic-bezier(0.16, 1, 0.3, 1);
}
.dialog-icon-wrapper {
  @apply w-12 h-12 rounded-full flex items-center justify-center mb-4 relative;
}
.dialog-icon-wrapper::after {
  content: '';
  @apply absolute inset-0 rounded-full opacity-20 blur-md;
}
.dialog-icon-wrapper--warning,
.dialog-icon-wrapper--confirm {
  @apply bg-amber-500/10 border border-amber-500/20;
}
.dialog-icon-wrapper--warning::after,
.dialog-icon-wrapper--confirm::after {
  @apply bg-amber-500;
}
.dialog-icon-wrapper--error {
  @apply bg-rose-500/10 border border-rose-500/20;
}
.dialog-icon-wrapper--error::after {
  @apply bg-rose-500;
}
.dialog-icon-wrapper--success {
  @apply bg-emerald-500/10 border border-emerald-500/20;
}
.dialog-icon-wrapper--success::after {
  @apply bg-emerald-500;
}
.dialog-icon-wrapper--info {
  @apply bg-blue-500/10 border border-blue-500/20;
}
.dialog-icon-wrapper--info::after {
  @apply bg-blue-500;
}

.dialog-title {
  @apply text-sm font-bold text-gray-100 tracking-wide mb-2;
}
.dialog-message-container {
  @apply w-full max-h-[180px] overflow-y-auto mb-5 px-1;
}
.dialog-message {
  @apply text-xs text-gray-400 leading-relaxed whitespace-pre-wrap text-center font-sans;
}
.dialog-actions {
  @apply flex gap-3 w-full justify-center;
}
.dialog-btn {
  @apply flex-1 py-2 rounded-xl text-xs font-semibold transition-all duration-200 outline-none select-none;
}
.dialog-btn--cancel {
  @apply bg-gray-950 border border-gray-800 text-gray-400 hover:bg-gray-900 hover:text-gray-300;
}
.dialog-btn--primary {
  @apply bg-gradient-to-r from-emerald-600 to-teal-500 hover:from-emerald-500 hover:to-teal-400 text-white shadow-lg shadow-emerald-950/20;
}
.dialog-btn--warning {
  @apply bg-gradient-to-r from-amber-600 to-amber-500 hover:from-amber-500 hover:to-amber-400 text-white shadow-lg shadow-amber-950/20;
}
.dialog-btn--error {
  @apply bg-gradient-to-r from-rose-600 to-rose-500 hover:from-rose-500 hover:to-rose-400 text-white shadow-lg shadow-rose-950/20;
}

@keyframes dialog-fade-in {
  from { opacity: 0; }
  to { opacity: 1; }
}
@keyframes dialog-slide-up {
  from { transform: translateY(12px) scale(0.96); opacity: 0; }
  to { transform: translateY(0) scale(1); opacity: 1; }
}
</style>
