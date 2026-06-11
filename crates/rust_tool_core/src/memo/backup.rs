use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use zip::{write::FileOptions, ZipWriter};

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

pub fn create_backup_zip(data_dir: &Path, output_zip_path: &Path) -> Result<(), String> {
    let file = File::create(output_zip_path)
        .map_err(|e| format!("Failed to create backup zip file: {:?}", e))?;
    let mut zip = ZipWriter::new(file);
    let options: FileOptions<'_, ()> =
        FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    add_file_if_exists(
        &mut zip,
        options,
        &data_dir.join("config.json"),
        "config.json",
    )?;
    add_file_if_exists(
        &mut zip,
        options,
        &data_dir.join("secrets.kdbx"),
        "secrets.kdbx",
    )?;
    add_directory_tree(
        &mut zip,
        options,
        &data_dir.join("documents"),
        "documents",
        true,
    )?;
    add_directory_tree(
        &mut zip,
        options,
        &data_dir.join("embeddings"),
        "embeddings",
        false,
    )?;

    zip.finish()
        .map_err(|e| format!("Failed to write zip metadata: {:?}", e))?;
    Ok(())
}

fn add_file_if_exists(
    zip: &mut ZipWriter<File>,
    options: FileOptions<'_, ()>,
    path: &Path,
    name: &str,
) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }

    zip.start_file(name, options)
        .map_err(|e| format!("Zip error for {name}: {:?}", e))?;
    let mut f = File::open(path).map_err(|e| e.to_string())?;
    let mut buffer = Vec::new();
    f.read_to_end(&mut buffer).map_err(|e| e.to_string())?;
    zip.write_all(&buffer).map_err(|e| e.to_string())
}

fn add_directory_tree(
    zip: &mut ZipWriter<File>,
    options: FileOptions<'_, ()>,
    root: &Path,
    archive_root: &str,
    include_empty_root: bool,
) -> Result<(), String> {
    if !root.exists() {
        if include_empty_root {
            zip.add_directory(format!("{archive_root}/"), options)
                .map_err(|e| format!("Zip error adding directory {archive_root}: {:?}", e))?;
        }
        return Ok(());
    }

    zip.add_directory(format!("{archive_root}/"), options)
        .map_err(|e| format!("Zip error adding directory {archive_root}: {:?}", e))?;

    for path in walk_dir(root)? {
        let rel_path = path.strip_prefix(root).map_err(|e| e.to_string())?;
        let name = format!(
            "{archive_root}/{}",
            rel_path.to_string_lossy().replace('\\', "/")
        );

        if path.is_file() {
            add_file_if_exists(zip, options, &path, &name)?;
        } else if path.is_dir() {
            zip.add_directory(&name, options)
                .map_err(|e| format!("Zip error adding directory {}: {:?}", name, e))?;
        }
    }

    Ok(())
}

