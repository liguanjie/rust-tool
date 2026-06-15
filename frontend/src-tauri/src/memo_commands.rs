use rust_tool_core::memo::{
    AuditEvent, AuditFixPreview, AuditScanResponse, ChangeMasterPasswordResponse, ChatMessage,
    ChecklistItem, ChecklistStatus, DocumentDetail, DocumentRiskDiff, DraftResponse,
    GovernanceSummary, MemoManager, MemoMetadata, RedactMarkdownResponse, SafeShareExport,
    SafeShareRequest, SaveDocumentResponse, SearchAnswerResponse, SecretListItem,
    SecretRevealResponse, SecurityAsset, SecurityAssetDetail, SecurityAssetGraph, SecurityCase,
    SecurityCaseStatus, SecurityReport, SecurityReportRequest, StandardEntry,
    StandardsChecklistResponse, WebDavConfig,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::State;

pub(crate) struct MemoDesktopState {
    manager: Arc<MemoManager>,
    default_data_dir: PathBuf,
    active_data_dir: PathBuf,
}

pub(crate) fn create_memo_state() -> Result<MemoDesktopState, String> {
    let default_data_dir = get_default_base_dir();
    let active_data_dir = resolve_memo_data_dir(&default_data_dir);
    let manager = Arc::new(MemoManager::new(&active_data_dir)?);

    Ok(MemoDesktopState {
        manager,
        default_data_dir,
        active_data_dir,
    })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UnlockRequest {
    password: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UnlockResponse {
    unlocked: bool,
}

#[tauri::command]
pub(crate) async fn memo_unlock(
    state: State<'_, MemoDesktopState>,
    payload: UnlockRequest,
) -> Result<UnlockResponse, String> {
    let unlocked = state.manager.unlock(&payload.password).await?;
    Ok(UnlockResponse { unlocked })
}

#[tauri::command]
pub(crate) async fn memo_lock(
    state: State<'_, MemoDesktopState>,
) -> Result<HashMap<String, bool>, String> {
    state.manager.lock().await;
    Ok(ok_response())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StatusResponse {
    unlocked: bool,
    ollama_url: String,
    has_api_key: bool,
    chat_model: String,
    embedding_model: String,
    reasoning_effort: String,
    disable_response_storage: bool,
    allow_ai_secrets: bool,
    custom_data_dir: Option<String>,
}

#[tauri::command]
pub(crate) async fn memo_status(
    state: State<'_, MemoDesktopState>,
) -> Result<StatusResponse, String> {
    let unlocked = !state.manager.is_locked().await;
    let (
        ollama_url,
        _api_key,
        chat_model,
        embedding_model,
        reasoning_effort,
        disable_response_storage,
        allow_ai_secrets,
    ) = state.manager.get_llm_config();
    let local_config = read_local_config(&state.default_data_dir);

    Ok(StatusResponse {
        unlocked,
        ollama_url,
        has_api_key: state.manager.has_llm_api_key()?,
        chat_model,
        embedding_model,
        reasoning_effort,
        disable_response_storage,
        allow_ai_secrets,
        custom_data_dir: local_config.custom_data_dir,
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DataDirResponse {
    default_data_dir: String,
    active_data_dir: String,
    custom_data_dir: Option<String>,
    using_custom_data_dir: bool,
    config_path: String,
}

#[tauri::command]
pub(crate) async fn memo_data_dir(
    state: State<'_, MemoDesktopState>,
) -> Result<DataDirResponse, String> {
    let local_config = read_local_config(&state.default_data_dir);
    Ok(data_dir_response(&state, local_config.custom_data_dir))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SettingsRequest {
    ollama_url: String,
    api_key: Option<String>,
    chat_model: String,
    embedding_model: String,
    reasoning_effort: String,
    disable_response_storage: bool,
    allow_ai_secrets: bool,
}

#[tauri::command]
pub(crate) async fn memo_update_settings(
    state: State<'_, MemoDesktopState>,
    payload: SettingsRequest,
) -> Result<HashMap<String, bool>, String> {
    require_unlocked(&state).await?;
    state
        .manager
        .update_llm_config(
            &payload.ollama_url,
            payload.api_key.as_deref(),
            &payload.chat_model,
            &payload.embedding_model,
            &payload.reasoning_effort,
            payload.disable_response_storage,
            payload.allow_ai_secrets,
        )
        .await?;
    Ok(ok_response())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TestConnectionRequest {
    ollama_url: String,
    api_key: Option<String>,
    chat_model: String,
    embedding_model: String,
    reasoning_effort: Option<String>,
    disable_response_storage: Option<bool>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TestConnectionResponse {
    ok: bool,
    message: String,
}

#[tauri::command]
pub(crate) async fn memo_test_connection(
    state: State<'_, MemoDesktopState>,
    payload: TestConnectionRequest,
) -> Result<TestConnectionResponse, String> {
    require_unlocked(&state).await?;
    let (_, saved_api_key, _, _, saved_reasoning_effort, saved_disable_response_storage, _) =
        state.manager.get_llm_config();
    let api_key = payload
        .api_key
        .as_deref()
        .filter(|key| !key.trim().is_empty())
        .unwrap_or(&saved_api_key);

    let client = rust_tool_core::memo::LlmClient::new(
        &payload.ollama_url,
        Some(api_key),
        &payload.chat_model,
        &payload.embedding_model,
        payload
            .reasoning_effort
            .as_deref()
            .or(Some(&saved_reasoning_effort)),
        payload
            .disable_response_storage
            .unwrap_or(saved_disable_response_storage),
    );
    let messages = vec![ChatMessage {
        role: "user".to_string(),
        content: "Reply with exactly: OK".to_string(),
    }];
    let reply = client.chat(messages, false).await?;

    Ok(TestConnectionResponse {
        ok: true,
        message: format!("连接成功，模型响应：{}", reply.trim()),
    })
}

#[tauri::command]
pub(crate) async fn memo_list_documents(
    state: State<'_, MemoDesktopState>,
) -> Result<Vec<MemoMetadata>, String> {
    require_unlocked(&state).await?;
    state.manager.get_store().get_all_memos()
}

#[tauri::command]
pub(crate) async fn memo_get_document(
    state: State<'_, MemoDesktopState>,
    id: String,
) -> Result<DocumentDetail, String> {
    require_unlocked(&state).await?;
    state.manager.get_document(&id).await
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AuditScanRequest {
    doc_id: String,
    markdown: Option<String>,
}

#[tauri::command]
pub(crate) async fn memo_audit_scan(
    state: State<'_, MemoDesktopState>,
    payload: AuditScanRequest,
) -> Result<AuditScanResponse, String> {
    require_unlocked(&state).await?;
    state
        .manager
        .audit_document(&payload.doc_id, payload.markdown.as_deref())
        .await
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AuditFindingStatusRequest {
    finding_id: String,
    status: String,
}

#[tauri::command]
pub(crate) async fn memo_audit_update_finding_status(
    state: State<'_, MemoDesktopState>,
    payload: AuditFindingStatusRequest,
) -> Result<HashMap<String, bool>, String> {
    require_unlocked(&state).await?;
    state
        .manager
        .update_audit_finding_status(&payload.finding_id, &payload.status)
        .await?;
    Ok(ok_response())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AuditFixPreviewRequest {
    doc_id: String,
    finding_id: String,
    markdown: Option<String>,
}

#[tauri::command]
pub(crate) async fn memo_audit_fix_preview(
    state: State<'_, MemoDesktopState>,
    payload: AuditFixPreviewRequest,
) -> Result<AuditFixPreview, String> {
    require_unlocked(&state).await?;
    state
        .manager
        .audit_fix_preview(
            &payload.doc_id,
            &payload.finding_id,
            payload.markdown.as_deref(),
        )
        .await
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AuditRedactRequest {
    markdown: String,
}

#[tauri::command]
pub(crate) async fn memo_audit_redact(
    state: State<'_, MemoDesktopState>,
    payload: AuditRedactRequest,
) -> Result<RedactMarkdownResponse, String> {
    require_unlocked(&state).await?;
    state.manager.redact_markdown(&payload.markdown).await
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DocumentRiskDiffRequest {
    doc_id: String,
    markdown: Option<String>,
}

#[tauri::command]
pub(crate) async fn memo_document_risk_diff(
    state: State<'_, MemoDesktopState>,
    payload: DocumentRiskDiffRequest,
) -> Result<DocumentRiskDiff, String> {
    require_unlocked(&state).await?;
    state
        .manager
        .document_risk_diff(&payload.doc_id, payload.markdown.as_deref())
        .await
}

#[tauri::command]
pub(crate) async fn memo_governance_summary(
    state: State<'_, MemoDesktopState>,
) -> Result<GovernanceSummary, String> {
    require_unlocked(&state).await?;
    state.manager.governance_summary().await
}

#[tauri::command]
pub(crate) async fn memo_governance_cases(
    state: State<'_, MemoDesktopState>,
) -> Result<Vec<SecurityCase>, String> {
    require_unlocked(&state).await?;
    state.manager.governance_cases().await
}

#[tauri::command]
pub(crate) async fn memo_governance_events(
    state: State<'_, MemoDesktopState>,
) -> Result<Vec<AuditEvent>, String> {
    require_unlocked(&state).await?;
    state.manager.governance_events().await
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GovernanceCaseStatusRequest {
    case_id: String,
    status: String,
    owner: Option<String>,
    due_at: Option<String>,
    rationale: Option<String>,
}

#[tauri::command]
pub(crate) async fn memo_governance_update_case_status(
    state: State<'_, MemoDesktopState>,
    payload: GovernanceCaseStatusRequest,
) -> Result<SecurityCase, String> {
    require_unlocked(&state).await?;
    let status = SecurityCaseStatus::from_action(&payload.status)
        .ok_or_else(|| "Invalid security case status".to_string())?;
    state
        .manager
        .update_security_case_status(
            &payload.case_id,
            status,
            payload.owner,
            payload.due_at,
            payload.rationale,
        )
        .await
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GovernanceCaseAcceptRequest {
    case_id: String,
    rationale: String,
    accepted_until: String,
    impact_scope: String,
    compensating_controls: String,
    reviewer: String,
    owner: Option<String>,
}

#[tauri::command]
pub(crate) async fn memo_governance_accept_case(
    state: State<'_, MemoDesktopState>,
    payload: GovernanceCaseAcceptRequest,
) -> Result<SecurityCase, String> {
    require_unlocked(&state).await?;
    state
        .manager
        .accept_security_case(
            &payload.case_id,
            payload.rationale,
            payload.accepted_until,
            payload.impact_scope,
            payload.compensating_controls,
            payload.reviewer,
            payload.owner,
        )
        .await
}

#[tauri::command]
pub(crate) async fn memo_assets_list(
    state: State<'_, MemoDesktopState>,
) -> Result<Vec<SecurityAsset>, String> {
    require_unlocked(&state).await?;
    state.manager.security_assets().await
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AssetDetailRequest {
    asset_id: Option<String>,
    query: Option<String>,
}

#[tauri::command]
pub(crate) async fn memo_assets_detail(
    state: State<'_, MemoDesktopState>,
    payload: AssetDetailRequest,
) -> Result<SecurityAssetDetail, String> {
    require_unlocked(&state).await?;
    state
        .manager
        .security_asset_detail(payload.asset_id.as_deref(), payload.query.as_deref())
        .await
}

#[tauri::command]
pub(crate) async fn memo_assets_graph(
    state: State<'_, MemoDesktopState>,
    payload: AssetDetailRequest,
) -> Result<SecurityAssetGraph, String> {
    require_unlocked(&state).await?;
    state
        .manager
        .security_asset_graph(payload.asset_id.as_deref(), payload.query.as_deref())
        .await
}

#[tauri::command]
pub(crate) async fn memo_generate_security_report(
    state: State<'_, MemoDesktopState>,
    payload: SecurityReportRequest,
) -> Result<SecurityReport, String> {
    require_unlocked(&state).await?;
    state
        .manager
        .generate_security_report_with_request(payload)
        .await
}

#[tauri::command]
pub(crate) async fn memo_safe_share_export(
    state: State<'_, MemoDesktopState>,
    payload: SafeShareRequest,
) -> Result<SafeShareExport, String> {
    require_unlocked(&state).await?;
    state.manager.safe_share_document(payload).await
}

#[tauri::command]
pub(crate) async fn memo_standards_list(
    state: State<'_, MemoDesktopState>,
) -> Result<Vec<StandardEntry>, String> {
    require_unlocked(&state).await?;
    state.manager.standards_list().await
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StandardsChecklistRequest {
    doc_id: Option<String>,
}

#[tauri::command]
pub(crate) async fn memo_standards_checklist(
    state: State<'_, MemoDesktopState>,
    payload: StandardsChecklistRequest,
) -> Result<StandardsChecklistResponse, String> {
    require_unlocked(&state).await?;
    state
        .manager
        .standards_checklist(payload.doc_id.as_deref())
        .await
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ChecklistStatusRequest {
    doc_id: Option<String>,
    item_id: String,
    status: String,
    note: Option<String>,
}

#[tauri::command]
pub(crate) async fn memo_standards_update_checklist_status(
    state: State<'_, MemoDesktopState>,
    payload: ChecklistStatusRequest,
) -> Result<ChecklistItem, String> {
    require_unlocked(&state).await?;
    let status = ChecklistStatus::from_action(&payload.status)
        .ok_or_else(|| "Invalid checklist status".to_string())?;
    state
        .manager
        .update_checklist_item_status(payload.doc_id, &payload.item_id, status, payload.note)
        .await
}

#[tauri::command]
pub(crate) async fn memo_list_secrets(
    state: State<'_, MemoDesktopState>,
) -> Result<Vec<SecretListItem>, String> {
    require_unlocked(&state).await?;
    state.manager.list_secrets().await
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RevealSecretRequest {
    id: String,
}

#[tauri::command]
pub(crate) async fn memo_reveal_secret(
    state: State<'_, MemoDesktopState>,
    payload: RevealSecretRequest,
) -> Result<SecretRevealResponse, String> {
    require_unlocked(&state).await?;
    state.manager.reveal_secret(&payload.id).await
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ChangeMasterPasswordRequest {
    current_password: String,
    new_password: String,
}

#[tauri::command]
pub(crate) async fn memo_change_master_password(
    state: State<'_, MemoDesktopState>,
    payload: ChangeMasterPasswordRequest,
) -> Result<ChangeMasterPasswordResponse, String> {
    require_unlocked(&state).await?;
    state
        .manager
        .change_master_password(&payload.current_password, &payload.new_password)
        .await
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SaveDocRequest {
    id: Option<String>,
    file_name: String,
    title: String,
    markdown: String,
    secrets: HashMap<String, String>,
    summary: String,
}

#[tauri::command]
pub(crate) async fn memo_save_document(
    state: State<'_, MemoDesktopState>,
    payload: SaveDocRequest,
) -> Result<SaveDocumentResponse, String> {
    require_unlocked(&state).await?;
    state
        .manager
        .save_document_with_risk_diff(
            payload.id,
            &payload.file_name,
            &payload.title,
            &payload.markdown,
            payload.secrets,
            &payload.summary,
        )
        .await
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DraftRequest {
    raw_input: String,
}

#[tauri::command]
pub(crate) async fn memo_draft_document(
    state: State<'_, MemoDesktopState>,
    payload: DraftRequest,
) -> Result<DraftResponse, String> {
    require_unlocked(&state).await?;
    state
        .manager
        .draft_document_with_ai(&payload.raw_input)
        .await
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DeleteRequest {
    id: String,
}

#[tauri::command]
pub(crate) async fn memo_delete_document(
    state: State<'_, MemoDesktopState>,
    payload: DeleteRequest,
) -> Result<HashMap<String, bool>, String> {
    require_unlocked(&state).await?;
    state.manager.delete_document(&payload.id).await?;
    Ok(ok_response())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryRequest {
    pub query: String,
    pub context: Option<String>,
}

#[tauri::command]
pub(crate) async fn memo_query(
    state: State<'_, MemoDesktopState>,
    payload: QueryRequest,
) -> Result<SearchAnswerResponse, String> {
    require_unlocked(&state).await?;
    state.manager.search_and_answer(&payload.query, payload.context.as_deref()).await
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ChatResponse {
    answer: String,
}

#[tauri::command]
pub(crate) async fn memo_chat(
    state: State<'_, MemoDesktopState>,
    payload: QueryRequest,
) -> Result<ChatResponse, String> {
    require_unlocked(&state).await?;
    let answer = state.manager.chat_with_ai(&payload.query).await?;
    Ok(ChatResponse { answer })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BackupRequest {
    local_backup_dir: Option<String>,
    webdav_url: Option<String>,
    webdav_user: Option<String>,
    webdav_pass: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BackupResponse {
    message: String,
}

#[tauri::command]
pub(crate) async fn memo_backup(
    state: State<'_, MemoDesktopState>,
    payload: BackupRequest,
) -> Result<BackupResponse, String> {
    require_unlocked(&state).await?;
    let webdav_config = build_webdav_config(
        payload.webdav_url.as_deref(),
        payload.webdav_user.as_deref(),
        payload.webdav_pass.as_deref(),
    )?;

    let message = state
        .manager
        .backup(payload.local_backup_dir.as_deref(), webdav_config)
        .await?;
    Ok(BackupResponse { message })
}

fn build_webdav_config(
    url: Option<&str>,
    username: Option<&str>,
    password: Option<&str>,
) -> Result<Option<WebDavConfig>, String> {
    let url = url.map(str::trim).unwrap_or_default();
    let username = username.map(str::trim).unwrap_or_default();
    let password = password.map(str::trim).unwrap_or_default();
    let filled_count = [url, username, password]
        .iter()
        .filter(|value| !value.is_empty())
        .count();

    if filled_count == 0 {
        return Ok(None);
    }

    if filled_count != 3 {
        return Err(
            "WebDAV backup config is incomplete. URL, account and password must be filled together."
                .to_string(),
        );
    }

    Ok(Some(WebDavConfig {
        url: url.to_string(),
        username: username.to_string(),
        password: Some(password.to_string()),
    }))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RestoreRequest {
    zip_path: String,
}

#[tauri::command]
pub(crate) async fn memo_restore(
    state: State<'_, MemoDesktopState>,
    payload: RestoreRequest,
) -> Result<HashMap<String, bool>, String> {
    require_unlocked(&state).await?;
    state.manager.restore(Path::new(&payload.zip_path)).await?;
    Ok(ok_response())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TranslateKeyRequest {
    text: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TranslateKeyResponse {
    key: String,
}

#[tauri::command]
pub(crate) async fn memo_translate_key(
    state: State<'_, MemoDesktopState>,
    payload: TranslateKeyRequest,
) -> Result<TranslateKeyResponse, String> {
    require_unlocked(&state).await?;
    let client = state.manager.get_ollama_client();
    let prompt = format!(
        "You are an API key formatting assistant.
Translate or convert the following text into a single concise camelCase English identifier (alphanumeric only, no spaces, hyphens, or underscores).
Do not output any markdown formatting, quotes, explanations, or punctuation. Output ONLY the raw camelCase string.

Text: {}
Identifier:",
        payload.text
    );

    let messages = vec![ChatMessage {
        role: "user".to_string(),
        content: prompt,
    }];

    let result = client.chat(messages, false).await?;
    let cleaned = result
        .trim()
        .replace(['"', '`'], "")
        .chars()
        .filter(|character| character.is_alphanumeric())
        .collect();

    Ok(TranslateKeyResponse { key: cleaned })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MigrateDataDirRequest {
    target_dir: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MigrateDataDirResponse {
    ok: bool,
    message: String,
    backup_path: String,
    target_dir: String,
}

#[tauri::command]
pub(crate) async fn memo_migrate_data_dir(
    state: State<'_, MemoDesktopState>,
    payload: MigrateDataDirRequest,
) -> Result<MigrateDataDirResponse, String> {
    require_unlocked(&state).await?;
    migrate_data_dir_impl(&state, &payload.target_dir).await
}

async fn require_unlocked(state: &MemoDesktopState) -> Result<(), String> {
    if state.manager.is_locked().await {
        Err("Vault is locked. Please unlock first.".to_string())
    } else {
        Ok(())
    }
}

async fn migrate_data_dir_impl(
    state: &MemoDesktopState,
    target_dir: &str,
) -> Result<MigrateDataDirResponse, String> {
    let target_dir = resolve_target_data_dir(target_dir)?;
    fs::create_dir_all(&target_dir)
        .map_err(|error| format!("Failed to create target data directory: {error:?}"))?;

    let active_dir = fs::canonicalize(&state.active_data_dir)
        .map_err(|error| format!("Failed to resolve active data directory: {error:?}"))?;
    let target_dir = fs::canonicalize(&target_dir)
        .map_err(|error| format!("Failed to resolve target data directory: {error:?}"))?;

    validate_migration_target(&active_dir, &target_dir)?;
    ensure_target_directory_empty(&target_dir)?;
    verify_directory_writable(&target_dir)?;

    let backup_dir = target_dir.join(".rusttool-migration-backups");
    let backup_dir_string = path_to_string(&backup_dir);
    state.manager.backup(Some(&backup_dir_string), None).await?;

    copy_core_data_files(&active_dir, &target_dir)?;
    write_local_config(
        &state.default_data_dir,
        &LocalConfig {
            custom_data_dir: Some(path_to_string(&target_dir)),
            ..Default::default()
        },
    )?;

    state.manager.lock().await;

    Ok(MigrateDataDirResponse {
        ok: true,
        message: "资料库已迁移到新目录。旧目录已保留，请重启应用后重新解锁。".to_string(),
        backup_path: backup_dir_string,
        target_dir: path_to_string(&target_dir),
    })
}

fn ok_response() -> HashMap<String, bool> {
    HashMap::from([("ok".to_string(), true)])
}

fn data_dir_response(state: &MemoDesktopState, custom_data_dir: Option<String>) -> DataDirResponse {
    let custom_data_dir = custom_data_dir.filter(|value| !value.trim().is_empty());
    let using_custom_data_dir = custom_data_dir.is_some();
    DataDirResponse {
        default_data_dir: path_to_string(&state.default_data_dir),
        active_data_dir: path_to_string(&state.active_data_dir),
        custom_data_dir,
        using_custom_data_dir,
        config_path: path_to_string(&get_local_config_path(&state.default_data_dir)),
    }
}

#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(default)]
struct LocalConfig {
    #[serde(alias = "customDataDir")]
    custom_data_dir: Option<String>,
    #[serde(flatten)]
    extra: Map<String, Value>,
}

fn get_default_base_dir() -> PathBuf {
    if let Ok(dir_str) = std::env::var("RUSTTOOL_DATA_DIR") {
        PathBuf::from(dir_str)
    } else if cfg!(windows) {
        std::env::var("APPDATA")
            .map(PathBuf::from)
            .map(|base| base.join("rust-tool"))
            .unwrap_or_else(|_| PathBuf::from(".").join("memos_data"))
    } else if cfg!(target_os = "macos") {
        std::env::var("HOME")
            .map(PathBuf::from)
            .map(|home| {
                home.join("Library")
                    .join("Application Support")
                    .join("rust-tool")
            })
            .unwrap_or_else(|_| PathBuf::from(".").join("memos_data"))
    } else if let Ok(app_data) = std::env::var("APPDATA") {
        PathBuf::from(app_data).join("rust-tool")
    } else if cfg!(unix) {
        std::env::var("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|_| std::env::var("HOME").map(|home| PathBuf::from(home).join(".local/share")))
            .map(|base| base.join("rust-tool"))
            .unwrap_or_else(|_| PathBuf::from(".").join("memos_data"))
    } else {
        PathBuf::from(".").join("memos_data")
    }
}

fn get_local_config_path(default_dir: &Path) -> PathBuf {
    default_dir.join("config.json")
}

fn read_local_config(default_dir: &Path) -> LocalConfig {
    let config_path = get_local_config_path(default_dir);
    if config_path.exists() {
        if let Ok(content) = fs::read_to_string(&config_path) {
            if let Ok(cfg) = serde_json::from_str::<LocalConfig>(&content) {
                return cfg;
            }
        }
    }
    LocalConfig::default()
}

fn write_local_config(default_dir: &Path, config: &LocalConfig) -> Result<(), String> {
    fs::create_dir_all(default_dir)
        .map_err(|error| format!("Failed to create config directory: {error:?}"))?;
    let mut merged = read_local_config(default_dir);
    merged.custom_data_dir = config.custom_data_dir.clone();
    let config_path = get_local_config_path(default_dir);
    let json = serde_json::to_string_pretty(&merged)
        .map_err(|error| format!("Failed to serialize local config: {error:?}"))?;
    fs::write(&config_path, json)
        .map_err(|error| format!("Failed to write local config: {error:?}"))
}

fn resolve_memo_data_dir(default_dir: &Path) -> PathBuf {
    let cfg = read_local_config(default_dir);
    if let Some(custom) = cfg.custom_data_dir {
        if !custom.trim().is_empty() {
            return PathBuf::from(custom);
        }
    }
    default_dir.to_path_buf()
}

fn resolve_target_data_dir(input: &str) -> Result<PathBuf, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("Target directory cannot be empty.".to_string());
    }

    let expanded = expand_home_path(trimmed);
    if !expanded.is_absolute() {
        return Err("Target directory must be an absolute path, or start with ~/.".to_string());
    }

    Ok(expanded)
}

fn expand_home_path(input: &str) -> PathBuf {
    if input == "~" || input.starts_with("~/") || input.starts_with("~\\") {
        if let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
            let home = PathBuf::from(home);
            if input == "~" {
                return home;
            }
            return home.join(
                input
                    .trim_start_matches('~')
                    .trim_start_matches(['/', '\\']),
            );
        }
    }

    PathBuf::from(input)
}

fn validate_migration_target(active_dir: &Path, target_dir: &Path) -> Result<(), String> {
    if target_dir == active_dir {
        return Err("Target directory is already the active data directory.".to_string());
    }
    if target_dir.starts_with(active_dir) {
        return Err("Target directory cannot be inside the current data directory.".to_string());
    }
    if active_dir.starts_with(target_dir) {
        return Err(
            "Target directory cannot be a parent of the current data directory.".to_string(),
        );
    }
    Ok(())
}

fn ensure_target_directory_empty(target_dir: &Path) -> Result<(), String> {
    for entry in fs::read_dir(target_dir)
        .map_err(|error| format!("Failed to read target data directory: {error:?}"))?
    {
        let entry = entry.map_err(|error| format!("Failed to read target entry: {error:?}"))?;
        if entry.file_name().to_string_lossy() == ".DS_Store" {
            continue;
        }
        return Err("Target directory must be empty before migration.".to_string());
    }
    Ok(())
}

fn verify_directory_writable(target_dir: &Path) -> Result<(), String> {
    let test_path = target_dir.join(".rusttool_write_test");
    fs::write(&test_path, b"ok")
        .map_err(|error| format!("Target directory is not writable: {error:?}"))?;
    fs::remove_file(&test_path)
        .map_err(|error| format!("Failed to clean target write test file: {error:?}"))
}

fn copy_core_data_files(active_dir: &Path, target_dir: &Path) -> Result<(), String> {
    copy_file_if_exists(
        &active_dir.join("config.json"),
        &target_dir.join("config.json"),
    )?;
    copy_file_if_exists(
        &active_dir.join("secrets.kdbx"),
        &target_dir.join("secrets.kdbx"),
    )?;
    copy_dir_if_exists(&active_dir.join("documents"), &target_dir.join("documents"))?;
    copy_dir_if_exists(
        &active_dir.join("embeddings"),
        &target_dir.join("embeddings"),
    )?;
    copy_dir_if_exists(&active_dir.join("indexes"), &target_dir.join("indexes"))?;
    copy_dir_if_exists(
        &active_dir.join("governance"),
        &target_dir.join("governance"),
    )?;
    copy_dir_if_exists(&active_dir.join("reports"), &target_dir.join("reports"))?;
    copy_dir_if_exists(&active_dir.join("standards"), &target_dir.join("standards"))?;
    Ok(())
}

fn copy_file_if_exists(source: &Path, target: &Path) -> Result<(), String> {
    if !source.exists() {
        return Ok(());
    }
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Failed to create target parent directory: {error:?}"))?;
    }
    fs::copy(source, target)
        .map(|_| ())
        .map_err(|error| format!("Failed to copy data file {}: {error:?}", source.display()))
}

fn copy_dir_if_exists(source: &Path, target: &Path) -> Result<(), String> {
    if !source.exists() {
        return Ok(());
    }
    fs::create_dir_all(target)
        .map_err(|error| format!("Failed to create target directory: {error:?}"))?;
    for entry in fs::read_dir(source)
        .map_err(|error| format!("Failed to read source directory: {error:?}"))?
    {
        let entry = entry.map_err(|error| format!("Failed to read source entry: {error:?}"))?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        let file_type = entry
            .file_type()
            .map_err(|error| format!("Failed to read source file type: {error:?}"))?;
        if file_type.is_dir() {
            copy_dir_if_exists(&source_path, &target_path)?;
        } else if file_type.is_file() {
            copy_file_if_exists(&source_path, &target_path)?;
        }
    }
    Ok(())
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

#[tauri::command]
pub async fn memo_get_tree_state(state: State<'_, MemoDesktopState>) -> Result<serde_json::Value, String> {
    require_unlocked(&state).await?;
    state.manager.read_tree_state()
}

#[tauri::command]
pub async fn memo_set_tree_state(state: State<'_, MemoDesktopState>, payload: serde_json::Value) -> Result<serde_json::Value, String> {
    require_unlocked(&state).await?;
    
    // The payload comes wrapped as {"payload": {"state": ...}} from memoApi.ts
    // Wait, let's see how I send it in memoApi.ts. If I send `payload`, it's just the object.
    let tree_state = if payload.get("state").is_some() {
        &payload["state"]
    } else {
        &payload
    };

    state.manager.write_tree_state(tree_state)?;
    Ok(serde_json::json!({ "success": true }))
}

#[tauri::command]
pub async fn memo_rename_folder(state: State<'_, MemoDesktopState>, payload: serde_json::Value) -> Result<serde_json::Value, String> {
    require_unlocked(&state).await?;
    let old_path = payload["oldPath"].as_str().unwrap_or_default();
    let new_path = payload["newPath"].as_str().unwrap_or_default();
    state.manager.rename_folder(old_path, new_path)?;
    Ok(serde_json::json!({ "success": true }))
}

#[tauri::command]
pub async fn memo_delete_folder(state: State<'_, MemoDesktopState>, payload: serde_json::Value) -> Result<serde_json::Value, String> {
    require_unlocked(&state).await?;
    let path = payload["path"].as_str().unwrap_or_default();
    state.manager.delete_folder_to_unarchived(path)?;
    Ok(serde_json::json!({ "success": true }))
}
