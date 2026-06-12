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
import SecurePasswordInput from '../components/SecurePasswordInput.vue'
import ToolShell from '../components/ToolShell.vue'
import { memoRequest } from '../services/memoApi'

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

type FindingSeverity = 'critical' | 'warning' | 'info'
type FindingStatus = 'open' | 'fixed' | 'ignored' | 'reviewing'
type FindingKind = 'hardcodedSecret' | 'weakJwt' | 'insecureLink' | 'sensitiveOperation' | 'governanceGap'
type SecurityCaseType = 'risk' | 'exception' | 'review'
type SecurityAssetType = 'url' | 'apiEndpoint' | 'secret' | 'service' | 'database' | 'dependency' | 'environment' | 'dataType'
type SecurityGraphNodeType = 'document' | 'asset' | 'finding' | 'secret' | 'case'
type SecurityGraphEdgeType = 'documentAsset' | 'assetFinding' | 'assetSecret' | 'findingCase' | 'assetCase'
type SecurityReportScope = 'all' | 'document' | 'asset' | 'tags'
type MemoView = 'archive' | 'workspace'
type SecurityCaseStatus =
  | 'open'
  | 'acknowledged'
  | 'accepted'
  | 'fixing'
  | 'fixed'
  | 'reviewing'
  | 'closed'
  | 'reopened'
type RightPanelTab = 'assistant' | 'audit' | 'preview'

interface SecurityFinding {
  id: string
  docId: string
  lineStart: number
  lineEnd: number
  severity: FindingSeverity
  kind: FindingKind
  title: string
  detail: string
  evidence: string
  recommendation: string
  status: FindingStatus
}

interface AuditSummary {
  total: number
  critical: number
  warning: number
  info: number
  open: number
  ignored: number
  fixed: number
  reviewing: number
  lastScannedAt: number
}

interface DocumentRiskListSummary {
  total: number
  critical: number
  warning: number
  info: number
  reviewing: number
}

interface AuditFixPreview {
  patchMarkdown: string
  explanation: string
}

interface DetectedSecret {
  key: string
  placeholder: string
  value: string
  label: string
}

interface RedactMarkdownResponse {
  markdown: string
  secrets: DetectedSecret[]
  redactedSecretCount: number
}

interface GovernanceSummary {
  riskSummary: {
    total: number
    open: number
    critical: number
    warning: number
    info: number
    reviewing: number
    ignored: number
    accepted: number
    expiringSoon: number
    expiredAcceptances: number
  }
  assetSummary: {
    total: number
    services: number
    apiEndpoints: number
    urls: number
    secrets: number
    databases?: number
    dependencies?: number
    environments?: number
    dataTypes?: number
  }
  recentFindings: SecurityFinding[]
  recentAssets: Array<{ id: string; name: string; assetType: string }>
  recentActivities: Array<{ id: string; title: string; summary: string; createdAt: number }>
}

interface SecurityCaseEvent {
  eventType: string
  summary: string
  createdAt: number
}

interface SecurityCase {
  id: string
  caseType: SecurityCaseType
  title: string
  severity: FindingSeverity
  status: SecurityCaseStatus
  sourceDocId: string
  sourceFindingId?: string | null
  linkedAssets: string[]
  owner?: string | null
  dueAt?: string | null
  acceptedUntil?: string | null
  rationale?: string | null
  impactScope?: string | null
  compensatingControls?: string | null
  reviewer?: string | null
  createdAt: number
  updatedAt: number
  events: SecurityCaseEvent[]
}

interface SecurityAsset {
  id: string
  assetType: SecurityAssetType
  name: string
  aliases: string[]
  tags: string[]
  sourceDocIds: string[]
  linkedSecretKeys: string[]
  linkedCaseIds: string[]
  lastSeenAt: number
}

interface SecurityAssetDetail {
  asset: SecurityAsset
  documents: Array<{ id: string; title: string; fileName: string; summary: string; updatedAt: number }>
  findings: SecurityFinding[]
  cases: SecurityCase[]
}

interface SecurityGraphNode {
  id: string
  nodeType: SecurityGraphNodeType
  label: string
  severity?: FindingSeverity | null
  status?: string | null
}

interface SecurityGraphEdge {
  id: string
  edgeType: SecurityGraphEdgeType
  from: string
  to: string
  label: string
}

interface SecurityAssetGraph {
  nodes: SecurityGraphNode[]
  edges: SecurityGraphEdge[]
}

interface DocumentRiskDiffSummary {
  added: number
  resolved: number
  severityChanged: number
  moved: number
  unchanged: number
  previousTotal: number
  currentTotal: number
}

interface DocumentRiskDiffItem {
  fingerprint: string
  title: string
  severity: FindingSeverity
  previousSeverity?: FindingSeverity | null
  kind: FindingKind
  lineStart: number
  lineEnd: number
  previousLineStart?: number | null
  previousLineEnd?: number | null
  evidence: string
  status: FindingStatus
}

interface DocumentRiskDiff {
  docId: string
  previousSavedAt?: number | null
  currentSavedAt: number
  previousHash?: string | null
  currentHash: string
  added: DocumentRiskDiffItem[]
  resolved: DocumentRiskDiffItem[]
  changed: DocumentRiskDiffItem[]
  summary: DocumentRiskDiffSummary
}

interface AuditEvent {
  id: string
  eventType: string
  actor: string
  targetId: string
  summary: string
  createdAt: number
  metadata: Record<string, unknown>
}

interface SecurityReport {
  id: string
  fileName: string
  path: string
  markdown: string
  summary: string
  createdAt: number
}

interface SafeShareExport {
  id: string
  fileName: string
  path: string
  markdown: string
  summary: string
  redactedSecretCount: number
  findingCount: number
  createdAt: number
}

type ChecklistStatus = 'open' | 'done' | 'waived'

interface StandardEntry {
  id: string
  category: string
  title: string
  description: string
  controls: string[]
}

interface ChecklistItem {
  id: string
  title: string
  description: string
  standardIds: string[]
  recommended: boolean
  status: ChecklistStatus
  note?: string | null
  evidence: string[]
  updatedAt?: number | null
}

interface StandardsChecklistResponse {
  docId?: string | null
  items: ChecklistItem[]
  standards: StandardEntry[]
  updatedAt: number
}

interface CaseActionDraft {
  owner: string
  dueAt: string
  rationale: string
  acceptedUntil: string
  impactScope: string
  compensatingControls: string
  reviewer: string
}

type SettingsTab = 'ai' | 'data' | 'backup' | 'security'

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
const defaultDataDir = ref('')
const activeDataDir = ref('')
const dataDirConfigPath = ref('')
const usingCustomDataDir = ref(false)
const migrationTargetDir = ref('')
const migrationLoading = ref(false)
const migrationMessage = ref('')
const showSettings = ref(false)
const settingsTab = ref<SettingsTab>('ai')
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
const restoreConfirmText = ref('')
const restoreLoading = ref(false)

// Documents list
const documents = ref<any[]>([])
const searchFilter = ref('')
const listLoading = ref(false)

// Selection & editing
const showEditorSecrets = ref(false)
const showDocSidebar = ref(true)
const selectedDocId = ref<string | null>(null)
const activeMemoView = ref<MemoView>('archive')
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
const documentRedactLoading = ref(false)

// Audit and governance state
const rightPanelTab = ref<RightPanelTab>('audit')
const auditFindings = ref<SecurityFinding[]>([])
const auditSummary = ref<AuditSummary | null>(null)
const auditLoading = ref(false)
const batchFixLoading = ref(false)
const selectedFindingId = ref<string | null>(null)
const governanceSummary = ref<GovernanceSummary | null>(null)
const governanceCases = ref<SecurityCase[]>([])
const governanceEvents = ref<AuditEvent[]>([])
const securityAssets = ref<SecurityAsset[]>([])
const selectedAssetDetail = ref<SecurityAssetDetail | null>(null)
const securityAssetGraph = ref<SecurityAssetGraph | null>(null)
const documentRiskDiff = ref<DocumentRiskDiff | null>(null)
const governanceLoading = ref(false)
const assetLoading = ref(false)
const graphLoading = ref(false)
const diffLoading = ref(false)
const caseActionDrafts = ref<Record<string, CaseActionDraft>>({})
const caseActionLoading = ref<Record<string, boolean>>({})
const reportLoading = ref(false)
const safeShareLoading = ref(false)
const reportTagsInput = ref('secret jwt')
const standardsChecklist = ref<StandardsChecklistResponse | null>(null)
const checklistLoading = ref(false)
const checklistActionLoading = ref<Record<string, boolean>>({})
const selectedTextRange = ref<{ start: number; end: number; text: string } | null>(null)

// Chat state
const chatInput = ref('')
const chatMessages = ref<any[]>([
  {
    role: 'assistant',
    content: '你好！我是您的 AI 安全文档助手。\n您可以随时在这里打下杂乱的文字，我会先在本地脱敏，再帮您整理成 Markdown 文档；也可以向我提问，我会从本地文档中检索。',
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

const activeFindings = computed(() =>
  auditFindings.value.filter((finding) => finding.status !== 'ignored' && finding.status !== 'fixed')
)

const batchFixCandidates = computed(() =>
  activeFindings.value.filter((finding) => finding.status === 'open' || finding.status === 'reviewing')
)

const selectedFinding = computed(() =>
  auditFindings.value.find((finding) => finding.id === selectedFindingId.value) || null
)

const editorLineNumbers = computed(() => {
  const lineCount = Math.max(1, (editingDoc.value.markdown || '').split('\n').length)
  return Array.from({ length: lineCount }, (_, index) => index + 1)
})

const riskLineMap = computed(() => {
  const map = new Map<number, SecurityFinding[]>()
  for (const finding of activeFindings.value) {
    for (let line = finding.lineStart; line <= finding.lineEnd; line += 1) {
      const current = map.get(line) || []
      current.push(finding)
      map.set(line, current)
    }
  }
  return map
})

const governanceRiskSummary = computed(() => governanceSummary.value?.riskSummary)
const governanceAssetSummary = computed(() => governanceSummary.value?.assetSummary)
const dashboardOpenCases = computed(() =>
  governanceCases.value
    .filter((item) => ['open', 'acknowledged', 'fixing', 'reviewing', 'reopened'].includes(item.status))
    .slice(0, 5)
)
const dashboardReviewCases = computed(() =>
  governanceCases.value.filter((item) => item.status === 'reviewing').slice(0, 4)
)
const dashboardRecentDocs = computed(() =>
  [...documents.value]
    .sort((left, right) => (right.updatedAt || 0) - (left.updatedAt || 0))
    .slice(0, 4)
)
const dashboardRecentFindings = computed(() => governanceSummary.value?.recentFindings?.slice(0, 5) || [])
const dashboardRecentAssets = computed(() => governanceSummary.value?.recentAssets?.slice(0, 8) || [])
const dashboardRecentActivities = computed(() => governanceSummary.value?.recentActivities?.slice(0, 6) || [])
const visibleGovernanceCases = computed(() => governanceCases.value.slice(0, 6))
const visibleGovernanceEvents = computed(() => governanceEvents.value.slice(-4).reverse())
const visibleSecurityAssets = computed(() => securityAssets.value.slice(0, 8))
const visibleGraphNodes = computed(() => securityAssetGraph.value?.nodes.slice(0, 12) || [])
const visibleGraphEdges = computed(() => securityAssetGraph.value?.edges.slice(0, 12) || [])
const hasRiskDiffChanges = computed(() => {
  const summary = documentRiskDiff.value?.summary
  if (!summary) return false
  return summary.added + summary.resolved + summary.severityChanged + summary.moved > 0
})
const visibleRiskDiffItems = computed(() => {
  const diff = documentRiskDiff.value
  if (!diff) return []
  return [
    ...diff.added.map((item) => ({ ...item, changeType: 'added' as const })),
    ...diff.resolved.map((item) => ({ ...item, changeType: 'resolved' as const })),
    ...diff.changed.map((item) => ({ ...item, changeType: 'changed' as const })),
  ].slice(0, 4)
})
const visibleChecklistItems = computed(() => standardsChecklist.value?.items.slice(0, 5) || [])
const standardsById = computed(() => {
  const map = new Map<string, StandardEntry>()
  for (const standard of standardsChecklist.value?.standards || []) {
    map.set(standard.id, standard)
  }
  return map
})
const documentRiskSummaryMap = computed(() => {
  const map = new Map<string, DocumentRiskListSummary>()
  for (const doc of documents.value) {
    if (doc.id) map.set(doc.id, emptyDocumentRiskSummary())
  }

  for (const caseItem of governanceCases.value) {
    if (!caseItem.sourceDocId || ['closed', 'fixed'].includes(caseItem.status)) continue
    const summary = map.get(caseItem.sourceDocId) || emptyDocumentRiskSummary()
    summary.total += 1
    if (caseItem.severity === 'critical') summary.critical += 1
    else if (caseItem.severity === 'warning') summary.warning += 1
    else summary.info += 1
    if (caseItem.status === 'reviewing') summary.reviewing += 1
    map.set(caseItem.sourceDocId, summary)
  }

  const selectedId = selectedDocId.value && selectedDocId.value !== 'new'
    ? (editingDoc.value.id || selectedDocId.value)
    : null
  if (selectedId && auditFindings.value.length > 0) {
    const summary = emptyDocumentRiskSummary()
    for (const finding of activeFindings.value) {
      summary.total += 1
      if (finding.severity === 'critical') summary.critical += 1
      else if (finding.severity === 'warning') summary.warning += 1
      else summary.info += 1
      if (finding.status === 'reviewing') summary.reviewing += 1
    }
    map.set(selectedId, summary)
  }

  return map
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
    const res = await memoRequest('/status')
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
      if (data.unlocked) {
        activeMemoView.value = 'archive'
        void loadDocuments()
      }
    }
    await fetchDataDirStatus()
  } catch (e) {
    console.error('Failed to fetch memo status:', e)
  } finally {
    statusLoading.value = false
  }
}

async function fetchDataDirStatus() {
  try {
    const res = await memoRequest('/data-dir')
    if (res.ok) {
      const data = await res.json()
      defaultDataDir.value = data.defaultDataDir || ''
      activeDataDir.value = data.activeDataDir || ''
      customDataDir.value = data.customDataDir || ''
      dataDirConfigPath.value = data.configPath || ''
      usingCustomDataDir.value = data.usingCustomDataDir || false
      if (!migrationTargetDir.value.trim()) {
        migrationTargetDir.value = data.customDataDir || ''
      }
    }
  } catch (e) {
    console.error('Failed to fetch data directory status:', e)
  }
}

async function handleUnlock() {
  if (!masterPassword.value) return
  try {
    const res = await memoRequest('/unlock', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ password: masterPassword.value }),
    })
    if (res.ok) {
      const data = await res.json()
      if (data.unlocked) {
        unlocked.value = true
        activeMemoView.value = 'archive'
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
    await customAlert('请求失败，请确认本地服务可用。', '连接错误', 'error')
  }
}

async function handleLock() {
  try {
    const res = await memoRequest('/lock', { method: 'POST' })
    if (res.ok) {
      unlocked.value = false
      documents.value = []
      selectedDocId.value = null
      activeMemoView.value = 'archive'
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
    const res = await memoRequest('/list')
    if (res.ok) {
      documents.value = await res.json()
      void loadGovernanceSummary()
    }
  } catch (e) {
    console.error('Failed to load documents:', e)
  } finally {
    listLoading.value = false
  }
}

async function loadGovernanceSummary() {
  if (!unlocked.value) return
  try {
    governanceLoading.value = true
    const [summaryRes, casesRes, eventsRes] = await Promise.all([
      memoRequest('/governance/summary'),
      memoRequest('/governance/cases'),
      memoRequest('/governance/events'),
    ])
    if (summaryRes.ok) {
      governanceSummary.value = await summaryRes.json()
    }
    if (casesRes.ok) {
      const cases = await casesRes.json()
      syncCaseActionDrafts(cases || [])
      governanceCases.value = cases || []
    }
    if (eventsRes.ok) {
      governanceEvents.value = await eventsRes.json()
    }
    void loadSecurityAssets()
  } catch (e) {
    console.error('Failed to load governance summary:', e)
  } finally {
    governanceLoading.value = false
  }
  void loadStandardsChecklist()
}

async function loadSecurityAssets() {
  if (!unlocked.value) return
  try {
    assetLoading.value = true
    const res = await memoRequest('/assets/list')
    if (res.ok) {
      securityAssets.value = await res.json()
      if (
        selectedAssetDetail.value &&
        !securityAssets.value.some((asset) => asset.id === selectedAssetDetail.value?.asset.id)
      ) {
        selectedAssetDetail.value = null
      }
      void loadSecurityAssetGraph(selectedAssetDetail.value?.asset || null)
    }
  } catch (e) {
    console.error('Failed to load security assets:', e)
  } finally {
    assetLoading.value = false
  }
}

async function loadSecurityAssetGraph(asset: SecurityAsset | null = null) {
  if (!unlocked.value) return
  const query = asset ? `?assetId=${encodeURIComponent(asset.id)}` : ''
  try {
    graphLoading.value = true
    const res = await memoRequest(`/assets/graph${query}`)
    if (res.ok) {
      securityAssetGraph.value = await res.json()
    }
  } catch (e) {
    console.error('Failed to load security asset graph:', e)
  } finally {
    graphLoading.value = false
  }
}

async function loadStandardsChecklist() {
  if (!unlocked.value) return
  const docId =
    selectedDocId.value && selectedDocId.value !== 'new'
      ? selectedDocId.value
      : editingDoc.value.id || ''
  const query = docId ? `?docId=${encodeURIComponent(docId)}` : ''
  try {
    checklistLoading.value = true
    const res = await memoRequest(`/standards/checklist${query}`)
    if (res.ok) {
      standardsChecklist.value = await res.json()
    }
  } catch (e) {
    console.error('Failed to load standards checklist:', e)
  } finally {
    checklistLoading.value = false
  }
}

function syncCaseActionDrafts(cases: SecurityCase[]) {
  const next: Record<string, CaseActionDraft> = {}
  for (const item of cases) {
    const current = caseActionDrafts.value[item.id]
    next[item.id] = {
      owner: current?.owner ?? item.owner ?? '',
      dueAt: current?.dueAt ?? item.dueAt ?? '',
      rationale: current?.rationale ?? item.rationale ?? '',
      acceptedUntil: current?.acceptedUntil ?? item.acceptedUntil ?? '',
      impactScope: current?.impactScope ?? item.impactScope ?? '',
      compensatingControls: current?.compensatingControls ?? item.compensatingControls ?? '',
      reviewer: current?.reviewer ?? item.reviewer ?? '',
    }
  }
  caseActionDrafts.value = next
}

function cleanDraftValue(value: string) {
  const trimmed = value.trim()
  return trimmed ? trimmed : null
}

function isCaseBusy(caseId: string) {
  return Boolean(caseActionLoading.value[caseId])
}

async function updateCaseStatus(caseItem: SecurityCase, status: SecurityCaseStatus) {
  const draft = caseActionDrafts.value[caseItem.id]
  try {
    caseActionLoading.value = { ...caseActionLoading.value, [caseItem.id]: true }
    const res = await memoRequest('/governance/case/status', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        caseId: caseItem.id,
        status,
        owner: cleanDraftValue(draft?.owner || ''),
        dueAt: cleanDraftValue(draft?.dueAt || ''),
        rationale: cleanDraftValue(draft?.rationale || ''),
      }),
    })
    if (res.ok) {
      await loadGovernanceSummary()
    } else {
      await customAlert('更新治理状态失败: ' + await readApiError(res, '更新失败'), '更新失败', 'error')
    }
  } catch (e) {
    await customAlert('更新治理状态出错: ' + e, '更新失败', 'error')
  } finally {
    caseActionLoading.value = { ...caseActionLoading.value, [caseItem.id]: false }
  }
}

