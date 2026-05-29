use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use zip::{write::FileOptions, ZipWriter};

/// Recursively walk a directory and gather all file and directory paths.
fn walk_dir(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    if dir.is_dir() {
        for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if path.is_dir() {
                files.push(path.clone());
                files.extend(walk_dir(&path)?);
            } else {
                files.push(path);
            }
        }
    }
    Ok(files)
}

/// Create a backup ZIP containing the database and the documents folder.
pub fn create_backup_zip(
    vault_path: &Path,
    db_path: &Path,
    output_zip_path: &Path,
) -> Result<(), String> {
    let file = File::create(output_zip_path)
        .map_err(|e| format!("Failed to create backup zip file: {:?}", e))?;
    let mut zip = ZipWriter::new(file);
    let options: FileOptions<'_, ()> =
        FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    // 1. Backup SQLite database
    if db_path.exists() {
        zip.start_file("memos.db", options)
            .map_err(|e| format!("Zip error for database: {:?}", e))?;
        let mut f = File::open(db_path).map_err(|e| e.to_string())?;
        let mut buffer = Vec::new();
        f.read_to_end(&mut buffer).map_err(|e| e.to_string())?;
        zip.write_all(&buffer).map_err(|e| e.to_string())?;
    }

    // 2. Backup Documents folder recursively
    if vault_path.exists() {
        let paths = walk_dir(vault_path)?;
        for path in paths {
            let rel_path = path.strip_prefix(vault_path).map_err(|e| e.to_string())?;

            // Standardize path separators to forward slashes for zip compatibility
            let name = format!(
                "documents/{}",
                rel_path.to_string_lossy().replace('\\', "/")
            );

            if path.is_file() {
                zip.start_file(&name, options)
                    .map_err(|e| format!("Zip error for file {}: {:?}", name, e))?;
                let mut f = File::open(&path).map_err(|e| e.to_string())?;
                let mut buffer = Vec::new();
                f.read_to_end(&mut buffer).map_err(|e| e.to_string())?;
                zip.write_all(&buffer).map_err(|e| e.to_string())?;
            } else if path.is_dir() {
                zip.add_directory(&name, options)
                    .map_err(|e| format!("Zip error adding directory {}: {:?}", name, e))?;
            }
        }
    }

    zip.finish()
        .map_err(|e| format!("Failed to write zip metadata: {:?}", e))?;
    Ok(())
}

/// Restore database and documents from a backup ZIP file.
pub fn restore_from_zip(zip_path: &Path, vault_path: &Path, db_path: &Path) -> Result<(), String> {
    let file = File::open(zip_path).map_err(|e| format!("Failed to open backup zip: {:?}", e))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| format!("Failed to parse zip archive: {:?}", e))?;

    let data_dir = db_path
        .parent()
        .or_else(|| vault_path.parent())
        .ok_or_else(|| "Cannot resolve memo data directory for restore".to_string())?;
    fs::create_dir_all(data_dir)
        .map_err(|e| format!("Failed to create restore data directory: {:?}", e))?;

    let temp_dir = unique_backup_path(data_dir, "restore_stage");
    fs::create_dir_all(&temp_dir)
        .map_err(|e| format!("Failed to create restore temp directory: {:?}", e))?;
    let temp_vault_path = temp_dir.join("documents");
    let temp_db_path = temp_dir.join("memos.db");
    fs::create_dir_all(&temp_vault_path)
        .map_err(|e| format!("Failed to create temp documents folder: {:?}", e))?;

    let mut found_db = false;

    // 1. Extract into a staging directory first. Existing data is untouched until
    // the whole archive has been read and the required database is present.
    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| format!("Failed to extract file at index {}: {:?}", i, e))?;

        let outpath = match file.enclosed_name() {
            Some(path) => path.to_owned(),
            None => continue,
        };

        if outpath == Path::new("memos.db") {
            let mut outfile = File::create(&temp_db_path)
                .map_err(|e| format!("Failed to create temp DB path for restore: {:?}", e))?;
            std::io::copy(&mut file, &mut outfile)
                .map_err(|e| format!("Failed to restore database: {:?}", e))?;
            found_db = true;
        } else if outpath.starts_with(Path::new("documents")) {
            let rel_path = outpath
                .strip_prefix("documents")
                .map_err(|e| format!("Failed to resolve document path in backup: {:?}", e))?;
            if rel_path.as_os_str().is_empty() {
                continue;
            }

            let target_path = temp_vault_path.join(rel_path);
            if !target_path.starts_with(&temp_vault_path) {
                return Err("Backup contains an invalid document path".to_string());
            }

            if file.name().ends_with('/') {
                fs::create_dir_all(&target_path).map_err(|e| {
                    format!(
                        "Failed to create directory {}: {:?}",
                        target_path.display(),
                        e
                    )
                })?;
            } else {
                if let Some(parent) = target_path.parent() {
                    if !parent.exists() {
                        fs::create_dir_all(parent)
                            .map_err(|e| format!("Failed to create parent folder: {:?}", e))?;
                    }
                }
                let mut outfile = File::create(&target_path).map_err(|e| {
                    format!(
                        "Failed to create restored file {}: {:?}",
                        target_path.display(),
                        e
                    )
                })?;
                std::io::copy(&mut file, &mut outfile)
                    .map_err(|e| format!("Failed to copy file contents: {:?}", e))?;
            }
        }
    }

    if !found_db {
        let _ = fs::remove_dir_all(&temp_dir);
        return Err("Backup archive is missing memos.db".to_string());
    }

    // 2. Replace the live data only after staging succeeded. Keep local backups
    // until both database and document folder replacements have finished.
    let backup_db_path = unique_backup_path(data_dir, "memos.db");
    let backup_vault_path = unique_backup_path(data_dir, "documents");
    let had_db = db_path.exists();
    let had_vault = vault_path.exists();

    if had_db {
        fs::copy(db_path, &backup_db_path).map_err(|e| {
            format!(
                "Failed to preserve current database before restore: {:?}",
                e
            )
        })?;
    }

    if had_vault {
        fs::rename(vault_path, &backup_vault_path).map_err(|e| {
            format!(
                "Failed to preserve current documents before restore: {:?}",
                e
            )
        })?;
    }

    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create database parent directory: {:?}", e))?;
    }

    if let Err(err) = fs::copy(&temp_db_path, db_path) {
        restore_preserved_data(
            db_path,
            vault_path,
            &backup_db_path,
            &backup_vault_path,
            had_db,
            had_vault,
        );
        let _ = fs::remove_dir_all(&temp_dir);
        return Err(format!(
            "Failed to replace database during restore: {:?}",
            err
        ));
    }

    if let Err(err) = fs::rename(&temp_vault_path, vault_path) {
        restore_preserved_data(
            db_path,
            vault_path,
            &backup_db_path,
            &backup_vault_path,
            had_db,
            had_vault,
        );
        let _ = fs::remove_dir_all(&temp_dir);
        return Err(format!(
            "Failed to replace documents during restore: {:?}",
            err
        ));
    }

    cleanup_preserved_data(&backup_db_path, &backup_vault_path, had_db, had_vault);
    let _ = fs::remove_dir_all(&temp_dir);

    Ok(())
}

