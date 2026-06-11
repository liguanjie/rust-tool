use crate::app::AppState;
use axum::{
    extract::{Path as AxumPath, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use rust_tool_core::memo::{
    ChatMessage, DocumentDetail, DraftResponse, MemoMetadata, SearchAnswerResponse, WebDavConfig,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

type MemoResult<T> = Result<Json<T>, MemoApiError>;

#[derive(Debug)]
pub struct MemoApiError {
    status: StatusCode,
    code: &'static str,
    message: String,
}

#[derive(Serialize)]
pub struct MemoErrorResponse {
    error: MemoErrorBody,
}

#[derive(Serialize)]
pub struct MemoErrorBody {
    code: &'static str,
    message: String,
}

impl MemoApiError {
    fn locked() -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            code: "vault_locked",
            message: "Vault is locked. Please unlock first.".to_string(),
        }
    }

    fn from_message(message: String) -> Self {
        let status =
            if message.contains("Document not found") || message.contains("Secret not found") {
                StatusCode::NOT_FOUND
            } else if message.contains("Current master password is incorrect") {
                StatusCode::UNAUTHORIZED
            } else if message.contains("Invalid file path")
                || message.contains("already exists")
                || message.contains("RustTool memo data")
                || message.contains("Backup archive")
                || message.contains("WebDAV backup config")
                || message.contains("data directory")
                || message.contains("Target directory")
                || message.contains("Secret id cannot be empty")
                || message.contains("master password cannot be empty")
                || message.contains("Master password is not initialized")
                || message.contains("New master password must be different")
            {
                StatusCode::BAD_REQUEST
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };

        Self {
            status,
            code: if status == StatusCode::NOT_FOUND {
                "not_found"
            } else if status == StatusCode::UNAUTHORIZED {
                "unauthorized"
            } else if status == StatusCode::BAD_REQUEST {
                "bad_request"
            } else {
                "internal_error"
            },
            message,
        }
    }
}

impl From<String> for MemoApiError {
    fn from(value: String) -> Self {
        Self::from_message(value)
    }
}

impl IntoResponse for MemoApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(MemoErrorResponse {
                error: MemoErrorBody {
                    code: self.code,
                    message: self.message,
                },
            }),
        )
            .into_response()
    }
}

