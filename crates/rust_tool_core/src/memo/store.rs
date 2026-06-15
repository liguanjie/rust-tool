use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use super::governance::{AuditEvent, SecurityCase};
use super::history::DocumentRiskSnapshot;
use super::markdown_store;
use super::standards::{self, ChecklistStatusRecord};

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
    indexes_path: PathBuf,
    governance_path: PathBuf,
    reports_path: PathBuf,
    standards_path: PathBuf,
}

impl MemoStore {
    pub fn new(data_dir: &Path) -> Result<Self, String> {
        let data_dir = data_dir.to_path_buf();
        let config_path = data_dir.join("config.json");
        let vault_path = data_dir.join("documents");
        let embeddings_path = data_dir.join("embeddings");
        let indexes_path = data_dir.join("indexes");
        let governance_path = data_dir.join("governance");
        let reports_path = data_dir.join("reports");
        let standards_path = data_dir.join("standards");

        fs::create_dir_all(&data_dir)
            .map_err(|e| format!("Failed to create data directory: {:?}", e))?;
        fs::create_dir_all(&vault_path)
            .map_err(|e| format!("Failed to create documents directory: {:?}", e))?;

        Ok(Self {
            data_dir,
            config_path,
            vault_path,
            embeddings_path,
            indexes_path,
            governance_path,
            reports_path,
            standards_path,
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

    pub fn get_indexes_path(&self) -> &Path {
        &self.indexes_path
    }

    pub fn get_governance_path(&self) -> &Path {
        &self.governance_path
    }

    pub fn get_reports_path(&self) -> &Path {
        &self.reports_path
    }

    pub fn get_standards_path(&self) -> &Path {
        &self.standards_path
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

    pub fn read_finding_statuses(&self) -> Result<HashMap<String, String>, String> {
        let path = self.finding_statuses_path();
        if !path.exists() {
            return Ok(HashMap::new());
        }
        let content = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read finding statuses: {:?}", e))?;
        if content.trim().is_empty() {
            return Ok(HashMap::new());
        }
        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse finding statuses: {:?}", e))
    }

    pub fn set_finding_status(&self, finding_id: &str, status: &str) -> Result<(), String> {
        let finding_id = finding_id.trim();
        if finding_id.is_empty() {
            return Err("Finding id cannot be empty".to_string());
        }

        let mut statuses = self.read_finding_statuses()?;
        if status == "open" {
            statuses.remove(finding_id);
        } else {
            statuses.insert(finding_id.to_string(), status.to_string());
        }

        fs::create_dir_all(&self.indexes_path)
            .map_err(|e| format!("Failed to create indexes directory: {:?}", e))?;
        let json = serde_json::to_string_pretty(&statuses)
            .map_err(|e| format!("Failed to serialize finding statuses: {:?}", e))?;
        fs::write(self.finding_statuses_path(), json)
            .map_err(|e| format!("Failed to write finding statuses: {:?}", e))
    }

    fn finding_statuses_path(&self) -> PathBuf {
        self.indexes_path.join("finding-statuses.json")
    }

    pub fn read_document_risk_snapshot(
        &self,
        doc_id: &str,
    ) -> Result<Option<DocumentRiskSnapshot>, String> {
        let path = self.document_risk_snapshot_path(doc_id)?;
        if !path.exists() {
            return Ok(None);
        }
        let content = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read document risk snapshot: {:?}", e))?;
        if content.trim().is_empty() {
            return Ok(None);
        }
        serde_json::from_str(&content)
            .map(Some)
            .map_err(|e| format!("Failed to parse document risk snapshot: {:?}", e))
    }

    pub fn write_document_risk_snapshot(
        &self,
        snapshot: &DocumentRiskSnapshot,
    ) -> Result<(), String> {
        if snapshot.doc_id.trim().is_empty() {
            return Err("Document id cannot be empty".to_string());
        }
        fs::create_dir_all(self.document_history_dir())
            .map_err(|e| format!("Failed to create document history directory: {:?}", e))?;
        let json = serde_json::to_string_pretty(snapshot)
            .map_err(|e| format!("Failed to serialize document risk snapshot: {:?}", e))?;
        fs::write(self.document_risk_snapshot_path(&snapshot.doc_id)?, json)
            .map_err(|e| format!("Failed to write document risk snapshot: {:?}", e))
    }

    pub fn delete_document_risk_snapshot(&self, doc_id: &str) -> Result<(), String> {
        let path = self.document_risk_snapshot_path(doc_id)?;
        if path.exists() {
            fs::remove_file(path)
                .map_err(|e| format!("Failed to delete document risk snapshot: {:?}", e))?;
        }
        Ok(())
    }

    pub fn read_security_cases(&self) -> Result<Vec<SecurityCase>, String> {
        let path = self.security_cases_path();
        if !path.exists() {
            return Ok(Vec::new());
        }
        let content = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read security cases: {:?}", e))?;
        if content.trim().is_empty() {
            return Ok(Vec::new());
        }
        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse security cases: {:?}", e))
    }

    pub fn write_security_cases(&self, cases: &[SecurityCase]) -> Result<(), String> {
        fs::create_dir_all(&self.governance_path)
            .map_err(|e| format!("Failed to create governance directory: {:?}", e))?;
        let json = serde_json::to_string_pretty(cases)
            .map_err(|e| format!("Failed to serialize security cases: {:?}", e))?;
        fs::write(self.security_cases_path(), json)
            .map_err(|e| format!("Failed to write security cases: {:?}", e))
    }

    pub fn append_audit_events(&self, events: &[AuditEvent]) -> Result<(), String> {
        if events.is_empty() {
            return Ok(());
        }
        fs::create_dir_all(&self.governance_path)
            .map_err(|e| format!("Failed to create governance directory: {:?}", e))?;
        let mut content = String::new();
        for event in events {
            let json = serde_json::to_string(event)
                .map_err(|e| format!("Failed to serialize audit event: {:?}", e))?;
            content.push_str(&json);
            content.push('\n');
        }
        use std::io::Write;
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.audit_events_path())
            .map_err(|e| format!("Failed to open audit events: {:?}", e))?;
        file.write_all(content.as_bytes())
            .map_err(|e| format!("Failed to append audit events: {:?}", e))
    }

    pub fn read_audit_events(&self) -> Result<Vec<AuditEvent>, String> {
        let path = self.audit_events_path();
        if !path.exists() {
            return Ok(Vec::new());
        }
        let content = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read audit events: {:?}", e))?;
        let mut events = Vec::new();
        for (index, line) in content.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let event = serde_json::from_str::<AuditEvent>(line).map_err(|e| {
                format!("Failed to parse audit event at line {}: {:?}", index + 1, e)
            })?;
            events.push(event);
        }
        Ok(events)
    }

    pub fn write_security_report(
        &self,
        file_name: &str,
        markdown: &str,
    ) -> Result<PathBuf, String> {
        if file_name.trim().is_empty() || file_name.contains('/') || file_name.contains('\\') {
            return Err("Invalid report file name".to_string());
        }
        fs::create_dir_all(&self.reports_path)
            .map_err(|e| format!("Failed to create reports directory: {:?}", e))?;
        let path = self.reports_path.join(file_name);
        fs::write(&path, markdown).map_err(|e| format!("Failed to write report: {:?}", e))?;
        Ok(path)
    }

    pub fn read_checklist_statuses(
        &self,
    ) -> Result<HashMap<String, ChecklistStatusRecord>, String> {
        let path = self.checklist_statuses_path();
        if !path.exists() {
            return Ok(HashMap::new());
        }
        let content = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read checklist statuses: {:?}", e))?;
        if content.trim().is_empty() {
            return Ok(HashMap::new());
        }
        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse checklist statuses: {:?}", e))
    }

    pub fn set_checklist_status(
        &self,
        doc_id: Option<&str>,
        item_id: &str,
        record: ChecklistStatusRecord,
    ) -> Result<(), String> {
        if item_id.trim().is_empty() {
            return Err("Checklist item id cannot be empty".to_string());
        }
        fs::create_dir_all(&self.standards_path)
            .map_err(|e| format!("Failed to create standards directory: {:?}", e))?;
        let mut statuses = self.read_checklist_statuses()?;
        statuses.insert(standards::checklist_status_key(doc_id, item_id), record);
        let json = serde_json::to_string_pretty(&statuses)
            .map_err(|e| format!("Failed to serialize checklist statuses: {:?}", e))?;
        fs::write(self.checklist_statuses_path(), json)
            .map_err(|e| format!("Failed to write checklist statuses: {:?}", e))
    }

    fn security_cases_path(&self) -> PathBuf {
        self.governance_path.join("cases.json")
    }

    fn audit_events_path(&self) -> PathBuf {
        self.governance_path.join("events.jsonl")
    }

    fn checklist_statuses_path(&self) -> PathBuf {
        self.standards_path.join("checklist-statuses.json")
    }

    fn document_history_dir(&self) -> PathBuf {
        self.indexes_path.join("doc-history")
    }

    fn document_risk_snapshot_path(&self, doc_id: &str) -> Result<PathBuf, String> {
        let doc_id = doc_id.trim();
        if doc_id.is_empty() || doc_id.contains('/') || doc_id.contains('\\') {
            return Err("Invalid document id".to_string());
        }
        Ok(self
            .document_history_dir()
            .join(format!("{}.json", hex_encode(doc_id.as_bytes()))))
    }

    pub fn read_tree_state(&self) -> Result<serde_json::Value, String> {
        let path = self.indexes_path.join("tree-state.json");
        if !path.exists() {
            return Ok(serde_json::json!({}));
        }
        let content = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read tree state: {:?}", e))?;
        if content.trim().is_empty() {
            return Ok(serde_json::json!({}));
        }
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse tree state: {:?}", e))
    }

    pub fn write_tree_state(&self, state: &serde_json::Value) -> Result<(), String> {
        fs::create_dir_all(&self.indexes_path)
            .map_err(|e| format!("Failed to create indexes directory: {:?}", e))?;
        let json = serde_json::to_string_pretty(state)
            .map_err(|e| format!("Failed to serialize tree state: {:?}", e))?;
        fs::write(self.indexes_path.join("tree-state.json"), json)
            .map_err(|e| format!("Failed to write tree state: {:?}", e))
    }

    pub fn rename_folder(&self, old_path: &str, new_path: &str) -> Result<(), String> {
        let old_dir = self.resolve_document_path(old_path)?;
        let new_dir = self.resolve_document_path(new_path)?;

        if !old_dir.exists() || !old_dir.is_dir() {
            return Err("Old folder does not exist or is not a directory".to_string());
        }
        if new_dir.exists() {
            return Err("New folder path already exists".to_string());
        }

        if let Some(parent) = new_dir.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create parent directories: {:?}", e))?;
        }

        fs::rename(&old_dir, &new_dir)
            .map_err(|e| format!("Failed to rename folder: {:?}", e))?;
        Ok(())
    }

    pub fn delete_folder_to_unarchived(&self, path: &str) -> Result<(), String> {
        if path.trim() == "未归档" {
            return Err("Cannot delete the unarchived folder".to_string());
        }
        
        let target_dir = self.resolve_document_path(path)?;
        if !target_dir.exists() || !target_dir.is_dir() {
            return Err("Folder does not exist or is not a directory".to_string());
        }

        let unarchived_dir = self.vault_path.join("未归档");
        fs::create_dir_all(&unarchived_dir)
            .map_err(|e| format!("Failed to create unarchived directory: {:?}", e))?;

        let mut files_to_move = Vec::new();
        let mut dirs_to_visit = vec![target_dir.clone()];

        while let Some(dir) = dirs_to_visit.pop() {
            let entries = fs::read_dir(&dir)
                .map_err(|e| format!("Failed to read directory: {:?}", e))?;
            for entry in entries {
                let entry = entry.map_err(|e| format!("Failed to read entry: {:?}", e))?;
                let file_type = entry.file_type().map_err(|e| format!("Failed to get file type: {:?}", e))?;
                if file_type.is_dir() {
                    dirs_to_visit.push(entry.path());
                } else if file_type.is_file() {
                    let path = entry.path();
                    if path.extension().and_then(|ext| ext.to_str()).is_some_and(|ext| ext.eq_ignore_ascii_case("md")) {
                        files_to_move.push(path);
                    }
                }
            }
        }

        for file_path in files_to_move {
            let file_name = file_path.file_name().unwrap().to_string_lossy().to_string();
            let mut dest_path = unarchived_dir.join(&file_name);
            let mut counter = 1;
            while dest_path.exists() {
                let stem = file_path.file_stem().unwrap().to_string_lossy();
                let ext = file_path.extension().unwrap_or_default().to_string_lossy();
                let new_name = if ext.is_empty() {
                    format!("{}_{}", stem, counter)
                } else {
                    format!("{}_{}.{}", stem, counter, ext)
                };
                dest_path = unarchived_dir.join(new_name);
                counter += 1;
            }
            fs::rename(&file_path, &dest_path)
                .map_err(|e| format!("Failed to move file to unarchived: {:?}", e))?;
        }

        // Clean up the empty directories
        fs::remove_dir_all(&target_dir)
            .map_err(|e| format!("Failed to remove folder: {:?}", e))?;

        Ok(())
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
    fn persists_security_cases_and_audit_events() {
        use super::super::audit::FindingSeverity;
        use super::super::governance::{
            audit_event, SecurityCaseEvent, SecurityCaseStatus, SecurityCaseType,
        };
        use super::super::standards::{ChecklistStatus, ChecklistStatusRecord};

        let temp_dir = make_test_dir("governance_state");
        let store = MemoStore::new(&temp_dir).expect("store");
        let case = SecurityCase {
            id: "case-1".to_string(),
            case_type: SecurityCaseType::Risk,
            title: "硬编码密钥泄露".to_string(),
            severity: FindingSeverity::Critical,
            status: SecurityCaseStatus::Open,
            source_doc_id: "doc-1".to_string(),
            source_finding_id: Some("finding-1".to_string()),
            linked_assets: Vec::new(),
            owner: Some("sec".to_string()),
            due_at: Some("2026-06-30".to_string()),
            accepted_until: None,
            rationale: None,
            impact_scope: None,
            compensating_controls: None,
            reviewer: None,
            created_at: 100,
            updated_at: 100,
            events: vec![SecurityCaseEvent {
                event_type: "findingDetected".to_string(),
                summary: "发现风险".to_string(),
                created_at: 100,
            }],
        };

        store
            .write_security_cases(std::slice::from_ref(&case))
            .expect("write cases");
        let cases = store.read_security_cases().expect("read cases");
        assert_eq!(cases.len(), 1);
        assert_eq!(cases[0].id, "case-1");
        assert_eq!(cases[0].owner.as_deref(), Some("sec"));

        let event = audit_event(
            "caseStatusChanged",
            "user",
            "case-1",
            "风险进入修复中",
            101,
            serde_json::json!({ "status": "fixing" }),
        );
        store
            .append_audit_events(std::slice::from_ref(&event))
            .expect("append event");
        let events = store.read_audit_events().expect("read events");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "caseStatusChanged");
        assert_eq!(events[0].target_id, "case-1");

        let report_path = store
            .write_security_report("security-report-test.md", "# 安全治理审计报告")
            .expect("write report");
        assert_eq!(
            report_path,
            store.get_reports_path().join("security-report-test.md")
        );
        assert!(store.write_security_report("../leak.md", "nope").is_err());

        store
            .set_checklist_status(
                Some("doc-1"),
                "secret-management",
                ChecklistStatusRecord {
                    item_id: "secret-management".to_string(),
                    status: ChecklistStatus::Done,
                    note: Some("已完成轮换".to_string()),
                    updated_at: 102,
                },
            )
            .expect("write checklist status");
        let statuses = store.read_checklist_statuses().expect("read checklist");
        let record = statuses
            .get("doc-1:secret-management")
            .expect("checklist record");
        assert_eq!(record.status, ChecklistStatus::Done);
        assert_eq!(record.note.as_deref(), Some("已完成轮换"));
        assert_eq!(store.get_standards_path(), temp_dir.join("standards"));

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
