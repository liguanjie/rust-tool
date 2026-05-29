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
use std::path::PathBuf;

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
        let status = if message.contains("Document not found") {
            StatusCode::NOT_FOUND
        } else if message.contains("Invalid file path")
            || message.contains("already exists")
            || message.contains("missing memos.db")
            || message.contains("Backup archive")
        {
            StatusCode::BAD_REQUEST
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        };

        Self {
            status,
            code: if status == StatusCode::NOT_FOUND {
                "not_found"
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
        api_key,
        chat_model,
        embedding_model,
        reasoning_effort,
        disable_response_storage,
        allow_ai_secrets,
    ) = state.memo_manager.get_llm_config();

    let default_dir = crate::app::get_default_base_dir();
    let config_path = default_dir.join("config.json");
    let mut custom_data_dir = None;
    if config_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&config_path) {
            if let Ok(cfg) = serde_json::from_str::<crate::app::LocalConfig>(&content) {
                custom_data_dir = cfg.custom_data_dir;
            }
        }
    }

    Ok(Json(StatusResponse {
        unlocked,
        ollama_url,
        has_api_key: !api_key.trim().is_empty(),
        chat_model,
        embedding_model,
        reasoning_effort,
        disable_response_storage,
        allow_ai_secrets,
        custom_data_dir,
    }))
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
    pub custom_data_dir: Option<String>,
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
    state.memo_manager.update_llm_config(
        &payload.ollama_url,
        payload.api_key.as_deref(),
        &payload.chat_model,
        &payload.embedding_model,
        &payload.reasoning_effort,
        payload.disable_response_storage,
        payload.allow_ai_secrets,
    )?;

    let default_dir = crate::app::get_default_base_dir();
    let _ = std::fs::create_dir_all(&default_dir);
    let config_path = default_dir.join("config.json");
    let local_cfg = crate::app::LocalConfig {
        custom_data_dir: payload.custom_data_dir,
    };
    if let Ok(json_str) = serde_json::to_string_pretty(&local_cfg) {
        let _ = std::fs::write(config_path, json_str);
    }

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
    let webdav_config = match (payload.webdav_url, payload.webdav_user, payload.webdav_pass) {
        (Some(url), Some(user), pass) => Some(WebDavConfig {
            url,
            username: user,
            password: pass,
        }),
        _ => None,
    };

    let message = state
        .memo_manager
        .backup(payload.local_backup_dir.as_deref(), webdav_config)
        .await?;

    Ok(Json(BackupResponse { message }))
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
    state.memo_manager.restore(&path)?;
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