#[derive(Deserialize)]
pub struct UnlockRequest {
    pub password: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnlockResponse {
    pub unlocked: bool,
}

pub async fn unlock(
    State(state): State<AppState>,
    Json(payload): Json<UnlockRequest>,
) -> MemoResult<UnlockResponse> {
    let unlocked = state.memo_manager.unlock(&payload.password).await?;
    Ok(Json(UnlockResponse { unlocked }))
}

pub async fn lock(State(state): State<AppState>) -> MemoResult<HashMap<String, bool>> {
    state.memo_manager.lock().await;
    let mut res = HashMap::new();
    res.insert("ok".to_string(), true);
    Ok(Json(res))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusResponse {
    pub unlocked: bool,
    pub ollama_url: String,
    pub has_api_key: bool,
    pub chat_model: String,
    pub embedding_model: String,
    pub reasoning_effort: String,
    pub disable_response_storage: bool,
    pub allow_ai_secrets: bool,
    pub custom_data_dir: Option<String>,
}

pub async fn status(State(state): State<AppState>) -> MemoResult<StatusResponse> {
    let unlocked = !state.memo_manager.is_locked().await;
    let (
        ollama_url,
        _api_key,
        chat_model,
        embedding_model,
        reasoning_effort,
        disable_response_storage,
        allow_ai_secrets,
    ) = state.memo_manager.get_llm_config();

    let local_config = crate::app::read_local_config(&state.default_data_dir);

    Ok(Json(StatusResponse {
        unlocked,
        ollama_url,
        has_api_key: state.memo_manager.has_llm_api_key()?,
        chat_model,
        embedding_model,
        reasoning_effort,
        disable_response_storage,
        allow_ai_secrets,
        custom_data_dir: local_config.custom_data_dir,
    }))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataDirResponse {
    pub default_data_dir: String,
    pub active_data_dir: String,
    pub custom_data_dir: Option<String>,
    pub using_custom_data_dir: bool,
    pub config_path: String,
}

pub async fn data_dir(State(state): State<AppState>) -> MemoResult<DataDirResponse> {
    let local_config = crate::app::read_local_config(&state.default_data_dir);
    Ok(Json(data_dir_response(
        &state,
        local_config.custom_data_dir,
    )))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrateDataDirRequest {
    pub target_dir: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrateDataDirResponse {
    pub ok: bool,
    pub message: String,
    pub backup_path: String,
    pub target_dir: String,
}

pub async fn migrate_data_dir(
    State(state): State<AppState>,
    Json(payload): Json<MigrateDataDirRequest>,
) -> MemoResult<MigrateDataDirResponse> {
    require_unlocked(&state).await?;
    let response = migrate_data_dir_impl(&state, &payload.target_dir).await?;
    Ok(Json(response))
}

async fn migrate_data_dir_impl(
    state: &AppState,
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
    state
        .memo_manager
        .backup(Some(&backup_dir_string), None)
        .await?;

    copy_core_data_files(&active_dir, &target_dir)?;

    crate::app::write_local_config(
        &state.default_data_dir,
        &crate::app::LocalConfig {
            custom_data_dir: Some(path_to_string(&target_dir)),
            ..Default::default()
        },
    )?;

    state.memo_manager.lock().await;

    Ok(MigrateDataDirResponse {
        ok: true,
        message: "资料库已迁移到新目录。旧目录已保留，请重启应用后重新解锁。".to_string(),
        backup_path: backup_dir_string,
        target_dir: path_to_string(&target_dir),
    })
}

fn data_dir_response(state: &AppState, custom_data_dir: Option<String>) -> DataDirResponse {
    let custom_data_dir = custom_data_dir.filter(|value| !value.trim().is_empty());
    let using_custom_data_dir = custom_data_dir.is_some();
    DataDirResponse {
        default_data_dir: path_to_string(&state.default_data_dir),
        active_data_dir: path_to_string(&state.active_data_dir),
        custom_data_dir,
        using_custom_data_dir,
        config_path: path_to_string(&crate::app::get_local_config_path(&state.default_data_dir)),
    }
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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsRequest {
    pub ollama_url: String,
    pub api_key: Option<String>,
    pub chat_model: String,
    pub embedding_model: String,
    pub reasoning_effort: String,
    pub disable_response_storage: bool,
    pub allow_ai_secrets: bool,
}

async fn require_unlocked(state: &AppState) -> Result<(), MemoApiError> {
    if state.memo_manager.is_locked().await {
        Err(MemoApiError::locked())
    } else {
        Ok(())
    }
}

pub async fn update_settings(
    State(state): State<AppState>,
    Json(payload): Json<SettingsRequest>,
) -> MemoResult<HashMap<String, bool>> {
    require_unlocked(&state).await?;
    state
        .memo_manager
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

    let mut res = HashMap::new();
    res.insert("ok".to_string(), true);
    Ok(Json(res))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestConnectionRequest {
    pub ollama_url: String,
    pub api_key: Option<String>,
    pub chat_model: String,
    pub embedding_model: String,
    pub reasoning_effort: Option<String>,
    pub disable_response_storage: Option<bool>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestConnectionResponse {
    pub ok: bool,
    pub message: String,
}

pub async fn test_connection(
    State(state): State<AppState>,
    Json(payload): Json<TestConnectionRequest>,
) -> MemoResult<TestConnectionResponse> {
    require_unlocked(&state).await?;

    let (_, saved_api_key, _, _, saved_reasoning_effort, saved_disable_response_storage, _) =
        state.memo_manager.get_llm_config();
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

    Ok(Json(TestConnectionResponse {
        ok: true,
        message: format!("连接成功，模型响应：{}", reply.trim()),
    }))
}

pub async fn list_documents(State(state): State<AppState>) -> MemoResult<Vec<MemoMetadata>> {
    require_unlocked(&state).await?;
    let docs = state.memo_manager.get_store().get_all_memos()?;
    Ok(Json(docs))
}

pub async fn get_document(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<String>,
) -> MemoResult<DocumentDetail> {
    require_unlocked(&state).await?;
    let detail = state.memo_manager.get_document(&id).await?;
    Ok(Json(detail))
}

pub async fn list_secrets(
    State(state): State<AppState>,
) -> MemoResult<Vec<rust_tool_core::memo::SecretListItem>> {
    require_unlocked(&state).await?;
    let secrets = state.memo_manager.list_secrets().await?;
    Ok(Json(secrets))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RevealSecretRequest {
    pub id: String,
}

pub async fn reveal_secret(
    State(state): State<AppState>,
    Json(payload): Json<RevealSecretRequest>,
) -> MemoResult<rust_tool_core::memo::SecretRevealResponse> {
    require_unlocked(&state).await?;
    let secret = state.memo_manager.reveal_secret(&payload.id).await?;
    Ok(Json(secret))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeMasterPasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

pub async fn change_master_password(
    State(state): State<AppState>,
    Json(payload): Json<ChangeMasterPasswordRequest>,
) -> MemoResult<rust_tool_core::memo::ChangeMasterPasswordResponse> {
    require_unlocked(&state).await?;
    let response = state
        .memo_manager
        .change_master_password(&payload.current_password, &payload.new_password)
        .await?;
    Ok(Json(response))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveDocRequest {
    pub id: Option<String>,
    pub file_name: String,
    pub title: String,
    pub markdown: String,
    pub secrets: HashMap<String, String>,
    pub summary: String,
}

pub async fn save_document(
    State(state): State<AppState>,
    Json(payload): Json<SaveDocRequest>,
) -> MemoResult<MemoMetadata> {
    require_unlocked(&state).await?;
    let meta = state
        .memo_manager
        .save_document(
            payload.id,
            &payload.file_name,
            &payload.title,
            &payload.markdown,
            payload.secrets,
            &payload.summary,
        )
        .await?;
    Ok(Json(meta))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftRequest {
    pub raw_input: String,
}

pub async fn draft_document(
    State(state): State<AppState>,
    Json(payload): Json<DraftRequest>,
) -> MemoResult<DraftResponse> {
    require_unlocked(&state).await?;
    let draft = state
        .memo_manager
        .draft_document_with_ai(&payload.raw_input)
        .await?;
    Ok(Json(draft))
}

#[derive(Deserialize)]
pub struct DeleteRequest {
    pub id: String,
}

pub async fn delete_document(
    State(state): State<AppState>,
    Json(payload): Json<DeleteRequest>,
) -> MemoResult<HashMap<String, bool>> {
    require_unlocked(&state).await?;
    state.memo_manager.delete_document(&payload.id).await?;
    let mut res = HashMap::new();
    res.insert("ok".to_string(), true);
    Ok(Json(res))
}

#[derive(Deserialize)]
pub struct QueryRequest {
    pub query: String,
}

pub async fn query_memos(
    State(state): State<AppState>,
    Json(payload): Json<QueryRequest>,
) -> MemoResult<SearchAnswerResponse> {
    require_unlocked(&state).await?;
    let answer = state.memo_manager.search_and_answer(&payload.query).await?;
    Ok(Json(answer))
}

#[derive(Serialize)]
pub struct ChatResponse {
    pub answer: String,
}

pub async fn chat(
    State(state): State<AppState>,
    Json(payload): Json<QueryRequest>,
) -> MemoResult<ChatResponse> {
    require_unlocked(&state).await?;
    let answer = state.memo_manager.chat_with_ai(&payload.query).await?;
    Ok(Json(ChatResponse { answer }))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupRequest {
    pub local_backup_dir: Option<String>,
    pub webdav_url: Option<String>,
    pub webdav_user: Option<String>,
    pub webdav_pass: Option<String>,
}

#[derive(Serialize)]
pub struct BackupResponse {
    pub message: String,
}

pub async fn backup_memos(
    State(state): State<AppState>,
    Json(payload): Json<BackupRequest>,
) -> MemoResult<BackupResponse> {
    require_unlocked(&state).await?;
    let webdav_config = build_webdav_config(
        payload.webdav_url.as_deref(),
        payload.webdav_user.as_deref(),
        payload.webdav_pass.as_deref(),
    )?;

    let message = state
        .memo_manager
        .backup(payload.local_backup_dir.as_deref(), webdav_config)
        .await?;

    Ok(Json(BackupResponse { message }))
}

fn build_webdav_config(
    url: Option<&str>,
    username: Option<&str>,
    password: Option<&str>,
) -> Result<Option<WebDavConfig>, MemoApiError> {
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
        return Err(MemoApiError::from_message(
            "WebDAV backup config is incomplete. URL, account and password must be filled together."
                .to_string(),
        ));
    }

    Ok(Some(WebDavConfig {
        url: url.to_string(),
        username: username.to_string(),
        password: Some(password.to_string()),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn webdav_config_allows_empty_settings() {
        let config = build_webdav_config(None, None, None).expect("empty config");
        assert!(config.is_none());
    }

    #[test]
    fn webdav_config_rejects_partial_settings() {
        let err = build_webdav_config(Some("https://dav.example.com/backup"), Some("ben"), None)
            .expect_err("partial config should fail");

        assert_eq!(err.status, StatusCode::BAD_REQUEST);
        assert_eq!(err.code, "bad_request");
        assert!(err.message.contains("WebDAV backup config is incomplete"));
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreRequest {
    pub zip_path: String,
}

pub async fn restore_memos(
    State(state): State<AppState>,
    Json(payload): Json<RestoreRequest>,
) -> MemoResult<HashMap<String, bool>> {
    require_unlocked(&state).await?;
    let path = PathBuf::from(payload.zip_path);
    state.memo_manager.restore(&path).await?;
    let mut res = HashMap::new();
    res.insert("ok".to_string(), true);
    Ok(Json(res))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslateKeyRequest {
    pub text: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslateKeyResponse {
    pub key: String,
}

pub async fn translate_key(
    State(state): State<AppState>,
    Json(payload): Json<TranslateKeyRequest>,
) -> MemoResult<TranslateKeyResponse> {
    require_unlocked(&state).await?;
    let client = state.memo_manager.get_ollama_client();
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

    let mut result = client.chat(messages, false).await?;
    result = result.trim().replace("\"", "").replace("`", "").to_string();

    // Clean up to ensure it is alphanumeric camelCase
    let cleaned: String = result.chars().filter(|c| c.is_alphanumeric()).collect();

    Ok(Json(TranslateKeyResponse { key: cleaned }))
}