async function acceptCase(caseItem: SecurityCase) {
  const draft = caseActionDrafts.value[caseItem.id]
  const rationale = draft?.rationale.trim() || ''
  const acceptedUntil = draft?.acceptedUntil.trim() || ''
  const impactScope = draft?.impactScope.trim() || ''
  const compensatingControls = draft?.compensatingControls.trim() || ''
  const reviewer = draft?.reviewer.trim() || ''
  if (!rationale || !acceptedUntil || !impactScope || !compensatingControls || !reviewer) {
    await customAlert('接受风险需要填写理由、影响范围、补偿控制、有效期和复核人。', '缺少接受依据', 'warning')
    return
  }

  try {
    caseActionLoading.value = { ...caseActionLoading.value, [caseItem.id]: true }
    const res = await memoRequest('/governance/case/accept', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        caseId: caseItem.id,
        rationale,
        acceptedUntil,
        impactScope,
        compensatingControls,
        reviewer,
        owner: cleanDraftValue(draft?.owner || ''),
      }),
    })
    if (res.ok) {
      await loadGovernanceSummary()
    } else {
      await customAlert('接受风险失败: ' + await readApiError(res, '接受失败'), '接受失败', 'error')
    }
  } catch (e) {
    await customAlert('接受风险出错: ' + e, '接受失败', 'error')
  } finally {
    caseActionLoading.value = { ...caseActionLoading.value, [caseItem.id]: false }
  }
}

function parseReportTagsInput() {
  return reportTagsInput.value
    .split(/[,\s，、;；]+/)
    .map((tag) => tag.trim())
    .filter(Boolean)
}

async function generateSecurityReport(scope: SecurityReportScope = 'all', sinceDays?: number) {
  const payload: Record<string, string | number | string[]> = { scope }
  if (sinceDays) {
    payload.sinceDays = sinceDays
  }
  if (scope === 'document') {
    const docId = selectedDocId.value && selectedDocId.value !== 'new' ? selectedDocId.value : editingDoc.value.id
    if (!docId) {
      await customAlert('请先选择一篇文档。', '无法生成文档报告', 'warning')
      return
    }
    payload.docId = docId
  }
  if (scope === 'asset') {
    if (!selectedAssetDetail.value) {
      await customAlert('请先在资产地图中选择一个资产。', '无法生成资产报告', 'warning')
      return
    }
    payload.assetId = selectedAssetDetail.value.asset.id
  }
  if (scope === 'tags') {
    const tags = parseReportTagsInput()
    if (tags.length === 0) {
      await customAlert('请先输入至少一个标签。', '无法生成标签报告', 'warning')
      return
    }
    payload.tags = tags
  }
  try {
    reportLoading.value = true
    const res = await memoRequest('/reports/generate', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(payload),
    })
    if (res.ok) {
      const report: SecurityReport = await res.json()
      await loadGovernanceSummary()
      await customAlert(`已生成脱敏审计报告：${report.fileName}\n${report.path}`, '报告已生成', 'success')
    } else {
      await customAlert('生成报告失败: ' + await readApiError(res, '生成失败'), '生成失败', 'error')
    }
  } catch (e) {
    await customAlert('生成报告出错: ' + e, '生成失败', 'error')
  } finally {
    reportLoading.value = false
  }
}

async function exportSafeShare() {
  const docId = selectedDocId.value && selectedDocId.value !== 'new' ? selectedDocId.value : editingDoc.value.id
  if (!docId) {
    await customAlert('请先选择一篇已保存的文档。', '无法生成安全分享', 'warning')
    return
  }
  try {
    safeShareLoading.value = true
    const res = await memoRequest('/share/export', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        docId,
        markdown: editingDoc.value.markdown || '',
        includeAudit: true,
      }),
    })
    if (res.ok) {
      const share: SafeShareExport = await res.json()
      await loadGovernanceSummary()
      await customAlert(
        `已生成安全分享文件：${share.fileName}\n${share.path}\n已脱敏 ${share.redactedSecretCount} 处，附带 ${share.findingCount} 个审计发现。`,
        '安全分享已生成',
        'success'
      )
    } else {
      await customAlert('生成安全分享失败: ' + await readApiError(res, '生成失败'), '生成失败', 'error')
    }
  } catch (e) {
    await customAlert('生成安全分享出错: ' + e, '生成失败', 'error')
  } finally {
    safeShareLoading.value = false
  }
}

function checklistStatusLabel(status: ChecklistStatus) {
  const labels: Record<ChecklistStatus, string> = {
    open: '待办',
    done: '已完成',
    waived: '不适用',
  }
  return labels[status]
}

function checklistStandardsLabel(item: ChecklistItem) {
  return item.standardIds
    .map((id) => standardsById.value.get(id)?.title || id)
    .slice(0, 2)
    .join(' / ')
}

function isChecklistBusy(itemId: string) {
  return Boolean(checklistActionLoading.value[itemId])
}

async function updateChecklistStatus(item: ChecklistItem, status: ChecklistStatus) {
  const docId =
    standardsChecklist.value?.docId ||
    (selectedDocId.value && selectedDocId.value !== 'new' ? selectedDocId.value : null)
  try {
    checklistActionLoading.value = { ...checklistActionLoading.value, [item.id]: true }
    const res = await memoRequest('/standards/checklist/status', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        docId,
        itemId: item.id,
        status,
        note: item.note || null,
      }),
    })
    if (res.ok) {
      await loadStandardsChecklist()
    } else {
      await customAlert('更新 checklist 失败: ' + await readApiError(res, '更新失败'), '更新失败', 'error')
    }
  } catch (e) {
    await customAlert('更新 checklist 出错: ' + e, '更新失败', 'error')
  } finally {
    checklistActionLoading.value = { ...checklistActionLoading.value, [item.id]: false }
  }
}

function assetTypeLabel(type: SecurityAssetType | string) {
  const labels: Record<string, string> = {
    service: '服务',
    apiEndpoint: '接口',
    url: 'URL',
    secret: 'Secret',
    database: '数据库',
    dependency: '依赖',
    environment: '环境',
    dataType: '数据类型',
  }
  return labels[type] || type
}

function graphNodeTypeLabel(type: SecurityGraphNodeType) {
  const labels: Record<SecurityGraphNodeType, string> = {
    document: '文档',
    asset: '资产',
    finding: '风险',
    secret: 'Secret',
    case: '治理',
  }
  return labels[type] || type
}

function graphEdgeTypeLabel(type: SecurityGraphEdgeType) {
  const labels: Record<SecurityGraphEdgeType, string> = {
    documentAsset: '文档引用资产',
    assetFinding: '资产关联风险',
    assetSecret: '资产使用 Secret',
    findingCase: '风险进入治理',
    assetCase: '资产进入治理',
  }
  return labels[type] || type
}

async function selectSecurityAsset(asset: SecurityAsset) {
  try {
    assetLoading.value = true
    const res = await memoRequest(`/assets/detail?assetId=${encodeURIComponent(asset.id)}`)
    if (res.ok) {
      selectedAssetDetail.value = await res.json()
      await loadSecurityAssetGraph(asset)
    } else {
      await customAlert('加载资产详情失败: ' + await readApiError(res, '加载失败'), '加载失败', 'error')
    }
  } catch (e) {
    await customAlert('加载资产详情出错: ' + e, '加载失败', 'error')
  } finally {
    assetLoading.value = false
  }
}

async function openSecurityAssetById(assetId: string) {
  activeMemoView.value = 'workspace'
  rightPanelTab.value = 'audit'
  const existing = securityAssets.value.find((asset) => asset.id === assetId)
  if (existing) {
    await selectSecurityAsset(existing)
    return
  }
  try {
    assetLoading.value = true
    const res = await memoRequest(`/assets/detail?assetId=${encodeURIComponent(assetId)}`)
    if (res.ok) {
      selectedAssetDetail.value = await res.json()
      await loadSecurityAssetGraph(selectedAssetDetail.value?.asset || null)
    } else {
      await customAlert('加载资产详情失败: ' + await readApiError(res, '加载失败'), '加载失败', 'error')
    }
  } catch (e) {
    await customAlert('加载资产详情出错: ' + e, '加载失败', 'error')
  } finally {
    assetLoading.value = false
  }
}

async function saveSettings() {
  const confirmed = await customConfirm(
    '确定要保存当前设置吗？',
    '保存设置确认',
    'confirm'
  )
  if (!confirmed) {
    return
  }
  try {
    const res = await memoRequest('/settings', {
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
      }),
    })
    if (res.ok) {
      await fetchStatus()
      await customAlert('设置已保存。', '配置保存成功', 'success')
      showSettings.value = false
    } else {
      const text = await readApiError(res, '保存配置失败')
      await customAlert('保存配置失败: ' + text, '配置保存失败', 'error')
    }
  } catch (e) {
    await customAlert('请求失败: ' + e, '连接错误', 'error')
  }
}

function optionalTrimmed(value: string) {
  const trimmed = value.trim()
  return trimmed ? trimmed : null
}

function getWebDavConfigState() {
  const url = webdavUrl.value.trim()
  const user = webdavUser.value.trim()
  const pass = webdavPass.value.trim()
  const filledCount = [url, user, pass].filter(Boolean).length

  return {
    url,
    user,
    pass,
    filledCount,
    complete: filledCount === 3,
    empty: filledCount === 0,
  }
}

async function migrateDataDir() {
  const target = migrationTargetDir.value.trim()
  if (!target) {
    await customAlert('请输入新的资料库目录绝对路径。', '提示', 'warning')
    return
  }
  const confirmed = await customConfirm(
    '迁移会先自动备份当前资料库，然后复制文档、数据库和 KDBX 密码库到新目录。\n\n' +
    '迁移成功后当前保密库会锁定，旧目录会保留。请重启应用后使用新目录。',
    '迁移资料库确认',
    'warning'
  )
  if (!confirmed) {
    return
  }

  try {
    migrationLoading.value = true
    migrationMessage.value = '正在迁移资料库...'
    const res = await memoRequest('/data-dir/migrate', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ targetDir: target }),
    })

    if (res.ok) {
      const data = await res.json()
      migrationMessage.value = data.message || '资料库迁移成功。'
      unlocked.value = false
      documents.value = []
      selectedDocId.value = null
      await fetchDataDirStatus()
      await customAlert(
        `${migrationMessage.value}\n\n备份目录：${data.backupPath || '已创建'}\n新目录：${data.targetDir || target}`,
        '迁移成功',
        'success'
      )
      showSettings.value = false
    } else {
      migrationMessage.value = '迁移失败: ' + await readApiError(res, '迁移失败')
      await customAlert(migrationMessage.value, '迁移失败', 'error')
    }
  } catch (e) {
    migrationMessage.value = '迁移出错: ' + e
    await customAlert(migrationMessage.value, '迁移失败', 'error')
  } finally {
    migrationLoading.value = false
  }
}

