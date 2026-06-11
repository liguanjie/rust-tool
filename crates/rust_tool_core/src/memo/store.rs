use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use super::markdown_store;

const MEMO_CONFIG_KEY: &str = "memoConfig";

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MemoMetadata {
    pub id: String,
    pub file_name: String,
    pub title: String,
    pub summary: String,
    pub updated_at: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MemoSecretInfo {
    pub id: String,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct EmbeddingRecord {
    id: String,
    embedding: Vec<f32>,
}

pub struct MemoStore {
    data_dir: PathBuf,
    config_path: PathBuf,
    vault_path: PathBuf,
    embeddings_path: PathBuf,
}

impl MemoStore {
    pub fn new(data_dir: &Path) -> Result<Self, String> {
        let data_dir = data_dir.to_path_buf();
        let config_path = data_dir.join("config.json");
        let vault_path = data_dir.join("documents");
        let embeddings_path = data_dir.join("embeddings");

        fs::create_dir_all(&data_dir)
            .map_err(|e| format!("Failed to create data directory: {:?}", e))?;
        fs::create_dir_all(&vault_path)
            .map_err(|e| format!("Failed to create documents directory: {:?}", e))?;

        Ok(Self {
            data_dir,
            config_path,
            vault_path,
            embeddings_path,
        })
    }

    // --- Configuration management ---

    pub fn get_config(&self, key: &str) -> Result<Option<String>, String> {
        let document = self.read_config_document()?;
        let Some(Value::Object(config)) = document.get(MEMO_CONFIG_KEY) else {
            return Ok(None);
        };

        Ok(config.get(key).and_then(Value::as_str).map(str::to_string))
    }

    pub fn set_config(&self, key: &str, value: &str) -> Result<(), String> {
        let mut document = self.read_config_document()?;
        let config = ensure_memo_config_object(&mut document)?;
        config.insert(key.to_string(), Value::String(value.to_string()));
        self.write_config_document(&document)
    }

    pub fn delete_config(&self, key: &str) -> Result<(), String> {
        let mut document = self.read_config_document()?;
        if let Some(Value::Object(config)) = document.get_mut(MEMO_CONFIG_KEY) {
            config.remove(key);
        }
        self.write_config_document(&document)
    }

    fn read_config_document(&self) -> Result<Map<String, Value>, String> {
        if !self.config_path.exists() {
            return Ok(Map::new());
        }

        let content = fs::read_to_string(&self.config_path)
            .map_err(|e| format!("Failed to read config.json: {:?}", e))?;
        if content.trim().is_empty() {
            return Ok(Map::new());
        }

        match serde_json::from_str::<Value>(&content)
            .map_err(|e| format!("Failed to parse config.json: {:?}", e))?
        {
            Value::Object(object) => Ok(object),
            _ => Err("config.json must contain a JSON object".to_string()),
        }
    }

    fn write_config_document(&self, document: &Map<String, Value>) -> Result<(), String> {
        if let Some(parent) = self.config_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config directory: {:?}", e))?;
        }
        let value = Value::Object(document.clone());
        let json = serde_json::to_string_pretty(&value)
            .map_err(|e| format!("Failed to serialize config.json: {:?}", e))?;
        fs::write(&self.config_path, json)
            .map_err(|e| format!("Failed to write config.json: {:?}", e))
    }

    // --- Password master salt & validation ---

    pub fn get_or_create_salt(&self) -> Result<Vec<u8>, String> {
        match self.get_config("master_salt")? {
            Some(salt_hex) => hex_decode(&salt_hex),
            None => {
                use aes_gcm::aead::rand_core::{OsRng, RngCore};
                let mut salt = [0u8; 16];
                OsRng.fill_bytes(&mut salt);
                let salt_hex = hex_encode(&salt);
                self.set_config("master_salt", &salt_hex)?;
                Ok(salt.to_vec())
            }
        }
    }

    pub fn set_password_verifier(&self, encrypted_verifier: &str) -> Result<(), String> {
        self.set_config("password_verifier", encrypted_verifier)
    }

    pub fn get_password_verifier(&self) -> Result<Option<String>, String> {
        self.get_config("password_verifier")
    }

    // --- Documents read/write ---

    pub fn get_data_dir(&self) -> &Path {
        &self.data_dir
    }

    pub fn get_config_path(&self) -> &Path {
        &self.config_path
    }

    pub fn get_vault_path(&self) -> &Path {
        &self.vault_path
    }

    pub fn get_embeddings_path(&self) -> &Path {
        &self.embeddings_path
    }

    pub fn get_secret_vault_path(&self) -> PathBuf {
        self.data_dir.join("secrets.kdbx")
    }

    fn resolve_document_path(&self, file_name: &str) -> Result<PathBuf, String> {
        let relative_path = Path::new(file_name);
        if relative_path.is_absolute() {
            return Err("Invalid file path: absolute paths are not allowed".to_string());
        }

        if relative_path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        }) {
            return Err("Invalid file path: parent traversal is not allowed".to_string());
        }

        if file_name.trim().is_empty()
            || relative_path
                .components()
                .all(|component| matches!(component, Component::CurDir))
        {
            return Err("Invalid file path: file name cannot be empty".to_string());
        }

        Ok(self.vault_path.join(relative_path))
    }

    pub fn save_document_file(&self, file_name: &str, markdown: &str) -> Result<PathBuf, String> {
        let doc_path = self.resolve_document_path(file_name)?;

        if let Some(parent) = doc_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create subdirectories: {:?}", e))?;
        }

        fs::write(&doc_path, markdown).map_err(|e| format!("Failed to write document: {:?}", e))?;

        Ok(doc_path)
    }

    pub fn save_document_with_metadata(
        &self,
        meta: &MemoMetadata,
        markdown: &str,
    ) -> Result<PathBuf, String> {
        let rendered = markdown_store::render_document(meta, markdown)?;
        self.save_document_file(&meta.file_name, &rendered)
    }

    pub fn read_document_file(&self, file_name: &str) -> Result<String, String> {
        let doc_path = self.resolve_document_path(file_name)?;

        let content = fs::read_to_string(&doc_path)
            .map_err(|e| format!("Failed to read document: {:?}", e))?;
        Ok(markdown_store::strip_frontmatter(&content)
            .trim_start_matches('\n')
            .to_string())
    }

    pub fn delete_document_file(&self, file_name: &str) -> Result<(), String> {
        let doc_path = self.resolve_document_path(file_name)?;

        if doc_path.exists() {
            fs::remove_file(&doc_path)
                .map_err(|e| format!("Failed to delete document file: {:?}", e))?;
        }
        Ok(())
    }

    // --- Metadata management ---

    pub fn upsert_memo_metadata(&self, _meta: &MemoMetadata) -> Result<(), String> {
        // Metadata lives in each Markdown file's frontmatter.
        Ok(())
    }

    pub fn get_all_memos(&self) -> Result<Vec<MemoMetadata>, String> {
        markdown_store::list_documents(&self.vault_path, &HashMap::new())
    }

    pub fn get_memo_metadata(&self, id: &str) -> Result<Option<MemoMetadata>, String> {
        Ok(self.get_all_memos()?.into_iter().find(|meta| meta.id == id))
    }

    pub fn file_name_owner(&self, file_name: &str) -> Result<Option<String>, String> {
        Ok(self
            .get_all_memos()?
            .into_iter()
            .find(|meta| meta.file_name == file_name)
            .map(|meta| meta.id))
    }

    pub fn delete_memo_metadata(&self, id: &str) -> Result<(), String> {
        self.delete_embedding(id)
    }

    // --- Vector Embeddings ---

    pub fn save_embedding(&self, id: &str, embedding: &[f32]) -> Result<(), String> {
        fs::create_dir_all(&self.embeddings_path)
            .map_err(|e| format!("Failed to create embeddings directory: {:?}", e))?;
        let record = EmbeddingRecord {
            id: id.to_string(),
            embedding: embedding.to_vec(),
        };
        let json = serde_json::to_vec(&record)
            .map_err(|e| format!("Failed to serialize embedding: {:?}", e))?;
        fs::write(self.embedding_path(id), json)
            .map_err(|e| format!("Failed to write embedding: {:?}", e))
    }

    pub fn get_embedding(&self, id: &str) -> Result<Option<Vec<f32>>, String> {
        let path = self.embedding_path(id);
        if !path.exists() {
            return Ok(None);
        }
        let bytes = fs::read(&path).map_err(|e| format!("Failed to read embedding: {:?}", e))?;
        let record: EmbeddingRecord = serde_json::from_slice(&bytes)
            .map_err(|e| format!("Failed to parse embedding: {:?}", e))?;
        Ok(Some(record.embedding))
    }

    pub fn get_all_embeddings(&self) -> Result<Vec<(String, Vec<f32>)>, String> {
        let mut list = Vec::new();
        if !self.embeddings_path.exists() {
            return Ok(list);
        }

        for entry in fs::read_dir(&self.embeddings_path)
            .map_err(|e| format!("Failed to read embeddings directory: {:?}", e))?
        {
            let entry = entry.map_err(|e| format!("Failed to read embedding entry: {:?}", e))?;
            let path = entry.path();
            if !path.is_file()
                || !path
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
            {
                continue;
            }
            let bytes =
                fs::read(&path).map_err(|e| format!("Failed to read embedding: {:?}", e))?;
            let record: EmbeddingRecord = serde_json::from_slice(&bytes)
                .map_err(|e| format!("Failed to parse embedding: {:?}", e))?;
            list.push((record.id, record.embedding));
        }

        Ok(list)
    }

    fn delete_embedding(&self, id: &str) -> Result<(), String> {
        let path = self.embedding_path(id);
        if path.exists() {
            fs::remove_file(path).map_err(|e| format!("Failed to delete embedding: {:?}", e))?;
        }
        Ok(())
    }

    fn embedding_path(&self, id: &str) -> PathBuf {
        self.embeddings_path
            .join(format!("{}.json", hex_encode(id.as_bytes())))
    }
}

