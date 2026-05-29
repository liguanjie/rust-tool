use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

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

pub struct MemoStore {
    pub db_path: PathBuf,
    pub vault_path: PathBuf,
}

impl MemoStore {
    pub fn new(data_dir: &Path) -> Result<Self, String> {
        let db_path = data_dir.join("memos.db");
        let vault_path = data_dir.join("documents");

        // Ensure directories exist
        fs::create_dir_all(&vault_path)
            .map_err(|e| format!("Failed to create vault directory: {:?}", e))?;

        let store = Self {
            db_path,
            vault_path,
        };
        store
            .init_db()
            .map_err(|e| format!("Database initialization failed: {}", e))?;

        Ok(store)
    }

    pub fn connect(&self) -> Result<Connection, rusqlite::Error> {
        Connection::open(&self.db_path)
    }

    fn init_db(&self) -> Result<(), String> {
        let conn = self.connect().map_err(|e| e.to_string())?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS memo_config (
                key TEXT PRIMARY KEY,
                value TEXT
            );",
            [],
        )
        .map_err(|e| e.to_string())?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS memo_secrets (
                id TEXT PRIMARY KEY,
                encrypted_value TEXT,
                description TEXT
            );",
            [],
        )
        .map_err(|e| e.to_string())?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS memos (
                id TEXT PRIMARY KEY,
                file_name TEXT,
                title TEXT,
                summary TEXT,
                updated_at INTEGER
            );",
            [],
        )
        .map_err(|e| e.to_string())?;

        let duplicate_file_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM (
                SELECT file_name FROM memos
                WHERE file_name IS NOT NULL AND file_name != ''
                GROUP BY file_name
                HAVING COUNT(*) > 1
            )",
                [],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        if duplicate_file_count > 0 {
            return Err(
                "Duplicate memo file names exist; please resolve them before upgrading."
                    .to_string(),
            );
        }

        conn.execute(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_memos_file_name_unique
             ON memos(file_name)",
            [],
        )
        .map_err(|e| e.to_string())?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS memo_embeddings (
                id TEXT PRIMARY KEY,
                embedding BLOB
            );",
            [],
        )
        .map_err(|e| e.to_string())?;

        Ok(())
    }

    // --- Configuration management ---

    pub fn get_config(&self, key: &str) -> Result<Option<String>, String> {
        let conn = self.connect().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT value FROM memo_config WHERE key = ?")
            .map_err(|e| e.to_string())?;

        let val: Option<String> = stmt
            .query_row(params![key], |row| row.get(0))
            .optional()
            .map_err(|e| e.to_string())?;

        Ok(val)
    }

    pub fn set_config(&self, key: &str, value: &str) -> Result<(), String> {
        let conn = self.connect().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT OR REPLACE INTO memo_config (key, value) VALUES (?, ?)",
            params![key, value],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
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

    // --- Secrets management ---

    pub fn get_secret(&self, id: &str) -> Result<Option<String>, String> {
        let conn = self.connect().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT encrypted_value FROM memo_secrets WHERE id = ?")
            .map_err(|e| e.to_string())?;

        let val: Option<String> = stmt
            .query_row(params![id], |row| row.get(0))
            .optional()
            .map_err(|e| e.to_string())?;

        Ok(val)
    }

    pub fn delete_secret(&self, id: &str) -> Result<(), String> {
        let conn = self.connect().map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM memo_secrets WHERE id = ?", params![id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn replace_document_secrets(
        &self,
        doc_id: &str,
        secrets: &[(String, String, String)],
    ) -> Result<(), String> {
        let mut conn = self.connect().map_err(|e| e.to_string())?;
        let tx = conn.transaction().map_err(|e| e.to_string())?;

        tx.execute(
            "DELETE FROM memo_secrets WHERE id LIKE ?",
            params![format!("{}:%", doc_id)],
        )
        .map_err(|e| e.to_string())?;

        for (id, encrypted_value, description) in secrets {
            tx.execute(
                "INSERT INTO memo_secrets (id, encrypted_value, description) VALUES (?, ?, ?)",
                params![id, encrypted_value, description],
            )
            .map_err(|e| e.to_string())?;
        }

        tx.commit().map_err(|e| e.to_string())?;
        Ok(())
    }

    // --- Documents read/write ---

    pub fn get_vault_path(&self) -> &Path {
        &self.vault_path
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

        // Create parent directories if any
        if let Some(parent) = doc_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create subdirectories: {:?}", e))?;
        }

        fs::write(&doc_path, markdown).map_err(|e| format!("Failed to write document: {:?}", e))?;

        Ok(doc_path)
    }

    pub fn read_document_file(&self, file_name: &str) -> Result<String, String> {
        let doc_path = self.resolve_document_path(file_name)?;

        fs::read_to_string(&doc_path).map_err(|e| format!("Failed to read document: {:?}", e))
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

    pub fn upsert_memo_metadata(&self, meta: &MemoMetadata) -> Result<(), String> {
        let conn = self.connect().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT OR REPLACE INTO memos (id, file_name, title, summary, updated_at) VALUES (?, ?, ?, ?, ?)",
            params![meta.id, meta.file_name, meta.title, meta.summary, meta.updated_at],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_all_memos(&self) -> Result<Vec<MemoMetadata>, String> {
        let conn = self.connect().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT id, file_name, title, summary, updated_at FROM memos ORDER BY updated_at DESC")
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                Ok(MemoMetadata {
                    id: row.get(0)?,
                    file_name: row.get(1)?,
                    title: row.get(2)?,
                    summary: row.get(3)?,
                    updated_at: row.get(4)?,
                })
            })
            .map_err(|e| e.to_string())?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r.map_err(|e| e.to_string())?);
        }
        Ok(list)
    }

    pub fn get_memo_metadata(&self, id: &str) -> Result<Option<MemoMetadata>, String> {
        let conn = self.connect().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT id, file_name, title, summary, updated_at FROM memos WHERE id = ?")
            .map_err(|e| e.to_string())?;

        let meta = stmt
            .query_row(params![id], |row| {
                Ok(MemoMetadata {
                    id: row.get(0)?,
                    file_name: row.get(1)?,
                    title: row.get(2)?,
                    summary: row.get(3)?,
                    updated_at: row.get(4)?,
                })
            })
            .optional()
            .map_err(|e| e.to_string())?;

        Ok(meta)
    }

    pub fn file_name_owner(&self, file_name: &str) -> Result<Option<String>, String> {
        let conn = self.connect().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT id FROM memos WHERE file_name = ?")
            .map_err(|e| e.to_string())?;

        stmt.query_row(params![file_name], |row| row.get(0))
            .optional()
            .map_err(|e| e.to_string())
    }

    pub fn delete_memo_metadata(&self, id: &str) -> Result<(), String> {
        let conn = self.connect().map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM memos WHERE id = ?", params![id])
            .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM memo_embeddings WHERE id = ?", params![id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    // --- Vector Embeddings ---

    pub fn save_embedding(&self, id: &str, embedding: &[f32]) -> Result<(), String> {
        let conn = self.connect().map_err(|e| e.to_string())?;

        // Convert Vec<f32> to Vec<u8> byte array (little-endian representation)
        let mut blob = Vec::with_capacity(embedding.len() * 4);
        for &val in embedding {
            blob.extend_from_slice(&val.to_le_bytes());
        }

        conn.execute(
            "INSERT OR REPLACE INTO memo_embeddings (id, embedding) VALUES (?, ?)",
            params![id, blob],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_embedding(&self, id: &str) -> Result<Option<Vec<f32>>, String> {
        let conn = self.connect().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT embedding FROM memo_embeddings WHERE id = ?")
            .map_err(|e| e.to_string())?;

        let blob: Option<Vec<u8>> = stmt
            .query_row(params![id], |row| row.get(0))
            .optional()
            .map_err(|e| e.to_string())?;

        match blob {
            Some(bytes) => {
                if bytes.len() % 4 != 0 {
                    return Err("Malformed embedding blob size".to_string());
                }
                let mut vec = Vec::with_capacity(bytes.len() / 4);
                for chunk in bytes.chunks_exact(4) {
                    let mut arr = [0u8; 4];
                    arr.copy_from_slice(chunk);
                    vec.push(f32::from_le_bytes(arr));
                }
                Ok(Some(vec))
            }
            None => Ok(None),
        }
    }

    pub fn get_all_embeddings(&self) -> Result<Vec<(String, Vec<f32>)>, String> {
        let conn = self.connect().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT id, embedding FROM memo_embeddings")
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                let id: String = row.get(0)?;
                let bytes: Vec<u8> = row.get(1)?;
                Ok((id, bytes))
            })
            .map_err(|e| e.to_string())?;

        let mut list = Vec::new();
        for r in rows {
            let (id, bytes) = r.map_err(|e| e.to_string())?;
            if bytes.len() % 4 != 0 {
                return Err("Malformed embedding blob".to_string());
            }
            let mut vec = Vec::with_capacity(bytes.len() / 4);
            for chunk in bytes.chunks_exact(4) {
                let mut arr = [0u8; 4];
                arr.copy_from_slice(chunk);
                vec.push(f32::from_le_bytes(arr));
            }
            list.push((id, vec));
        }
        Ok(list)
    }

    pub fn get_db_path(&self) -> &Path {
        &self.db_path
    }
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