pub fn restore_from_zip(zip_path: &Path, data_dir: &Path) -> Result<(), String> {
    let file = File::open(zip_path).map_err(|e| format!("Failed to open backup zip: {:?}", e))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| format!("Failed to parse zip archive: {:?}", e))?;

    fs::create_dir_all(data_dir)
        .map_err(|e| format!("Failed to create restore data directory: {:?}", e))?;

    let temp_dir = unique_backup_path(data_dir, "restore_stage");
    fs::create_dir_all(&temp_dir)
        .map_err(|e| format!("Failed to create restore temp directory: {:?}", e))?;
    let temp_config_path = temp_dir.join("config.json");
    let temp_documents_path = temp_dir.join("documents");
    let temp_secret_vault_path = temp_dir.join("secrets.kdbx");
    let temp_embeddings_path = temp_dir.join("embeddings");

    let mut found_config = false;
    let mut found_documents = false;
    let mut found_secret_vault = false;
    let mut found_embeddings = false;

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| format!("Failed to extract file at index {}: {:?}", i, e))?;

        let outpath = match file.enclosed_name() {
            Some(path) => path.to_owned(),
            None => continue,
        };

        let is_directory = file.name().ends_with('/');

        if outpath == Path::new("config.json") {
            write_zip_file(&mut file, &temp_config_path)?;
            found_config = true;
        } else if outpath == Path::new("secrets.kdbx") {
            write_zip_file(&mut file, &temp_secret_vault_path)?;
            found_secret_vault = true;
        } else if outpath.starts_with(Path::new("documents")) {
            found_documents = true;
            restore_zip_tree_entry(
                &mut file,
                &outpath,
                is_directory,
                "documents",
                &temp_documents_path,
            )?;
        } else if outpath.starts_with(Path::new("embeddings")) {
            found_embeddings = true;
            restore_zip_tree_entry(
                &mut file,
                &outpath,
                is_directory,
                "embeddings",
                &temp_embeddings_path,
            )?;
        }
    }

    if !found_config && !found_documents && !found_secret_vault && !found_embeddings {
        let _ = fs::remove_dir_all(&temp_dir);
        return Err("Backup archive does not contain RustTool memo data".to_string());
    }

    let config_path = data_dir.join("config.json");
    let documents_path = data_dir.join("documents");
    let secret_vault_path = data_dir.join("secrets.kdbx");
    let embeddings_path = data_dir.join("embeddings");

    let backup_config_path = unique_backup_path(data_dir, "config.json");
    let backup_documents_path = unique_backup_path(data_dir, "documents");
    let backup_secret_vault_path = unique_backup_path(data_dir, "secrets.kdbx");
    let backup_embeddings_path = unique_backup_path(data_dir, "embeddings");

    let had_config = found_config && config_path.exists();
    let had_documents = found_documents && documents_path.exists();
    let had_secret_vault = found_secret_vault && secret_vault_path.exists();
    let had_embeddings = found_embeddings && embeddings_path.exists();

    if had_config {
        fs::copy(&config_path, &backup_config_path)
            .map_err(|e| format!("Failed to preserve current config before restore: {:?}", e))?;
    }
    if had_documents {
        fs::rename(&documents_path, &backup_documents_path).map_err(|e| {
            format!(
                "Failed to preserve current documents before restore: {:?}",
                e
            )
        })?;
    }
    if had_secret_vault {
        fs::copy(&secret_vault_path, &backup_secret_vault_path).map_err(|e| {
            format!(
                "Failed to preserve current KDBX vault before restore: {:?}",
                e
            )
        })?;
    }
    if had_embeddings {
        fs::rename(&embeddings_path, &backup_embeddings_path).map_err(|e| {
            format!(
                "Failed to preserve current embeddings before restore: {:?}",
                e
            )
        })?;
    }

    let result = replace_staged_data(
        found_config,
        found_documents,
        found_secret_vault,
        found_embeddings,
        &temp_config_path,
        &temp_documents_path,
        &temp_secret_vault_path,
        &temp_embeddings_path,
        &config_path,
        &documents_path,
        &secret_vault_path,
        &embeddings_path,
    );

    if let Err(error) = result {
        restore_preserved_data(
            found_config,
            found_documents,
            found_secret_vault,
            found_embeddings,
            &config_path,
            &documents_path,
            &secret_vault_path,
            &embeddings_path,
            &backup_config_path,
            &backup_documents_path,
            &backup_secret_vault_path,
            &backup_embeddings_path,
            had_config,
            had_documents,
            had_secret_vault,
            had_embeddings,
        );
        let _ = fs::remove_dir_all(&temp_dir);
        return Err(error);
    }

    cleanup_preserved_data(
        &backup_config_path,
        &backup_documents_path,
        &backup_secret_vault_path,
        &backup_embeddings_path,
        had_config,
        had_documents,
        had_secret_vault,
        had_embeddings,
    );
    let _ = fs::remove_dir_all(&temp_dir);

    Ok(())
}

fn write_zip_file<R: Read>(file: &mut R, path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create restore parent directory: {:?}", e))?;
    }
    let mut outfile = File::create(path)
        .map_err(|e| format!("Failed to create restore path {}: {:?}", path.display(), e))?;
    std::io::copy(file, &mut outfile)
        .map(|_| ())
        .map_err(|e| format!("Failed to copy restored file: {:?}", e))
}

