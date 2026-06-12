pub mod assets;
pub mod audit;
pub mod backup;
pub mod crypto;
pub mod governance;
pub mod history;
pub mod markdown_store;
pub mod redactor;
pub mod reports;
pub mod secret_vault;
pub mod standards;
pub mod store;
pub mod vector;

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use self::secret_vault::{KdbxSecretVault, SecretMetadata, SecretVault};

pub use assets::{
    SecurityAsset, SecurityAssetDetail, SecurityAssetGraph, SecurityAssetType, SecurityGraphEdge,
    SecurityGraphEdgeType, SecurityGraphNode, SecurityGraphNodeType,
};
pub use audit::{
    AuditFixPreview, AuditScanResponse, AuditSummary, FindingKind, FindingSeverity, FindingStatus,
    SecurityFinding,
};
pub use governance::{AuditEvent, GovernanceSummary, SecurityCase, SecurityCaseStatus};
pub use history::{
    DocumentRiskDiff, DocumentRiskDiffItem, DocumentRiskDiffSummary, DocumentRiskSnapshot,
};
pub use redactor::DetectedSecret;
pub use reports::{
    SafeShareExport, SafeShareRequest, SecurityReport, SecurityReportRequest, SecurityReportScope,
};
pub use standards::{ChecklistItem, ChecklistStatus, StandardEntry, StandardsChecklistResponse};
pub use store::{current_timestamp, MemoMetadata, MemoStore};
pub use vector::{cosine_similarity, ChatMessage, LlmClient};

const LLM_API_KEY_SECRET_ID: &str = "__rusttool_config:llm_api_key";
const LLM_API_KEY_PRESENT_CONFIG: &str = "ollama_api_key_saved";
const LEGACY_LLM_API_KEY_CONFIG: &str = "ollama_api_key";

pub struct LlmConfig {
    pub base_url: String,
    pub api_key: String,
    pub chat_model: String,
    pub embedding_model: String,
    pub reasoning_effort: String,
    pub disable_response_storage: bool,
    pub allow_ai_secrets: bool,
}