async function testConnection() {
  try {
    testingConnection.value = true
    connectionMessage.value = '正在测试模型服务...'
    connectionOk.value = null
    const res = await memoRequest('/test-connection', {
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
  const webdavState = getWebDavConfigState()
  if (!webdavState.empty && !webdavState.complete) {
    await customAlert(
      'WebDAV 云备份需要同时填写 URL、账号和密码。\n\n当前只填写了一部分，系统不会静默跳过云端备份，请补全后再执行。',
      'WebDAV 配置不完整',
      'warning'
    )
    return
  }

  const confirmed = await customConfirm(
    '确定要立即执行安全备份吗？\n\n' +
    (webdavState.complete
      ? '系统将打包本地所有的文档、数据库和 KDBX 密码库，并同步到本地目录及 WebDAV。'
      : '系统将打包本地所有的文档、数据库和 KDBX 密码库；当前未配置 WebDAV，因此不会上传云端。'),
    '安全备份确认',
    'confirm'
  )
  if (!confirmed) {
    return
  }
  try {
    backupLoading.value = true
    backupMessage.value = '备份运行中...'
    const res = await memoRequest('/backup', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        localBackupDir: optionalTrimmed(localBackupDir.value),
        webdavUrl: webdavState.complete ? webdavState.url : null,
        webdavUser: webdavState.complete ? webdavState.user : null,
        webdavPass: webdavState.complete ? webdavState.pass : null,
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
  if (!restorePath.value.trim()) {
    await customAlert('请输入备份 ZIP 压缩包路径！', '提示', 'warning')
    return
  }
  if (restoreConfirmText.value.trim() !== 'RESTORE') {
    await customAlert('请输入 RESTORE 后才能执行还原。', '安全确认未完成', 'warning')
    return
  }
  const confirmed = await customConfirm(
    '还原将覆盖当前的本地文档、数据库和 KDBX 密码库。\n\n系统会先尝试保留当前数据副本，但这仍然是高风险操作。是否确定继续？',
    '数据还原确认',
    'warning'
  )
  if (!confirmed) {
    return
  }
  try {
    restoreLoading.value = true
    const res = await memoRequest('/restore', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ zipPath: restorePath.value.trim() }),
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
    activeMemoView.value = 'workspace'
    const res = await memoRequest(`/doc/${encodeURIComponent(id)}`)
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
      selectedFindingId.value = null
      documentRiskDiff.value = null
      await scanCurrentDocument()
    } else {
      await customAlert('加载文档失败: ' + await readApiError(res, '加载文档失败'), '错误', 'error')
    }
  } catch (e) {
    await customAlert('加载文档出错: ' + e, '错误', 'error')
  }
}

async function openFindingFromArchive(finding: SecurityFinding) {
  activeMemoView.value = 'workspace'
  rightPanelTab.value = 'audit'
  await selectDocument(finding.docId)
  const current = auditFindings.value.find((item) => item.id === finding.id)
  if (current) {
    revealFinding(current)
  }
}

async function openCaseFromArchive(caseItem: SecurityCase) {
  activeMemoView.value = 'workspace'
  rightPanelTab.value = 'audit'
  await selectDocument(caseItem.sourceDocId)
  const finding = caseItem.sourceFindingId
    ? auditFindings.value.find((item) => item.id === caseItem.sourceFindingId)
    : null
  if (finding) {
    revealFinding(finding)
  }
}

function createNewDocumentManual() {
  activeMemoView.value = 'workspace'
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
  auditFindings.value = []
  auditSummary.value = null
  documentRiskDiff.value = null
  selectedFindingId.value = null
  nextTick(() => {
    editorTextareaRef.value?.focus()
  })
  void scanCurrentDocument()
}

async function scanCurrentDocument(options: { silent?: boolean } = {}) {
  if (!unlocked.value || !selectedDocId.value) return
  try {
    auditLoading.value = true
    const res = await memoRequest('/audit/scan', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        docId: editingDoc.value.id || selectedDocId.value || 'new',
        markdown: editingDoc.value.markdown || '',
      }),
    })
    if (res.ok) {
      const data = await res.json()
      auditFindings.value = data.findings || []
      auditSummary.value = data.summary || null
      if (selectedFindingId.value && !auditFindings.value.some((finding) => finding.id === selectedFindingId.value)) {
        selectedFindingId.value = null
      }
      await loadDocumentRiskDiff({ silent: true })
      void loadGovernanceSummary()
    } else if (!options.silent) {
      await customAlert('风险扫描失败: ' + await readApiError(res, '风险扫描失败'), '扫描失败', 'error')
    }
  } catch (e) {
    if (!options.silent) {
      await customAlert('风险扫描出错: ' + e, '扫描失败', 'error')
    }
  } finally {
    auditLoading.value = false
  }
}

async function loadDocumentRiskDiff(options: { silent?: boolean } = {}) {
  if (!unlocked.value || !selectedDocId.value || selectedDocId.value === 'new') {
    documentRiskDiff.value = null
    return
  }
  try {
    diffLoading.value = true
    const res = await memoRequest('/history/doc-diff', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        docId: editingDoc.value.id || selectedDocId.value,
        markdown: editingDoc.value.markdown || '',
      }),
    })
    if (res.ok) {
      documentRiskDiff.value = await res.json()
    } else if (!options.silent) {
      await customAlert('风险变化读取失败: ' + await readApiError(res, '读取失败'), '读取失败', 'error')
    }
  } catch (e) {
    if (!options.silent) {
      await customAlert('风险变化读取出错: ' + e, '读取失败', 'error')
    }
  } finally {
    diffLoading.value = false
  }
}

async function updateFindingStatus(finding: SecurityFinding, status: FindingStatus) {
  try {
    const res = await memoRequest('/audit/finding/status', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        findingId: finding.id,
        status,
      }),
    })
    if (res.ok) {
      await scanCurrentDocument({ silent: true })
    } else {
      await customAlert('更新风险状态失败: ' + await readApiError(res, '更新失败'), '更新失败', 'error')
    }
  } catch (e) {
    await customAlert('更新风险状态出错: ' + e, '更新失败', 'error')
  }
}

function revealFinding(finding: SecurityFinding) {
  selectedFindingId.value = finding.id
  rightPanelTab.value = 'audit'
  nextTick(() => {
    const textarea = editorTextareaRef.value
    if (!textarea) return
    textarea.focus()
    const lineHeight = 20
    textarea.scrollTop = Math.max(0, (finding.lineStart - 2) * lineHeight)
  })
}

async function explainFinding(finding: SecurityFinding) {
  selectedFindingId.value = finding.id
  rightPanelTab.value = 'audit'
  await customAlert(
    `${findingLineLabel(finding)} · ${severityLabel(finding.severity)} · ${finding.title}\n\n` +
      `风险类型：${kindLabel(finding.kind)}\n` +
      `当前状态：${statusLabel(finding.status)}\n\n` +
      `为什么需要关注：\n${finding.detail}\n\n` +
      `证据摘要：\n${finding.evidence}\n\n` +
      `建议处理：\n${finding.recommendation}`,
    '风险解释',
    finding.severity === 'critical' ? 'warning' : 'info'
  )
}

function revealRiskDiffItem(item: DocumentRiskDiffItem & { changeType: 'added' | 'resolved' | 'changed' }) {
  if (item.changeType === 'resolved') return
  const finding = auditFindings.value.find(
    (candidate) =>
      candidate.kind === item.kind &&
      candidate.title === item.title &&
      candidate.evidence === item.evidence &&
      candidate.lineStart === item.lineStart
  )
  if (finding) {
    revealFinding(finding)
    return
  }
  rightPanelTab.value = 'audit'
  nextTick(() => {
    const textarea = editorTextareaRef.value
    if (!textarea) return
    textarea.focus()
    textarea.scrollTop = Math.max(0, (item.lineStart - 2) * 20)
  })
}

async function applyFindingFix(finding: SecurityFinding) {
  try {
    const data = await fetchFindingFixPreview(finding, editingDoc.value.markdown)
    const confirmed = await customConfirm(
      `${data.explanation}\n\n应用后会更新当前编辑区，但不会自动保存。请确认后再点击保存。`,
      '应用修复预览',
      'confirm'
    )
    if (confirmed) {
      editingDoc.value.markdown = data.patchMarkdown
      await updateFindingStatus(finding, 'reviewing')
      await scanCurrentDocument({ silent: true })
    }
  } catch (e) {
    await customAlert('生成修复预览出错: ' + e, '生成失败', 'error')
  }
}

async function fetchFindingFixPreview(finding: SecurityFinding, markdown: string): Promise<AuditFixPreview> {
  const res = await memoRequest('/audit/fix-preview', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      docId: editingDoc.value.id || selectedDocId.value || 'new',
      findingId: finding.id,
      markdown,
    }),
  })
  if (!res.ok) {
    throw new Error(await readApiError(res, '生成失败'))
  }
  return await res.json()
}

async function applyBatchFindingFixes() {
  const candidates = batchFixCandidates.value
  if (candidates.length === 0) {
    await customAlert('当前没有可批量处理的风险。', '无需处理', 'info')
    return
  }

  try {
    batchFixLoading.value = true
    let draftMarkdown = editingDoc.value.markdown
    const previews: Array<{ finding: SecurityFinding; preview: AuditFixPreview }> = []
    const skipped: Array<{ finding: SecurityFinding; reason: string }> = []

    for (const finding of candidates) {
      try {
        const preview = await fetchFindingFixPreview(finding, draftMarkdown)
        if (preview.patchMarkdown && preview.patchMarkdown !== draftMarkdown) {
          previews.push({ finding, preview })
          draftMarkdown = preview.patchMarkdown
        } else {
          skipped.push({ finding, reason: '没有生成新的候选改动' })
        }
      } catch (error) {
        skipped.push({ finding, reason: String(error) })
      }
    }

    if (previews.length === 0) {
      await customAlert('没有生成可应用的批量修复候选。', '批量处理未生成改动', 'warning')
      return
    }

    const affectedLines = previews.map(({ finding }) => findingLineLabel(finding)).join('、')
    const riskTypes = Array.from(new Set(previews.map(({ finding }) => kindLabel(finding.kind)))).join('、')
    const previewList = previews
      .map(({ finding, preview }) => `${findingLineLabel(finding)} · ${severityLabel(finding.severity)} · ${finding.title}\n${preview.explanation}`)
      .join('\n\n')
    const skippedText = skipped.length
      ? `\n\n未生成候选：${skipped
        .map(({ finding, reason }) => `${findingLineLabel(finding)} · ${finding.title}（${reason}）`)
        .join('；')}`
      : ''
    const confirmed = await customConfirm(
      `将应用 ${previews.length} 个批量修复候选。\n\n受影响行：${affectedLines}\n风险类型：${riskTypes}\n\n${previewList}${skippedText}\n\n应用后只更新当前编辑区，不会自动保存；请复核后再点击保存。`,
      '批量应用修复预览',
      'confirm'
    )

    if (!confirmed) return

    editingDoc.value.markdown = draftMarkdown
    for (const { finding } of previews) {
      await updateFindingStatus(finding, 'reviewing')
    }
    await scanCurrentDocument({ silent: true })
  } catch (e) {
    await customAlert('批量处理风险出错: ' + e, '批量处理失败', 'error')
  } finally {
    batchFixLoading.value = false
  }
}

function handleEditorSelection() {
  const textarea = editorTextareaRef.value
  if (!textarea) return
  const start = textarea.selectionStart
  const end = textarea.selectionEnd
  if (end > start) {
    selectedTextRange.value = {
      start,
      end,
      text: editingDoc.value.markdown.slice(start, end),
    }
  } else {
    selectedTextRange.value = null
  }
}

function replaceSelectedText(nextText: string) {
  const range = selectedTextRange.value
  if (!range) return
  editingDoc.value.markdown =
    editingDoc.value.markdown.slice(0, range.start) +
    nextText +
    editingDoc.value.markdown.slice(range.end)
  selectedTextRange.value = null
  nextTick(() => {
    editorTextareaRef.value?.focus()
  })
  void scanCurrentDocument({ silent: true })
}

async function runSelectionAction(action: 'rewrite' | 'summary' | 'redact' | 'explain') {
  const range = selectedTextRange.value
  if (!range || !range.text.trim()) return

  if (action === 'redact') {
    const key = nextSecretKey()
    editorSecretsList.value.push({
      key,
      value: range.text,
      masked: true,
      aiLoading: false,
    })
    replaceSelectedText(`{{secret:${key}}}`)
    await scanCurrentDocument({ silent: true })
    return
  }

  const prompt =
    action === 'rewrite'
      ? `请在不改变事实的前提下改写以下选中文本，只输出改写后的文本：\n\n${range.text}`
      : action === 'summary'
        ? `请总结以下选中文本，输出 3 条以内要点：\n\n${range.text}`
        : `请解释以下选中文本的安全含义、潜在风险和建议：\n\n${range.text}`

  try {
    editorAiLoading.value = true
    const res = await memoRequest('/chat', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ query: prompt }),
    })
    if (res.ok) {
      const data = await res.json()
      if (action === 'rewrite') {
        const confirmed = await customConfirm(
          `是否用 AI 改写结果替换选区？\n\n${data.answer}`,
          '确认替换选区',
          'confirm'
        )
        if (confirmed) {
          replaceSelectedText(data.answer.trim())
          await scanCurrentDocument({ silent: true })
        }
      } else {
        rightPanelTab.value = 'assistant'
        chatMessages.value.push({
          role: 'assistant',
          content: data.answer,
        })
      }
    } else {
      await customAlert('选区 AI 操作失败: ' + await readApiError(res, '操作失败'), '操作失败', 'error')
    }
  } catch (e) {
    await customAlert('选区 AI 操作出错: ' + e, '操作失败', 'error')
  } finally {
    editorAiLoading.value = false
  }
}

function nextSecretKey() {
  let index = editorSecretsList.value.length + 1
  let key = `selectedSecret${index}`
  while (editorSecretsList.value.some((secret) => secret.key === key)) {
    index += 1
    key = `selectedSecret${index}`
  }
  return key
}

function collectUsedSecretKeys(markdown = editingDoc.value.markdown || '') {
  const keys = new Set<string>()
  for (const secret of editorSecretsList.value) {
    const key = secret.key.trim()
    if (key) keys.add(key)
  }
  const marker = /{{secret:([^}]+)}}/g
  let match: RegExpExecArray | null
  while ((match = marker.exec(markdown)) !== null) {
    if (match[1]?.trim()) keys.add(match[1].trim())
  }
  return keys
}

function normalizeDetectedSecretKey(label: string, fallback: string, usedKeys: Set<string>) {
  const lower = label.toLowerCase()
  let base =
    lower.includes('api') || lower.startsWith('sk-') || lower.startsWith('akia') ? 'apiKey'
      : lower.startsWith('ghp_') || lower.startsWith('gho_') || lower.startsWith('github_pat_') ? 'githubToken'
        : lower.startsWith('xoxb-') ? 'slackBotToken'
          : lower.includes('bearer') ? 'bearerToken'
            : lower.includes('jwt') ? 'jwtToken'
              : lower.includes('private') || label.includes('私钥') ? 'privateKey'
                : lower.includes('connection') ? 'connectionCredentials'
                  : lower.includes('password') || lower.includes('passwd') || lower.includes('pwd') || label.includes('密码') || label.includes('口令') ? 'password'
                    : lower.includes('token') || label.includes('令牌') ? 'token'
                      : lower.includes('secret') || label.includes('密钥') ? 'secret'
                        : fallback || 'redactedSecret'
  base = base.replace(/[^A-Za-z0-9_]/g, '') || 'redactedSecret'
  if (/^[0-9]/.test(base)) {
    base = `secret${base}`
  }
  let key = base
  let index = 2
  while (usedKeys.has(key)) {
    key = `${base}${index}`
    index += 1
  }
  usedKeys.add(key)
  return key
}

function prepareDetectedSecretsForEditor(response: RedactMarkdownResponse) {
  const usedKeys = collectUsedSecretKeys()
  let markdown = response.markdown
  const secrets = response.secrets.map((secret) => {
    const key = normalizeDetectedSecretKey(secret.label, secret.key, usedKeys)
    markdown = markdown.split(secret.placeholder).join(`{{secret:${key}}}`)
    return {
      key,
      value: secret.value,
      masked: true,
      aiLoading: false,
    }
  })
  return { markdown, secrets }
}

function emptyDocumentRiskSummary(): DocumentRiskListSummary {
  return {
    total: 0,
    critical: 0,
    warning: 0,
    info: 0,
    reviewing: 0,
  }
}

function docRiskSummary(docId?: string): DocumentRiskListSummary {
  if (!docId) return emptyDocumentRiskSummary()
  return documentRiskSummaryMap.value.get(docId) || emptyDocumentRiskSummary()
}

function docRiskTone(summary: DocumentRiskListSummary) {
  if (summary.critical > 0) return 'critical'
  if (summary.warning > 0) return 'warning'
  if (summary.info > 0 || summary.reviewing > 0) return 'info'
  return 'safe'
}

function docRiskLabel(summary: DocumentRiskListSummary) {
  if (summary.critical > 0) return '高危'
  if (summary.warning > 0) return '警告'
  if (summary.info > 0 || summary.reviewing > 0) return '关注'
  return '安全'
}