fn restore_zip_tree_entry<R: Read>(
    file: &mut R,
    outpath: &Path,
    is_directory: bool,
    archive_root: &str,
    temp_root: &Path,
) -> Result<(), String> {
    let rel_path = outpath
        .strip_prefix(archive_root)
        .map_err(|e| format!("Failed to resolve path in backup: {:?}", e))?;
    if rel_path.as_os_str().is_empty() {
        fs::create_dir_all(temp_root)
            .map_err(|e| format!("Failed to create restore directory: {:?}", e))?;
        return Ok(());
    }

    let target_path = temp_root.join(rel_path);
    if !target_path.starts_with(temp_root) {
        return Err("Backup contains an invalid path".to_string());
    }

    if is_directory {
        fs::create_dir_all(&target_path)
            .map_err(|e| format!("Failed to create restored directory: {:?}", e))
    } else {
        write_zip_file(file, &target_path)
    }
}

fn replace_staged_data(
    found_config: bool,
    found_documents: bool,
    found_secret_vault: bool,
    found_embeddings: bool,
    temp_config_path: &Path,
    temp_documents_path: &Path,
    temp_secret_vault_path: &Path,
    temp_embeddings_path: &Path,
    config_path: &Path,
    documents_path: &Path,
    secret_vault_path: &Path,
    embeddings_path: &Path,
) -> Result<(), String> {
    if found_config {
        fs::copy(temp_config_path, config_path)
            .map(|_| ())
            .map_err(|e| format!("Failed to replace config during restore: {:?}", e))?;
    }
    if found_documents {
        if documents_path.exists() {
            fs::remove_dir_all(documents_path).map_err(|e| {
                format!("Failed to clear current documents during restore: {:?}", e)
            })?;
        }
        fs::rename(temp_documents_path, documents_path)
            .map_err(|e| format!("Failed to replace documents during restore: {:?}", e))?;
    }
    if found_secret_vault {
        fs::copy(temp_secret_vault_path, secret_vault_path)
            .map(|_| ())
            .map_err(|e| format!("Failed to replace KDBX vault during restore: {:?}", e))?;
    }
    if found_embeddings {
        if embeddings_path.exists() {
            fs::remove_dir_all(embeddings_path).map_err(|e| {
                format!("Failed to clear current embeddings during restore: {:?}", e)
            })?;
        }
        fs::rename(temp_embeddings_path, embeddings_path)
            .map_err(|e| format!("Failed to replace embeddings during restore: {:?}", e))?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn restore_preserved_data(
    found_config: bool,
    found_documents: bool,
    found_secret_vault: bool,
    found_embeddings: bool,
    config_path: &Path,
    documents_path: &Path,
    secret_vault_path: &Path,
    embeddings_path: &Path,
    backup_config_path: &Path,
    backup_documents_path: &Path,
    backup_secret_vault_path: &Path,
    backup_embeddings_path: &Path,
    had_config: bool,
    had_documents: bool,
    had_secret_vault: bool,
    had_embeddings: bool,
) {
    if found_config && config_path.exists() {
        let _ = fs::remove_file(config_path);
    }
    if had_config {
        let _ = fs::copy(backup_config_path, config_path);
    }

    if found_documents && documents_path.exists() {
        let _ = fs::remove_dir_all(documents_path);
    }
    if had_documents {
        let _ = fs::rename(backup_documents_path, documents_path);
    }

    if found_secret_vault && secret_vault_path.exists() {
        let _ = fs::remove_file(secret_vault_path);
    }
    if had_secret_vault {
        let _ = fs::copy(backup_secret_vault_path, secret_vault_path);
    }

    if found_embeddings && embeddings_path.exists() {
        let _ = fs::remove_dir_all(embeddings_path);
    }
    if had_embeddings {
        let _ = fs::rename(backup_embeddings_path, embeddings_path);
    }
}

fn cleanup_preserved_data(
    backup_config_path: &Path,
    backup_documents_path: &Path,
    backup_secret_vault_path: &Path,
    backup_embeddings_path: &Path,
    had_config: bool,
    had_documents: bool,
    had_secret_vault: bool,
    had_embeddings: bool,
) {
    if had_config {
        let _ = fs::remove_file(backup_config_path);
    }
    if had_documents {
        let _ = fs::remove_dir_all(backup_documents_path);
    }
    if had_secret_vault {
        let _ = fs::remove_file(backup_secret_vault_path);
    }
    if had_embeddings {
        let _ = fs::remove_dir_all(backup_embeddings_path);
    }
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
    fn backup_includes_config_kdbx_documents_and_embeddings() {
        let temp_dir = make_test_dir("backup_files");
        let zip_path = temp_dir.join("backup.zip");

        fs::write(temp_dir.join("config.json"), r#"{"memoConfig":{}}"#).expect("config");
        fs::write(temp_dir.join("secrets.kdbx"), "kdbx").expect("kdbx");
        fs::create_dir_all(temp_dir.join("documents")).expect("documents dir");
        fs::write(temp_dir.join("documents/note.md"), "# Note").expect("doc");
        fs::create_dir_all(temp_dir.join("embeddings")).expect("embeddings dir");
        fs::write(temp_dir.join("embeddings/doc.json"), "{}").expect("embedding");

        create_backup_zip(&temp_dir, &zip_path).expect("create backup");

        let file = File::open(&zip_path).expect("open zip");
        let mut archive = zip::ZipArchive::new(file).expect("zip archive");
        assert!(archive.by_name("config.json").is_ok());
        assert!(archive.by_name("secrets.kdbx").is_ok());
        assert!(archive.by_name("documents/note.md").is_ok());
        assert!(archive.by_name("embeddings/doc.json").is_ok());

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn restore_rejects_invalid_backup_without_touching_existing_data() {
        let temp_dir = make_test_dir("bad_restore");
        let bad_zip_path = temp_dir.join("bad.zip");

        fs::write(temp_dir.join("config.json"), "existing config").expect("existing config");
        fs::create_dir_all(temp_dir.join("documents")).expect("documents dir");
        fs::write(temp_dir.join("documents/keep.md"), "keep").expect("existing doc");
        fs::write(temp_dir.join("secrets.kdbx"), "existing kdbx").expect("existing kdbx");

        {
            let file = File::create(&bad_zip_path).expect("bad zip");
            let mut zip = ZipWriter::new(file);
            let options: FileOptions<'_, ()> =
                FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
            zip.start_file("other/new.md", options).expect("zip file");
            zip.write_all(b"new").expect("zip content");
            zip.finish().expect("finish zip");
        }

        let err =
            restore_from_zip(&bad_zip_path, &temp_dir).expect_err("missing memo data should fail");

        assert!(err.contains("RustTool memo data"));
        assert_eq!(
            fs::read_to_string(temp_dir.join("documents/keep.md")).expect("existing doc intact"),
            "keep"
        );
        assert_eq!(
            fs::read_to_string(temp_dir.join("config.json")).expect("existing config intact"),
            "existing config"
        );
        assert_eq!(
            fs::read_to_string(temp_dir.join("secrets.kdbx")).expect("existing kdbx intact"),
            "existing kdbx"
        );

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn restore_replaces_file_based_memo_data() {
        let temp_dir = make_test_dir("restore");
        let source_dir = make_test_dir("restore_source");
        let zip_path = source_dir.join("backup.zip");

        fs::write(
            source_dir.join("config.json"),
            r#"{"memoConfig":{"x":"y"}}"#,
        )
        .expect("config");
        fs::write(source_dir.join("secrets.kdbx"), "new kdbx").expect("kdbx");
        fs::create_dir_all(source_dir.join("documents")).expect("source docs");
        fs::write(source_dir.join("documents/new.md"), "new").expect("source doc");
        create_backup_zip(&source_dir, &zip_path).expect("backup");

        fs::write(temp_dir.join("config.json"), "old config").expect("old config");
        fs::create_dir_all(temp_dir.join("documents")).expect("target docs");
        fs::write(temp_dir.join("documents/old.md"), "old").expect("old doc");
        fs::write(temp_dir.join("secrets.kdbx"), "old kdbx").expect("old kdbx");

        restore_from_zip(&zip_path, &temp_dir).expect("restore");

        assert!(temp_dir.join("documents/new.md").exists());
        assert!(!temp_dir.join("documents/old.md").exists());
        assert_eq!(
            fs::read_to_string(temp_dir.join("secrets.kdbx")).expect("restored kdbx"),
            "new kdbx"
        );
        assert!(fs::read_to_string(temp_dir.join("config.json"))
            .expect("restored config")
            .contains("\"memoConfig\""));

        let _ = fs::remove_dir_all(temp_dir);
        let _ = fs::remove_dir_all(source_dir);
    }
}
