pub mod backup;
pub mod crypto;
pub mod store;
pub mod vector;

use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

pub use store::{current_timestamp, MemoMetadata, MemoStore};
pub use vector::{cosine_similarity, ChatMessage, LlmClient};

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
}

impl MemoManager {
    pub fn new(data_dir: &Path) -> Result<Self, String> {
        let store = MemoStore::new(data_dir)?;

        // Read configs or set defaults
        let llm_base_url = store
            .get_config("ollama_base_url")?
            .unwrap_or_else(|| "https://api.openai.com/v1".to_string());
        let llm_api_key = store.get_config("ollama_api_key")?.unwrap_or_default();
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
            api_key: llm_api_key,
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
                            let mut lock = self.master_key.write().await;
                            *lock = Some(derived);
                            Ok(true)
                        } else {
                            Ok(false)
                        }
                    }
                    Err(_) => Ok(false), // Password incorrect, decryption failed
                }
            }
            None => {
                // No password set yet. We initialize it with this password!
                let verifier_str = crypto::encrypt(b"verified_token", &derived)?;
                self.store.set_password_verifier(&verifier_str)?;

                let mut lock = self.master_key.write().await;
                *lock = Some(derived);
                Ok(true)
            }
        }
    }

    /// Lock the vault, erasing the key from memory.
    pub async fn lock(&self) {
        let mut lock = self.master_key.write().await;
        *lock = None;
    }

    /// Get current master key if unlocked, otherwise return error.
    async fn get_key(&self) -> Result<[u8; 32], String> {
        self.master_key
            .read()
            .await
            .ok_or_else(|| "Vault is locked. Please unlock first.".to_string())
    }

    /// Update OpenAI-compatible LLM configuration.
    pub fn update_llm_config(
        &self,
        base_url: &str,
        api_key: Option<&str>,
        chat_model: &str,
        embedding_model: &str,
        reasoning_effort: &str,
        disable_response_storage: bool,
        allow_ai_secrets: bool,
    ) -> Result<(), String> {
        {
            let mut config = self.llm_config.write().unwrap();
            config.base_url = base_url.to_string();
            if let Some(api_key) = api_key {
                config.api_key = api_key.to_string();
            }
            config.chat_model = chat_model.to_string();
            config.embedding_model = embedding_model.to_string();
            config.reasoning_effort = reasoning_effort.to_string();
            config.disable_response_storage = disable_response_storage;
            config.allow_ai_secrets = allow_ai_secrets;
        }

        self.store.set_config("ollama_base_url", base_url)?;
        if let Some(api_key) = api_key {
            self.store.set_config("ollama_api_key", api_key)?;
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
        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: "You are a concise assistant embedded in a local Markdown knowledge-base app. Answer normal chat directly. If the user asks to create, edit, or search local documents, briefly tell them what action to request in the app.".to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: query.to_string(),
            },
        ];

        client.chat(messages, false).await
    }

    // --- Core feature 1: AI-assisted write ---

    pub async fn draft_document_with_ai(&self, raw_input: &str) -> Result<DraftResponse, String> {
        // Vault must be unlocked to process and encrypt secrets
        let _key = self.get_key().await?;

        let client = self.get_ollama_client();

        let system_prompt = "You are a professional documentation organizer. Your task is to organize the user's raw, unstructured, or dictation notes into a well-structured, neat Markdown document.
Additionally, you must identify and extract sensitive information (such as passwords, API keys, credentials, secret tokens, etc.).

For each extracted secret:
1. Generate a unique, descriptive key in camelCase (e.g. \"mysqlPassword\", \"awsAccessKey\").
2. In the Markdown text, replace the plaintext value with a placeholder like `{{secret:key}}`.
   For example, if you find \"password is admin123\", replace it with \"password is {{secret:adminPassword}}\".

Your response MUST be a JSON object ONLY, with no extra markdown wrapping (do not use ```json or similar). The JSON structure:
{
  \"title\": \"A short, descriptive title of the document\",
  \"fileName\": \"A URL-safe filename ending with .md (e.g. vps_credentials.md)\",
  \"markdown\": \"The formatted Markdown content with secrets replaced by placeholders\",
  \"secrets\": {
    \"key1\": \"plaintext_value1\",
    \"key2\": \"plaintext_value2\"
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
                content: raw_input.to_string(),
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

        Ok(draft)
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
        let key = self.get_key().await?;
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

        if let Some(owner_id) = self.store.file_name_owner(file_name)? {
            if owner_id != doc_id {
                return Err("A document with this file name already exists.".to_string());
            }
        }

        // 1. Encrypt and replace the document's full secret set.
        let mut encrypted_secrets = Vec::with_capacity(secrets.len());
        for (sec_key, plain_val) in &secrets {
            let full_secret_id = format!("{}:{}", doc_id, sec_key);
            let encrypted = crypto::encrypt(plain_val.as_bytes(), &key)?;
            encrypted_secrets.push((
                full_secret_id,
                encrypted,
                format!("Secret for document: {}", title),
            ));
        }
        self.store
            .replace_document_secrets(&doc_id, &encrypted_secrets)?;

        // 2. Save Markdown file
        self.store.save_document_file(file_name, markdown)?;
        if let Some(previous_meta) = &previous_meta {
            if previous_meta.file_name != file_name {
                self.store.delete_document_file(&previous_meta.file_name)?;
            }
        }

        // 3. Upsert Metadata
        let meta = MemoMetadata {
            id: doc_id.clone(),
            file_name: file_name.to_string(),
            title: title.to_string(),
            summary: summary.to_string(),
            updated_at: current_timestamp(),
        };
        self.store.upsert_memo_metadata(&meta)?;

        // 4. Generate & Save Vector Embedding asynchronously
        let client = self.get_ollama_client();

        // Use title + summary + markdown (without actual plain passwords) as embedding text
        let embedding_text = format!(
            "Title: {}\nSummary: {}\nContent:\n{}",
            title, summary, markdown
        );

        let manager_store = self.store.connect().map_err(|e| e.to_string())?; // Check database accessibility
        drop(manager_store);

        let store_clone = self.store.db_path.clone();
        let doc_id_clone = doc_id.clone();

        // Spawn embedding generation in the background so API stays fast
        tokio::spawn(async move {
            if let Ok(vec) = client.get_embedding(&embedding_text).await {
                let temp_store = MemoStore::new(store_clone.parent().unwrap()).unwrap();
                let _ = temp_store.save_embedding(&doc_id_clone, &vec);
            }
        });

        Ok(meta)
    }

    /// Read document contents. If unlocked, decrypt placeholders.
    pub async fn get_document(&self, id: &str) -> Result<DocumentDetail, String> {
        let meta = self
            .store
            .get_memo_metadata(id)?
            .ok_or_else(|| "Document not found".to_string())?;

        let markdown = self.store.read_document_file(&meta.file_name)?;

        let mut decrypted_secrets = HashMap::new();
        let key_opt = self.master_key.read().await;

        if let Some(key) = *key_opt {
            // Document is unlocked, retrieve all secrets matching "doc_id:*"
            let conn = self.store.connect().map_err(|e| e.to_string())?;
            let mut stmt = conn
                .prepare("SELECT id, encrypted_value FROM memo_secrets WHERE id LIKE ?")
                .map_err(|e| e.to_string())?;

            let prefix = format!("{}:%", id);
            let rows = stmt
                .query_map(params![prefix], |row| {
                    let full_id: String = row.get(0)?;
                    let enc: String = row.get(1)?;
                    Ok((full_id, enc))
                })
                .map_err(|e| e.to_string())?;

            for r in rows {
                let (full_id, enc_val) = r.map_err(|e| e.to_string())?;
                // Extract original key from "doc_id:key"
                if let Some(sec_key) = full_id.strip_prefix(&format!("{}:", id)) {
                    if let Ok(dec_bytes) = crypto::decrypt(&enc_val, &key) {
                        let plain = String::from_utf8_lossy(&dec_bytes).to_string();
                        decrypted_secrets.insert(sec_key.to_string(), plain);
                    }
                }
            }
        }

        Ok(DocumentDetail {
            metadata: meta,
            markdown,
            secrets: decrypted_secrets,
            unlocked: key_opt.is_some(),
        })
    }

    /// Delete a document and its associated secrets and embeddings
    pub async fn delete_document(&self, id: &str) -> Result<(), String> {
        let meta = self.store.get_memo_metadata(id)?;
        if let Some(m) = meta {
            // Delete file
            self.store.delete_document_file(&m.file_name)?;
        }

        // Delete secrets from database
        let conn = self.store.connect().map_err(|e| e.to_string())?;
        conn.execute(
            "DELETE FROM memo_secrets WHERE id LIKE ?",
            params![format!("{}:%", id)],
        )
        .map_err(|e| e.to_string())?;

        // Delete metadata & embedding
        self.store.delete_memo_metadata(id)?;

        Ok(())
    }

    // --- Core feature 3: AI Search & QA (RAG) ---

    pub async fn search_and_answer(&self, query: &str) -> Result<SearchAnswerResponse, String> {
        let client = self.get_ollama_client();

        // 1. Get query embedding
        let query_vec = client.get_embedding(query).await?;

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
                content: query.to_string(),
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
        backup::create_backup_zip(
            self.store.get_vault_path(),
            self.store.get_db_path(),
            &temp_zip_path,
        )?;

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

    pub fn restore(&self, zip_path: &Path) -> Result<(), String> {
        backup::restore_from_zip(
            zip_path,
            self.store.get_vault_path(),
            self.store.get_db_path(),
        )?;
        Ok(())
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