function docRiskCountLabel(summary: DocumentRiskListSummary) {
  if (summary.total === 0) return '0'
  const parts = [`${summary.total}`]
  if (summary.critical > 0) parts.push(`高危 ${summary.critical}`)
  else if (summary.warning > 0) parts.push(`警告 ${summary.warning}`)
  if (summary.reviewing > 0) parts.push(`复核 ${summary.reviewing}`)
  return parts.join(' · ')
}

function severityLabel(severity: FindingSeverity) {
  return severity === 'critical' ? '高危' : severity === 'warning' ? '警告' : '提示'
}

function statusLabel(status: FindingStatus) {
  const labels: Record<FindingStatus, string> = {
    open: '待处理',
    fixed: '已修复',
    ignored: '已忽略',
    reviewing: '待复核',
  }
  return labels[status]
}

function caseStatusLabel(status: SecurityCaseStatus) {
  const labels: Record<SecurityCaseStatus, string> = {
    open: '待确认',
    acknowledged: '已确认',
    accepted: '已接受',
    fixing: '修复中',
    fixed: '已修复',
    reviewing: '待复核',
    closed: '已关闭',
    reopened: '已重开',
  }
  return labels[status]
}

function caseTypeLabel(type: SecurityCaseType) {
  const labels: Record<SecurityCaseType, string> = {
    risk: '风险',
    exception: '例外',
    review: '复核',
  }
  return labels[type]
}

function kindLabel(kind: FindingKind) {
  const labels: Record<FindingKind, string> = {
    hardcodedSecret: '密钥治理',
    weakJwt: '认证机制',
    insecureLink: '传输安全',
    sensitiveOperation: '安全开关',
    governanceGap: '治理例外',
  }
  return labels[kind]
}

function formatShortTime(timestamp?: number) {
  if (!timestamp) return '暂无'
  return new Date(timestamp * 1000).toLocaleString('zh-CN', {
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  })
}

function findingLineLabel(finding: SecurityFinding) {
  return finding.lineStart === finding.lineEnd
    ? `L${finding.lineStart}`
    : `L${finding.lineStart}-L${finding.lineEnd}`
}

function riskDiffItemLineLabel(item: DocumentRiskDiffItem) {
  const current = item.lineStart === item.lineEnd ? `L${item.lineStart}` : `L${item.lineStart}-L${item.lineEnd}`
  if (!item.previousLineStart || item.previousLineStart === item.lineStart) {
    return current
  }
  return `L${item.previousLineStart} -> ${current.slice(1)}`
}

function riskDiffTypeLabel(type: 'added' | 'resolved' | 'changed') {
  const labels = {
    added: '新增',
    resolved: '已修复',
    changed: '变化',
  }
  return labels[type]
}

function riskDiffSummaryText(diff: DocumentRiskDiff | null) {
  if (!diff) return '暂无风险变化'
  const summary = diff.summary
  if (!diff.previousSavedAt) {
    return `已建立风险基线，当前 ${summary.currentTotal} 个风险。`
  }
  return `新增 ${summary.added}，修复 ${summary.resolved}，移动 ${summary.moved}，等级变化 ${summary.severityChanged}。`
}

