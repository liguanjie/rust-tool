use keepass::{
    db::{fields, EntryId},
    Database, DatabaseKey,
};
use std::fs::File;
use std::path::{Path, PathBuf};

const RUSTTOOL_SECRETS_GROUP: &str = "RustTool Secrets";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SecretMetadata {
    pub label: Option<String>,
    pub document_path: Option<String>,
}

pub trait SecretVault {
    fn create(path: &Path, password: &str) -> Result<Self, String>
    where
        Self: Sized;

    fn open(path: &Path, password: &str) -> Result<Self, String>
    where
        Self: Sized;

    fn put_secret(
        &mut self,
        key: &str,
        value: &str,
        metadata: SecretMetadata,
    ) -> Result<(), String>;

    fn get_secret(&self, key: &str) -> Result<Option<String>, String>;
    fn delete_secret(&mut self, key: &str) -> Result<bool, String>;
    fn list_secret_keys(&self) -> Result<Vec<String>, String>;
    fn save(&mut self) -> Result<(), String>;
}

pub struct KdbxSecretVault {
    path: PathBuf,
    password: String,
    db: Database,
}

impl SecretVault for KdbxSecretVault {
    fn create(path: &Path, password: &str) -> Result<Self, String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("Failed to create KDBX parent directory: {error:?}"))?;
        }

        let mut vault = Self {
            path: path.to_path_buf(),
            password: password.to_string(),
            db: Database::new(),
        };
        vault.ensure_rusttool_group();
        vault.save()?;
        Ok(vault)
    }

    fn open(path: &Path, password: &str) -> Result<Self, String> {
        let mut file =
            File::open(path).map_err(|error| format!("Failed to open KDBX file: {error:?}"))?;
        let mut db = Database::open(&mut file, DatabaseKey::new().with_password(password))
            .map_err(|error| format!("Failed to unlock KDBX file: {error:?}"))?;

        {
            let mut vault = Self {
                path: path.to_path_buf(),
                password: password.to_string(),
                db,
            };
            vault.ensure_rusttool_group();
            db = vault.db;
        }

        Ok(Self {
            path: path.to_path_buf(),
            password: password.to_string(),
            db,
        })
    }

    fn put_secret(
        &mut self,
        key: &str,
        value: &str,
        metadata: SecretMetadata,
    ) -> Result<(), String> {
        let key = normalize_secret_key(key)?;
        let existing_id = self.entry_id_by_title(&key);

        if let Some(entry_id) = existing_id {
            let mut entry = self
                .db
                .entry_mut(entry_id)
                .ok_or_else(|| format!("KDBX entry disappeared for key: {key}"))?;
            write_secret_entry(&mut entry, &key, value, &metadata);
            return Ok(());
        }

        let mut group = self.rusttool_group_mut()?;
        let mut entry = group.add_entry();
        write_secret_entry(&mut entry, &key, value, &metadata);

        Ok(())
    }

    fn get_secret(&self, key: &str) -> Result<Option<String>, String> {
        let key = normalize_secret_key(key)?;
        Ok(self
            .db
            .iter_all_entries()
            .find(|entry| entry.get_title() == Some(key.as_str()))
            .and_then(|entry| entry.get_password().map(str::to_string)))
    }

    fn delete_secret(&mut self, key: &str) -> Result<bool, String> {
        let key = normalize_secret_key(key)?;
        let Some(entry_id) = self.entry_id_by_title(&key) else {
            return Ok(false);
        };

        self.db
            .entry_mut(entry_id)
            .ok_or_else(|| format!("KDBX entry disappeared for key: {key}"))?
            .remove();

        Ok(true)
    }

    fn list_secret_keys(&self) -> Result<Vec<String>, String> {
        let mut keys = self
            .db
            .iter_all_entries()
            .filter_map(|entry| entry.get_title().map(str::to_string))
            .collect::<Vec<_>>();
        keys.sort();
        Ok(keys)
    }

    fn save(&mut self) -> Result<(), String> {
        let mut file = File::create(&self.path)
            .map_err(|error| format!("Failed to create KDBX file: {error:?}"))?;
        self.db
            .save(&mut file, DatabaseKey::new().with_password(&self.password))
            .map_err(|error| format!("Failed to save KDBX file: {error:?}"))
    }
}

impl KdbxSecretVault {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn change_password(&mut self, new_password: &str) -> Result<(), String> {
        self.password = new_password.to_string();
        self.save()
    }