pub struct MemoManager {
    store: MemoStore,
    llm_config: Arc<std::sync::RwLock<LlmConfig>>,
    // Store derived master key in memory when unlocked. None means locked.
    master_key: Arc<RwLock<Option<[u8; 32]>>>,
    // Keep the KDBX vault open only while the app is unlocked.
    secret_vault: Arc<RwLock<Option<KdbxSecretVault>>>,
    secret_vault_path: PathBuf,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveDocumentResponse {
    #[serde(flatten)]
    pub metadata: MemoMetadata,
    pub risk_diff: DocumentRiskDiff,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RedactMarkdownResponse {
    pub markdown: String,
    pub secrets: Vec<DetectedSecret>,
    pub redacted_secret_count: usize,
}

impl MemoManager {
    pub fn new(data_dir: &Path) -> Result<Self, String> {
        let store = MemoStore::new(data_dir)?;
        let secret_vault_path = store.get_secret_vault_path();

        // Read configs or set defaults
        let llm_base_url = store
            .get_config("ollama_base_url")?
            .unwrap_or_else(|| "https://api.openai.com/v1".to_string());
        let llm_chat_model = store
            .get_config("ollama_chat_model")?
            .unwrap_or_else(|| "gpt-5.5".to_string());
        let llm_embedding_model = store
            .get_config("ollama_embedding_model")?
            .unwrap_or_else(|| "text-embedding-3-small".to_string());
        let reasoning_effort = store
            .get_config("model_reasoning_effort")?
            .unwrap_or_else(|| "xhigh".to_string());
        let disable_response_storage = store
            .get_config("disable_response_storage")?
            .map(|v| v == "true")
            .unwrap_or(true);
        let allow_ai_secrets = store
            .get_config("allow_ai_secrets")?
            .map(|v| v == "true")
            .unwrap_or(false);

        let llm_config = Arc::new(std::sync::RwLock::new(LlmConfig {
            base_url: llm_base_url,
            // Loaded from KDBX after unlock. Do not keep API keys in JSON config.
            api_key: String::new(),
            chat_model: llm_chat_model,
            embedding_model: llm_embedding_model,
            reasoning_effort,
            disable_response_storage,
            allow_ai_secrets,
        }));

        Ok(Self {
            store,
            llm_config,
            master_key: Arc::new(RwLock::new(None)),
            secret_vault: Arc::new(RwLock::new(None)),
            secret_vault_path,
        })
    }

    pub fn get_store(&self) -> &MemoStore {
        &self.store
    }

    pub async fn is_locked(&self) -> bool {
        self.master_key.read().await.is_none()
    }

    /// Unlock the vault using the master password.
    pub async fn unlock(&self, password: &str) -> Result<bool, String> {
        let salt = self.store.get_or_create_salt()?;
        let derived = crypto::derive_key(password, &salt);

        // Check verification token
        match self.store.get_password_verifier()? {
            Some(verifier_str) => {
                // Try to decrypt the verifier token
                match crypto::decrypt(&verifier_str, &derived) {
                    Ok(decrypted_bytes) => {
                        let val = String::from_utf8_lossy(&decrypted_bytes);
                        if val == "verified_token" {
                            {
                                let mut lock = self.master_key.write().await;
                                *lock = Some(derived);
                            }
                            self.open_or_create_secret_vault(password).await?;
                            self.load_or_migrate_llm_api_key().await?;
                            Ok(true)
                        } else {
                            Ok(false)
                        }
                    }
                    Err(_) => Ok(false), // Password incorrect, decryption failed
                }
            }
            None => {
                if self.secret_vault_path.exists() {
                    let vault = match KdbxSecretVault::open(&self.secret_vault_path, password) {
                        Ok(vault) => vault,
                        Err(_) => return Ok(false),
                    };
                    let verifier_str = crypto::encrypt(b"verified_token", &derived)?;
                    self.store.set_password_verifier(&verifier_str)?;
                    {
                        let mut lock = self.master_key.write().await;
                        *lock = Some(derived);
                    }
                    {
                        let mut guard = self.secret_vault.write().await;
                        *guard = Some(vault);
                    }
                    self.load_or_migrate_llm_api_key().await?;
                    return Ok(true);
                }

                // No password set yet and no existing KDBX vault. Initialize a new vault.
                let verifier_str = crypto::encrypt(b"verified_token", &derived)?;
                self.store.set_password_verifier(&verifier_str)?;

                {
                    let mut lock = self.master_key.write().await;
                    *lock = Some(derived);
                }
                self.open_or_create_secret_vault(password).await?;
                self.load_or_migrate_llm_api_key().await?;
                Ok(true)
            }
        }
    }

    /// Lock the vault, erasing the key from memory.
    pub async fn lock(&self) {
        {
            let mut lock = self.master_key.write().await;
            *lock = None;
        }
        {
            let mut config = self.llm_config.write().unwrap();
            config.api_key.clear();
        }
        let mut vault = self.secret_vault.write().await;
        *vault = None;
    }

    /// Get current master key if unlocked, otherwise return error.
    async fn get_key(&self) -> Result<[u8; 32], String> {
        self.master_key
            .read()
            .await
            .ok_or_else(|| "Vault is locked. Please unlock first.".to_string())
    }

    async fn open_or_create_secret_vault(&self, password: &str) -> Result<(), String> {
        let vault = if self.secret_vault_path.exists() {
            KdbxSecretVault::open(&self.secret_vault_path, password)?
        } else {
            KdbxSecretVault::create(&self.secret_vault_path, password)?
        };

        let mut guard = self.secret_vault.write().await;
        *guard = Some(vault);
        Ok(())
    }

    async fn load_or_migrate_llm_api_key(&self) -> Result<(), String> {
        let legacy_api_key = self
            .store
            .get_config(LEGACY_LLM_API_KEY_CONFIG)?
            .unwrap_or_default();
        if !legacy_api_key.trim().is_empty() {
            self.store_llm_api_key(legacy_api_key.trim()).await?;
            return Ok(());
        }
        if self.store.get_config(LEGACY_LLM_API_KEY_CONFIG)?.is_some() {
            self.store.delete_config(LEGACY_LLM_API_KEY_CONFIG)?;
        }

        let api_key = {
            let guard = self.secret_vault.read().await;
            match guard.as_ref() {
                Some(vault) => vault.get_secret(LLM_API_KEY_SECRET_ID)?,
                None => None,
            }
        };

        {
            let mut config = self.llm_config.write().unwrap();
            config.api_key = api_key.clone().unwrap_or_default();
        }
        self.store.set_config(
            LLM_API_KEY_PRESENT_CONFIG,
            if api_key
                .as_deref()
                .map(|value| !value.trim().is_empty())
                .unwrap_or(false)
            {
                "true"
            } else {
                "false"
            },
        )?;
        Ok(())
    }

    async fn store_llm_api_key(&self, api_key: &str) -> Result<(), String> {
        let api_key = api_key.trim();
        if api_key.is_empty() {
            return Ok(());
        }

        {
            let mut guard = self.secret_vault.write().await;
            let vault = guard
                .as_mut()
                .ok_or_else(|| "Secret vault is locked. Please unlock first.".to_string())?;
            vault.put_secret(
                LLM_API_KEY_SECRET_ID,
                api_key,
                SecretMetadata {
                    label: Some("OpenAI-compatible API Key".to_string()),
                    document_path: None,
                },
            )?;
            vault.save()?;
        }

        {
            let mut config = self.llm_config.write().unwrap();
            config.api_key = api_key.to_string();
        }
        self.store.set_config(LLM_API_KEY_PRESENT_CONFIG, "true")?;
        self.store.delete_config(LEGACY_LLM_API_KEY_CONFIG)?;
        Ok(())
    }

    fn verify_master_password(&self, password: &str) -> Result<[u8; 32], String> {
        let salt = self.store.get_or_create_salt()?;
        let derived = crypto::derive_key(password, &salt);
        let verifier = self
            .store
            .get_password_verifier()?
            .ok_or_else(|| "Master password is not initialized.".to_string())?;
        let decrypted = crypto::decrypt(&verifier, &derived)
            .map_err(|_| "Current master password is incorrect.".to_string())?;
        if String::from_utf8_lossy(&decrypted) != "verified_token" {
            return Err("Current master password is incorrect.".to_string());
        }
        Ok(derived)
    }

    fn create_password_change_backup(&self) -> Result<PathBuf, String> {
        let data_dir = self.store.get_data_dir();
        let backup_dir = data_dir.join("password-change-backups");
        fs::create_dir_all(&backup_dir)
            .map_err(|error| format!("Failed to create password backup directory: {error:?}"))?;
        let backup_path = backup_dir.join(format!(
            "rust_tool_before_password_change_{}.zip",
            current_timestamp()
        ));
        backup::create_backup_zip(self.store.get_data_dir(), &backup_path)?;
        Ok(backup_path)
    }

    pub async fn change_master_password(
        &self,
        current_password: &str,
        new_password: &str,
    ) -> Result<ChangeMasterPasswordResponse, String> {
        let _current_key = self.get_key().await?;

        if current_password.is_empty() {
            return Err("Current master password cannot be empty.".to_string());
        }
        if new_password.is_empty() {
            return Err("New master password cannot be empty.".to_string());
        }
        if current_password == new_password {
            return Err(
                "New master password must be different from the current password.".to_string(),
            );
        }

        self.verify_master_password(current_password)?;
        if self.secret_vault_path.exists() {
            KdbxSecretVault::open(&self.secret_vault_path, current_password)?;
        }

        let backup_path = self.create_password_change_backup()?;
        let salt = self.store.get_or_create_salt()?;
        let new_key = crypto::derive_key(new_password, &salt);
        let new_verifier = crypto::encrypt(b"verified_token", &new_key)?;

        {
            let mut guard = self.secret_vault.write().await;
            if let Some(vault) = guard.as_mut() {
                vault.change_password(new_password)?;
            } else if self.secret_vault_path.exists() {
                let mut vault = KdbxSecretVault::open(&self.secret_vault_path, current_password)?;
                vault.change_password(new_password)?;
                *guard = Some(vault);
            }
        }

        if let Err(error) = self.store.set_password_verifier(&new_verifier) {
            let mut guard = self.secret_vault.write().await;
            if let Some(vault) = guard.as_mut() {
                let _ = vault.change_password(current_password);
            }
            return Err(error);
        }

        self.lock().await;

        Ok(ChangeMasterPasswordResponse {
            ok: true,
            message: "主密码已修改，请使用新主密码重新解锁。".to_string(),
            backup_path: backup_path.to_string_lossy().to_string(),
        })
    }

    async fn replace_document_secrets_in_kdbx(
        &self,
        doc_id: &str,
        file_name: &str,
        secrets: &HashMap<String, String>,
        previous_secret_keys: &[String],
    ) -> Result<(), String> {
        let mut guard = self.secret_vault.write().await;
        let vault = guard
            .as_mut()
            .ok_or_else(|| "Secret vault is locked. Please unlock first.".to_string())?;

        for old_key in previous_secret_keys {
            if !secrets.contains_key(old_key) {
                vault.delete_secret(&document_secret_entry_key(doc_id, old_key))?;
            }
        }

        for (secret_key, plain_value) in secrets {
            vault.put_secret(
                &document_secret_entry_key(doc_id, secret_key),
                plain_value,
                SecretMetadata {
                    label: Some(secret_key.to_string()),
                    document_path: Some(file_name.to_string()),
                },
            )?;
        }

        if !secrets.is_empty() || !previous_secret_keys.is_empty() {
            vault.save()?;
        }
        Ok(())
    }

    async fn read_document_secrets_from_kdbx(
        &self,
        doc_id: &str,
        markdown: &str,
    ) -> Result<HashMap<String, String>, String> {
        let mut decrypted_secrets = HashMap::new();
        let secret_keys = extract_secret_placeholders(markdown);
        if secret_keys.is_empty() {
            return Ok(decrypted_secrets);
        }

        let guard = self.secret_vault.read().await;
        let Some(vault) = guard.as_ref() else {
            return Ok(decrypted_secrets);
        };

        for secret_key in secret_keys {
            if let Some(value) =
                vault.get_secret(&document_secret_entry_key(doc_id, &secret_key))?
            {
                decrypted_secrets.insert(secret_key, value);
            }
        }

        Ok(decrypted_secrets)
    }

    async fn delete_document_secrets_from_kdbx(
        &self,
        doc_id: &str,
        secret_keys: &[String],
    ) -> Result<(), String> {
        if secret_keys.is_empty() {
            return Ok(());
        }

        let mut guard = self.secret_vault.write().await;
        let Some(vault) = guard.as_mut() else {
            return Ok(());
        };

        for secret_key in secret_keys {
            vault.delete_secret(&document_secret_entry_key(doc_id, secret_key))?;
        }
        vault.save()?;
        Ok(())
    }

    /// Update OpenAI-compatible LLM configuration.
    pub async fn update_llm_config(
        &self,
        base_url: &str,
        api_key: Option<&str>,
        chat_model: &str,
        embedding_model: &str,
        reasoning_effort: &str,
        disable_response_storage: bool,
        allow_ai_secrets: bool,
    ) -> Result<(), String> {
        let next_api_key = api_key
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        {
            let mut config = self.llm_config.write().unwrap();
            config.base_url = base_url.to_string();
            if let Some(api_key) = &next_api_key {
                config.api_key = api_key.clone();
            }
            config.chat_model = chat_model.to_string();
            config.embedding_model = embedding_model.to_string();
            config.reasoning_effort = reasoning_effort.to_string();
            config.disable_response_storage = disable_response_storage;
            config.allow_ai_secrets = allow_ai_secrets;
        }

        self.store.set_config("ollama_base_url", base_url)?;
        if let Some(api_key) = next_api_key.as_deref() {
            self.store_llm_api_key(api_key).await?;
        } else {
            self.store.delete_config(LEGACY_LLM_API_KEY_CONFIG)?;
        }
        self.store.set_config("ollama_chat_model", chat_model)?;
        self.store
            .set_config("ollama_embedding_model", embedding_model)?;
        self.store
            .set_config("model_reasoning_effort", reasoning_effort)?;
        self.store.set_config(
            "disable_response_storage",
            if disable_response_storage {
                "true"
            } else {
                "false"
            },
        )?;
        self.store.set_config(
            "allow_ai_secrets",
            if allow_ai_secrets { "true" } else { "false" },
        )?;
        Ok(())
    }

    pub fn has_llm_api_key(&self) -> Result<bool, String> {
        let in_memory = {
            let config = self.llm_config.read().unwrap();
            !config.api_key.trim().is_empty()
        };
        if in_memory {
            return Ok(true);
        }

        let encrypted_marker = self
            .store
            .get_config(LLM_API_KEY_PRESENT_CONFIG)?
            .map(|value| value == "true")
            .unwrap_or(false);
        let legacy_marker = self
            .store
            .get_config(LEGACY_LLM_API_KEY_CONFIG)?
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false);
        Ok(encrypted_marker || legacy_marker)
    }

    pub fn get_llm_config(&self) -> (String, String, String, String, String, bool, bool) {
        let config = self.llm_config.read().unwrap();
        (
            config.base_url.clone(),
            config.api_key.clone(),
            config.chat_model.clone(),
            config.embedding_model.clone(),
            config.reasoning_effort.clone(),
            config.disable_response_storage,
            config.allow_ai_secrets,
        )
    }

    pub fn get_ollama_client(&self) -> LlmClient {
        let config = self.llm_config.read().unwrap();
        LlmClient::new(
            &config.base_url,
            Some(&config.api_key),
            &config.chat_model,
            &config.embedding_model,
            Some(&config.reasoning_effort),
            config.disable_response_storage,
        )
    }

    pub async fn chat_with_ai(&self, query: &str) -> Result<String, String> {
        let client = self.get_ollama_client();
        let redacted_query = redactor::redact_secrets(query);
        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: "You are a concise assistant embedded in a local Markdown knowledge-base app. Answer normal chat directly. If the user asks to create, edit, or search local documents, briefly tell them what action to request in the app.".to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: redacted_query.text,
            },
        ];