fn ensure_memo_config_object(
    document: &mut Map<String, Value>,
) -> Result<&mut Map<String, Value>, String> {
    if !document.contains_key(MEMO_CONFIG_KEY) {
        document.insert(MEMO_CONFIG_KEY.to_string(), Value::Object(Map::new()));
    }

    match document.get_mut(MEMO_CONFIG_KEY) {
        Some(Value::Object(config)) => Ok(config),
        _ => Err("config.json field memoConfig must contain an object".to_string()),
    }
}

// Helpers for hex encoding/decoding without external crate dependencies
fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

fn hex_decode(s: &str) -> Result<Vec<u8>, String> {
    if s.len() % 2 != 0 {
        return Err("Invalid hex string length".to_string());
    }
    let mut bytes = Vec::with_capacity(s.len() / 2);
    for i in (0..s.len()).step_by(2) {
        let res = u8::from_str_radix(&s[i..i + 2], 16)
            .map_err(|e| format!("Hex decode error: {:?}", e))?;
        bytes.push(res);
    }
    Ok(bytes)
}

pub fn current_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_dir(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("rusttool_{}_{}", name, stamp));
        fs::create_dir_all(&dir).expect("test temp dir");
        dir
    }

    #[test]
    fn rejects_document_path_traversal() {
        let temp_dir = make_test_dir("path_reject");
        let store = MemoStore::new(&temp_dir).expect("store");

        assert!(store.save_document_file("../outside.md", "nope").is_err());
        assert!(store
            .save_document_file("nested/../../outside.md", "nope")
            .is_err());
        assert!(store.read_document_file("../outside.md").is_err());
        assert!(store.delete_document_file("../outside.md").is_err());

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn accepts_nested_relative_document_paths() {
        let temp_dir = make_test_dir("path_accept");
        let store = MemoStore::new(&temp_dir).expect("store");

        store
            .save_document_file("folder/doc.md", "# ok")
            .expect("save nested doc");

        assert_eq!(
            store
                .read_document_file("folder/doc.md")
                .expect("read nested doc"),
            "# ok"
        );

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn lists_markdown_files_created_without_database() {
        let temp_dir = make_test_dir("markdown_list");
        let store = MemoStore::new(&temp_dir).expect("store");
        let doc_dir = store.get_vault_path().join("servers");
        fs::create_dir_all(&doc_dir).expect("doc dir");
        fs::write(
            doc_dir.join("prod.md"),
            "---\nid: prod-server\ntitle: 生产服务器\nsummary: 外部创建的 Markdown\nupdatedAt: 99\n---\n\n# 生产服务器",
        )
        .expect("write markdown");

        let docs = store.get_all_memos().expect("list memos");

        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].id, "prod-server");
        assert_eq!(docs[0].file_name, "servers/prod.md");
        assert_eq!(docs[0].title, "生产服务器");

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn saves_frontmatter_but_reads_editor_body() {
        let temp_dir = make_test_dir("markdown_frontmatter");
        let store = MemoStore::new(&temp_dir).expect("store");
        let meta = MemoMetadata {
            id: "doc-1".to_string(),
            file_name: "doc.md".to_string(),
            title: "文档标题".to_string(),
            summary: "文档摘要".to_string(),
            updated_at: 123,
        };

        store
            .save_document_with_metadata(&meta, "# 文档标题\n\n正文")
            .expect("save markdown with frontmatter");

        let raw = fs::read_to_string(store.get_vault_path().join("doc.md")).expect("raw file");
        assert!(raw.contains("id: doc-1"));
        assert_eq!(
            store.read_document_file("doc.md").expect("read doc"),
            "# 文档标题\n\n正文"
        );

        let docs = store.get_all_memos().expect("list memos");
        assert_eq!(docs[0].id, "doc-1");
        assert_eq!(docs[0].summary, "文档摘要");

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn stores_config_in_json_without_clobbering_local_fields() {
        let temp_dir = make_test_dir("json_config");
        fs::write(
            temp_dir.join("config.json"),
            r#"{"customDataDir":"/tmp/elsewhere"}"#,
        )
        .expect("seed config");
        let store = MemoStore::new(&temp_dir).expect("store");

        store
            .set_config("ollama_base_url", "https://api.openai.com/v1")
            .expect("set config");

        assert_eq!(
            store.get_config("ollama_base_url").expect("read config"),
            Some("https://api.openai.com/v1".to_string())
        );
        let content = fs::read_to_string(temp_dir.join("config.json")).expect("read config file");
        assert!(content.contains("customDataDir"));
        assert!(content.contains("memoConfig"));

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn stores_embeddings_as_json_files() {
        let temp_dir = make_test_dir("embeddings");
        let store = MemoStore::new(&temp_dir).expect("store");

        store
            .save_embedding("doc-1", &[0.1, 0.2, 0.3])
            .expect("save embedding");

        assert_eq!(
            store.get_embedding("doc-1").expect("read embedding"),
            Some(vec![0.1, 0.2, 0.3])
        );
        assert_eq!(store.get_all_embeddings().expect("all embeddings").len(), 1);
        store
            .delete_memo_metadata("doc-1")
            .expect("delete embedding");
        assert_eq!(store.get_embedding("doc-1").expect("read missing"), None);

        let _ = fs::remove_dir_all(temp_dir);
    }
}