function lineRiskClass(line: number) {
  const findings = riskLineMap.value.get(line) || []
  if (findings.some((finding) => finding.severity === 'critical')) return 'editor-line-no--critical'
  if (findings.some((finding) => finding.severity === 'warning')) return 'editor-line-no--warning'
  if (findings.length > 0) return 'editor-line-no--info'
  return ''
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
  void scanCurrentDocument({ silent: true })
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
    const res = await memoRequest('/translate-key', {
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
    const res = await memoRequest('/save', {
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
      const savedRiskDiff = (meta.riskDiff || null) as DocumentRiskDiff | null
      await customAlert(`文档已成功保存！${riskDiffSummaryText(savedRiskDiff)}`, '保存成功', 'success')
      selectedDocId.value = meta.id
      await loadDocuments()
      await selectDocument(meta.id)
      documentRiskDiff.value = savedRiskDiff
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
    const res = await memoRequest('/delete', {
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
      const res = await memoRequest('/draft', {
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
      const res = await memoRequest('/draft', {
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
      const res = await memoRequest('/query', {
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
      const res = await memoRequest('/chat', {
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

async function redactCurrentDocument() {
  const source = editingDoc.value.markdown || ''
  if (!source.trim()) {
    await customAlert('当前文档内容为空，无法脱敏。', '无需脱敏', 'info')
    return
  }

  try {
    documentRedactLoading.value = true
    const res = await memoRequest('/audit/redact', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ markdown: source }),
    })
    if (!res.ok) {
      await customAlert('一键脱敏失败: ' + await readApiError(res, '脱敏失败'), '脱敏失败', 'error')
      return
    }
    const data: RedactMarkdownResponse = await res.json()
    if (data.redactedSecretCount === 0) {
      await customAlert('当前文档没有识别到新的明文 secret。', '无需脱敏', 'info')
      return
    }
    const prepared = prepareDetectedSecretsForEditor(data)
    const secretKeys = prepared.secrets.map((secret) => secret.key).join('、')
    const confirmed = await customConfirm(
      `将替换 ${data.redactedSecretCount} 处疑似明文 secret，并把真实值加入当前文档密码箱。\n\n新增密码箱 key：${secretKeys}\n\n应用后只更新当前编辑区和密码箱草稿，不会自动保存；请复核后再点击保存。`,
      '应用一键脱敏',
      'confirm'
    )
    if (!confirmed) return

    editingDoc.value.markdown = prepared.markdown
    editorSecretsList.value.push(...prepared.secrets)
    showEditorSecrets.value = true
    await scanCurrentDocument({ silent: true })
  } catch (e) {
    await customAlert('一键脱敏出错: ' + e, '脱敏失败', 'error')
  } finally {
    documentRedactLoading.value = false
  }
}

async function extractDocumentTodos() {
  const source = editingDoc.value.markdown?.trim()
  if (!source) {
    await customAlert('当前文档内容为空，无法提取待办。', '无需提取', 'info')
    return
  }

  try {
    editorAiLoading.value = true
    const res = await memoRequest('/chat', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        query: `请从以下安全文档中提取可执行的安全治理待办。只输出 Markdown 任务列表，每条以 "- [ ]" 开头，最多 8 条，包含负责人/复核/截止日占位建议；不要输出 secret 明文。\n\n${source}`,
      }),
    })
    if (!res.ok) {
      await customAlert('提取待办失败: ' + await readApiError(res, '提取失败'), '提取失败', 'error')
      return
    }
    const data: SearchAnswerResponse = await res.json()
    const todos = (data.answer || '').trim()
    if (!todos) {
      await customAlert('AI 没有提取出可用待办。', '无待办', 'info')
      return
    }
    chatMessages.value.push({
      role: 'assistant',
      content: todos,
      sources: data.sources,
    })
    const confirmed = await customConfirm(
      `${todos}\n\n是否将这些待办追加到当前文档末尾？`,
      '提取安全待办',
      'confirm'
    )
    if (confirmed) {
      const heading = '\n\n## 安全治理待办\n\n'
      editingDoc.value.markdown = `${editingDoc.value.markdown.trimEnd()}${heading}${todos}\n`
      await scanCurrentDocument({ silent: true })
    }
  } catch (e) {
    await customAlert('提取待办出错: ' + e, '提取失败', 'error')
  } finally {
    editorAiLoading.value = false
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
    const res = await memoRequest('/draft', {
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

watch(showSettings, (visible) => {
  if (visible) {
    settingsTab.value = 'ai'
    restoreConfirmText.value = ''
    void fetchDataDirStatus()
  }
})

onMounted(() => {
  void fetchStatus()
})
</script>

<template>
  <ToolShell
    title="AI 安全文档"
    description="本地 Markdown 文档库。支持 AI 辅助整理、本地脱敏、安全字段加密以及文档检索问答。"
    :breadcrumbs="[
      { label: '工具箱', to: '/toolbox' },
      { label: 'AI 安全文档' },
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
          <SecurePasswordInput
            v-model="masterPassword"
            input-class="lock-input"
            autocomplete="current-password"
            placeholder="请输入主密码"
            autofocus
            show-title="显示主密码"
            hide-title="隐藏主密码"
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
                    <div class="doc-item-heading">
                      <h4 class="text-xs text-gray-200 truncate" :class="node.type === 'folder' ? 'font-bold text-gray-300' : 'font-medium'">
                        {{ node.name }}
                      </h4>
                      <div v-if="node.type === 'file'" class="doc-risk-meta">
                        <span
                          class="doc-risk-pill"
                          :class="`doc-risk-pill--${docRiskTone(docRiskSummary(node.doc.id))}`"
                        >
                          {{ docRiskLabel(docRiskSummary(node.doc.id)) }}
                        </span>
                        <span class="doc-risk-count">{{ docRiskCountLabel(docRiskSummary(node.doc.id)) }}</span>
                      </div>
                    </div>
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
            <div class="workspace-view-tabs" role="tablist" aria-label="AI 安全文档视图">
              <button
                type="button"
                class="workspace-view-tab"
                :class="{ 'workspace-view-tab--active': activeMemoView === 'archive' }"
                @click="activeMemoView = 'archive'"
              >
                安全档案
              </button>
              <button
                type="button"
                class="workspace-view-tab"
                :class="{ 'workspace-view-tab--active': activeMemoView === 'workspace' }"
                @click="activeMemoView = 'workspace'"
              >
                文档工作台
              </button>
            </div>
            <div class="min-w-0 flex-1">
              <div class="text-xs font-bold text-gray-200 truncate">
                {{ activeMemoView === 'archive' ? '团队安全治理档案' : (selectedDocId ? editingDoc.title : '选择或新建文档后开始协作') }}
              </div>
              <div class="text-[10px] text-gray-600 font-mono truncate">
                {{ activeMemoView === 'archive' ? '本地态势 · 风险队列 · 资产地图 · 审计活动' : (selectedDocId ? editingDoc.fileName : 'AI 聊天、Markdown 编辑和实时预览会同时显示') }}
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
                v-if="selectedDocId && selectedDocId !== 'new'"
                @click="exportSafeShare"
                :disabled="safeShareLoading"
                type="button"
                class="toolbar-secondary-btn"
              >
                <RefreshCw v-if="safeShareLoading" class="h-3.5 w-3.5 animate-spin" />
                <FileText v-else class="h-3.5 w-3.5" />
                安全分享
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

          <div class="governance-strip">
            <div class="governance-metric">
              <span class="metric-label">未关闭风险</span>
              <strong>{{ governanceRiskSummary?.open ?? 0 }}</strong>
              <span class="metric-foot">高危 {{ governanceRiskSummary?.critical ?? 0 }}</span>
            </div>
            <div class="governance-metric">
              <span class="metric-label">待复核</span>
              <strong>{{ governanceRiskSummary?.reviewing ?? 0 }}</strong>
              <span class="metric-foot">到期 {{ governanceRiskSummary?.expiredAcceptances ?? 0 }} · 14天内 {{ governanceRiskSummary?.expiringSoon ?? 0 }}</span>
            </div>
            <div class="governance-metric">
              <span class="metric-label">安全资产</span>
              <strong>{{ governanceAssetSummary?.total ?? 0 }}</strong>
              <span class="metric-foot">接口 {{ governanceAssetSummary?.apiEndpoints ?? 0 }} · 数据库 {{ governanceAssetSummary?.databases ?? 0 }}</span>
            </div>
            <div class="governance-metric governance-metric--wide">
              <span class="metric-label">最近活动</span>
              <strong>{{ governanceSummary?.recentActivities?.[0]?.title || '暂无审计活动' }}</strong>
              <span class="metric-foot">{{ governanceSummary?.recentActivities?.[0]?.summary || '打开文档后会生成本地审计记录' }}</span>
            </div>
          </div>

          <div v-if="activeMemoView === 'archive'" class="security-home">
            <section class="archive-hero">
              <div class="archive-hero-main">
                <span>安全态势</span>
                <strong>{{ governanceRiskSummary?.open ?? 0 }} 个未关闭风险</strong>
                <p>高危 {{ governanceRiskSummary?.critical ?? 0 }} · 警告 {{ governanceRiskSummary?.warning ?? 0 }} · 待复核 {{ governanceRiskSummary?.reviewing ?? 0 }}</p>
              </div>
              <div class="archive-hero-actions">
                <button type="button" @click="activeMemoView = 'workspace'; rightPanelTab = 'audit'">
                  查看治理队列
                </button>
                <button type="button" @click="generateSecurityReport('all', 30)" :disabled="reportLoading">
                  近30天报告
                </button>
              </div>
            </section>

            <section class="archive-metric-grid">
              <button type="button" class="archive-metric-card archive-metric-card--danger" @click="activeMemoView = 'workspace'; rightPanelTab = 'audit'">
                <span>高危风险</span>
                <strong>{{ governanceRiskSummary?.critical ?? 0 }}</strong>
                <em>未关闭 {{ governanceRiskSummary?.open ?? 0 }}</em>
              </button>
              <button type="button" class="archive-metric-card archive-metric-card--warn" @click="activeMemoView = 'workspace'; rightPanelTab = 'audit'">
                <span>待复核</span>
                <strong>{{ governanceRiskSummary?.reviewing ?? 0 }}</strong>
                <em>例外到期 {{ governanceRiskSummary?.expiredAcceptances ?? 0 }}</em>
              </button>
              <button type="button" class="archive-metric-card" @click="activeMemoView = 'workspace'; rightPanelTab = 'audit'">
                <span>安全资产</span>
                <strong>{{ governanceAssetSummary?.total ?? 0 }}</strong>
                <em>数据库 {{ governanceAssetSummary?.databases ?? 0 }} · 依赖 {{ governanceAssetSummary?.dependencies ?? 0 }}</em>
              </button>
              <button type="button" class="archive-metric-card" @click="createNewDocumentManual">
                <span>本地文档</span>
                <strong>{{ documents.length }}</strong>
                <em>新增安全档案</em>
              </button>
            </section>

            <section class="archive-board">
              <article class="archive-panel archive-panel--wide">
                <header>
                  <div>
                    <span>风险治理</span>
                    <strong>{{ dashboardOpenCases.length }} 项待处理</strong>
                  </div>
                  <button type="button" @click="activeMemoView = 'workspace'; rightPanelTab = 'audit'">队列</button>
                </header>
                <div v-if="dashboardOpenCases.length === 0" class="archive-empty">
                  当前没有未关闭风险。
                </div>
                <button
                  v-for="caseItem in dashboardOpenCases"
                  v-else
                  :key="caseItem.id"
                  type="button"
                  class="archive-risk-row"
                  :class="`archive-risk-row--${caseItem.severity}`"
                  @click="openCaseFromArchive(caseItem)"
                >
                  <span>{{ severityLabel(caseItem.severity) }} · {{ caseStatusLabel(caseItem.status) }}</span>
                  <strong>{{ caseItem.title }}</strong>
                  <em>{{ caseItem.owner || '未分配' }} · {{ caseItem.dueAt || '未设截止' }}</em>
                </button>
              </article>

              <article class="archive-panel">
                <header>
                  <div>
                    <span>最近新增风险</span>
                    <strong>{{ dashboardRecentFindings.length }} 条</strong>
                  </div>
                </header>
                <div v-if="dashboardRecentFindings.length === 0" class="archive-empty">
                  暂无近期风险。
                </div>
                <button
                  v-for="finding in dashboardRecentFindings"
                  v-else
                  :key="finding.id"
                  type="button"
                  class="archive-finding-row"
                  @click="openFindingFromArchive(finding)"
                >
                  <span>L{{ finding.lineStart }} · {{ severityLabel(finding.severity) }}</span>
                  <strong>{{ finding.title }}</strong>
                </button>
              </article>

              <article class="archive-panel">
                <header>
                  <div>
                    <span>复核队列</span>
                    <strong>{{ dashboardReviewCases.length }} 项</strong>
                  </div>
                </header>
                <div v-if="dashboardReviewCases.length === 0" class="archive-empty">
                  暂无复核项。
                </div>
                <button
                  v-for="caseItem in dashboardReviewCases"
                  v-else
                  :key="caseItem.id"
                  type="button"
                  class="archive-finding-row"
                  @click="openCaseFromArchive(caseItem)"
                >
                  <span>{{ caseItem.reviewer || '未指定复核人' }}</span>
                  <strong>{{ caseItem.title }}</strong>
                </button>
              </article>

              <article class="archive-panel">
                <header>
                  <div>
                    <span>资产概览</span>
                    <strong>{{ dashboardRecentAssets.length }} 个近期资产</strong>
                  </div>
                </header>
                <div v-if="dashboardRecentAssets.length === 0" class="archive-empty">
                  暂无资产索引。
                </div>
                <div v-else class="archive-asset-list">
                  <button
                    v-for="asset in dashboardRecentAssets"
                    :key="asset.id"
                    type="button"
                    @click="openSecurityAssetById(asset.id)"
                  >
                    <span>{{ assetTypeLabel(asset.assetType) }}</span>
                    <strong>{{ asset.name }}</strong>
                  </button>
                </div>
              </article>

              <article class="archive-panel">
                <header>
                  <div>
                    <span>最近文档</span>
                    <strong>{{ dashboardRecentDocs.length }} 篇</strong>
                  </div>
                  <button type="button" @click="createNewDocumentManual">新建</button>
                </header>
                <div v-if="dashboardRecentDocs.length === 0" class="archive-empty">
                  暂无本地文档。
                </div>
                <button
                  v-for="doc in dashboardRecentDocs"
                  v-else
                  :key="doc.id"
                  type="button"
                  class="archive-doc-row"
                  @click="selectDocument(doc.id)"
                >
                  <strong>{{ doc.title }}</strong>
                  <span>{{ doc.fileName }}</span>
                </button>
              </article>

              <article class="archive-panel archive-panel--wide">
                <header>
                  <div>
                    <span>最近活动</span>
                    <strong>{{ dashboardRecentActivities.length }} 条事件</strong>
                  </div>
                </header>
                <div v-if="dashboardRecentActivities.length === 0" class="archive-empty">
                  暂无审计活动。
                </div>
                <div v-else class="archive-activity-list">
                  <div v-for="activity in dashboardRecentActivities" :key="activity.id">
                    <span>{{ formatShortTime(activity.createdAt) }}</span>
                    <strong>{{ activity.title }}</strong>
                    <p>{{ activity.summary }}</p>
                  </div>
                </div>
              </article>
            </section>
          </div>

          <div v-else class="security-workspace">
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
                  <div class="editor-section-title">
                    <div class="flex items-center gap-1.5">
                      <PenTool class="h-3.5 w-3.5 text-emerald-400" />
                      <span>安全文档编辑</span>
                    </div>
                    <div class="flex items-center gap-2">
                      <span
                        v-if="auditSummary"
                        class="audit-mini-pill"
                        :class="{ 'audit-mini-pill--danger': auditSummary.critical > 0, 'audit-mini-pill--warn': auditSummary.critical === 0 && auditSummary.warning > 0 }"
                      >
                        {{ auditSummary.total }} 个发现
                      </span>
                      <button @click="scanCurrentDocument()" class="editor-ai-btn" type="button">
                        <RefreshCw v-if="auditLoading" class="h-3.5 w-3.5 animate-spin" />
                        <AlertTriangle v-else class="h-3.5 w-3.5" />
                        检查风险
                      </button>
                    </div>
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
                    <div class="selection-toolbar" v-if="selectedTextRange">
                      <button @click="runSelectionAction('rewrite')" type="button">AI 改写</button>
                      <button @click="runSelectionAction('summary')" type="button">总结</button>
                      <button @click="runSelectionAction('redact')" type="button">脱敏</button>
                      <button @click="runSelectionAction('explain')" type="button">解释</button>
                    </div>
                    <div class="editor-shell">
                      <div class="line-gutter">
                        <button
                          v-for="line in editorLineNumbers"
                          :key="line"
                          type="button"
                          class="editor-line-no"
                          :class="lineRiskClass(line)"
                        >
                          {{ line }}
                        </button>
                      </div>
                      <textarea
                        ref="editorTextareaRef"
                        v-model="editingDoc.markdown"
                        class="editor-textarea"
                        placeholder="在这里输入 Markdown 文档..."
                        @select="handleEditorSelection"
                        @keyup="handleEditorSelection"
                        @mouseup="handleEditorSelection"
                        @blur="handleEditorSelection"
                      ></textarea>
                    </div>
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

            <section class="intelligence-sidebar">
              <div class="right-tabs">
                <button
                  class="right-tab"
                  :class="{ 'right-tab--active': rightPanelTab === 'assistant' }"
                  @click="rightPanelTab = 'assistant'"
                  type="button"
                >
                  AI 助手
                </button>
                <button
                  class="right-tab"
                  :class="{ 'right-tab--active': rightPanelTab === 'audit' }"
                  @click="rightPanelTab = 'audit'"
                  type="button"
                >
                  安全审计
                  <span v-if="activeFindings.length" class="tab-count">{{ activeFindings.length }}</span>
                </button>
                <button
                  class="right-tab"
                  :class="{ 'right-tab--active': rightPanelTab === 'preview' }"
                  @click="rightPanelTab = 'preview'"
                  type="button"
                >
                  预览
                </button>
              </div>

              <div v-if="rightPanelTab === 'assistant'" class="right-panel-body">
                <section class="assistant-summary">
                  <h4>文档摘要</h4>
                  <p>{{ selectedDocId ? (editingDoc.summary || '当前文档暂无摘要') : '选择文档后显示摘要' }}</p>
                </section>

                <section class="assistant-tasks">
                  <h4>建议任务</h4>
                  <button @click="runEditorAi('organize')" :disabled="editorAiLoading" type="button">整理格式与层级</button>
                  <button @click="runEditorAi('summary')" :disabled="editorAiLoading" type="button">生成摘要</button>
                  <button @click="extractDocumentTodos()" :disabled="editorAiLoading" type="button">
                    <RefreshCw v-if="editorAiLoading" class="h-3.5 w-3.5 animate-spin" />
                    提取待办
                  </button>
                  <button @click="redactCurrentDocument()" :disabled="documentRedactLoading" type="button">
                    <RefreshCw v-if="documentRedactLoading" class="h-3.5 w-3.5 animate-spin" />
                    一键脱敏
                  </button>
                  <button @click="scanCurrentDocument()" :disabled="auditLoading" type="button">检查风险</button>
                </section>

                <div class="messages-area messages-area--compact">
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
                      placeholder="问 AI，或让它处理当前文档..."
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
              </div>

              <div v-else-if="rightPanelTab === 'audit'" class="right-panel-body">
                <div class="audit-summary-grid">
                  <div>
                    <span>高危</span>
                    <strong>{{ auditSummary?.critical ?? 0 }}</strong>
                  </div>
                  <div>
                    <span>警告</span>
                    <strong>{{ auditSummary?.warning ?? 0 }}</strong>
                  </div>
                  <div>
                    <span>待复核</span>
                    <strong>{{ auditSummary?.reviewing ?? 0 }}</strong>
                  </div>
                </div>

                <div class="audit-action-row">
                  <button @click="scanCurrentDocument()" class="audit-scan-btn" type="button">
                    <RefreshCw v-if="auditLoading" class="h-3.5 w-3.5 animate-spin" />
                    <AlertTriangle v-else class="h-3.5 w-3.5" />
                    重新扫描当前文档
                  </button>
                  <button
                    @click="applyBatchFindingFixes()"
                    :disabled="batchFixLoading || auditLoading || batchFixCandidates.length === 0"
                    class="audit-batch-btn"
                    type="button"
                  >
                    <RefreshCw v-if="batchFixLoading" class="h-3.5 w-3.5 animate-spin" />
                    <Sparkles v-else class="h-3.5 w-3.5" />
                    批量处理 {{ batchFixCandidates.length }}
                  </button>
                </div>

                <section class="risk-diff-panel" :class="{ 'risk-diff-panel--active': hasRiskDiffChanges }">
                  <header class="risk-diff-header">
                    <div>
                      <span>版本风险变化</span>
                      <strong>{{ riskDiffSummaryText(documentRiskDiff) }}</strong>
                    </div>
                    <RefreshCw v-if="diffLoading" class="h-3.5 w-3.5 animate-spin text-emerald-300" />
                  </header>
                  <div v-if="!documentRiskDiff" class="risk-diff-empty">
                    保存一次文档后建立风险基线。
                  </div>
                  <div v-else class="risk-diff-metrics">
                    <div>
                      <span>新增</span>
                      <strong>{{ documentRiskDiff.summary.added }}</strong>
                    </div>
                    <div>
                      <span>修复</span>
                      <strong>{{ documentRiskDiff.summary.resolved }}</strong>
                    </div>
                    <div>
                      <span>移动</span>
                      <strong>{{ documentRiskDiff.summary.moved }}</strong>
                    </div>
                  </div>
                  <div v-if="visibleRiskDiffItems.length > 0" class="risk-diff-list">
                    <button
                      v-for="item in visibleRiskDiffItems"
                      :key="`${item.changeType}-${item.fingerprint}`"
                      type="button"
                      class="risk-diff-item"
                      :class="`risk-diff-item--${item.changeType}`"
                      @click="revealRiskDiffItem(item)"
                    >
                      <span>{{ riskDiffTypeLabel(item.changeType) }} · {{ riskDiffItemLineLabel(item) }}</span>
                      <strong>{{ item.title }}</strong>
                    </button>
                  </div>
                </section>

                <section class="case-queue">
                  <header class="case-queue-header">
                    <div>
                      <span>治理队列</span>
                      <strong>{{ governanceCases.length }} 个案件</strong>
                    </div>
                    <div class="case-report-actions">
                      <button
                        type="button"
                        class="case-report-btn"
                        :disabled="reportLoading"
                        @click="generateSecurityReport('all')"
                      >
                        <RefreshCw v-if="reportLoading || governanceLoading" class="h-3.5 w-3.5 animate-spin" />
                        <FileText v-else class="h-3.5 w-3.5" />
                        全局
                      </button>
                      <button
                        type="button"
                        class="case-report-btn"
                        :disabled="reportLoading || !selectedDocId || selectedDocId === 'new'"
                        @click="generateSecurityReport('document')"
                      >
                        文档
                      </button>
                      <button
                        type="button"
                        class="case-report-btn"
                        :disabled="reportLoading || !selectedAssetDetail"
                        @click="generateSecurityReport('asset')"
                      >
                        资产
                      </button>
                      <button
                        type="button"
                        class="case-report-btn"
                        :disabled="reportLoading"
                        @click="generateSecurityReport('all', 30)"
                      >
                        近30天
                      </button>
                      <input
                        v-model="reportTagsInput"
                        class="case-report-tag-input"
                        placeholder="标签"
                        @keyup.enter="generateSecurityReport('tags')"
                      />
                      <button
                        type="button"
                        class="case-report-btn"
                        :disabled="reportLoading || parseReportTagsInput().length === 0"
                        @click="generateSecurityReport('tags')"
                      >
                        标签
                      </button>
                    </div>
                  </header>

                  <div v-if="visibleGovernanceCases.length === 0" class="case-empty">
                    暂无风险案件
                  </div>
                  <article
                    v-for="caseItem in visibleGovernanceCases"
                    v-else
                    :key="caseItem.id"
                    class="case-card"
                    :class="[`case-card--${caseItem.severity}`, `case-card--status-${caseItem.status}`]"
                  >
                    <header>
                      <div>
                        <span>{{ caseTypeLabel(caseItem.caseType) }} · {{ severityLabel(caseItem.severity) }}</span>
                        <strong>{{ caseItem.title }}</strong>
                      </div>
                      <span class="case-status">{{ caseStatusLabel(caseItem.status) }}</span>
                    </header>

                    <div class="case-meta-line">
                      <span>{{ caseItem.sourceFindingId || caseItem.id }}</span>
                      <span>{{ formatShortTime(caseItem.updatedAt) }}</span>
                    </div>

                    <div class="case-fields">
                      <input
                        v-model="caseActionDrafts[caseItem.id].owner"
                        type="text"
                        placeholder="责任人"
                      />
                      <input
                        v-model="caseActionDrafts[caseItem.id].dueAt"
                        type="date"
                        aria-label="截止日"
                      />
                    </div>
                    <textarea
                      v-model="caseActionDrafts[caseItem.id].rationale"
                      rows="2"
                      placeholder="处置备注 / 接受理由"
                    ></textarea>
                    <div class="case-fields">
                      <input
                        v-model="caseActionDrafts[caseItem.id].impactScope"
                        type="text"
                        placeholder="影响范围"
                      />
                      <input
                        v-model="caseActionDrafts[caseItem.id].reviewer"
                        type="text"
                        placeholder="复核人"
                      />
                    </div>
                    <textarea
                      v-model="caseActionDrafts[caseItem.id].compensatingControls"
                      rows="2"
                      placeholder="补偿控制"
                    ></textarea>
                    <div class="case-fields">
                      <input
                        v-model="caseActionDrafts[caseItem.id].acceptedUntil"
                        type="date"
                        aria-label="接受有效期"
                      />
                      <button
                        type="button"
                        class="case-accept-btn"
                        :disabled="isCaseBusy(caseItem.id)"
                        @click="acceptCase(caseItem)"
                      >
                        接受
                      </button>
                    </div>

                    <div v-if="caseItem.acceptedUntil || caseItem.impactScope || caseItem.compensatingControls || caseItem.reviewer" class="case-exception-detail">
                      <span v-if="caseItem.acceptedUntil">有效期 {{ caseItem.acceptedUntil }}</span>
                      <span v-if="caseItem.impactScope">影响 {{ caseItem.impactScope }}</span>
                      <span v-if="caseItem.compensatingControls">补偿 {{ caseItem.compensatingControls }}</span>
                      <span v-if="caseItem.reviewer">复核 {{ caseItem.reviewer }}</span>
                    </div>

                    <footer>
                      <button :disabled="isCaseBusy(caseItem.id)" @click="updateCaseStatus(caseItem, 'acknowledged')" type="button">确认</button>
                      <button :disabled="isCaseBusy(caseItem.id)" @click="updateCaseStatus(caseItem, 'fixing')" type="button">处理中</button>
                      <button :disabled="isCaseBusy(caseItem.id)" @click="updateCaseStatus(caseItem, 'reviewing')" type="button">复核</button>
                      <button :disabled="isCaseBusy(caseItem.id)" @click="updateCaseStatus(caseItem, 'closed')" type="button">关闭</button>
                    </footer>
                  </article>

                  <div v-if="visibleGovernanceEvents.length > 0" class="case-event-list">
                    <div v-for="event in visibleGovernanceEvents" :key="event.id">
                      <span>{{ formatShortTime(event.createdAt) }}</span>
                      <strong>{{ event.summary }}</strong>
                    </div>
                  </div>
                </section>

                <section class="checklist-panel">
                  <header class="checklist-header">
                    <div>
                      <span>安全 checklist</span>
                      <strong>{{ visibleChecklistItems.length }} 项</strong>
                    </div>
                    <RefreshCw v-if="checklistLoading" class="h-3.5 w-3.5 animate-spin text-emerald-300" />
                  </header>

                  <div v-if="visibleChecklistItems.length === 0" class="checklist-empty">
                    暂无推荐 checklist
                  </div>
                  <article
                    v-for="item in visibleChecklistItems"
                    v-else
                    :key="item.id"
                    class="checklist-card"
                    :class="{ 'checklist-card--recommended': item.recommended, 'checklist-card--done': item.status === 'done', 'checklist-card--waived': item.status === 'waived' }"
                  >
                    <header>
                      <div>
                        <span>{{ item.recommended ? '推荐复核' : '常规项' }}</span>
                        <strong>{{ item.title }}</strong>
                      </div>
                      <em>{{ checklistStatusLabel(item.status) }}</em>
                    </header>
                    <p>{{ item.description }}</p>
                    <div class="checklist-standard">
                      {{ checklistStandardsLabel(item) }}
                    </div>
                    <div v-if="item.evidence.length > 0" class="checklist-evidence">
                      <span v-for="evidence in item.evidence" :key="evidence">{{ evidence }}</span>
                    </div>
                    <input
                      v-model="item.note"
                      type="text"
                      placeholder="复核备注"
                    />
                    <footer>
                      <button :disabled="isChecklistBusy(item.id)" @click="updateChecklistStatus(item, 'done')" type="button">完成</button>
                      <button :disabled="isChecklistBusy(item.id)" @click="updateChecklistStatus(item, 'waived')" type="button">不适用</button>
                      <button :disabled="isChecklistBusy(item.id)" @click="updateChecklistStatus(item, 'open')" type="button">重开</button>
                    </footer>
                  </article>
                </section>

                <section class="asset-panel">
                  <header class="asset-header">
                    <div>
                      <span>资产地图</span>
                      <strong>{{ securityAssets.length }} 个资产</strong>
                    </div>
                    <RefreshCw v-if="assetLoading" class="h-3.5 w-3.5 animate-spin text-emerald-300" />
                  </header>

                  <div v-if="visibleSecurityAssets.length === 0" class="asset-empty">
                    暂无资产索引
                  </div>
                  <div v-else class="asset-list">
                    <button
                      v-for="asset in visibleSecurityAssets"
                      :key="asset.id"
                      type="button"
                      class="asset-chip"
                      :class="{ 'asset-chip--active': selectedAssetDetail?.asset.id === asset.id }"
                      @click="selectSecurityAsset(asset)"
                    >
                      <span>{{ assetTypeLabel(asset.assetType) }}</span>
                      <strong>{{ asset.name }}</strong>
                    </button>
                  </div>

                  <article v-if="selectedAssetDetail" class="asset-detail">
                    <header>
                      <span>{{ assetTypeLabel(selectedAssetDetail.asset.assetType) }}</span>
                      <strong>{{ selectedAssetDetail.asset.name }}</strong>
                    </header>
                    <div class="asset-detail-grid">
                      <div>
                        <span>文档</span>
                        <strong>{{ selectedAssetDetail.documents.length }}</strong>
                      </div>
                      <div>
                        <span>风险</span>
                        <strong>{{ selectedAssetDetail.findings.length }}</strong>
                      </div>
                      <div>
                        <span>治理项</span>
                        <strong>{{ selectedAssetDetail.cases.length }}</strong>
                      </div>
                    </div>
                    <div v-if="selectedAssetDetail.documents.length > 0" class="asset-related-list">
                      <button
                        v-for="doc in selectedAssetDetail.documents"
                        :key="doc.id"
                        type="button"
                        @click="selectDocument(doc.id)"
                      >
                        {{ doc.title }}
                      </button>
                    </div>
                    <div v-if="selectedAssetDetail.findings.length > 0" class="asset-finding-list">
                      <span v-for="finding in selectedAssetDetail.findings.slice(0, 3)" :key="finding.id">
                        L{{ finding.lineStart }} · {{ finding.title }}
                      </span>
                    </div>
                  </article>

                  <article v-if="securityAssetGraph" class="asset-graph">
                    <header>
                      <div>
                        <span>轻量关系图</span>
                        <strong>{{ securityAssetGraph.nodes.length }} 节点 · {{ securityAssetGraph.edges.length }} 关系</strong>
                      </div>
                      <RefreshCw v-if="graphLoading" class="h-3.5 w-3.5 animate-spin text-emerald-300" />
                    </header>
                    <div v-if="visibleGraphNodes.length === 0" class="asset-empty">
                      暂无关系图
                    </div>
                    <div v-else class="graph-node-list">
                      <span
                        v-for="node in visibleGraphNodes"
                        :key="node.id"
                        class="graph-node-chip"
                        :class="[`graph-node-chip--${node.nodeType}`, node.severity ? `graph-node-chip--${node.severity}` : '']"
                      >
                        <em>{{ graphNodeTypeLabel(node.nodeType) }}</em>
                        <strong>{{ node.label }}</strong>
                      </span>
                    </div>
                    <div v-if="visibleGraphEdges.length > 0" class="graph-edge-list">
                      <div v-for="edge in visibleGraphEdges" :key="edge.id">
                        <span>{{ graphEdgeTypeLabel(edge.edgeType) }}</span>
                        <strong>{{ edge.label }}</strong>
                      </div>
                    </div>
                  </article>
                </section>

                <div v-if="auditFindings.length === 0" class="audit-empty">
                  <CheckCircle2 class="h-8 w-8 text-emerald-400" />
                  <p>当前文档未发现规则风险。</p>
                </div>
                <div v-else class="audit-finding-list">
                  <article
                    v-for="finding in auditFindings"
                    :key="finding.id"
                    class="finding-card"
                    :class="[
                      `finding-card--${finding.severity}`,
                      { 'finding-card--selected': selectedFindingId === finding.id, 'finding-card--muted': finding.status === 'ignored' || finding.status === 'fixed' },
                    ]"
                  >
                    <header>
                      <div>
                        <span class="finding-line">{{ findingLineLabel(finding) }}</span>
                        <strong>{{ finding.title }}</strong>
                      </div>
                      <span class="finding-severity">{{ severityLabel(finding.severity) }}</span>
                    </header>
                    <p>{{ finding.detail }}</p>
                    <div class="finding-evidence">{{ finding.evidence }}</div>
                    <div class="finding-recommendation">{{ finding.recommendation }}</div>
                    <footer>
                      <button @click="revealFinding(finding)" type="button">查看</button>
                      <button @click="explainFinding(finding)" type="button">解释</button>
                      <button @click="applyFindingFix(finding)" type="button">处理</button>
                      <button @click="updateFindingStatus(finding, 'reviewing')" type="button">复核</button>
                      <button @click="updateFindingStatus(finding, finding.status === 'ignored' ? 'open' : 'ignored')" type="button">
                        {{ finding.status === 'ignored' ? '恢复' : '忽略' }}
                      </button>
                    </footer>
                    <div class="finding-meta">
                      {{ kindLabel(finding.kind) }} · {{ statusLabel(finding.status) }}
                    </div>
                  </article>
                </div>
              </div>

              <div v-else class="right-panel-body right-panel-body--preview">
                <div class="preview-surface" v-html="previewHtml"></div>
              </div>
            </section>
          </div>
        </div>
      </div>
    </div>

    <!-- Settings & Backup Modal -->
    <div v-if="showSettings" class="modal-overlay" @click.self="showSettings = false">
      <div class="modal-card shadow-2xl">
        <div class="modal-header">
          <h3 class="modal-title font-bold text-gray-100">AI 服务、资料库与备份</h3>
          <button @click="showSettings = false" class="text-gray-600 hover:text-gray-400 transition">
            <X class="h-5 w-5" />
          </button>
        </div>

        <div class="settings-tabs" role="tablist" aria-label="设置分类">
          <button
            type="button"
            class="settings-tab"
            :class="{ 'settings-tab--active': settingsTab === 'ai' }"
            @click="settingsTab = 'ai'"
          >
            <Sparkles class="h-4 w-4" />
            AI 服务
          </button>
          <button
            type="button"
            class="settings-tab"
            :class="{ 'settings-tab--active': settingsTab === 'data' }"
            @click="settingsTab = 'data'"
          >
            <FolderOpen class="h-4 w-4" />
            资料库
          </button>
          <button
            type="button"
            class="settings-tab"
            :class="{ 'settings-tab--active': settingsTab === 'backup' }"
            @click="settingsTab = 'backup'"
          >
            <Database class="h-4 w-4" />
            备份恢复
          </button>
          <button
            type="button"
            class="settings-tab"
            :class="{ 'settings-tab--active': settingsTab === 'security' }"
            @click="settingsTab = 'security'"
          >
            <Lock class="h-4 w-4" />
            安全
          </button>
        </div>

        <div class="modal-body">
          <section v-if="settingsTab === 'ai'" class="settings-panel">
            <h4 class="section-title">LLM 服务配置 (OpenAI 兼容)</h4>
            <div class="ai-settings-grid mt-3">
              <div class="ai-settings-field--full">
                <label class="m-label">API 基础路径 (默认 OpenAI: https://api.openai.com/v1)</label>
                <input v-model="ollamaUrl" type="text" class="m-input" />
              </div>
              <div class="ai-settings-field--full">
                <label class="m-label">API Key</label>
                <SecurePasswordInput
                  v-model="apiKey"
                  input-class="m-input"
                  autocomplete="off"
                  :placeholder="hasApiKey ? '已保存 API Key，留空则保留不变' : '输入您的 API Key'"
                  show-title="显示 API Key"
                  hide-title="隐藏 API Key"
                />
                <p class="mt-1 text-[10px] text-gray-500">
                  API Key 会写入本地 KDBX 密码库；保存后页面不回显明文。
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
              <label class="ai-setting-checkbox">
                <input v-model="disableResponseStorage" type="checkbox" class="h-4 w-4 rounded border-gray-800 bg-gray-950 text-emerald-500 focus:ring-emerald-500 focus:ring-offset-gray-905" />
                禁用响应存储 (store: false)
              </label>
              <div class="connection-test-row">
                <button
                  @click="testConnection"
                  :disabled="testingConnection || !ollamaUrl.trim() || !chatModel.trim()"
                  type="button"
                  class="d-cancel-btn connection-test-btn"
                >
                  <RefreshCw v-if="testingConnection" class="h-3.5 w-3.5 animate-spin" />
                  <CheckCircle2 v-else class="h-3.5 w-3.5" />
                  测试连通性
                </button>
                <span
                  v-if="connectionMessage"
                  class="connection-status"
                  :class="connectionOk ? 'text-emerald-400' : connectionOk === false ? 'text-rose-400' : 'text-gray-500'"
                >
                  {{ connectionMessage }}
                </span>
              </div>
            </div>
            <div class="settings-actions">
              <button @click="saveSettings" class="d-save-btn text-xs">
                保存 AI 服务配置
              </button>
              <span class="text-[10px] text-gray-600">
                保存后会重新读取后端状态确认 API Key 是否已保存。
              </span>
            </div>
          </section>

          <section v-else-if="settingsTab === 'data'" class="settings-panel">
            <h4 class="section-title flex items-center gap-1 text-emerald-400">
              <FolderOpen class="h-4 w-4" />
              资料库目录
            </h4>
            <div class="mt-3 space-y-3">
              <div class="settings-subpanel space-y-2">
                <div>
                  <div class="m-label">当前目录</div>
                  <div class="path-value">{{ activeDataDir || '读取中...' }}</div>
                </div>
                <div>
                  <div class="m-label">默认目录</div>
                  <div class="path-value">{{ defaultDataDir || '读取中...' }}</div>
                </div>
                <div class="flex items-center justify-between gap-3">
                  <span class="text-[10px] text-gray-500">
                    {{ usingCustomDataDir ? '当前使用自定义目录' : '当前使用系统默认目录' }}
                  </span>
                  <span class="text-[10px] text-gray-600 font-mono truncate">
                    {{ dataDirConfigPath }}
                  </span>
                </div>
              </div>

              <div>
                <label class="m-label">迁移到新目录</label>
                <input
                  v-model="migrationTargetDir"
                  type="text"
                  class="m-input font-mono"
                  placeholder="例如 /Users/ben/RustToolData"
                />
              </div>

              <div class="flex items-center justify-between gap-3">
                <button
                  @click="migrateDataDir"
                  :disabled="migrationLoading || !migrationTargetDir.trim()"
                  class="d-save-btn text-xs"
                >
                  <RefreshCw v-if="migrationLoading" class="animate-spin h-3.5 w-3.5 mr-1" />
                  <FolderOpen v-else class="h-3.5 w-3.5 mr-1" />
                  迁移资料库
                </button>
                <span v-if="migrationMessage" class="text-[10px] text-emerald-400 font-mono truncate">
                  {{ migrationMessage }}
                </span>
              </div>
            </div>
          </section>

          <section v-else-if="settingsTab === 'backup'" class="settings-panel">
            <h4 class="section-title flex items-center gap-1 text-emerald-400">
              <Database class="h-4 w-4" />
              异地备份与容灾
            </h4>
            <p class="text-xs text-gray-500 mb-3">
              支持一键将本地文档、配置、向量缓存和 KDBX 密码库压缩备份。
            </p>

            <div class="space-y-3">
              <div>
                <label class="m-label">本地备份目标目录（例如 /Users/ben/RustToolBackups）</label>
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
                  <SecurePasswordInput
                    v-model="webdavPass"
                    input-class="m-input"
                    autocomplete="off"
                    show-title="显示 WebDAV 密码"
                    hide-title="隐藏 WebDAV 密码"
                  />
                </div>
              </div>
              <p class="text-[10px] leading-relaxed text-gray-600">
                WebDAV URL、账号和密码必须同时填写；留空表示只做本地临时打包或本地目录备份。
              </p>
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

            <div class="restore-panel">
              <h4 class="section-title flex items-center gap-1 text-orange-400">
                <AlertTriangle class="h-4 w-4" />
                数据恢复 (Restore)
              </h4>
              <div class="mt-3 space-y-3">
                <div>
                  <label class="m-label">备份 ZIP 压缩包的绝对路径</label>
                  <input
                    v-model="restorePath"
                    type="text"
                    class="m-input font-mono"
                    placeholder="/Users/ben/RustToolBackups/rust_tool_memo_backup_xxx.zip"
                  />
                </div>
                <div>
                  <label class="m-label">安全确认</label>
                  <input
                    v-model="restoreConfirmText"
                    type="text"
                    class="m-input font-mono"
                    autocomplete="off"
                    spellcheck="false"
                    placeholder="输入 RESTORE 以启用还原"
                  />
                </div>
                <div class="flex items-center justify-between gap-3">
                  <p class="text-[10px] leading-relaxed text-orange-200/70">
                    还原会覆盖当前文档、数据库和 KDBX 密码库。
                  </p>
                  <button
                    @click="triggerRestore"
                    :disabled="restoreLoading || !restorePath.trim() || restoreConfirmText.trim() !== 'RESTORE'"
                    class="restore-btn"
                  >
                    <RefreshCw v-if="restoreLoading" class="animate-spin h-3.5 w-3.5 mr-1" />
                    还原
                  </button>
                </div>
              </div>
            </div>
          </section>

          <section v-else class="settings-panel">
            <h4 class="section-title flex items-center gap-1 text-amber-300">
              <AlertTriangle class="h-4 w-4" />
              高风险 AI 权限
            </h4>
            <div class="risk-panel mt-3">
              <div class="flex items-start gap-3">
                <AlertTriangle class="mt-0.5 h-5 w-5 shrink-0 text-amber-300" />
                <div class="min-w-0 flex-1">
                  <div class="text-sm font-bold text-amber-100">允许 AI 读取解密后的密码</div>
                  <p class="mt-1 text-xs leading-relaxed text-amber-100/70">
                    默认关闭。开启后，AI 检索问答可能把解密后的 secret 明文放入上下文；使用云端模型时不建议开启。
                  </p>
                  <label class="risk-toggle mt-4">
                    <input
                      id="allowAiSecrets"
                      :checked="allowAiSecrets"
                      @change="handleAllowSecretsChange"
                      type="checkbox"
                      class="h-4 w-4 rounded border-amber-500/40 bg-gray-950 text-amber-400 focus:ring-amber-400 focus:ring-offset-gray-950"
                    />
                    <span>我理解风险，允许 AI 检索并读取解密后的密码</span>
                  </label>
                </div>
              </div>
            </div>
            <div class="mt-4 flex items-center gap-2">
              <button @click="saveSettings" class="d-save-btn text-xs">
                保存安全设置
              </button>
              <span class="text-[10px] text-gray-600">
                建议保持关闭，除非后续接入可信本地模型。
              </span>
            </div>
          </section>
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
  @apply flex-1 grid grid-cols-[210px_minmax(0,1fr)] min-h-0 transition-all duration-300;
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
.doc-item-heading {
  @apply flex min-w-0 items-center justify-between gap-2;
}
.doc-risk-meta {
  @apply flex shrink-0 items-center gap-1;
}
.doc-risk-pill {
  @apply inline-flex h-5 min-w-[34px] items-center justify-center rounded-md border px-1.5 text-[9px] font-bold;
}
.doc-risk-pill--critical {
  @apply border-red-500/30 bg-red-500/10 text-red-300;
}
.doc-risk-pill--warning {
  @apply border-amber-400/30 bg-amber-400/10 text-amber-200;
}
.doc-risk-pill--info {
  @apply border-blue-400/20 bg-blue-400/10 text-blue-200;
}
.doc-risk-pill--safe {
  @apply border-emerald-500/25 bg-emerald-500/10 text-emerald-300;
}
.doc-risk-count {
  @apply max-w-[72px] truncate text-[9px] font-semibold text-gray-600;
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
.workspace-view-tabs {
  @apply flex shrink-0 items-center rounded-lg border border-gray-800/80 bg-gray-950/50 p-0.5;
}
.workspace-view-tab {
  @apply h-7 rounded-md px-2.5 text-[11px] font-bold text-gray-500 transition hover:text-gray-200;
}
.workspace-view-tab--active {
  @apply bg-emerald-500/10 text-emerald-300 ring-1 ring-emerald-500/25;
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
.governance-strip {
  @apply grid grid-cols-4 gap-2 border-b border-white/5 bg-gray-950/20 p-3;
}
.governance-metric {
  @apply min-w-0 rounded-lg border border-gray-800/80 bg-gray-950/45 px-3 py-2;
}
.governance-metric--wide {
  @apply col-span-1;
}
.metric-label {
  @apply block text-[10px] font-bold uppercase tracking-wider text-gray-600;
}
.governance-metric strong {
  @apply mt-1 block truncate text-sm font-bold text-gray-100;
}
.metric-foot {
  @apply mt-0.5 block truncate text-[10px] text-gray-500;
}
.security-home {
  @apply flex-1 min-h-0 overflow-y-auto bg-gray-950/5 p-4;
}
.archive-hero {
  @apply flex min-h-[96px] items-center justify-between gap-4 border-b border-gray-800/80 pb-4;
}
.archive-hero-main {
  @apply min-w-0;
}
.archive-hero-main span,
.archive-panel header span,
.archive-metric-card span {
  @apply block text-[10px] font-bold uppercase tracking-wider text-gray-600;
}
.archive-hero-main strong {
  @apply mt-1 block text-2xl font-bold text-gray-100;
}
.archive-hero-main p {
  @apply mt-1 text-xs text-gray-500;
}
.archive-hero-actions {
  @apply flex shrink-0 flex-wrap justify-end gap-2;
}
.archive-hero-actions button,
.archive-panel header button {
  @apply min-h-8 rounded-lg border border-emerald-500/25 bg-emerald-500/10 px-3 text-xs font-bold text-emerald-200 transition hover:bg-emerald-500/20 disabled:opacity-50;
}
.archive-metric-grid {
  @apply grid grid-cols-4 gap-2 border-b border-gray-800/80 py-4;
}
.archive-metric-card {
  @apply min-w-0 rounded-lg border border-gray-800/80 bg-gray-950/45 px-3 py-3 text-left transition hover:border-emerald-500/30 hover:bg-emerald-500/5;
}
.archive-metric-card--danger {
  @apply border-red-500/25 bg-red-500/5;
}
.archive-metric-card--warn {
  @apply border-amber-400/25 bg-amber-400/5;
}
.archive-metric-card strong {
  @apply mt-1 block text-xl font-bold text-gray-100;
}
.archive-metric-card em {
  @apply mt-1 block truncate text-[10px] not-italic text-gray-500;
}
.archive-board {
  @apply grid grid-cols-2 gap-3 pt-4;
}
.archive-panel {
  @apply min-w-0 rounded-lg border border-gray-800/80 bg-gray-950/35 p-3;
}
.archive-panel--wide {
  @apply col-span-2;
}
.archive-panel header {
  @apply mb-3 flex items-center justify-between gap-3;
}
.archive-panel header div {
  @apply min-w-0;
}
.archive-panel header strong {
  @apply mt-0.5 block truncate text-sm font-bold text-gray-100;
}
.archive-empty {
  @apply rounded-md border border-dashed border-gray-800 py-4 text-center text-xs text-gray-600;
}
.archive-risk-row,
.archive-finding-row,
.archive-doc-row {
  @apply mb-2 block w-full min-w-0 rounded-md border border-gray-800 bg-gray-950/70 px-3 py-2 text-left transition last:mb-0 hover:border-emerald-500/30 hover:bg-emerald-500/5;
}
.archive-risk-row--critical {
  @apply border-red-500/30 bg-red-500/5;
}
.archive-risk-row--warning {
  @apply border-amber-400/25 bg-amber-400/5;
}
.archive-risk-row span,
.archive-finding-row span,
.archive-doc-row span {
  @apply block truncate text-[10px] font-semibold text-gray-500;
}
.archive-risk-row strong,
.archive-finding-row strong,
.archive-doc-row strong {
  @apply mt-0.5 block truncate text-xs font-bold text-gray-100;
}
.archive-risk-row em {
  @apply mt-1 block truncate text-[10px] not-italic text-gray-600;
}
.archive-asset-list {
  @apply flex flex-wrap gap-1.5;
}
.archive-asset-list button {
  @apply flex max-w-full min-h-8 items-center gap-1.5 rounded-md border border-gray-800 bg-gray-950 px-2 text-left transition hover:border-emerald-500/30;
}
.archive-asset-list span {
  @apply shrink-0 rounded bg-gray-900 px-1 py-0.5 text-[9px] font-bold text-gray-500;
}
.archive-asset-list strong {
  @apply min-w-0 truncate text-[11px] font-semibold text-gray-300;
}
.archive-activity-list {
  @apply grid grid-cols-1 gap-2;
}
.archive-activity-list div {
  @apply rounded-md border border-gray-800 bg-gray-950/70 px-3 py-2;
}
.archive-activity-list span {
  @apply block text-[10px] text-gray-600;
}
.archive-activity-list strong {
  @apply mt-0.5 block truncate text-xs font-bold text-gray-200;
}
.archive-activity-list p {
  @apply mt-1 line-clamp-2 text-[11px] leading-relaxed text-gray-500;
}
.security-workspace {
  @apply flex-1 min-h-0 grid grid-cols-[minmax(320px,1fr)_320px];
}
.document-panel,
.intelligence-sidebar {
  @apply min-h-0 flex flex-col border-r border-white/5;
}
.intelligence-sidebar {
  @apply border-r-0 bg-gray-950/20;
}
.panel-heading {
  @apply h-10 px-4 border-b border-white/5 bg-gray-950/20 flex items-center gap-1.5 text-xs font-bold text-gray-400;
}
.editor-section-title {
  @apply mb-4 flex min-h-[40px] items-center justify-between gap-3 border-b border-white/5 pb-3 text-xs font-bold text-gray-400;
}
.messages-area {
  @apply flex-1 overflow-y-auto p-4 space-y-4 min-h-0;
}
.messages-area--compact {
  @apply p-3;
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
  @apply inline-flex shrink-0 items-center justify-center whitespace-nowrap px-4 py-2 border border-gray-800 hover:bg-gray-800 text-gray-400 rounded-xl text-xs font-semibold transition-colors;
}
.d-save-btn {
  @apply inline-flex shrink-0 items-center justify-center whitespace-nowrap px-4 py-2 bg-gradient-to-r from-emerald-600 to-teal-500 hover:from-emerald-500 hover:to-teal-400 text-white rounded-xl text-xs font-semibold transition shadow-md shadow-emerald-950/20;
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
  @apply flex-1 w-full px-4 py-3 bg-gray-950 text-xs text-gray-200 focus:outline-none transition font-mono resize-none min-h-[300px] leading-relaxed;
}
.editor-shell {
  @apply mt-1 flex flex-1 min-h-[300px] overflow-hidden rounded-xl border border-gray-800 bg-gray-950 focus-within:border-emerald-500 focus-within:ring-1 focus-within:ring-emerald-500;
}
.line-gutter {
  @apply w-12 shrink-0 overflow-hidden border-r border-gray-800 bg-gray-950/80 py-3 text-right font-mono;
}
.editor-line-no {
  @apply block h-5 w-full px-2 text-right text-[10px] leading-5 text-gray-700 transition;
}
.editor-line-no--critical {
  @apply border-l-2 border-red-500 bg-red-500/10 text-red-300;
}
.editor-line-no--warning {
  @apply border-l-2 border-amber-400 bg-amber-400/10 text-amber-300;
}
.editor-line-no--info {
  @apply border-l-2 border-blue-400 bg-blue-400/10 text-blue-300;
}
.selection-toolbar {
  @apply mb-2 flex w-fit items-center gap-1 rounded-lg border border-emerald-500/20 bg-gray-950 px-2 py-1 shadow-lg shadow-black/20;
}
.selection-toolbar button {
  @apply rounded-md px-2 py-1 text-[10px] font-semibold text-gray-300 transition hover:bg-emerald-500/10 hover:text-emerald-300;
}
.audit-mini-pill {
  @apply rounded-md border border-gray-800 bg-gray-950 px-2 py-1 text-[10px] font-bold text-gray-400;
}
.audit-mini-pill--danger {
  @apply border-red-500/30 bg-red-500/10 text-red-300;
}
.audit-mini-pill--warn {
  @apply border-amber-400/30 bg-amber-400/10 text-amber-300;
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

.right-tabs {
  @apply grid h-12 grid-cols-3 gap-1 border-b border-white/5 bg-gray-950/35 p-2;
}
.right-tab {
  @apply flex min-w-0 items-center justify-center gap-1 rounded-lg px-2 text-xs font-bold text-gray-500 transition hover:bg-gray-900 hover:text-gray-200;
}
.right-tab--active {
  @apply bg-emerald-500/10 text-emerald-300 ring-1 ring-emerald-500/25;
}
.tab-count {
  @apply inline-flex h-4 min-w-4 items-center justify-center rounded-full bg-red-500 px-1 text-[9px] text-white;
}
.right-panel-body {
  @apply flex flex-1 min-h-0 flex-col overflow-y-auto p-3;
}
.right-panel-body--preview {
  @apply p-0;
}
.assistant-summary,
.assistant-tasks {
  @apply mb-3 rounded-lg border border-gray-800/80 bg-gray-950/45 p-3;
}
.assistant-summary h4,
.assistant-tasks h4 {
  @apply mb-2 text-xs font-bold text-gray-300;
}
.assistant-summary p {
  @apply text-xs leading-relaxed text-gray-500;
}
.assistant-tasks {
  @apply grid grid-cols-1 gap-2;
}
.assistant-tasks button {
  @apply flex items-center gap-1.5 rounded-lg border border-gray-800 bg-gray-950 px-3 py-2 text-left text-xs font-semibold text-gray-300 transition hover:border-emerald-500/30 hover:bg-emerald-500/10 hover:text-emerald-300 disabled:cursor-not-allowed disabled:opacity-55;
}
.audit-summary-grid {
  @apply mb-3 grid grid-cols-3 gap-2;
}
.audit-summary-grid div {
  @apply rounded-lg border border-gray-800/80 bg-gray-950/45 p-2 text-center;
}
.audit-summary-grid span {
  @apply block text-[10px] font-bold text-gray-600;
}
.audit-summary-grid strong {
  @apply mt-1 block text-lg font-bold text-gray-100;
}
.audit-action-row {
  @apply mb-3 grid grid-cols-[1fr_auto] gap-2;
}
.audit-scan-btn {
  @apply flex h-9 w-full items-center justify-center gap-1.5 rounded-lg bg-emerald-600 px-2 text-xs font-bold text-white transition hover:bg-emerald-500 disabled:opacity-50;
}
.audit-batch-btn {
  @apply flex h-9 min-w-[104px] items-center justify-center gap-1.5 rounded-lg border border-emerald-500/25 bg-emerald-500/10 px-2.5 text-xs font-bold text-emerald-200 transition hover:border-emerald-400/40 hover:bg-emerald-500/15 disabled:cursor-not-allowed disabled:border-gray-800 disabled:bg-gray-950/50 disabled:text-gray-600;
}
.risk-diff-panel {
  @apply mb-3 space-y-2 rounded-lg border border-gray-800/80 bg-gray-950/35 p-2.5;
}
.risk-diff-panel--active {
  @apply border-emerald-500/25 bg-emerald-500/5;
}
.risk-diff-header {
  @apply flex items-start justify-between gap-2;
}
.risk-diff-header span {
  @apply block text-[10px] font-bold uppercase text-gray-600;
}
.risk-diff-header strong {
  @apply mt-0.5 block text-xs font-bold leading-snug text-gray-200;
}
.risk-diff-empty {
  @apply rounded-md border border-dashed border-gray-800 py-3 text-center text-xs text-gray-600;
}
.risk-diff-metrics {
  @apply grid grid-cols-3 gap-1.5;
}
.risk-diff-metrics div {
  @apply rounded-md border border-gray-800 bg-gray-950/70 px-2 py-1.5 text-center;
}
.risk-diff-metrics span {
  @apply block text-[10px] font-bold text-gray-600;
}
.risk-diff-metrics strong {
  @apply block text-sm font-bold text-gray-100;
}
.risk-diff-list {
  @apply space-y-1;
}
.risk-diff-item {
  @apply block w-full rounded-md border border-gray-800 bg-gray-950/70 px-2 py-1.5 text-left transition hover:border-emerald-500/30 hover:bg-emerald-500/10;
}
.risk-diff-item span {
  @apply block text-[10px] font-semibold text-gray-500;
}
.risk-diff-item strong {
  @apply mt-0.5 block truncate text-[11px] font-bold text-gray-200;
}
.risk-diff-item--added {
  @apply border-red-500/25 bg-red-500/5;
}
.risk-diff-item--resolved {
  @apply border-emerald-500/25 bg-emerald-500/5;
}
.risk-diff-item--changed {
  @apply border-blue-400/25 bg-blue-400/5;
}
.case-queue {
  @apply mb-3 space-y-2 rounded-lg border border-gray-800/80 bg-gray-950/35 p-2.5;
}
.case-queue-header {
  @apply flex items-center justify-between gap-2;
}
.case-queue-header span {
  @apply block text-[10px] font-bold uppercase text-gray-600;
}
.case-queue-header strong {
  @apply block text-xs font-bold text-gray-200;
}
.case-report-actions {
  @apply flex shrink-0 flex-wrap justify-end gap-1;
}
.case-report-btn {
  @apply flex min-h-8 shrink-0 items-center gap-1 rounded-md border border-emerald-500/25 bg-emerald-500/10 px-2 text-[10px] font-bold text-emerald-200 transition hover:bg-emerald-500/20 disabled:opacity-50;
}
.case-report-tag-input {
  @apply h-8 w-20 min-w-0 rounded-md border border-gray-800 bg-gray-950 px-2 text-[10px] font-semibold text-gray-300 outline-none transition placeholder:text-gray-600 focus:border-emerald-500/40;
}
.case-empty {
  @apply rounded-md border border-dashed border-gray-800 py-4 text-center text-xs text-gray-600;
}
.case-card {
  @apply rounded-lg border border-gray-800 bg-gray-950/60 p-2.5 shadow-sm shadow-black/10;
}
.case-card--critical {
  @apply border-red-500/30;
}
.case-card--warning {
  @apply border-amber-400/25;
}
.case-card--info {
  @apply border-blue-400/20;
}
.case-card--status-accepted,
.case-card--status-closed,
.case-card--status-fixed {
  @apply opacity-70;
}
.case-card header {
  @apply flex items-start justify-between gap-2;
}
.case-card header div {
  @apply min-w-0;
}
.case-card header span {
  @apply block text-[10px] font-semibold text-gray-500;
}
.case-card header strong {
  @apply mt-0.5 block truncate text-xs font-bold text-gray-100;
}
.case-status {
  @apply shrink-0 rounded-md border border-white/10 bg-gray-900 px-1.5 py-0.5 text-[10px] font-bold text-emerald-200;
}
.case-meta-line {
  @apply mt-2 flex items-center justify-between gap-2 text-[10px] text-gray-600;
}
.case-meta-line span:first-child {
  @apply min-w-0 truncate font-mono;
}
.case-fields {
  @apply mt-2 grid grid-cols-2 gap-1.5;
}
.case-fields input,
.case-card textarea {
  @apply min-w-0 rounded-md border border-gray-800 bg-gray-950 px-2 py-1.5 text-[11px] text-gray-200 outline-none transition placeholder:text-gray-600 focus:border-emerald-500/50;
}
.case-card textarea {
  @apply mt-1.5 w-full resize-none leading-relaxed;
}
.case-accept-btn {
  @apply rounded-md border border-emerald-500/25 bg-emerald-500/10 px-2 py-1.5 text-[11px] font-bold text-emerald-200 transition hover:bg-emerald-500/20 disabled:opacity-50;
}
.case-exception-detail {
  @apply mt-2 flex flex-wrap gap-1;
}
.case-exception-detail span {
  @apply max-w-full truncate rounded-md border border-amber-400/20 bg-amber-400/10 px-1.5 py-0.5 text-[10px] font-semibold text-amber-200;
}
.case-card footer {
  @apply mt-2 grid grid-cols-4 gap-1;
}
.case-card footer button {
  @apply min-h-7 rounded-md border border-gray-800 bg-gray-950 px-1.5 text-[10px] font-semibold text-gray-300 transition hover:border-emerald-500/30 hover:text-emerald-300 disabled:opacity-50;
}
.case-event-list {
  @apply space-y-1 border-t border-gray-800/80 pt-2;
}
.case-event-list div {
  @apply rounded-md bg-black/15 px-2 py-1.5;
}
.case-event-list span {
  @apply block text-[10px] text-gray-600;
}
.case-event-list strong {
  @apply mt-0.5 block text-[11px] font-semibold leading-snug text-gray-300;
}
.checklist-panel {
  @apply mb-3 space-y-2 rounded-lg border border-gray-800/80 bg-gray-950/30 p-2.5;
}
.checklist-header {
  @apply flex items-center justify-between gap-2;
}
.checklist-header span {
  @apply block text-[10px] font-bold uppercase text-gray-600;
}
.checklist-header strong {
  @apply block text-xs font-bold text-gray-200;
}
.checklist-empty {
  @apply rounded-md border border-dashed border-gray-800 py-4 text-center text-xs text-gray-600;
}
.checklist-card {
  @apply rounded-lg border border-gray-800 bg-gray-950/55 p-2.5;
}
.checklist-card--recommended {
  @apply border-emerald-500/25 bg-emerald-500/5;
}
.checklist-card--done,
.checklist-card--waived {
  @apply opacity-70;
}
.checklist-card header {
  @apply flex items-start justify-between gap-2;
}
.checklist-card header div {
  @apply min-w-0;
}
.checklist-card header span {
  @apply block text-[10px] font-semibold text-gray-500;
}
.checklist-card header strong {
  @apply mt-0.5 block truncate text-xs font-bold text-gray-100;
}
.checklist-card em {
  @apply shrink-0 rounded-md border border-white/10 bg-gray-900 px-1.5 py-0.5 text-[10px] not-italic font-bold text-gray-300;
}
.checklist-card p {
  @apply mt-2 text-[11px] leading-relaxed text-gray-400;
}
.checklist-standard {
  @apply mt-2 rounded-md border border-gray-800 bg-black/15 px-2 py-1.5 text-[10px] leading-snug text-emerald-200;
}
.checklist-evidence {
  @apply mt-2 flex flex-wrap gap-1;
}
.checklist-evidence span {
  @apply max-w-full truncate rounded-md bg-gray-900 px-1.5 py-0.5 text-[10px] text-gray-500;
}
.checklist-card input {
  @apply mt-2 min-h-8 w-full rounded-md border border-gray-800 bg-gray-950 px-2 text-[11px] text-gray-200 outline-none placeholder:text-gray-600 focus:border-emerald-500/50;
}
.checklist-card footer {
  @apply mt-2 grid grid-cols-3 gap-1;
}
.checklist-card footer button {
  @apply min-h-7 rounded-md border border-gray-800 bg-gray-950 px-1.5 text-[10px] font-semibold text-gray-300 transition hover:border-emerald-500/30 hover:text-emerald-300 disabled:opacity-50;
}
.asset-panel {
  @apply mb-3 space-y-2 rounded-lg border border-gray-800/80 bg-gray-950/30 p-2.5;
}
.asset-header {
  @apply flex items-center justify-between gap-2;
}
.asset-header span {
  @apply block text-[10px] font-bold uppercase text-gray-600;
}
.asset-header strong {
  @apply block text-xs font-bold text-gray-200;
}
.asset-empty {
  @apply rounded-md border border-dashed border-gray-800 py-4 text-center text-xs text-gray-600;
}
.asset-list {
  @apply flex flex-wrap gap-1.5;
}
.asset-chip {
  @apply flex max-w-full min-h-8 items-center gap-1.5 rounded-md border border-gray-800 bg-gray-950 px-2 text-left transition hover:border-emerald-500/30;
}
.asset-chip span {
  @apply shrink-0 rounded bg-gray-900 px-1 py-0.5 text-[9px] font-bold text-gray-500;
}
.asset-chip strong {
  @apply min-w-0 truncate text-[11px] font-semibold text-gray-300;
}
.asset-chip--active {
  @apply border-emerald-500/40 bg-emerald-500/10;
}
.asset-detail {
  @apply rounded-lg border border-emerald-500/20 bg-emerald-500/5 p-2.5;
}
.asset-detail header {
  @apply flex items-center justify-between gap-2;
}
.asset-detail header span {
  @apply rounded-md bg-gray-950 px-1.5 py-0.5 text-[10px] font-bold text-emerald-200;
}
.asset-detail header strong {
  @apply min-w-0 truncate text-xs font-bold text-gray-100;
}
.asset-detail-grid {
  @apply mt-2 grid grid-cols-3 gap-1.5;
}
.asset-detail-grid div {
  @apply rounded-md border border-gray-800 bg-gray-950/70 px-2 py-1.5 text-center;
}
.asset-detail-grid span {
  @apply block text-[10px] text-gray-600;
}
.asset-detail-grid strong {
  @apply block text-sm font-bold text-gray-100;
}
.asset-related-list {
  @apply mt-2 flex flex-wrap gap-1;
}
.asset-related-list button {
  @apply max-w-full truncate rounded-md border border-gray-800 bg-gray-950 px-2 py-1 text-[10px] font-semibold text-gray-300 hover:border-emerald-500/30 hover:text-emerald-300;
}
.asset-finding-list {
  @apply mt-2 space-y-1;
}
.asset-finding-list span {
  @apply block rounded-md bg-black/15 px-2 py-1 text-[10px] text-gray-400;
}
.asset-graph {
  @apply rounded-lg border border-gray-800/80 bg-gray-950/35 p-2.5;
}
.asset-graph header {
  @apply flex items-center justify-between gap-2;
}
.asset-graph header span {
  @apply block text-[10px] font-bold uppercase text-gray-600;
}
.asset-graph header strong {
  @apply block text-xs font-bold text-gray-200;
}
.graph-node-list {
  @apply mt-2 flex flex-wrap gap-1.5;
}
.graph-node-chip {
  @apply flex max-w-full min-h-7 items-center gap-1 rounded-md border border-gray-800 bg-gray-950 px-1.5;
}
.graph-node-chip em {
  @apply shrink-0 not-italic rounded bg-gray-900 px-1 py-0.5 text-[9px] font-bold text-gray-500;
}
.graph-node-chip strong {
  @apply min-w-0 truncate text-[10px] font-semibold text-gray-300;
}
.graph-node-chip--document {
  @apply border-blue-400/20 bg-blue-400/5;
}
.graph-node-chip--asset {
  @apply border-emerald-500/25 bg-emerald-500/5;
}
.graph-node-chip--finding,
.graph-node-chip--critical {
  @apply border-red-500/25 bg-red-500/5;
}
.graph-node-chip--secret {
  @apply border-purple-400/20 bg-purple-400/5;
}
.graph-node-chip--case {
  @apply border-amber-400/25 bg-amber-400/5;
}
.graph-node-chip--warning {
  @apply border-amber-400/25 bg-amber-400/5;
}
.graph-node-chip--info {
  @apply border-blue-400/20 bg-blue-400/5;
}
.graph-edge-list {
  @apply mt-2 space-y-1 border-t border-gray-800/70 pt-2;
}
.graph-edge-list div {
  @apply rounded-md bg-black/15 px-2 py-1.5;
}
.graph-edge-list span {
  @apply block text-[9px] font-bold uppercase text-gray-600;
}
.graph-edge-list strong {
  @apply mt-0.5 block truncate text-[10px] font-semibold text-gray-300;
}
.audit-empty {
  @apply flex flex-1 flex-col items-center justify-center gap-2 rounded-lg border border-gray-800/70 bg-gray-950/35 p-6 text-center text-xs text-gray-500;
}
.audit-finding-list {
  @apply space-y-3;
}
.finding-card {
  @apply rounded-lg border border-gray-800 bg-gray-950/55 p-3 shadow-md shadow-black/10 transition;
}
.finding-card--critical {
  @apply border-red-500/35 bg-red-500/10;
}
.finding-card--warning {
  @apply border-amber-400/30 bg-amber-400/10;
}
.finding-card--info {
  @apply border-blue-400/25 bg-blue-400/10;
}
.finding-card--selected {
  @apply ring-1 ring-emerald-400;
}
.finding-card--muted {
  @apply opacity-55;
}
.finding-card header {
  @apply flex items-start justify-between gap-2;
}
.finding-card header div {
  @apply min-w-0;
}
.finding-card strong {
  @apply block truncate text-sm font-bold text-gray-100;
}
.finding-line {
  @apply mb-1 inline-flex rounded-md border border-gray-700 bg-gray-950 px-1.5 py-0.5 font-mono text-[10px] text-gray-400;
}
.finding-severity {
  @apply shrink-0 rounded-md border border-white/10 bg-gray-950 px-1.5 py-0.5 text-[10px] font-bold text-gray-300;
}
.finding-card p {
  @apply mt-2 text-xs leading-relaxed text-gray-400;
}
.finding-evidence {
  @apply mt-2 break-all rounded-md border border-gray-800 bg-gray-950/70 px-2 py-1.5 font-mono text-[10px] text-gray-300;
}
.finding-recommendation {
  @apply mt-2 rounded-md bg-black/15 px-2 py-1.5 text-[11px] leading-relaxed text-gray-400;
}
.finding-card footer {
  @apply mt-3 grid grid-cols-4 gap-1;
}
.finding-card footer button {
  @apply rounded-md border border-gray-800 bg-gray-950 px-2 py-1.5 text-[10px] font-semibold text-gray-300 transition hover:border-emerald-500/30 hover:text-emerald-300;
}
.finding-meta {
  @apply mt-2 text-[10px] text-gray-600;
}

@media (max-width: 1400px) {
  .security-workspace {
    @apply grid-cols-[minmax(300px,1fr)_300px];
  }
}

@media (max-width: 1180px) {
  .governance-strip {
    @apply grid-cols-2;
  }
  .archive-metric-grid,
  .archive-board {
    @apply grid-cols-2;
  }
  .security-workspace {
    @apply grid-cols-[minmax(0,1fr)] grid-rows-[minmax(0,1fr)_340px];
  }
  .document-panel {
    @apply border-r-0 border-b border-white/5;
  }
}

@media (max-width: 760px) {
  .memo-layout {
    @apply h-[82vh] min-h-[620px];
  }
  .action-bar,
  .workspace-toolbar {
    @apply flex-wrap;
  }
  .memo-grid,
  .memo-grid--collapsed {
    @apply grid-cols-1 grid-rows-[180px_minmax(0,1fr)];
  }
  .doc-sidebar {
    @apply border-r-0 border-b border-white/5;
  }
  .governance-strip {
    @apply grid-cols-2;
  }
  .archive-hero {
    @apply items-start flex-col;
  }
  .archive-hero-actions {
    @apply justify-start;
  }
  .archive-metric-grid,
  .archive-board {
    @apply grid-cols-1;
  }
  .archive-panel--wide {
    @apply col-span-1;
  }
  .workspace-view-tabs {
    @apply order-3 w-full;
  }
  .workspace-view-tab {
    @apply flex-1;
  }
  .security-workspace {
    @apply grid-cols-1 grid-rows-[minmax(0,1fr)_320px];
  }
}

/* Modal configuration */
.modal-overlay {
  @apply fixed inset-0 z-50 flex items-center justify-center bg-black/85 backdrop-blur-md p-4;
}
.modal-card {
  @apply w-full max-w-3xl bg-gray-900/90 border border-white/5 rounded-2xl shadow-2xl overflow-hidden flex flex-col max-h-[85vh] backdrop-blur-2xl;
}
.modal-header {
  @apply px-5 py-3.5 border-b border-gray-800/80 flex justify-between items-center bg-gray-950/40;
}
.modal-title {
  @apply text-xs font-bold text-gray-200 uppercase tracking-wide;
}
.settings-tabs {
  @apply flex gap-1 overflow-x-auto border-b border-gray-800/80 bg-gray-950/30 px-4 py-2;
  scrollbar-width: none;
}
.settings-tabs::-webkit-scrollbar {
  display: none;
}
.settings-tab {
  @apply flex min-h-[36px] shrink-0 items-center gap-1.5 whitespace-nowrap rounded-lg px-3 py-2 text-xs font-semibold text-gray-500 transition-colors hover:bg-gray-800/60 hover:text-gray-200;
}
.settings-tab--active {
  @apply bg-emerald-500/10 text-emerald-300 ring-1 ring-emerald-500/30;
}
.modal-body {
  @apply p-5 overflow-y-auto flex-1 min-h-0;
}
.settings-panel {
  @apply min-h-[420px];
}
.settings-subpanel {
  @apply rounded-xl border border-gray-800 bg-gray-950/50 p-3;
}
.section-title {
  @apply text-xs font-bold text-gray-300 uppercase tracking-wider mb-1;
}
.ai-settings-grid {
  @apply grid grid-cols-1 gap-4 md:grid-cols-2;
}
.ai-settings-field--full,
.connection-test-row {
  @apply md:col-span-2;
}
.ai-setting-checkbox {
  @apply flex min-w-0 items-center gap-2 self-end text-xs text-gray-300 select-none cursor-pointer;
}
.connection-test-row {
  @apply grid gap-2 sm:grid-cols-[auto_minmax(0,1fr)] sm:items-start;
}
.connection-test-btn {
  @apply inline-flex w-fit shrink-0 items-center justify-center gap-1.5 whitespace-nowrap text-xs;
}
.connection-status {
  @apply min-w-0 rounded-xl border border-gray-800/80 bg-gray-950/50 px-3 py-2 text-[10px] leading-relaxed;
  overflow-wrap: anywhere;
}
.settings-actions {
  @apply mt-4 flex flex-wrap items-center gap-2;
}
.m-label {
  @apply block text-[10px] font-semibold text-gray-500 mb-1.5;
}
.m-input {
  @apply w-full px-3 py-2 bg-gray-950 border border-gray-800 rounded-xl text-xs text-gray-300 focus:outline-none focus:border-emerald-500 focus:ring-1 focus:ring-emerald-500 transition-colors;
}
.path-value {
  @apply break-all rounded-lg border border-gray-800/80 bg-gray-950 px-2.5 py-1.5 font-mono text-[10px] leading-relaxed text-gray-300;
}
.risk-panel {
  @apply rounded-xl border border-amber-500/30 bg-amber-500/10 p-4 shadow-lg shadow-amber-950/10;
}
.risk-toggle {
  @apply flex items-center gap-2 rounded-lg border border-amber-500/20 bg-gray-950/40 px-3 py-2 text-xs font-semibold text-amber-100/90 select-none cursor-pointer;
}
.restore-panel {
  @apply mt-5 rounded-xl border border-orange-500/25 bg-orange-500/10 p-4;
}
.restore-btn {
  @apply flex items-center rounded-lg bg-orange-600 px-4 py-2 text-xs font-semibold text-white transition hover:bg-orange-500 disabled:cursor-not-allowed disabled:opacity-40;
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