        client.chat(messages, false).await
    }

    // --- Core feature 1: AI-assisted write ---

    pub async fn draft_document_with_ai(&self, raw_input: &str) -> Result<DraftResponse, String> {
        // Vault must be unlocked to process and encrypt secrets
        let _key = self.get_key().await?;

        let client = self.get_ollama_client();
        let redacted_input = redactor::redact_secrets(raw_input);

        let system_prompt = "You are a professional documentation organizer. Your task is to organize the user's raw, unstructured, or dictation notes into a well-structured, neat Markdown document.
The user input has already been redacted locally before it reaches you. Real passwords, API keys, credentials, secret tokens, and private keys have been replaced with placeholders like `{{secret:pending_1}}`.

Rules for secret placeholders:
1. Never invent, reveal, or ask for plaintext secret values.
2. Preserve every `{{secret:pending_n}}` placeholder in the Markdown, or rename it to a unique descriptive camelCase key such as `{{secret:mysqlPassword}}`.
3. If you rename a placeholder, add an entry to `secrets` where the final key maps to the original pending key.
   Example: `\"mysqlPassword\": \"pending_1\"`.
4. If you keep the pending key, add an entry like `\"pending_1\": \"pending_1\"`.

Your response MUST be a JSON object ONLY, with no extra markdown wrapping (do not use ```json or similar). The JSON structure:
{
  \"title\": \"A short, descriptive title of the document\",
  \"fileName\": \"A URL-safe filename ending with .md (e.g. vps_credentials.md)\",
  \"markdown\": \"The formatted Markdown content with secrets replaced by placeholders\",
  \"secrets\": {
    \"finalKey1\": \"pending_1\",
    \"finalKey2\": \"pending_2\"
  },
  \"summary\": \"A one-sentence summary of the document (max 50 characters)\"
}";

        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: system_prompt.to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: redacted_input.text.clone(),
            },
        ];

        let llm_json = client.chat(messages, true).await?;

        // Parse JSON response from LLM
        let draft: DraftResponse = serde_json::from_str(&llm_json).map_err(|e| {
            format!(
                "Failed to parse LLM JSON response: {:?}. Raw response: {}",
                e, llm_json
            )
        })?;

        Ok(resolve_redacted_draft_secrets(draft, &redacted_input))
    }

    /// Save a document, encrypting any secrets.
    pub async fn save_document(
        &self,
        id: Option<String>,
        file_name: &str,
        title: &str,
        markdown: &str,
        secrets: HashMap<String, String>,
        summary: &str,
    ) -> Result<MemoMetadata, String> {
        Ok(self
            .save_document_with_risk_diff(id, file_name, title, markdown, secrets, summary)
            .await?
            .metadata)
    }

    pub async fn save_document_with_risk_diff(
        &self,
        id: Option<String>,
        file_name: &str,
        title: &str,
        markdown: &str,
        secrets: HashMap<String, String>,
        summary: &str,
    ) -> Result<SaveDocumentResponse, String> {
        let _key = self.get_key().await?;
        let previous_meta = match id {
            Some(existing_id) => Some(
                self.store
                    .get_memo_metadata(&existing_id)?
                    .ok_or_else(|| "Document not found".to_string())?,
            ),
            None => None,
        };
        let doc_id = previous_meta
            .as_ref()
            .map(|meta| meta.id.clone())
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        let previous_secret_keys = previous_meta
            .as_ref()
            .and_then(|meta| self.store.read_document_file(&meta.file_name).ok())
            .map(|previous_markdown| extract_secret_placeholders(&previous_markdown))
            .unwrap_or_default();
        let previous_snapshot = self.store.read_document_risk_snapshot(&doc_id)?;

        if let Some(owner_id) = self.store.file_name_owner(file_name)? {
            if owner_id != doc_id {
                return Err("A document with this file name already exists.".to_string());
            }
        }

        // 1. Replace the document's full secret set in KDBX.
        self.replace_document_secrets_in_kdbx(&doc_id, file_name, &secrets, &previous_secret_keys)
            .await?;

        let meta = MemoMetadata {
            id: doc_id.clone(),
            file_name: file_name.to_string(),
            title: title.to_string(),
            summary: summary.to_string(),
            updated_at: current_timestamp(),
        };

        // 2. Save Markdown file with frontmatter so the file is self-describing.
        self.store.save_document_with_metadata(&meta, markdown)?;
        if let Some(previous_meta) = &previous_meta {
            if previous_meta.file_name != file_name {
                self.store.delete_document_file(&previous_meta.file_name)?;
            }
        }

        // 3. Metadata is embedded in the Markdown frontmatter.
        self.store.upsert_memo_metadata(&meta)?;

        let risk_diff = self.persist_document_risk_snapshot(
            &doc_id,
            markdown,
            meta.updated_at,
            previous_snapshot,
        )?;

        // 4. Generate & Save Vector Embedding asynchronously
        let client = self.get_ollama_client();

        // Use title + summary + markdown (without actual plain passwords) as embedding text
        let embedding_text = format!(
            "Title: {}\nSummary: {}\nContent:\n{}",
            title, summary, markdown
        );

        let data_dir = self.store.get_data_dir().to_path_buf();
        let doc_id_clone = doc_id.clone();

        // Spawn embedding generation in the background so API stays fast
        tokio::spawn(async move {
            if let Ok(vec) = client.get_embedding(&embedding_text).await {
                let temp_store = MemoStore::new(&data_dir).unwrap();
                let _ = temp_store.save_embedding(&doc_id_clone, &vec);
            }
        });

        Ok(SaveDocumentResponse {
            metadata: meta,
            risk_diff,
        })
    }

    /// Read document contents. If unlocked, decrypt placeholders.
    pub async fn get_document(&self, id: &str) -> Result<DocumentDetail, String> {
        let meta = self
            .store
            .get_memo_metadata(id)?
            .ok_or_else(|| "Document not found".to_string())?;

        let markdown = self.store.read_document_file(&meta.file_name)?;

        let mut decrypted_secrets = HashMap::new();
        let key_opt = *self.master_key.read().await;

        if key_opt.is_some() {
            decrypted_secrets = self.read_document_secrets_from_kdbx(id, &markdown).await?;
        }

        Ok(DocumentDetail {
            metadata: meta,
            markdown,
            secrets: decrypted_secrets,
            unlocked: key_opt.is_some(),
        })
    }

    pub async fn audit_document(
        &self,
        doc_id: &str,
        markdown_override: Option<&str>,
    ) -> Result<AuditScanResponse, String> {
        let _key = self.get_key().await?;
        let doc_id = if doc_id.trim().is_empty() {
            "__draft__"
        } else {
            doc_id.trim()
        };
        let markdown = match markdown_override {
            Some(markdown) => markdown.to_string(),
            None => {
                let meta = self
                    .store
                    .get_memo_metadata(doc_id)?
                    .ok_or_else(|| "Document not found".to_string())?;
                self.store.read_document_file(&meta.file_name)?
            }
        };
        let statuses = self.store.read_finding_statuses()?;
        Ok(audit::scan_markdown(
            doc_id,
            &markdown,
            &statuses,
            current_timestamp(),
        ))
    }

    pub async fn document_risk_diff(
        &self,
        doc_id: &str,
        markdown_override: Option<&str>,
    ) -> Result<DocumentRiskDiff, String> {
        let _key = self.get_key().await?;
        let doc_id = doc_id.trim();
        if doc_id.is_empty() || doc_id == "new" {
            return Err("Document id is required for risk diff".to_string());
        }
        let markdown = match markdown_override {
            Some(markdown) => markdown.to_string(),
            None => {
                let meta = self
                    .store
                    .get_memo_metadata(doc_id)?
                    .ok_or_else(|| "Document not found".to_string())?;
                self.store.read_document_file(&meta.file_name)?
            }
        };
        let statuses = self.store.read_finding_statuses()?;
        let now = current_timestamp();
        let scan = audit::scan_markdown(doc_id, &markdown, &statuses, now);
        let current = history::build_snapshot(doc_id, now, &markdown, scan.findings);
        let previous = self.store.read_document_risk_snapshot(doc_id)?;
        Ok(history::diff_snapshots(doc_id, previous.as_ref(), &current))
    }

    pub async fn update_audit_finding_status(
        &self,
        finding_id: &str,
        status: &str,
    ) -> Result<(), String> {
        let _key = self.get_key().await?;
        let normalized = match status {
            "open" | "fixed" | "ignored" | "reviewing" => status,
            _ => return Err("Invalid finding status".to_string()),
        };
        self.store.set_finding_status(finding_id, normalized)?;

        let case_status = match normalized {
            "fixed" => SecurityCaseStatus::Fixed,
            "ignored" => SecurityCaseStatus::Closed,
            "reviewing" => SecurityCaseStatus::Reviewing,
            _ => SecurityCaseStatus::Open,
        };
        let mut cases = self.sync_governance_cases()?;
        if let Some(case) = cases
            .iter_mut()
            .find(|case| case.source_finding_id.as_deref() == Some(finding_id))
        {
            let event = governance::transition_case(
                case,
                case_status,
                None,
                None,
                Some(format!("同步 finding 状态：{normalized}")),
                "user",
                current_timestamp(),
            );
            self.store.write_security_cases(&cases)?;
            self.store.append_audit_events(&[event])?;
        }

        Ok(())
    }

    pub async fn audit_fix_preview(
        &self,
        doc_id: &str,
        finding_id: &str,
        markdown_override: Option<&str>,
    ) -> Result<AuditFixPreview, String> {
        let _key = self.get_key().await?;
        let markdown = match markdown_override {
            Some(markdown) => markdown.to_string(),
            None => {
                let meta = self
                    .store
                    .get_memo_metadata(doc_id)?
                    .ok_or_else(|| "Document not found".to_string())?;
                self.store.read_document_file(&meta.file_name)?
            }
        };
        audit::preview_fix(doc_id, &markdown, finding_id)
            .ok_or_else(|| "Finding not found or cannot be fixed automatically".to_string())
    }

    pub async fn redact_markdown(&self, markdown: &str) -> Result<RedactMarkdownResponse, String> {
        let _key = self.get_key().await?;
        let redacted = redactor::redact_secrets(markdown);
        Ok(RedactMarkdownResponse {
            markdown: redacted.text,
            redacted_secret_count: redacted.secrets.len(),
            secrets: redacted.secrets,
        })
    }

    pub async fn governance_summary(&self) -> Result<GovernanceSummary, String> {
        let _key = self.get_key().await?;
        let (findings, assets) = self.collect_security_state()?;
        let cases = self.sync_governance_cases_from_findings(&findings)?;
        let events = self.store.read_audit_events()?;
        Ok(governance::build_governance_summary(
            findings,
            assets,
            &cases,
            &events,
            current_timestamp(),
        ))
    }

    pub async fn governance_cases(&self) -> Result<Vec<SecurityCase>, String> {
        let _key = self.get_key().await?;
        self.sync_governance_cases()
    }

    pub async fn governance_events(&self) -> Result<Vec<AuditEvent>, String> {
        let _key = self.get_key().await?;
        self.store.read_audit_events()
    }

    pub async fn update_security_case_status(
        &self,
        case_id: &str,
        status: SecurityCaseStatus,
        owner: Option<String>,
        due_at: Option<String>,
        rationale: Option<String>,
    ) -> Result<SecurityCase, String> {
        let _key = self.get_key().await?;
        let mut cases = self.sync_governance_cases()?;
        let now = current_timestamp();
        let case = cases
            .iter_mut()
            .find(|case| case.id == case_id)
            .ok_or_else(|| "Security case not found".to_string())?;
        let event =
            governance::transition_case(case, status, owner, due_at, rationale, "user", now);
        let response = case.clone();
        self.store.write_security_cases(&cases)?;
        self.store.append_audit_events(&[event])?;
        Ok(response)
    }

    pub async fn accept_security_case(
        &self,
        case_id: &str,
        rationale: String,
        accepted_until: String,
        impact_scope: String,
        compensating_controls: String,
        reviewer: String,
        owner: Option<String>,
    ) -> Result<SecurityCase, String> {
        let _key = self.get_key().await?;
        if rationale.trim().is_empty() {
            return Err("Risk acceptance rationale cannot be empty".to_string());
        }
        if accepted_until.trim().is_empty() {
            return Err("Risk acceptance expiry cannot be empty".to_string());
        }
        if governance::parse_date_days(&accepted_until).is_none() {
            return Err("Risk acceptance expiry must use YYYY-MM-DD".to_string());
        }
        if impact_scope.trim().is_empty() {
            return Err("Risk acceptance impact scope cannot be empty".to_string());
        }
        if compensating_controls.trim().is_empty() {
            return Err("Risk acceptance compensating controls cannot be empty".to_string());
        }
        if reviewer.trim().is_empty() {
            return Err("Risk acceptance reviewer cannot be empty".to_string());
        }

        let mut cases = self.sync_governance_cases()?;
        let now = current_timestamp();
        let case = cases
            .iter_mut()
            .find(|case| case.id == case_id)
            .ok_or_else(|| "Security case not found".to_string())?;
        let event = governance::accept_case(
            case,
            rationale.trim().to_string(),
            accepted_until.trim().to_string(),
            impact_scope.trim().to_string(),
            compensating_controls.trim().to_string(),
            reviewer.trim().to_string(),
            owner,
            "user",
            now,
        );
        let response = case.clone();
        self.store.write_security_cases(&cases)?;
        self.store.append_audit_events(&[event])?;
        Ok(response)
    }

    pub async fn security_assets(&self) -> Result<Vec<SecurityAsset>, String> {
        let _key = self.get_key().await?;
        let (_, assets) = self.collect_security_state()?;
        Ok(assets)
    }

    pub async fn security_asset_detail(
        &self,
        asset_id: Option<&str>,
        query: Option<&str>,
    ) -> Result<SecurityAssetDetail, String> {
        let _key = self.get_key().await?;
        let (findings, assets) = self.collect_security_state()?;
        let asset = assets::find_asset(&assets, asset_id, query)
            .cloned()
            .ok_or_else(|| "Security asset not found".to_string())?;
        let cases = self.sync_governance_cases_from_findings(&findings)?;
        let documents = self.store.get_all_memos()?;
        Ok(assets::build_asset_detail(
            asset, &documents, &findings, &cases,
        ))
    }

    pub async fn security_asset_graph(
        &self,
        asset_id: Option<&str>,
        query: Option<&str>,
    ) -> Result<SecurityAssetGraph, String> {
        let _key = self.get_key().await?;
        let (findings, assets) = self.collect_security_state()?;
        let focus_asset_id = if asset_id
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_some()
            || query
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .is_some()
        {
            Some(
                assets::find_asset(&assets, asset_id, query)
                    .ok_or_else(|| "Security asset not found".to_string())?
                    .id
                    .clone(),
            )
        } else {
            None
        };
        let cases = self.sync_governance_cases_from_findings(&findings)?;
        let documents = self.store.get_all_memos()?;
        Ok(assets::build_asset_graph(
            &assets,
            &documents,
            &findings,
            &cases,
            focus_asset_id.as_deref(),
        ))
    }

    pub async fn generate_security_report(&self) -> Result<SecurityReport, String> {
        self.generate_security_report_with_request(SecurityReportRequest::default())
            .await
    }

    pub async fn generate_security_report_with_request(
        &self,
        request: SecurityReportRequest,
    ) -> Result<SecurityReport, String> {
        let _key = self.get_key().await?;
        let (all_findings, all_assets) = self.collect_security_state()?;
        let all_cases = self.sync_governance_cases_from_findings(&all_findings)?;
        let events = self.store.read_audit_events()?;
        let checklist_statuses = self.store.read_checklist_statuses()?;
        let documents = self.store.get_all_memos()?;
        let scope = request.scope.clone().unwrap_or(SecurityReportScope::All);
        let now = current_timestamp();
        let requested_since_days = request.since_days.filter(|days| *days > 0);
        let since_cutoff =
            requested_since_days.map(|days| now.saturating_sub(i64::from(days) * 86_400));

        let (
            report_title,
            mut scope_summary,
            mut scope_slug,
            mut findings,
            mut assets,
            mut cases,
            mut doc_ids,
        ) = match scope.clone() {
            SecurityReportScope::All => (
                "安全治理审计报告".to_string(),
                "全部本地安全档案".to_string(),
                "all".to_string(),
                all_findings.clone(),
                all_assets.clone(),
                all_cases.clone(),
                documents
                    .iter()
                    .map(|document| document.id.clone())
                    .collect::<Vec<_>>(),
            ),
            SecurityReportScope::Document => {
                let doc_id = request
                    .doc_id
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| "Document id is required for document report".to_string())?;
                let document = self
                    .store
                    .get_memo_metadata(doc_id)?
                    .ok_or_else(|| "Document not found".to_string())?;
                let (findings, assets) = self.collect_security_state_for_doc(doc_id)?;
                let cases = all_cases
                    .iter()
                    .filter(|case| case.source_doc_id == doc_id)
                    .cloned()
                    .collect::<Vec<_>>();
                (
                    format!("{} · 安全审计报告", document.title),
                    format!("单篇文档：{} ({})", document.title, document.file_name),
                    format!("doc-{}", safe_report_slug(&document.id)),
                    findings,
                    assets,
                    cases,
                    vec![document.id],
                )
            }
            SecurityReportScope::Asset => {
                let asset = assets::find_asset(
                    &all_assets,
                    request.asset_id.as_deref(),
                    request.query.as_deref(),
                )
                .cloned()
                .ok_or_else(|| "Security asset not found".to_string())?;
                let detail =
                    assets::build_asset_detail(asset, &documents, &all_findings, &all_cases);
                let doc_ids = detail
                    .documents
                    .iter()
                    .map(|document| document.id.clone())
                    .collect::<Vec<_>>();
                (
                    format!("{} · 资产安全报告", detail.asset.name),
                    format!("安全资产：{}", detail.asset.name),
                    format!("asset-{}", safe_report_slug(&detail.asset.id)),
                    detail.findings,
                    vec![detail.asset],
                    detail.cases,
                    doc_ids,
                )
            }
            SecurityReportScope::Tags => {
                let requested_tags = normalize_report_tags(request.tags.as_deref());
                if requested_tags.is_empty() {
                    return Err("At least one tag is required for tag report".to_string());
                }
                let matching_doc_ids = documents
                    .iter()
                    .filter(|document| {
                        let document_tags =
                            collect_report_tags(document, &all_assets, &all_findings);
                        report_tags_match(&document_tags, &requested_tags)
                    })
                    .map(|document| document.id.clone())
                    .collect::<HashSet<_>>();
                let findings = all_findings
                    .iter()
                    .filter(|finding| matching_doc_ids.contains(&finding.doc_id))
                    .cloned()
                    .collect::<Vec<_>>();
                let assets = all_assets
                    .iter()
                    .filter(|asset| {
                        asset
                            .source_doc_ids
                            .iter()
                            .any(|doc_id| matching_doc_ids.contains(doc_id))
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                let cases = all_cases
                    .iter()
                    .filter(|case| matching_doc_ids.contains(&case.source_doc_id))
                    .cloned()
                    .collect::<Vec<_>>();
                let doc_ids = documents
                    .iter()
                    .filter(|document| matching_doc_ids.contains(&document.id))
                    .map(|document| document.id.clone())
                    .collect::<Vec<_>>();
                let tag_summary = requested_tags.join("、");
                (
                    "标签范围安全报告".to_string(),
                    format!("标签：{tag_summary}"),
                    format!("tags-{}", safe_report_slug(&requested_tags.join("-"))),
                    findings,
                    assets,
                    cases,
                    doc_ids,
                )
            }
        };
        if let (Some(days), Some(cutoff)) = (requested_since_days, since_cutoff) {
            let recent_doc_ids = documents
                .iter()
                .filter(|document| document.updated_at >= cutoff)
                .map(|document| document.id.as_str())
                .collect::<HashSet<_>>();
            findings.retain(|finding| recent_doc_ids.contains(finding.doc_id.as_str()));
            assets.retain(|asset| {
                asset.last_seen_at >= cutoff
                    || asset
                        .source_doc_ids
                        .iter()
                        .any(|doc_id| recent_doc_ids.contains(doc_id.as_str()))
            });
            cases.retain(|case| {
                case.updated_at >= cutoff || recent_doc_ids.contains(case.source_doc_id.as_str())
            });
            doc_ids.retain(|doc_id| recent_doc_ids.contains(doc_id.as_str()));
            scope_summary = format!("{scope_summary} · 最近 {days} 天");
            scope_slug = format!("{scope_slug}-recent-{days}");
        }

        let case_ids = cases
            .iter()
            .map(|case| case.id.as_str())
            .collect::<HashSet<_>>();
        let doc_id_set = doc_ids.iter().map(String::as_str).collect::<HashSet<_>>();
        let mut filtered_events = if matches!(scope, SecurityReportScope::All) {
            events.clone()
        } else {
            events
                .iter()
                .filter(|event| {
                    case_ids.contains(event.target_id.as_str())
                        || doc_id_set.contains(event.target_id.as_str())
                        || event
                            .metadata
                            .get("docId")
                            .and_then(serde_json::Value::as_str)
                            .is_some_and(|doc_id| doc_id_set.contains(doc_id))
                })
                .cloned()
                .collect::<Vec<_>>()
        };
        if let Some(cutoff) = since_cutoff {
            filtered_events.retain(|event| event.created_at >= cutoff);
        }
        let mut risk_diffs = Vec::new();
        for doc_id in &doc_ids {
            if let Ok(diff) = self.document_risk_diff(doc_id, None).await {
                risk_diffs.push(diff);
            }
        }
        let checklist_doc_id = if matches!(scope, SecurityReportScope::Document) {
            doc_ids.first().map(String::as_str)
        } else {
            None
        };
        let checklist = standards::build_checklist(
            checklist_doc_id,
            &findings,
            &assets,
            &checklist_statuses,
            now,
        );
        let markdown = reports::render_security_report(
            &report_title,
            &scope_summary,
            &findings,
            &assets,
            &cases,
            &filtered_events,
            &checklist.items,
            &checklist.standards,
            &risk_diffs,
            now,
        );
        let id = format!("report-{now}");
        let file_name = format!("security-report-{scope_slug}-{now}.md");
        let path = self.store.write_security_report(&file_name, &markdown)?;
        let summary = format!("包含 {} 个风险、{} 个治理项。", findings.len(), cases.len());
        let report_case_ids = cases.iter().map(|case| case.id.clone()).collect::<Vec<_>>();
        let report_asset_ids = assets
            .iter()
            .map(|asset| asset.id.clone())
            .collect::<Vec<_>>();
        let event = governance::audit_event(
            "reportGenerated",
            "user",
            &id,
            &format!("生成安全审计报告：{file_name}"),
            now,
            serde_json::json!({
                "fileName": file_name.clone(),
                "scope": scope_summary,
                "tags": request.tags.clone(),
                "sinceDays": request.since_days,
                "sinceCutoff": since_cutoff,
                "docIds": doc_ids.clone(),
                "caseIds": report_case_ids,
                "assetIds": report_asset_ids,
                "findingCount": findings.len(),
                "caseCount": cases.len(),
            }),
        );
        self.store.append_audit_events(&[event])?;

        Ok(SecurityReport {
            id,
            file_name,
            path: path.to_string_lossy().to_string(),
            markdown,
            summary,
            created_at: now,
        })
    }

    pub async fn safe_share_document(
        &self,
        request: SafeShareRequest,
    ) -> Result<SafeShareExport, String> {
        let _key = self.get_key().await?;
        let doc_id = request.doc_id.trim();
        if doc_id.is_empty() || doc_id == "new" {
            return Err("Document id is required for safe share".to_string());
        }
        let document = self
            .store
            .get_memo_metadata(doc_id)?
            .ok_or_else(|| "Document not found".to_string())?;
        let source_markdown = match request.markdown {
            Some(markdown) => markdown,
            None => self.store.read_document_file(&document.file_name)?,
        };
        let redacted = redactor::redact_secrets(&source_markdown);
        let statuses = self.store.read_finding_statuses()?;
        let now = current_timestamp();
        let scan = audit::scan_markdown(&document.id, &redacted.text, &statuses, now);
        let include_audit = request.include_audit.unwrap_or(true);
        let markdown = reports::render_safe_share_markdown(
            &document.title,
            &document.file_name,
            &redacted.text,
            redacted.secrets.len(),
            &scan.findings,
            include_audit,
            now,
        );
        let id = format!("safe-share-{now}");
        let file_name = format!("safe-share-{}-{now}.md", safe_report_slug(&document.id));
        let path = self.store.write_security_report(&file_name, &markdown)?;
        let summary = format!(
            "安全分享已脱敏 {} 处，附带 {} 个审计发现。",
            redacted.secrets.len(),
            scan.findings.len()
        );
        let event = governance::audit_event(
            "safeShareExported",
            "user",
            &document.id,
            &format!("生成安全分享文件：{file_name}"),
            now,
            serde_json::json!({
                "docId": document.id,
                "fileName": file_name.clone(),
                "includeAudit": include_audit,
                "redactedSecretCount": redacted.secrets.len(),
                "findingCount": scan.findings.len(),
            }),
        );
        self.store.append_audit_events(&[event])?;

        Ok(SafeShareExport {
            id,
            file_name,
            path: path.to_string_lossy().to_string(),
            markdown,
            summary,
            redacted_secret_count: redacted.secrets.len(),
            finding_count: scan.findings.len(),
            created_at: now,
        })
    }

    pub async fn standards_list(&self) -> Result<Vec<StandardEntry>, String> {
        let _key = self.get_key().await?;
        Ok(standards::builtin_standards())
    }

    pub async fn standards_checklist(
        &self,
        doc_id: Option<&str>,
    ) -> Result<StandardsChecklistResponse, String> {
        let _key = self.get_key().await?;
        let (findings, assets) = match doc_id.map(str::trim).filter(|value| !value.is_empty()) {
            Some(doc_id) => self.collect_security_state_for_doc(doc_id)?,
            None => self.collect_security_state()?,
        };
        let statuses = self.store.read_checklist_statuses()?;
        Ok(standards::build_checklist(
            doc_id,
            &findings,
            &assets,
            &statuses,
            current_timestamp(),
        ))
    }

    pub async fn update_checklist_item_status(
        &self,
        doc_id: Option<String>,
        item_id: &str,
        status: ChecklistStatus,
        note: Option<String>,
    ) -> Result<ChecklistItem, String> {
        let _key = self.get_key().await?;
        let item_id = item_id.trim();
        if item_id.is_empty() {
            return Err("Checklist item id cannot be empty".to_string());
        }
        let doc_id = doc_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        let checklist = self.standards_checklist(doc_id.as_deref()).await?;
        if !checklist.items.iter().any(|item| item.id == item_id) {
            return Err("Checklist item not found".to_string());
        }
        let now = current_timestamp();
        self.store.set_checklist_status(
            doc_id.as_deref(),
            item_id,
            standards::ChecklistStatusRecord {
                item_id: item_id.to_string(),
                status,
                note: note
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty()),
                updated_at: now,
            },
        )?;
        let checklist = self.standards_checklist(doc_id.as_deref()).await?;
        checklist
            .items
            .into_iter()
            .find(|item| item.id == item_id)
            .ok_or_else(|| "Checklist item not found".to_string())
    }

    fn collect_security_state(&self) -> Result<(Vec<SecurityFinding>, Vec<SecurityAsset>), String> {
        let statuses = self.store.read_finding_statuses()?;
        let mut all_findings = Vec::new();
        let mut asset_sets = Vec::new();
        let now = current_timestamp();

        for meta in self.store.get_all_memos()? {
            let markdown = self.store.read_document_file(&meta.file_name)?;
            let scan = audit::scan_markdown(&meta.id, &markdown, &statuses, now);
            all_findings.extend(scan.findings);
            asset_sets.push(assets::extract_assets(
                &meta.id,
                &meta.title,
                &markdown,
                meta.updated_at,
            ));
        }

        Ok((all_findings, assets::merge_assets(asset_sets)))
    }

    fn collect_security_state_for_doc(
        &self,
        doc_id: &str,
    ) -> Result<(Vec<SecurityFinding>, Vec<SecurityAsset>), String> {
        let meta = self
            .store
            .get_memo_metadata(doc_id)?
            .ok_or_else(|| "Document not found".to_string())?;
        let markdown = self.store.read_document_file(&meta.file_name)?;
        let statuses = self.store.read_finding_statuses()?;
        let scan = audit::scan_markdown(&meta.id, &markdown, &statuses, current_timestamp());
        let assets = assets::extract_assets(&meta.id, &meta.title, &markdown, meta.updated_at);
        Ok((scan.findings, assets))
    }

    fn sync_governance_cases(&self) -> Result<Vec<SecurityCase>, String> {
        let (findings, _) = self.collect_security_state()?;
        self.sync_governance_cases_from_findings(&findings)
    }

    fn sync_governance_cases_from_findings(
        &self,
        findings: &[SecurityFinding],
    ) -> Result<Vec<SecurityCase>, String> {
        let existing_cases = self.store.read_security_cases()?;
        let (cases, events) =
            governance::sync_cases_with_findings(existing_cases, findings, current_timestamp());
        self.store.write_security_cases(&cases)?;
        self.store.append_audit_events(&events)?;
        Ok(cases)
    }

    fn persist_document_risk_snapshot(
        &self,
        doc_id: &str,
        markdown: &str,
        saved_at: i64,
        previous_snapshot: Option<DocumentRiskSnapshot>,
    ) -> Result<DocumentRiskDiff, String> {
        let statuses = self.store.read_finding_statuses()?;
        let scan = audit::scan_markdown(doc_id, markdown, &statuses, saved_at);
        let current = history::build_snapshot(doc_id, saved_at, markdown, scan.findings);
        let diff = history::diff_snapshots(doc_id, previous_snapshot.as_ref(), &current);
        self.store.write_document_risk_snapshot(&current)?;

        let changed_count = diff.summary.added
            + diff.summary.resolved
            + diff.summary.severity_changed
            + diff.summary.moved;
        if previous_snapshot.is_some() && changed_count > 0 {
            let event = governance::audit_event(
                "riskDiff",
                "user",
                doc_id,
                &format!(
                    "文档风险变化：新增 {}，修复 {}，移动 {}，等级变化 {}。",
                    diff.summary.added,
                    diff.summary.resolved,
                    diff.summary.moved,
                    diff.summary.severity_changed
                ),
                saved_at,
                serde_json::json!({
                    "docId": doc_id,
                    "added": diff.summary.added,
                    "resolved": diff.summary.resolved,
                    "moved": diff.summary.moved,
                    "severityChanged": diff.summary.severity_changed,
                    "previousHash": diff.previous_hash,
                    "currentHash": diff.current_hash,
                }),
            );
            self.store.append_audit_events(&[event])?;
        }

        Ok(diff)
    }

    pub async fn list_secrets(&self) -> Result<Vec<SecretListItem>, String> {
        let _key = self.get_key().await?;
        let docs = self.store.get_all_memos()?;
        let mut referenced = HashMap::new();

        for meta in &docs {
            let markdown = self.store.read_document_file(&meta.file_name)?;
            for secret_key in extract_secret_placeholders(&markdown) {
                referenced.insert(
                    document_secret_entry_key(&meta.id, &secret_key),
                    (meta.clone(), secret_key),
                );
            }
        }

        let kdbx_ids = {
            let guard = self.secret_vault.read().await;
            match guard.as_ref() {
                Some(vault) => vault
                    .list_secret_keys()?
                    .into_iter()
                    .filter(|id| !is_system_secret_id(id))
                    .collect::<Vec<_>>(),
                None => Vec::new(),
            }
        };
        let kdbx_id_set = kdbx_ids.into_iter().collect::<HashSet<_>>();
        let mut all_ids = referenced.keys().cloned().collect::<HashSet<_>>();
        all_ids.extend(kdbx_id_set.iter().cloned());

        let mut items = all_ids
            .into_iter()
            .map(|id| {
                let referenced_entry = referenced.get(&id);
                let parsed = split_document_secret_entry_key(&id);
                let document_id = referenced_entry
                    .map(|(meta, _)| meta.id.clone())
                    .or_else(|| parsed.as_ref().map(|(doc_id, _)| doc_id.clone()));
                let fallback_meta = document_id
                    .as_deref()
                    .and_then(|doc_id| self.store.get_memo_metadata(doc_id).ok().flatten());
                let meta = referenced_entry
                    .map(|(meta, _)| meta.clone())
                    .or(fallback_meta);
                let key = referenced_entry
                    .map(|(_, key)| key.clone())
                    .or_else(|| parsed.as_ref().map(|(_, key)| key.clone()))
                    .unwrap_or_else(|| id.clone());
                let in_kdbx = kdbx_id_set.contains(&id);
                let source = if in_kdbx { "kdbx" } else { "missing" }.to_string();

                SecretListItem {
                    id,
                    key,
                    document_id,
                    document_title: meta.as_ref().map(|meta| meta.title.clone()),
                    file_name: meta.as_ref().map(|meta| meta.file_name.clone()),
                    updated_at: meta.as_ref().map(|meta| meta.updated_at),
                    referenced: referenced_entry.is_some(),
                    has_value: in_kdbx,
                    source,
                }
            })
            .collect::<Vec<_>>();

        items.sort_by(|left, right| {
            right
                .updated_at
                .unwrap_or_default()
                .cmp(&left.updated_at.unwrap_or_default())
                .then_with(|| left.document_title.cmp(&right.document_title))
                .then_with(|| left.key.cmp(&right.key))
        });

        Ok(items)
    }

    pub async fn reveal_secret(&self, id: &str) -> Result<SecretRevealResponse, String> {
        let _key = self.get_key().await?;
        let id = id.trim();
        if id.is_empty() {
            return Err("Secret id cannot be empty".to_string());
        }
        if is_system_secret_id(id) {
            return Err("Secret not found".to_string());
        }

        {
            let guard = self.secret_vault.read().await;
            if let Some(vault) = guard.as_ref() {
                if let Some(value) = vault.get_secret(id)? {
                    return Ok(SecretRevealResponse {
                        id: id.to_string(),
                        value,
                    });
                }
            }
        }

        Err("Secret not found".to_string())
    }

    /// Delete a document and its associated secrets and embeddings
    pub async fn delete_document(&self, id: &str) -> Result<(), String> {
        let meta = self.store.get_memo_metadata(id)?;
        if let Some(m) = meta {
            let secret_keys = self
                .store
                .read_document_file(&m.file_name)
                .ok()
                .map(|markdown| extract_secret_placeholders(&markdown))
                .unwrap_or_default();
            self.delete_document_secrets_from_kdbx(id, &secret_keys)
                .await?;

            // Delete file
            self.store.delete_document_file(&m.file_name)?;
        }

        // Delete metadata & embedding
        self.store.delete_memo_metadata(id)?;
        self.store.delete_document_risk_snapshot(id)?;

        Ok(())
    }

    // --- Core feature 3: AI Search & QA (RAG) ---

    pub async fn search_and_answer(&self, query: &str) -> Result<SearchAnswerResponse, String> {
        let client = self.get_ollama_client();
        let redacted_query = redactor::redact_secrets(query);

        // 1. Get query embedding
        let query_vec = client.get_embedding(&redacted_query.text).await?;

        // 2. Compute similarity with all docs
        let all_embeddings = self.store.get_all_embeddings()?;
        let mut scores = Vec::new();
        for (doc_id, doc_vec) in all_embeddings {
            let sim = cosine_similarity(&query_vec, &doc_vec);
            scores.push((doc_id, sim));
        }

        // Sort by similarity descending
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Take Top-3 most relevant documents
        let mut references = Vec::new();
        let mut source_docs = Vec::new();

        // We will read contents of top matches
        for (doc_id, score) in scores.into_iter().take(3) {
            if score < 0.2 {
                // Filter out highly irrelevant documents
                continue;
            }
            if let Ok(detail) = self.get_document(&doc_id).await {
                // If unlocked and allow_ai_secrets is true, we replace placeholders with decrypted values.
                // Otherwise, the LLM will only see {{secret:key}}.
                let mut content_for_llm = detail.markdown.clone();
                let allow_secrets = {
                    let config = self.llm_config.read().unwrap();
                    config.allow_ai_secrets
                };
                if allow_secrets {
                    for (sec_key, sec_val) in &detail.secrets {
                        let placeholder = format!("{{{{secret:{}}}}}", sec_key);
                        content_for_llm = content_for_llm.replace(&placeholder, sec_val);
                    }
                }

                references.push(format!(
                    "--- DOCUMENT: {} ---\nSummary: {}\nContent:\n{}",
                    detail.metadata.title, detail.metadata.summary, content_for_llm
                ));

                source_docs.push(SearchSourceDoc {
                    id: doc_id,
                    title: detail.metadata.title,
                    file_name: detail.metadata.file_name,
                    score,
                });
            }
        }

        if references.is_empty() {
            return Ok(SearchAnswerResponse {
                answer: "没有在本地文档库中找到相关内容。".to_string(),
                sources: Vec::new(),
            });
        }

        // 3. Call Chat API to generate answer
        let system_prompt = "You are a professional documentation question-answering assistant. 
Please answer the user's question based strictly on the provided local documents.
Do not hallucinate. If the documents do not contain the answer, reply that you could not find the information in the documents.

【LOCAL REFERENCE DOCUMENTS】:
";
        let context = format!("{}{}\n", system_prompt, references.join("\n\n"));

        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: context,
            },
            ChatMessage {
                role: "user".to_string(),
                content: redacted_query.text,
            },
        ];

        let answer = client.chat(messages, false).await?;

        Ok(SearchAnswerResponse {
            answer,
            sources: source_docs,
        })
    }

    // --- Core feature 5: Backup & Restore ---

    pub async fn backup(
        &self,
        local_backup_dir: Option<&str>,
        webdav_config: Option<WebDavConfig>,
    ) -> Result<String, String> {
        let temp_dir = std::env::temp_dir();
        let backup_filename = format!("rust_tool_memo_backup_{}.zip", current_timestamp());
        let temp_zip_path = temp_dir.join(&backup_filename);

        // 1. Create ZIP
        backup::create_backup_zip(self.store.get_data_dir(), &temp_zip_path)?;

        let mut status = "本地打包成功。".to_string();

        // 2. Local copy
        if let Some(dir_str) = local_backup_dir {
            let target_dir = Path::new(dir_str);
            fs::create_dir_all(target_dir)
                .map_err(|e| format!("Failed to create backup target dir: {:?}", e))?;
            let target_zip = target_dir.join(&backup_filename);
            fs::copy(&temp_zip_path, &target_zip)
                .map_err(|e| format!("Failed to copy zip file: {:?}", e))?;
            status.push_str(&format!(" 已保存至本地目录: {}", target_zip.display()));
        }

        // 3. WebDAV upload
        if let Some(webdav) = webdav_config {
            let full_url = format!("{}/{}", webdav.url.trim_end_matches('/'), backup_filename);
            backup::upload_to_webdav(
                &temp_zip_path,
                &full_url,
                &webdav.username,
                webdav.password.as_deref().unwrap_or(""),
            )
            .await?;
            status.push_str(&format!(" 已上传至 WebDAV: {}", full_url));
        }

        // Cleanup temp file
        let _ = fs::remove_file(temp_zip_path);

        Ok(status)
    }

    pub async fn restore(&self, zip_path: &Path) -> Result<(), String> {
        backup::restore_from_zip(zip_path, self.store.get_data_dir())?;
        self.lock().await;
        Ok(())
    }
}

fn document_secret_entry_key(doc_id: &str, secret_key: &str) -> String {
    format!("{}:{}", doc_id, secret_key.trim())
}

fn split_document_secret_entry_key(id: &str) -> Option<(String, String)> {
    let (doc_id, secret_key) = id.split_once(':')?;
    if doc_id.trim().is_empty() || secret_key.trim().is_empty() {
        return None;
    }
    Some((doc_id.to_string(), secret_key.to_string()))
}

fn is_system_secret_id(id: &str) -> bool {
    id.starts_with("__rusttool_config:")
}

fn normalize_report_tags(tags: Option<&[String]>) -> Vec<String> {
    let mut normalized = Vec::new();
    let mut seen = HashSet::new();
    for tag in tags.unwrap_or_default() {
        for part in
            tag.split(|ch: char| ch.is_whitespace() || matches!(ch, ',' | '，' | '、' | ';' | '；'))
        {
            let clean = normalize_report_tag(part);
            if !clean.is_empty() && seen.insert(clean.clone()) {
                normalized.push(clean);
            }
        }
    }
    normalized
}

fn normalize_report_tag(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .chars()
        .filter(|ch| ch.is_alphanumeric() || matches!(ch, '-' | '_' | '/' | '.'))
        .collect()
}

fn collect_report_tags(
    document: &MemoMetadata,
    assets: &[SecurityAsset],
    findings: &[SecurityFinding],
) -> HashSet<String> {
    let mut tags = HashSet::new();
    insert_report_tag(&mut tags, &document.title);
    insert_report_tag(&mut tags, &document.summary);
    insert_report_tag(&mut tags, &document.file_name);

    for asset in assets
        .iter()
        .filter(|asset| asset.source_doc_ids.contains(&document.id))
    {
        insert_report_tag(&mut tags, &asset.name);
        for alias in &asset.aliases {
            insert_report_tag(&mut tags, alias);
        }
        for tag in &asset.tags {
            insert_report_tag(&mut tags, tag);
        }
        insert_asset_type_report_tags(&mut tags, &asset.asset_type);
    }

    for finding in findings
        .iter()
        .filter(|finding| finding.doc_id.as_str() == document.id.as_str())
    {
        insert_report_tag(&mut tags, &finding.title);
        insert_severity_report_tags(&mut tags, &finding.severity);
        insert_finding_kind_report_tags(&mut tags, &finding.kind);
    }

    tags
}

fn report_tags_match(candidates: &HashSet<String>, requested: &[String]) -> bool {
    requested.iter().any(|tag| {
        candidates
            .iter()
            .any(|candidate| candidate == tag || candidate.contains(tag))
    })
}

fn insert_report_tag(tags: &mut HashSet<String>, value: &str) {
    let clean = normalize_report_tag(value);
    if !clean.is_empty() {
        tags.insert(clean);
    }
}

fn insert_asset_type_report_tags(tags: &mut HashSet<String>, asset_type: &SecurityAssetType) {
    match asset_type {
        SecurityAssetType::Service => {
            tags.insert("service".to_string());
            tags.insert("服务".to_string());
        }
        SecurityAssetType::ApiEndpoint => {
            tags.insert("api".to_string());
            tags.insert("endpoint".to_string());
            tags.insert("接口".to_string());
        }
        SecurityAssetType::Url => {
            tags.insert("url".to_string());
            tags.insert("http".to_string());
            tags.insert("链接".to_string());
        }
        SecurityAssetType::Secret => {
            tags.insert("secret".to_string());
            tags.insert("密钥".to_string());
        }
        SecurityAssetType::Database => {
            tags.insert("database".to_string());
            tags.insert("db".to_string());
            tags.insert("数据库".to_string());
        }
        SecurityAssetType::Dependency => {
            tags.insert("dependency".to_string());
            tags.insert("package".to_string());
            tags.insert("依赖".to_string());
        }
        SecurityAssetType::Environment => {
            tags.insert("environment".to_string());
            tags.insert("env".to_string());
            tags.insert("环境".to_string());
        }
        SecurityAssetType::DataType => {
            tags.insert("data".to_string());
            tags.insert("datatype".to_string());
            tags.insert("数据类型".to_string());
        }
    }
}

fn insert_severity_report_tags(tags: &mut HashSet<String>, severity: &FindingSeverity) {
    match severity {
        FindingSeverity::Critical => {
            tags.insert("critical".to_string());
            tags.insert("高危".to_string());
        }
        FindingSeverity::Warning => {
            tags.insert("warning".to_string());
            tags.insert("警告".to_string());
        }
        FindingSeverity::Info => {
            tags.insert("info".to_string());
            tags.insert("提示".to_string());
        }
    }
}

fn insert_finding_kind_report_tags(tags: &mut HashSet<String>, kind: &FindingKind) {
    match kind {
        FindingKind::HardcodedSecret => {
            tags.insert("secret".to_string());
            tags.insert("hardcoded-secret".to_string());
            tags.insert("密钥".to_string());
        }
        FindingKind::WeakJwt => {
            tags.insert("jwt".to_string());
            tags.insert("token".to_string());
            tags.insert("令牌".to_string());
        }
        FindingKind::InsecureLink => {
            tags.insert("http".to_string());
            tags.insert("url".to_string());
            tags.insert("链接".to_string());
        }
        FindingKind::SensitiveOperation => {
            tags.insert("ops".to_string());
            tags.insert("operation".to_string());
            tags.insert("运维".to_string());
        }
        FindingKind::GovernanceGap => {
            tags.insert("governance".to_string());
            tags.insert("治理".to_string());
        }
    }
}

fn safe_report_slug(value: &str) -> String {
    let normalized = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    let slug = normalized
        .split('-')
        .filter(|part| !part.is_empty())
        .take(6)
        .collect::<Vec<_>>()
        .join("-");
    if slug.is_empty() {
        "scope".to_string()
    } else {
        slug
    }
}

fn extract_secret_placeholders(markdown: &str) -> Vec<String> {
    let marker = "{{secret:";
    let mut keys = Vec::new();
    let mut offset = 0;

    while let Some(start_rel) = markdown[offset..].find(marker) {
        let key_start = offset + start_rel + marker.len();
        let Some(end_rel) = markdown[key_start..].find("}}") else {
            break;
        };
        let key = markdown[key_start..key_start + end_rel].trim();
        if !key.is_empty() && !keys.iter().any(|existing| existing == key) {
            keys.push(key.to_string());
        }
        offset = key_start + end_rel + 2;
    }

    keys
}

fn resolve_redacted_draft_secrets(
    mut draft: DraftResponse,
    redacted_input: &redactor::RedactedInput,
) -> DraftResponse {
    if redacted_input.secrets.is_empty() {
        return draft;
    }

    let pending_values: HashMap<&str, &str> = redacted_input
        .secrets
        .iter()
        .map(|secret| (secret.key.as_str(), secret.value.as_str()))
        .collect();

    let mut resolved_secrets = HashMap::new();
    let mut placeholder_replacements = Vec::new();
    let llm_secret_map = std::mem::take(&mut draft.secrets);

    for (candidate_key, pending_reference) in llm_secret_map {
        let Some(pending_key) = extract_pending_secret_key(&pending_reference) else {
            continue;
        };
        let Some(plain_value) = pending_values.get(pending_key.as_str()) else {
            continue;
        };

        let final_key = normalize_secret_key(&candidate_key, &pending_key);
        resolved_secrets.insert(final_key.clone(), (*plain_value).to_string());
        placeholder_replacements.push((pending_key, final_key));
    }

    for secret in &redacted_input.secrets {
        if !resolved_secrets.contains_key(&secret.key)
            && draft.markdown.contains(&secret.placeholder)
        {
            resolved_secrets.insert(secret.key.clone(), secret.value.clone());
            placeholder_replacements.push((secret.key.clone(), secret.key.clone()));
        }
    }

    for (pending_key, final_key) in &placeholder_replacements {
        let pending_placeholder = format!("{{{{secret:{pending_key}}}}}");
        let final_placeholder = format!("{{{{secret:{final_key}}}}}");
        draft.markdown = draft
            .markdown
            .replace(&pending_placeholder, &final_placeholder);
    }

    for (key, value) in &resolved_secrets {
        if !value.trim().is_empty() && draft.markdown.contains(value) {
            let placeholder = format!("{{{{secret:{key}}}}}");
            draft.markdown = draft.markdown.replace(value, &placeholder);
        }
    }

    draft.secrets = resolved_secrets;
    draft
}

fn extract_pending_secret_key(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.starts_with("pending_") {
        return Some(trimmed.to_string());
    }

    let marker = "{{secret:";
    let start = trimmed.find(marker)? + marker.len();
    let end = trimmed[start..].find("}}")? + start;
    Some(trimmed[start..end].trim().to_string())
}

fn normalize_secret_key(candidate: &str, fallback: &str) -> String {
    let mut normalized = String::new();
    for ch in candidate.trim().chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            normalized.push(ch);
        }
    }

    if normalized.is_empty() {
        fallback.to_string()
    } else {
        normalized
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftResponse {
    pub title: String,
    pub file_name: String,
    pub markdown: String,
    pub secrets: HashMap<String, String>,
    pub summary: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentDetail {
    pub metadata: MemoMetadata,
    pub markdown: String,
    pub secrets: HashMap<String, String>,
    pub unlocked: bool,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SecretListItem {
    pub id: String,
    pub key: String,
    pub document_id: Option<String>,
    pub document_title: Option<String>,
    pub file_name: Option<String>,
    pub updated_at: Option<i64>,
    pub referenced: bool,
    pub has_value: bool,
    pub source: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretRevealResponse {
    pub id: String,
    pub value: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeMasterPasswordResponse {
    pub ok: bool,
    pub message: String,
    pub backup_path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchAnswerResponse {
    pub answer: String,
    pub sources: Vec<SearchSourceDoc>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchSourceDoc {
    pub id: String,
    pub title: String,
    pub file_name: String,
    pub score: f32,
}

#[derive(Debug, Deserialize)]
pub struct WebDavConfig {
    pub url: String,
    pub username: String,
    pub password: Option<String>, // We make password optional just in case, but usually required
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn make_test_dir(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("rusttool_memo_{}_{}", name, stamp));
        fs::create_dir_all(&dir).expect("test temp dir");
        dir
    }

    #[test]
    fn resolves_pending_secret_references_without_leaking_to_markdown() {
        let redacted_input = redactor::redact_secrets("服务器密码 abc123。");
        let draft = DraftResponse {
            title: "服务器".to_string(),
            file_name: "server.md".to_string(),
            markdown: "SSH 密码：{{secret:pending_1}}".to_string(),
            secrets: HashMap::from([("sshPassword".to_string(), "pending_1".to_string())]),
            summary: "服务器登录信息".to_string(),
        };

        let resolved = resolve_redacted_draft_secrets(draft, &redacted_input);

        assert_eq!(resolved.markdown, "SSH 密码：{{secret:sshPassword}}");
        assert_eq!(
            resolved.secrets.get("sshPassword"),
            Some(&"abc123".to_string())
        );
        assert!(!resolved.markdown.contains("abc123"));
    }

    #[test]
    fn keeps_pending_key_when_model_omits_secret_map() {
        let redacted_input = redactor::redact_secrets("数据库密码 db888。");
        let draft = DraftResponse {
            title: "数据库".to_string(),
            file_name: "database.md".to_string(),
            markdown: "数据库密码：{{secret:pending_1}}".to_string(),
            secrets: HashMap::new(),
            summary: "数据库登录信息".to_string(),
        };

        let resolved = resolve_redacted_draft_secrets(draft, &redacted_input);

        assert_eq!(resolved.markdown, "数据库密码：{{secret:pending_1}}");
        assert_eq!(
            resolved.secrets.get("pending_1"),
            Some(&"db888".to_string())
        );
    }

    #[tokio::test]
    async fn saves_document_secrets_to_kdbx() {
        let temp_dir = make_test_dir("kdbx_save");
        let manager = MemoManager::new(&temp_dir).expect("manager");
        assert!(manager
            .unlock("test-master-password")
            .await
            .expect("unlock"));

        let meta = manager
            .save_document(
                None,
                "servers/prod.md",
                "Prod Server",
                "SSH password: {{secret:sshPassword}}",
                HashMap::from([("sshPassword".to_string(), "abc123".to_string())]),
                "Prod credentials",
            )
            .await
            .expect("save document");

        let detail = manager.get_document(&meta.id).await.expect("get document");
        assert_eq!(
            detail.secrets.get("sshPassword"),
            Some(&"abc123".to_string())
        );

        let secret_vault_path = temp_dir.join("secrets.kdbx");
        assert!(secret_vault_path.exists());

        let vault =
            KdbxSecretVault::open(&secret_vault_path, "test-master-password").expect("open kdbx");
        assert_eq!(
            vault
                .get_secret(&document_secret_entry_key(&meta.id, "sshPassword"))
                .expect("read kdbx secret"),
            Some("abc123".to_string())
        );

        manager.lock().await;
        let locked_detail = manager
            .get_document(&meta.id)
            .await
            .expect("get locked document");
        assert!(locked_detail.secrets.is_empty());
        assert!(!locked_detail.unlocked);

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[tokio::test]
    async fn save_document_reports_risk_diff_against_previous_snapshot() {
        let temp_dir = make_test_dir("risk_diff_save");
        let manager = MemoManager::new(&temp_dir).expect("manager");
        assert!(manager
            .unlock("test-master-password")
            .await
            .expect("unlock"));

        let first = manager
            .save_document_with_risk_diff(
                None,
                "audit.md",
                "Audit",
                "api_key = \"sk-test-1234567890abcdef\"",
                HashMap::new(),
                "Audit sample",
            )
            .await
            .expect("first save");
        assert_eq!(first.risk_diff.summary.current_total, 1);
        assert!(first.risk_diff.previous_saved_at.is_none());

        let second = manager
            .save_document_with_risk_diff(
                Some(first.metadata.id.clone()),
                "audit.md",
                "Audit",
                "api_key = {{secret:apiKey}}",
                HashMap::from([("apiKey".to_string(), "sk-test-1234567890abcdef".to_string())]),
                "Audit sample",
            )
            .await
            .expect("second save");

        assert_eq!(second.risk_diff.summary.resolved, 1);
        assert_eq!(second.risk_diff.summary.current_total, 0);
        assert!(!serde_json::to_string(&second.risk_diff)
            .expect("serialize diff")
            .contains("sk-test-1234567890abcdef"));

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[tokio::test]
    async fn lists_and_reveals_document_secrets_without_returning_plaintext_in_list() {
        let temp_dir = make_test_dir("secret_inventory");
        let manager = MemoManager::new(&temp_dir).expect("manager");
        assert!(manager
            .unlock("test-master-password")
            .await
            .expect("unlock"));

        let meta = manager
            .save_document(
                None,
                "servers/prod.md",
                "Prod Server",
                "SSH password: {{secret:sshPassword}}",
                HashMap::from([("sshPassword".to_string(), "abc123".to_string())]),
                "Prod credentials",
            )
            .await
            .expect("save document");

        let secrets = manager.list_secrets().await.expect("list secrets");
        assert_eq!(secrets.len(), 1);
        assert_eq!(secrets[0].key, "sshPassword");
        assert_eq!(secrets[0].document_id.as_deref(), Some(meta.id.as_str()));
        assert_eq!(secrets[0].document_title.as_deref(), Some("Prod Server"));
        assert!(secrets[0].referenced);
        assert!(secrets[0].has_value);
        assert_eq!(secrets[0].source, "kdbx");

        let revealed = manager
            .reveal_secret(&secrets[0].id)
            .await
            .expect("reveal secret");
        assert_eq!(revealed.value, "abc123");

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[tokio::test]
    async fn stores_llm_api_key_in_kdbx_without_listing_it_as_document_secret() {
        let temp_dir = make_test_dir("llm_api_key_kdbx");
        let manager = MemoManager::new(&temp_dir).expect("manager");
        assert!(manager
            .unlock("test-master-password")
            .await
            .expect("unlock"));

        manager
            .update_llm_config(
                "https://api.openai.com/v1",
                Some("sk-test-secret"),
                "gpt-5.5",
                "text-embedding-3-small",
                "xhigh",
                true,
                false,
            )
            .await
            .expect("save settings");

        assert_eq!(
            manager
                .store
                .get_config(LEGACY_LLM_API_KEY_CONFIG)
                .expect("legacy config"),
            None
        );
        assert_eq!(
            manager
                .store
                .get_config(LLM_API_KEY_PRESENT_CONFIG)
                .expect("api key marker")
                .as_deref(),
            Some("true")
        );

        let vault = KdbxSecretVault::open(&temp_dir.join("secrets.kdbx"), "test-master-password")
            .expect("open kdbx");
        assert_eq!(
            vault
                .get_secret(LLM_API_KEY_SECRET_ID)
                .expect("read kdbx api key"),
            Some("sk-test-secret".to_string())
        );
        assert!(manager
            .list_secrets()
            .await
            .expect("list secrets")
            .is_empty());

        manager.lock().await;
        assert!(manager.has_llm_api_key().expect("has marker"));
        assert!(manager.get_llm_config().1.is_empty());
        assert!(manager
            .unlock("test-master-password")
            .await
            .expect("unlock again"));
        assert_eq!(manager.get_llm_config().1, "sk-test-secret");

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[tokio::test]
    async fn migrates_plaintext_llm_api_key_config_to_kdbx_on_unlock() {
        let temp_dir = make_test_dir("llm_api_key_config_migration");
        let manager = MemoManager::new(&temp_dir).expect("manager");
        manager
            .store
            .set_config(LEGACY_LLM_API_KEY_CONFIG, "legacy-secret")
            .expect("set legacy api key");

        assert!(manager.has_llm_api_key().expect("has legacy key"));
        assert!(manager
            .unlock("test-master-password")
            .await
            .expect("unlock"));

        assert_eq!(manager.get_llm_config().1, "legacy-secret");
        assert_eq!(
            manager
                .store
                .get_config(LEGACY_LLM_API_KEY_CONFIG)
                .expect("legacy config removed"),
            None
        );

        let vault = KdbxSecretVault::open(&temp_dir.join("secrets.kdbx"), "test-master-password")
            .expect("open kdbx");
        assert_eq!(
            vault
                .get_secret(LLM_API_KEY_SECRET_ID)
                .expect("read migrated key"),
            Some("legacy-secret".to_string())
        );

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[tokio::test]
    async fn changes_master_password_and_preserves_kdbx_secrets() {
        let temp_dir = make_test_dir("change_master_password");
        let manager = MemoManager::new(&temp_dir).expect("manager");
        assert!(manager.unlock("old-master-password").await.expect("unlock"));

        let meta = manager
            .save_document(
                None,
                "servers/prod.md",
                "Prod Server",
                "SSH password: {{secret:sshPassword}}",
                HashMap::from([("sshPassword".to_string(), "abc123".to_string())]),
                "Prod credentials",
            )
            .await
            .expect("save kdbx document");

        let response = manager
            .change_master_password("old-master-password", "new-master-password")
            .await
            .expect("change password");

        assert!(response.ok);
        assert!(Path::new(&response.backup_path).exists());
        assert!(manager.is_locked().await);
        assert!(!manager
            .unlock("old-master-password")
            .await
            .expect("old password should not unlock"));
        assert!(manager
            .unlock("new-master-password")
            .await
            .expect("new password unlocks"));

        let detail = manager
            .get_document(&meta.id)
            .await
            .expect("read kdbx document");
        assert_eq!(
            detail.secrets.get("sshPassword"),
            Some(&"abc123".to_string())
        );

        assert!(
            KdbxSecretVault::open(&temp_dir.join("secrets.kdbx"), "old-master-password").is_err()
        );
        assert!(
            KdbxSecretVault::open(&temp_dir.join("secrets.kdbx"), "new-master-password").is_ok()
        );

        let _ = fs::remove_dir_all(temp_dir);
    }
}