    fn ensure_rusttool_group(&mut self) {
        if self
            .db
            .root()
            .group_by_name(RUSTTOOL_SECRETS_GROUP)
            .is_some()
        {
            return;
        }

        let mut root = self.db.root_mut();
        root.add_group().edit(|group| {
            group.name = RUSTTOOL_SECRETS_GROUP.to_string();
            group.notes = Some("Secrets managed by RustTool.".to_string());
        });
    }

    fn rusttool_group_mut(&mut self) -> Result<keepass::db::GroupMut<'_>, String> {
        self.ensure_rusttool_group();
        let group_id = self
            .db
            .root()
            .group_by_name(RUSTTOOL_SECRETS_GROUP)
            .map(|group| group.id())
            .ok_or_else(|| "Failed to resolve RustTool KDBX group".to_string())?;

        self.db
            .group_mut(group_id)
            .ok_or_else(|| "Failed to access RustTool KDBX group".to_string())
    }

    fn entry_id_by_title(&self, key: &str) -> Option<EntryId> {
        self.db
            .iter_all_entries()
            .find(|entry| entry.get_title() == Some(key))
            .map(|entry| entry.id())
    }
}

fn write_secret_entry(
    entry: &mut keepass::db::EntryMut<'_>,
    key: &str,
    value: &str,
    metadata: &SecretMetadata,
) {
    entry.set_unprotected(fields::TITLE, key);
    entry.set_unprotected(fields::USERNAME, "");
    entry.set_protected(fields::PASSWORD, value);
    entry.set_unprotected(fields::NOTES, render_secret_notes(metadata));
    if !entry.tags.iter().any(|tag| tag == "rusttool") {
        entry.tags.push("rusttool".to_string());
    }
}

fn render_secret_notes(metadata: &SecretMetadata) -> String {
    let mut lines = vec!["managedBy=rusttool".to_string()];
    if let Some(label) = metadata.label.as_deref().filter(|value| !value.is_empty()) {
        lines.push(format!("label={label}"));
    }
    if let Some(path) = metadata
        .document_path
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        lines.push(format!("documentPath={path}"));
    }
    lines.join("\n")
}

fn normalize_secret_key(key: &str) -> Result<String, String> {
    let trimmed = key.trim();
    if trimmed.is_empty() {
        return Err("Secret key cannot be empty".to_string());
    }
    if trimmed.contains('\n') || trimmed.contains('\r') {
        return Err("Secret key cannot contain line breaks".to_string());
    }
    Ok(trimmed.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn make_test_dir(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("rusttool_kdbx_{}_{}", name, stamp));
        std::fs::create_dir_all(&dir).expect("test temp dir");
        dir
    }

    #[test]
    fn creates_writes_saves_and_reopens_kdbx_vault() {
        let temp_dir = make_test_dir("roundtrip");
        let path = temp_dir.join("secrets.kdbx");
        let password = "test-master-password";

        {
            let mut vault = KdbxSecretVault::create(&path, password).expect("create vault");
            vault
                .put_secret(
                    "customerAProdSshPassword",
                    "abc123",
                    SecretMetadata {
                        label: Some("客户 A SSH 密码".to_string()),
                        document_path: Some("documents/customer-a.md".to_string()),
                    },
                )
                .expect("put secret");
            vault.save().expect("save vault");
        }

        let vault = KdbxSecretVault::open(&path, password).expect("open vault");
        assert_eq!(
            vault
                .get_secret("customerAProdSshPassword")
                .expect("get secret"),
            Some("abc123".to_string())
        );
        assert_eq!(
            vault.list_secret_keys().expect("list keys"),
            vec!["customerAProdSshPassword".to_string()]
        );

        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn updates_and_deletes_existing_kdbx_secret() {
        let temp_dir = make_test_dir("update_delete");
        let path = temp_dir.join("secrets.kdbx");
        let password = "test-master-password";
        let mut vault = KdbxSecretVault::create(&path, password).expect("create vault");

        vault
            .put_secret("apiKey", "first", SecretMetadata::default())
            .expect("put first");
        vault
            .put_secret("apiKey", "second", SecretMetadata::default())
            .expect("update");

        assert_eq!(
            vault.get_secret("apiKey").expect("get updated"),
            Some("second".to_string())
        );
        assert!(vault.delete_secret("apiKey").expect("delete"));
        assert_eq!(vault.get_secret("apiKey").expect("get missing"), None);

        let _ = std::fs::remove_dir_all(temp_dir);
    }
}