fn unique_backup_path(data_dir: &Path, label: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();

    for index in 0..1000 {
        let candidate = data_dir.join(format!(".{}_restore_backup_{}_{}", label, stamp, index));
        if !candidate.exists() {
            return candidate;
        }
    }

    data_dir.join(format!(".{}_restore_backup_{}_fallback", label, stamp))
}

fn restore_preserved_data(
    db_path: &Path,
    vault_path: &Path,
    backup_db_path: &Path,
    backup_vault_path: &Path,
    had_db: bool,
    had_vault: bool,
) {
    if db_path.exists() {
        let _ = fs::remove_file(db_path);
    }
    if had_db {
        let _ = fs::copy(backup_db_path, db_path);
    }

    if vault_path.exists() {
        let _ = fs::remove_dir_all(vault_path);
    }
    if had_vault {
        let _ = fs::rename(backup_vault_path, vault_path);
    }
}

fn cleanup_preserved_data(
    backup_db_path: &Path,
    backup_vault_path: &Path,
    had_db: bool,
    had_vault: bool,
) {
    if had_db {
        let _ = fs::remove_file(backup_db_path);
    }
    if had_vault {
        let _ = fs::remove_dir_all(backup_vault_path);
    }
}

/// Helper to upload backup file to a WebDAV server.
pub async fn upload_to_webdav(
    zip_path: &Path,
    url: &str,
    username: &str,
    password: &str,
) -> Result<(), String> {
    let client = reqwest::Client::new();
    let file_data = fs::read(zip_path)
        .map_err(|e| format!("Failed to read zip file for WebDAV upload: {:?}", e))?;

    let res = client
        .put(url)
        .basic_auth(username, Some(password))
        .body(file_data)
        .send()
        .await
        .map_err(|e| format!("WebDAV HTTP PUT failed: {:?}", e))?;

    if !res.status().is_success() {
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        return Err(format!("WebDAV returned error {}: {}", status, body));
    }

    Ok(())
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
    fn restore_rejects_invalid_backup_without_touching_existing_data() {
        let temp_dir = make_test_dir("bad_restore");
        let vault_path = temp_dir.join("documents");
        let db_path = temp_dir.join("memos.db");
        let bad_zip_path = temp_dir.join("bad.zip");

        fs::create_dir_all(&vault_path).expect("vault dir");
        fs::write(vault_path.join("keep.md"), "keep").expect("existing doc");
        fs::write(&db_path, "existing db").expect("existing db");

        {
            let file = File::create(&bad_zip_path).expect("bad zip");
            let mut zip = ZipWriter::new(file);
            let options: FileOptions<'_, ()> =
                FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
            zip.start_file("documents/new.md", options)
                .expect("zip file");
            zip.write_all(b"new").expect("zip content");
            zip.finish().expect("finish zip");
        }

        let err = restore_from_zip(&bad_zip_path, &vault_path, &db_path)
            .expect_err("missing database should fail");

        assert!(err.contains("memos.db"));
        assert_eq!(
            fs::read_to_string(vault_path.join("keep.md")).expect("existing doc intact"),
            "keep"
        );
        assert_eq!(
            fs::read_to_string(&db_path).expect("existing db intact"),
            "existing db"
        );

        let _ = fs::remove_dir_all(temp_dir);
    }
}
