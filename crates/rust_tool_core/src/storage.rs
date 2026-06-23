use crate::tools::osv_scanner::{OsvCommandExecutionRecord, OsvCommandKind, OsvScanResult};
use serde::{Deserialize, Serialize};
use sqlx::migrate::Migrator;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::{Row, SqlitePool};
use std::path::{Path, PathBuf};
use thiserror::Error;
use tokio::fs;

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

#[derive(Debug, Clone)]
pub struct StorageDatabase {
    database_path: PathBuf,
    pool: SqlitePool,
    schema_version: i64,
    applied_migrations: u32,
}

impl StorageDatabase {
    pub fn database_path(&self) -> &Path {
        &self.database_path
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub fn schema_version(&self) -> i64 {
        self.schema_version
    }

    pub fn applied_migrations(&self) -> u32 {
        self.applied_migrations
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseHealth {
    pub database_path: String,
    pub status: DatabaseHealthStatus,
    pub database_exists: bool,
    pub parent_directory_exists: bool,
    pub schema_version: Option<i64>,
    pub applied_migrations: u32,
    pub message: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseFileStats {
    pub database_path: String,
    pub main_file_size_bytes: u64,
    pub wal_file_size_bytes: u64,
    pub shm_file_size_bytes: u64,
    pub total_size_bytes: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseRecordCount {
    pub key: String,
    pub label: String,
    pub count: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseStorageDiagnostics {
    pub total_records: i64,
    pub record_counts: Vec<DatabaseRecordCount>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DatabaseHealthStatus {
    Ready,
    Error,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AgentExecutionHistoryRecord {
    pub id: String,
    pub timestamp: i64,
    #[serde(alias = "script_name")]
    pub script_name: String,
    pub args: String,
    #[serde(rename = "exit_code", alias = "exitCode")]
    pub exit_code: i32,
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct AgentSkillsSettings {
    pub script_dir: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct OsvProjectRecord {
    pub name: String,
    pub path: String,
    pub last_scanned: Option<String>,
    pub health_score: Option<u32>,
}

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("数据库路径不能为空")]
    EmptyDatabasePath,
    #[error("执行历史记录无效: {0}")]
    InvalidAgentHistoryRecord(String),
    #[error("OSV 最近扫描结果无效: {0}")]
    InvalidOsvLatestScanResult(String),
    #[error("OSV 项目记录无效: {0}")]
    InvalidOsvProjectRecord(String),
    #[error("OSV 命令历史记录无效: {0}")]
    InvalidOsvCommandHistoryRecord(String),
    #[error("数据库备份路径无效: {0}")]
    InvalidBackupPath(String),
    #[error("数据库恢复路径无效: {0}")]
    InvalidRestorePath(String),
    #[error("创建数据库目录失败 {path}: {source}")]
    CreateDirectory {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("{action}失败 {path}: {source}")]
    FileOperation {
        action: &'static str,
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("打开或初始化 SQLite 数据库失败: {0}")]
    Database(#[from] sqlx::Error),
    #[error("序列化或解析 JSON 失败: {0}")]
    Json(#[from] serde_json::Error),
    #[error("执行 SQLite migration 失败: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
}

pub async fn initialize_database(
    database_path: impl AsRef<Path>,
) -> Result<StorageDatabase, StorageError> {
    let database_path = normalize_database_path(database_path.as_ref())?;
    create_database_parent_dir(&database_path).await?;

    let options = SqliteConnectOptions::new()
        .filename(&database_path)
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal);
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;

    MIGRATOR.run(&pool).await?;

    let schema_version = read_schema_version(&pool).await?;
    let applied_migrations = read_applied_migrations(&pool).await?;

    Ok(StorageDatabase {
        database_path,
        pool,
        schema_version,
        applied_migrations,
    })
}

pub async fn check_database_health(database_path: impl AsRef<Path>) -> DatabaseHealth {
    let database_path = database_path.as_ref().to_path_buf();
    let database_path_display = database_path.display().to_string();

    match initialize_database(&database_path).await {
        Ok(database) => DatabaseHealth {
            database_path: database.database_path().display().to_string(),
            status: DatabaseHealthStatus::Ready,
            database_exists: database.database_path().exists(),
            parent_directory_exists: database_parent_exists(database.database_path()),
            schema_version: Some(database.schema_version()),
            applied_migrations: database.applied_migrations(),
            message: "SQLite 数据库已就绪。".to_string(),
        },
        Err(error) => DatabaseHealth {
            database_path: database_path_display,
            status: DatabaseHealthStatus::Error,
            database_exists: database_path.exists(),
            parent_directory_exists: database_parent_exists(&database_path),
            schema_version: None,
            applied_migrations: 0,
            message: error.to_string(),
        },
    }
}

pub async fn database_file_stats(
    database_path: impl AsRef<Path>,
) -> Result<DatabaseFileStats, StorageError> {
    let database_path = normalize_database_path(database_path.as_ref())?;
    let main_file_size_bytes = file_size_or_zero(&database_path).await?;
    let wal_file_size_bytes =
        file_size_or_zero(&sidecar_database_path(&database_path, "-wal")).await?;
    let shm_file_size_bytes =
        file_size_or_zero(&sidecar_database_path(&database_path, "-shm")).await?;
    let total_size_bytes = main_file_size_bytes
        .saturating_add(wal_file_size_bytes)
        .saturating_add(shm_file_size_bytes);

    Ok(DatabaseFileStats {
        database_path: database_path.display().to_string(),
        main_file_size_bytes,
        wal_file_size_bytes,
        shm_file_size_bytes,
        total_size_bytes,
    })
}

pub async fn backup_database(
    database: &StorageDatabase,
    backup_path: impl AsRef<Path>,
) -> Result<PathBuf, StorageError> {
    let backup_path = normalize_backup_path(backup_path.as_ref())?;
    create_database_parent_dir(&backup_path).await?;

    if backup_path.exists() {
        return Err(StorageError::InvalidBackupPath(format!(
            "备份文件已存在: {}",
            backup_path.display()
        )));
    }

    sqlx::query("VACUUM INTO ?")
        .bind(backup_path.display().to_string())
        .execute(database.pool())
        .await?;

    Ok(backup_path)
}

pub async fn vacuum_database(database: &StorageDatabase) -> Result<(), StorageError> {
    sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
        .execute(database.pool())
        .await?;
    sqlx::query("VACUUM").execute(database.pool()).await?;
    sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
        .execute(database.pool())
        .await?;

    Ok(())
}

pub async fn restore_database_file(
    database_path: impl AsRef<Path>,
    backup_path: impl AsRef<Path>,
) -> Result<(), StorageError> {
    let database_path = normalize_database_path(database_path.as_ref())?;
    let backup_path = normalize_restore_backup_path(backup_path.as_ref())?;
    if backup_path == database_path {
        return Err(StorageError::InvalidRestorePath(
            "恢复来源不能是当前数据库文件".to_string(),
        ));
    }
    create_database_parent_dir(&database_path).await?;
    remove_database_sidecar_files(&database_path).await?;
    fs::copy(&backup_path, &database_path)
        .await
        .map_err(|source| StorageError::FileOperation {
            action: "恢复数据库文件",
            path: database_path.clone(),
            source,
        })?;

    Ok(())
}

pub async fn database_storage_diagnostics(
    database: &StorageDatabase,
) -> Result<DatabaseStorageDiagnostics, StorageError> {
    let record_counts = vec![
        database_record_count(
            database,
            "agentExecutionHistory",
            "Agent 执行历史",
            "SELECT COUNT(*) AS count FROM agent_execution_history",
        )
        .await?,
        database_record_count(
            database,
            "osvProjects",
            "OSV 项目清单",
            "SELECT COUNT(*) AS count FROM osv_projects",
        )
        .await?,
        database_record_count(
            database,
            "osvCommandHistory",
            "OSV 命令历史",
            "SELECT COUNT(*) AS count FROM osv_command_history",
        )
        .await?,
        database_record_count(
            database,
            "osvLatestScanResults",
            "OSV 最新完整扫描结果",
            "SELECT COUNT(*) AS count FROM osv_latest_scan_results",
        )
        .await?,
        database_record_count(
            database,
            "toolSettings",
            "工具配置",
            "SELECT COUNT(*) AS count FROM tool_settings",
        )
        .await?,
        database_record_count(
            database,
            "exportRecords",
            "导出记录",
            "SELECT COUNT(*) AS count FROM export_records",
        )
        .await?,
        database_record_count(
            database,
            "appEvents",
            "应用事件",
            "SELECT COUNT(*) AS count FROM app_events",
        )
        .await?,
    ];
    let total_records = record_counts
        .iter()
        .map(|record_count| record_count.count)
        .sum();

    Ok(DatabaseStorageDiagnostics {
        total_records,
        record_counts,
    })
}

pub async fn save_agent_execution_history_record(
    database: &StorageDatabase,
    record: AgentExecutionHistoryRecord,
    limit: usize,
) -> Result<(), StorageError> {
    let record = normalize_agent_execution_history_record(record)?;
    let history_limit = history_limit_i64(limit);
    let success = if record.success { 1_i64 } else { 0_i64 };
    let mut transaction = database.pool().begin().await?;

    sqlx::query(
        r#"
        DELETE FROM agent_execution_history
        WHERE script_name = ? AND args = ?
        "#,
    )
    .bind(&record.script_name)
    .bind(&record.args)
    .execute(&mut *transaction)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO agent_execution_history (
            id,
            script_name,
            args,
            exit_code,
            success,
            stdout,
            stderr,
            timestamp
        )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
            script_name = excluded.script_name,
            args = excluded.args,
            exit_code = excluded.exit_code,
            success = excluded.success,
            stdout = excluded.stdout,
            stderr = excluded.stderr,
            timestamp = excluded.timestamp
        "#,
    )
    .bind(&record.id)
    .bind(&record.script_name)
    .bind(&record.args)
    .bind(record.exit_code)
    .bind(success)
    .bind(&record.stdout)
    .bind(&record.stderr)
    .bind(record.timestamp)
    .execute(&mut *transaction)
    .await?;

    sqlx::query(
        r#"
        DELETE FROM agent_execution_history
        WHERE id IN (
            SELECT id
            FROM agent_execution_history
            ORDER BY timestamp DESC, created_at DESC, id DESC
            LIMIT -1 OFFSET ?
        )
        "#,
    )
    .bind(history_limit)
    .execute(&mut *transaction)
    .await?;

    transaction.commit().await?;

    Ok(())
}

pub async fn list_agent_execution_history(
    database: &StorageDatabase,
    limit: usize,
) -> Result<Vec<AgentExecutionHistoryRecord>, StorageError> {
    let rows = sqlx::query(
        r#"
        SELECT id, timestamp, script_name, args, exit_code, success, stdout, stderr
        FROM agent_execution_history
        ORDER BY timestamp DESC, created_at DESC, id DESC
        LIMIT ?
        "#,
    )
    .bind(history_limit_i64(limit))
    .fetch_all(database.pool())
    .await?;

    rows.into_iter()
        .map(agent_execution_history_record_from_row)
        .collect()
}

pub async fn clear_agent_execution_history(database: &StorageDatabase) -> Result<(), StorageError> {
    sqlx::query("DELETE FROM agent_execution_history")
        .execute(database.pool())
        .await?;

    Ok(())
}

pub async fn clear_osv_command_history(database: &StorageDatabase) -> Result<(), StorageError> {
    sqlx::query("DELETE FROM osv_command_history")
        .execute(database.pool())
        .await?;

    Ok(())
}

pub async fn get_agent_skills_settings(
    database: &StorageDatabase,
) -> Result<AgentSkillsSettings, StorageError> {
    let Some(row) = sqlx::query(
        r#"
        SELECT settings_json
        FROM tool_settings
        WHERE tool_key = ?
        "#,
    )
    .bind(AGENT_SKILLS_TOOL_KEY)
    .fetch_optional(database.pool())
    .await?
    else {
        return Ok(AgentSkillsSettings::default());
    };

    let settings_json: String = row.try_get("settings_json")?;
    Ok(normalize_agent_skills_settings(serde_json::from_str(
        &settings_json,
    )?))
}

pub async fn save_agent_skills_settings(
    database: &StorageDatabase,
    settings: AgentSkillsSettings,
) -> Result<AgentSkillsSettings, StorageError> {
    let settings = normalize_agent_skills_settings(settings);
    let settings_json = serde_json::to_string(&settings)?;

    sqlx::query(
        r#"
        INSERT INTO tool_settings (tool_key, settings_json, updated_at)
        VALUES (?, ?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
        ON CONFLICT(tool_key) DO UPDATE SET
            settings_json = excluded.settings_json,
            updated_at = excluded.updated_at
        "#,
    )
    .bind(AGENT_SKILLS_TOOL_KEY)
    .bind(settings_json)
    .execute(database.pool())
    .await?;

    Ok(settings)
}

pub async fn save_osv_latest_scan_result(
    database: &StorageDatabase,
    result: OsvScanResult,
) -> Result<(), StorageError> {
    let result = normalize_osv_latest_scan_result(result)?;
    let scanned_at = osv_scan_result_scanned_at(&result);
    let result_json = serde_json::to_string(&result)?;

    sqlx::query(
        r#"
        INSERT INTO osv_latest_scan_results (
            project_path,
            result_json,
            scanned_at,
            updated_at
        )
        VALUES (?, ?, ?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
        ON CONFLICT(project_path) DO UPDATE SET
            result_json = excluded.result_json,
            scanned_at = excluded.scanned_at,
            updated_at = excluded.updated_at
        "#,
    )
    .bind(&result.project_path)
    .bind(result_json)
    .bind(scanned_at)
    .execute(database.pool())
    .await?;

    Ok(())
}

pub async fn get_osv_latest_scan_result(
    database: &StorageDatabase,
    project_path: impl AsRef<str>,
) -> Result<Option<OsvScanResult>, StorageError> {
    let project_path = normalize_project_path(project_path.as_ref())?;
    let Some(row) = sqlx::query(
        r#"
        SELECT result_json
        FROM osv_latest_scan_results
        WHERE project_path = ?
        "#,
    )
    .bind(project_path)
    .fetch_optional(database.pool())
    .await?
    else {
        return Ok(None);
    };

    let result_json: String = row.try_get("result_json")?;
    Ok(Some(serde_json::from_str(&result_json)?))
}

pub async fn delete_osv_latest_scan_result(
    database: &StorageDatabase,
    project_path: impl AsRef<str>,
) -> Result<(), StorageError> {
    let project_path = normalize_project_path(project_path.as_ref())?;
    sqlx::query("DELETE FROM osv_latest_scan_results WHERE project_path = ?")
        .bind(project_path)
        .execute(database.pool())
        .await?;

    Ok(())
}

pub async fn replace_osv_projects(
    database: &StorageDatabase,
    projects: Vec<OsvProjectRecord>,
) -> Result<(), StorageError> {
    let projects = projects
        .into_iter()
        .map(normalize_osv_project_record)
        .collect::<Result<Vec<_>, _>>()?;
    let mut transaction = database.pool().begin().await?;

    sqlx::query("DELETE FROM osv_projects")
        .execute(&mut *transaction)
        .await?;

    for (index, project) in projects.into_iter().enumerate() {
        insert_osv_project(&mut transaction, project, index).await?;
    }

    transaction.commit().await?;

    Ok(())
}

pub async fn list_osv_projects(
    database: &StorageDatabase,
) -> Result<Vec<OsvProjectRecord>, StorageError> {
    let rows = sqlx::query(
        r#"
        SELECT name, path, last_scanned, health_score
        FROM osv_projects
        ORDER BY sort_order ASC, name ASC, path ASC
        "#,
    )
    .fetch_all(database.pool())
    .await?;

    rows.into_iter().map(osv_project_record_from_row).collect()
}

pub async fn replace_osv_command_history(
    database: &StorageDatabase,
    history: Vec<OsvCommandExecutionRecord>,
    limit: usize,
) -> Result<(), StorageError> {
    let mut history = history
        .into_iter()
        .map(normalize_osv_command_history_record)
        .collect::<Result<Vec<_>, _>>()?;
    trim_osv_command_history_records(&mut history, limit);

    let mut transaction = database.pool().begin().await?;

    sqlx::query("DELETE FROM osv_command_history")
        .execute(&mut *transaction)
        .await?;

    for record in history {
        insert_osv_command_history_record(&mut transaction, record).await?;
    }

    transaction.commit().await?;

    Ok(())
}

pub async fn list_osv_command_history(
    database: &StorageDatabase,
    limit: usize,
) -> Result<Vec<OsvCommandExecutionRecord>, StorageError> {
    let rows = sqlx::query(
        r#"
        SELECT record_json
        FROM (
            SELECT record_json, started_at, created_at, id
            FROM osv_command_history
            ORDER BY started_at DESC, created_at DESC, id DESC
            LIMIT ?
        )
        ORDER BY started_at ASC, created_at ASC, id ASC
        "#,
    )
    .bind(history_limit_i64(limit))
    .fetch_all(database.pool())
    .await?;

    rows.into_iter()
        .map(osv_command_history_record_from_row)
        .collect()
}

async fn create_database_parent_dir(database_path: &Path) -> Result<(), StorageError> {
    let Some(parent) = database_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
    else {
        return Ok(());
    };

    fs::create_dir_all(parent)
        .await
        .map_err(|source| StorageError::CreateDirectory {
            path: parent.to_path_buf(),
            source,
        })
}

fn normalize_database_path(database_path: &Path) -> Result<PathBuf, StorageError> {
    if database_path.as_os_str().is_empty() {
        return Err(StorageError::EmptyDatabasePath);
    }

    Ok(database_path.to_path_buf())
}

fn normalize_backup_path(backup_path: &Path) -> Result<PathBuf, StorageError> {
    if backup_path.as_os_str().is_empty() {
        return Err(StorageError::InvalidBackupPath(
            "备份路径不能为空".to_string(),
        ));
    }

    Ok(backup_path.to_path_buf())
}

fn normalize_restore_backup_path(backup_path: &Path) -> Result<PathBuf, StorageError> {
    if backup_path.as_os_str().is_empty() {
        return Err(StorageError::InvalidRestorePath(
            "恢复来源不能为空".to_string(),
        ));
    }
    if !backup_path.exists() {
        return Err(StorageError::InvalidRestorePath(format!(
            "恢复来源不存在: {}",
            backup_path.display()
        )));
    }

    Ok(backup_path.to_path_buf())
}

fn sidecar_database_path(database_path: &Path, suffix: &str) -> PathBuf {
    PathBuf::from(format!("{}{}", database_path.display(), suffix))
}

fn database_parent_exists(database_path: &Path) -> bool {
    database_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .map(Path::exists)
        .unwrap_or(true)
}

async fn file_size_or_zero(path: &Path) -> Result<u64, StorageError> {
    match fs::metadata(path).await {
        Ok(metadata) => Ok(metadata.len()),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(0),
        Err(source) => Err(StorageError::FileOperation {
            action: "读取文件信息",
            path: path.to_path_buf(),
            source,
        }),
    }
}

async fn remove_database_sidecar_files(database_path: &Path) -> Result<(), StorageError> {
    for path in [
        database_path.to_path_buf(),
        sidecar_database_path(database_path, "-wal"),
        sidecar_database_path(database_path, "-shm"),
    ] {
        match fs::remove_file(&path).await {
            Ok(()) => {}
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => {}
            Err(source) => {
                return Err(StorageError::FileOperation {
                    action: "删除旧数据库文件",
                    path,
                    source,
                })
            }
        }
    }

    Ok(())
}

async fn database_record_count(
    database: &StorageDatabase,
    key: &str,
    label: &str,
    query: &str,
) -> Result<DatabaseRecordCount, StorageError> {
    let row = sqlx::query(query).fetch_one(database.pool()).await?;
    let count = row.try_get::<i64, _>("count")?;

    Ok(DatabaseRecordCount {
        key: key.to_string(),
        label: label.to_string(),
        count,
    })
}

async fn read_schema_version(pool: &SqlitePool) -> Result<i64, sqlx::Error> {
    let row = sqlx::query("PRAGMA user_version").fetch_one(pool).await?;
    row.try_get::<i64, _>("user_version")
}

async fn read_applied_migrations(pool: &SqlitePool) -> Result<u32, sqlx::Error> {
    let row = sqlx::query("SELECT COUNT(*) AS count FROM _sqlx_migrations WHERE success = 1")
        .fetch_one(pool)
        .await?;
    let count = row.try_get::<i64, _>("count")?;
    Ok(u32::try_from(count).unwrap_or(u32::MAX))
}

fn normalize_agent_execution_history_record(
    mut record: AgentExecutionHistoryRecord,
) -> Result<AgentExecutionHistoryRecord, StorageError> {
    record.id = record.id.trim().to_string();
    record.script_name = record.script_name.trim().to_string();

    if record.id.is_empty() {
        return Err(StorageError::InvalidAgentHistoryRecord(
            "记录 ID 不能为空".to_string(),
        ));
    }
    if record.script_name.is_empty() {
        return Err(StorageError::InvalidAgentHistoryRecord(
            "脚本名称不能为空".to_string(),
        ));
    }
    if record.timestamp < 0 {
        return Err(StorageError::InvalidAgentHistoryRecord(
            "执行时间不能小于 0".to_string(),
        ));
    }

    Ok(record)
}

fn normalize_osv_latest_scan_result(
    mut result: OsvScanResult,
) -> Result<OsvScanResult, StorageError> {
    result.project_path = normalize_project_path(&result.project_path)?;
    Ok(result)
}

fn normalize_osv_project_record(
    mut record: OsvProjectRecord,
) -> Result<OsvProjectRecord, StorageError> {
    record.name = record.name.trim().to_string();
    record.path = record.path.trim().to_string();
    record.last_scanned = record
        .last_scanned
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    if record.path.is_empty() {
        return Err(StorageError::InvalidOsvProjectRecord(
            "项目路径不能为空".to_string(),
        ));
    }
    if record.name.is_empty() {
        record.name = Path::new(&record.path)
            .file_name()
            .and_then(|value| value.to_str())
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(&record.path)
            .to_string();
    }

    Ok(record)
}

fn normalize_osv_command_history_record(
    mut record: OsvCommandExecutionRecord,
) -> Result<OsvCommandExecutionRecord, StorageError> {
    record.id = record.id.trim().to_string();
    record.project_path = record.project_path.trim().to_string();
    record.working_dir = record.working_dir.trim().to_string();
    record.display_command = record.display_command.trim().to_string();
    record.started_at = record.started_at.trim().to_string();

    if record.id.is_empty() {
        return Err(StorageError::InvalidOsvCommandHistoryRecord(
            "记录 ID 不能为空".to_string(),
        ));
    }
    if record.project_path.is_empty() {
        return Err(StorageError::InvalidOsvCommandHistoryRecord(
            "项目路径不能为空".to_string(),
        ));
    }
    if record.started_at.is_empty() {
        return Err(StorageError::InvalidOsvCommandHistoryRecord(
            "开始时间不能为空".to_string(),
        ));
    }

    Ok(record)
}

fn normalize_project_path(project_path: &str) -> Result<String, StorageError> {
    let project_path = project_path.trim().to_string();
    if project_path.is_empty() {
        return Err(StorageError::InvalidOsvLatestScanResult(
            "项目路径不能为空".to_string(),
        ));
    }

    Ok(project_path)
}

fn osv_scan_result_scanned_at(result: &OsvScanResult) -> String {
    result
        .command
        .finished_at
        .clone()
        .unwrap_or_else(|| result.command.started_at.clone())
}

fn history_limit_i64(limit: usize) -> i64 {
    i64::try_from(limit).unwrap_or(i64::MAX)
}

const AGENT_SKILLS_TOOL_KEY: &str = "agentSkills";

fn normalize_agent_skills_settings(mut settings: AgentSkillsSettings) -> AgentSkillsSettings {
    settings.script_dir = settings.script_dir.trim().to_string();
    settings
}

fn trim_osv_command_history_records(history: &mut Vec<OsvCommandExecutionRecord>, limit: usize) {
    if history.len() > limit {
        let keep_from = history.len() - limit;
        history.drain(0..keep_from);
    }
}

fn osv_command_kind_value(kind: OsvCommandKind) -> &'static str {
    match kind {
        OsvCommandKind::Scan => "scan",
        OsvCommandKind::Export => "export",
        OsvCommandKind::Fix => "fix",
    }
}

async fn insert_osv_project(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    project: OsvProjectRecord,
    index: usize,
) -> Result<(), StorageError> {
    let sort_order = i64::try_from(index).unwrap_or(i64::MAX);
    let health_score = project.health_score.map(i64::from);

    sqlx::query(
        r#"
        INSERT INTO osv_projects (
            path,
            name,
            last_scanned,
            health_score,
            sort_order,
            updated_at
        )
        VALUES (?, ?, ?, ?, ?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
        "#,
    )
    .bind(&project.path)
    .bind(&project.name)
    .bind(&project.last_scanned)
    .bind(health_score)
    .bind(sort_order)
    .execute(&mut **transaction)
    .await?;

    Ok(())
}

async fn insert_osv_command_history_record(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    record: OsvCommandExecutionRecord,
) -> Result<(), StorageError> {
    let kind = osv_command_kind_value(record.kind);
    let record_json = serde_json::to_string(&record)?;

    sqlx::query(
        r#"
        INSERT INTO osv_command_history (
            id,
            kind,
            project_path,
            record_json,
            started_at
        )
        VALUES (?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
            kind = excluded.kind,
            project_path = excluded.project_path,
            record_json = excluded.record_json,
            started_at = excluded.started_at
        "#,
    )
    .bind(&record.id)
    .bind(kind)
    .bind(&record.project_path)
    .bind(record_json)
    .bind(&record.started_at)
    .execute(&mut **transaction)
    .await?;

    Ok(())
}

fn agent_execution_history_record_from_row(
    row: sqlx::sqlite::SqliteRow,
) -> Result<AgentExecutionHistoryRecord, StorageError> {
    Ok(AgentExecutionHistoryRecord {
        id: row.try_get("id")?,
        timestamp: row.try_get("timestamp")?,
        script_name: row.try_get("script_name")?,
        args: row.try_get("args")?,
        exit_code: row.try_get("exit_code")?,
        success: row.try_get::<i64, _>("success")? != 0,
        stdout: row.try_get("stdout")?,
        stderr: row.try_get("stderr")?,
    })
}

fn osv_project_record_from_row(
    row: sqlx::sqlite::SqliteRow,
) -> Result<OsvProjectRecord, StorageError> {
    let health_score = row
        .try_get::<Option<i64>, _>("health_score")?
        .and_then(|value| u32::try_from(value).ok());

    Ok(OsvProjectRecord {
        name: row.try_get("name")?,
        path: row.try_get("path")?,
        last_scanned: row.try_get("last_scanned")?,
        health_score,
    })
}

fn osv_command_history_record_from_row(
    row: sqlx::sqlite::SqliteRow,
) -> Result<OsvCommandExecutionRecord, StorageError> {
    let record_json: String = row.try_get("record_json")?;
    Ok(serde_json::from_str(&record_json)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

    #[tokio::test]
    async fn initialize_database_creates_file_and_runs_migrations() {
        let path = unique_database_path();

        let database = initialize_database(&path).await.unwrap();

        assert!(path.exists());
        assert_eq!(database.schema_version(), 4);
        assert_eq!(database.applied_migrations(), 4);

        cleanup_database_files(&path);
    }

    #[tokio::test]
    async fn check_database_health_reports_ready_database() {
        let path = unique_database_path();

        let health = check_database_health(&path).await;

        assert_eq!(health.status, DatabaseHealthStatus::Ready);
        assert!(health.database_exists);
        assert!(health.parent_directory_exists);
        assert_eq!(health.schema_version, Some(4));
        assert_eq!(health.applied_migrations, 4);

        cleanup_database_files(&path);
    }

    #[tokio::test]
    async fn check_database_health_reports_empty_path_error() {
        let health = check_database_health("").await;

        assert_eq!(health.status, DatabaseHealthStatus::Error);
        assert_eq!(health.schema_version, None);
        assert!(health.message.contains("数据库路径不能为空"));
    }

    #[tokio::test]
    async fn database_file_stats_reports_existing_database_size() {
        let path = unique_database_path();
        let _database = initialize_database(&path).await.unwrap();

        let stats = database_file_stats(&path).await.unwrap();

        assert_eq!(stats.database_path, path.display().to_string());
        assert!(stats.main_file_size_bytes > 0);
        assert!(
            stats.total_size_bytes
                >= stats
                    .main_file_size_bytes
                    .saturating_add(stats.wal_file_size_bytes)
        );

        cleanup_database_files(&path);
    }

    #[tokio::test]
    async fn backup_database_creates_sqlite_snapshot() {
        let path = unique_database_path();
        let backup_path = unique_database_path();
        let database = initialize_database(&path).await.unwrap();

        save_agent_execution_history_record(
            &database,
            history_record("record-1", "script-a", "", 1, true),
            50,
        )
        .await
        .unwrap();
        let backup_path = backup_database(&database, &backup_path).await.unwrap();

        assert!(backup_path.exists());
        assert!(
            database_file_stats(&backup_path)
                .await
                .unwrap()
                .main_file_size_bytes
                > 0
        );

        cleanup_database_files(&path);
        cleanup_database_files(&backup_path);
    }

    #[tokio::test]
    async fn vacuum_database_runs_without_removing_data() {
        let path = unique_database_path();
        let database = initialize_database(&path).await.unwrap();

        save_agent_execution_history_record(
            &database,
            history_record("record-1", "script-a", "", 1, true),
            50,
        )
        .await
        .unwrap();
        vacuum_database(&database).await.unwrap();

        let history = list_agent_execution_history(&database, 50).await.unwrap();

        assert_eq!(history.len(), 1);
        assert_eq!(history[0].id, "record-1");

        cleanup_database_files(&path);
    }

    #[tokio::test]
    async fn restore_database_file_replaces_current_database_from_backup() {
        let path = unique_database_path();
        let backup_path = unique_database_path();
        let database = initialize_database(&path).await.unwrap();

        save_agent_execution_history_record(
            &database,
            history_record("record-1", "script-a", "", 1, true),
            50,
        )
        .await
        .unwrap();
        backup_database(&database, &backup_path).await.unwrap();
        save_agent_execution_history_record(
            &database,
            history_record("record-2", "script-b", "", 2, true),
            50,
        )
        .await
        .unwrap();
        database.pool().close().await;

        restore_database_file(&path, &backup_path).await.unwrap();

        let restored_database = initialize_database(&path).await.unwrap();
        let history = list_agent_execution_history(&restored_database, 50)
            .await
            .unwrap();

        assert_eq!(history.len(), 1);
        assert_eq!(history[0].id, "record-1");

        cleanup_database_files(&path);
        cleanup_database_files(&backup_path);
    }

    #[tokio::test]
    async fn database_storage_diagnostics_reports_record_counts() {
        let path = unique_database_path();
        let database = initialize_database(&path).await.unwrap();

        save_agent_execution_history_record(
            &database,
            history_record("record-1", "script-a", "", 1, true),
            50,
        )
        .await
        .unwrap();
        replace_osv_projects(
            &database,
            vec![osv_project("project-a", "/tmp/project-a", None, None)],
        )
        .await
        .unwrap();
        replace_osv_command_history(
            &database,
            vec![osv_command_history_record("cmd-1", "/tmp/project-a", "1")],
            50,
        )
        .await
        .unwrap();
        save_osv_latest_scan_result(&database, osv_scan_result("/tmp/project-a", 90, "scan-1"))
            .await
            .unwrap();

        let diagnostics = database_storage_diagnostics(&database).await.unwrap();

        assert!(diagnostics.total_records >= 4);
        assert_eq!(
            diagnostic_count(&diagnostics, "agentExecutionHistory"),
            Some(1)
        );
        assert_eq!(diagnostic_count(&diagnostics, "osvProjects"), Some(1));
        assert_eq!(diagnostic_count(&diagnostics, "osvCommandHistory"), Some(1));
        assert_eq!(
            diagnostic_count(&diagnostics, "osvLatestScanResults"),
            Some(1)
        );

        cleanup_database_files(&path);
    }

    #[tokio::test]
    async fn agent_execution_history_is_saved_deduplicated_and_trimmed() {
        let path = unique_database_path();
        let database = initialize_database(&path).await.unwrap();

        save_agent_execution_history_record(
            &database,
            history_record("record-1", "script-a", "--help", 1, true),
            2,
        )
        .await
        .unwrap();
        save_agent_execution_history_record(
            &database,
            history_record("record-2", "script-a", "--help", 2, false),
            2,
        )
        .await
        .unwrap();
        save_agent_execution_history_record(
            &database,
            history_record("record-3", "script-b", "", 3, true),
            2,
        )
        .await
        .unwrap();

        let history = list_agent_execution_history(&database, 50).await.unwrap();

        assert_eq!(history.len(), 2);
        assert_eq!(history[0].id, "record-3");
        assert_eq!(history[1].id, "record-2");
        assert_eq!(history[1].script_name, "script-a");
        assert!(!history[1].success);

        cleanup_database_files(&path);
    }

    #[tokio::test]
    async fn clear_agent_execution_history_removes_all_records() {
        let path = unique_database_path();
        let database = initialize_database(&path).await.unwrap();

        save_agent_execution_history_record(
            &database,
            history_record("record-1", "script-a", "", 1, true),
            50,
        )
        .await
        .unwrap();
        clear_agent_execution_history(&database).await.unwrap();

        let history = list_agent_execution_history(&database, 50).await.unwrap();

        assert!(history.is_empty());

        cleanup_database_files(&path);
    }

    #[tokio::test]
    async fn agent_skills_settings_are_saved_and_loaded() {
        let path = unique_database_path();
        let database = initialize_database(&path).await.unwrap();

        let initial_settings = get_agent_skills_settings(&database).await.unwrap();

        assert_eq!(initial_settings, AgentSkillsSettings::default());

        let saved_settings = save_agent_skills_settings(
            &database,
            AgentSkillsSettings {
                script_dir: "  /Users/alice/work/99_codex  ".to_string(),
            },
        )
        .await
        .unwrap();
        let loaded_settings = get_agent_skills_settings(&database).await.unwrap();

        assert_eq!(saved_settings.script_dir, "/Users/alice/work/99_codex");
        assert_eq!(loaded_settings, saved_settings);

        cleanup_database_files(&path);
    }

    #[tokio::test]
    async fn osv_latest_scan_result_is_saved_and_replaced_per_project() {
        let path = unique_database_path();
        let database = initialize_database(&path).await.unwrap();

        save_osv_latest_scan_result(&database, osv_scan_result("/tmp/project", 90, "scan-1"))
            .await
            .unwrap();
        save_osv_latest_scan_result(&database, osv_scan_result("/tmp/project", 40, "scan-2"))
            .await
            .unwrap();

        let result = get_osv_latest_scan_result(&database, "/tmp/project")
            .await
            .unwrap()
            .unwrap();

        assert_eq!(result.project_path, "/tmp/project");
        assert_eq!(result.summary.health_score, 40);
        assert_eq!(result.command.id, "scan-2");

        cleanup_database_files(&path);
    }

    #[tokio::test]
    async fn delete_osv_latest_scan_result_removes_project_result() {
        let path = unique_database_path();
        let database = initialize_database(&path).await.unwrap();

        save_osv_latest_scan_result(&database, osv_scan_result("/tmp/project", 90, "scan-1"))
            .await
            .unwrap();
        delete_osv_latest_scan_result(&database, "/tmp/project")
            .await
            .unwrap();

        let result = get_osv_latest_scan_result(&database, "/tmp/project")
            .await
            .unwrap();

        assert!(result.is_none());

        cleanup_database_files(&path);
    }

    #[tokio::test]
    async fn osv_projects_are_replaced_and_listed_in_saved_order() {
        let path = unique_database_path();
        let database = initialize_database(&path).await.unwrap();

        replace_osv_projects(
            &database,
            vec![
                osv_project("project-b", "/tmp/project-b", Some("2"), Some(80)),
                osv_project("project-a", "/tmp/project-a", None, None),
            ],
        )
        .await
        .unwrap();

        let projects = list_osv_projects(&database).await.unwrap();

        assert_eq!(projects.len(), 2);
        assert_eq!(projects[0].path, "/tmp/project-b");
        assert_eq!(projects[0].health_score, Some(80));
        assert_eq!(projects[1].path, "/tmp/project-a");

        replace_osv_projects(
            &database,
            vec![osv_project("", "/tmp/project-c", None, Some(30))],
        )
        .await
        .unwrap();

        let projects = list_osv_projects(&database).await.unwrap();

        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].name, "project-c");
        assert_eq!(projects[0].path, "/tmp/project-c");

        cleanup_database_files(&path);
    }

    #[tokio::test]
    async fn osv_command_history_is_replaced_trimmed_and_listed_oldest_first() {
        let path = unique_database_path();
        let database = initialize_database(&path).await.unwrap();

        replace_osv_command_history(
            &database,
            vec![
                osv_command_history_record("cmd-1", "/tmp/project", "1"),
                osv_command_history_record("cmd-2", "/tmp/project", "2"),
                osv_command_history_record("cmd-3", "/tmp/project", "3"),
            ],
            2,
        )
        .await
        .unwrap();

        let history = list_osv_command_history(&database, 50).await.unwrap();

        assert_eq!(history.len(), 2);
        assert_eq!(history[0].id, "cmd-2");
        assert_eq!(history[1].id, "cmd-3");

        cleanup_database_files(&path);
    }

    #[tokio::test]
    async fn clear_osv_command_history_removes_history_records() {
        let path = unique_database_path();
        let database = initialize_database(&path).await.unwrap();

        replace_osv_command_history(
            &database,
            vec![osv_command_history_record("cmd-1", "/tmp/project", "1")],
            50,
        )
        .await
        .unwrap();
        clear_osv_command_history(&database).await.unwrap();

        let history = list_osv_command_history(&database, 50).await.unwrap();

        assert!(history.is_empty());

        cleanup_database_files(&path);
    }

    fn unique_database_path() -> PathBuf {
        let id = NEXT_TEMP_ID.fetch_add(1, Ordering::SeqCst);
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        std::env::temp_dir()
            .join("rusttool-storage-tests")
            .join(format!("storage-{millis}-{id}.db"))
    }

    fn cleanup_database_files(path: &Path) {
        for extension in ["", "-shm", "-wal"] {
            let candidate = PathBuf::from(format!("{}{}", path.display(), extension));
            let _ = std::fs::remove_file(candidate);
        }
    }

    fn diagnostic_count(diagnostics: &DatabaseStorageDiagnostics, key: &str) -> Option<i64> {
        diagnostics
            .record_counts
            .iter()
            .find(|record_count| record_count.key == key)
            .map(|record_count| record_count.count)
    }

    fn history_record(
        id: &str,
        script_name: &str,
        args: &str,
        timestamp: i64,
        success: bool,
    ) -> AgentExecutionHistoryRecord {
        AgentExecutionHistoryRecord {
            id: id.to_string(),
            timestamp,
            script_name: script_name.to_string(),
            args: args.to_string(),
            exit_code: if success { 0 } else { 1 },
            success,
            stdout: "ok".to_string(),
            stderr: String::new(),
        }
    }

    fn osv_project(
        name: &str,
        path: &str,
        last_scanned: Option<&str>,
        health_score: Option<u32>,
    ) -> OsvProjectRecord {
        OsvProjectRecord {
            name: name.to_string(),
            path: path.to_string(),
            last_scanned: last_scanned.map(str::to_string),
            health_score,
        }
    }

    fn osv_command_history_record(
        id: &str,
        project_path: &str,
        started_at: &str,
    ) -> OsvCommandExecutionRecord {
        use crate::tools::osv_scanner::OsvCommandStatus;

        OsvCommandExecutionRecord {
            id: id.to_string(),
            kind: OsvCommandKind::Scan,
            project_path: project_path.to_string(),
            working_dir: project_path.to_string(),
            argv: vec!["osv-scanner".to_string(), "scan".to_string()],
            display_command: "osv-scanner scan source --format json .".to_string(),
            started_at: started_at.to_string(),
            finished_at: None,
            duration_ms: None,
            exit_code: None,
            status: OsvCommandStatus::Succeeded,
            summary: "未发现已知漏洞。".to_string(),
            stderr_excerpt: None,
        }
    }

    fn osv_scan_result(
        project_path: &str,
        health_score: u32,
        command_id: &str,
    ) -> crate::tools::osv_scanner::OsvScanResult {
        use crate::tools::osv_scanner::{
            OsvCommandExecutionRecord, OsvCommandKind, OsvCommandStatus, OsvScanSummary,
            OsvSeverity, OsvSeverityCounts,
        };

        crate::tools::osv_scanner::OsvScanResult {
            project_path: project_path.to_string(),
            vulnerabilities: Vec::new(),
            summary: OsvScanSummary {
                total_vulnerabilities: 0,
                severity_counts: OsvSeverityCounts::default(),
                highest_severity: OsvSeverity::Unknown,
                health_score,
                message: "未发现已知漏洞。".to_string(),
            },
            command: OsvCommandExecutionRecord {
                id: command_id.to_string(),
                kind: OsvCommandKind::Scan,
                project_path: project_path.to_string(),
                working_dir: project_path.to_string(),
                argv: vec!["osv-scanner".to_string(), "scan".to_string()],
                display_command: "osv-scanner scan source --format json .".to_string(),
                started_at: "1".to_string(),
                finished_at: Some("2".to_string()),
                duration_ms: Some(10),
                exit_code: Some(0),
                status: OsvCommandStatus::Succeeded,
                summary: "未发现已知漏洞。".to_string(),
                stderr_excerpt: None,
            },
        }
    }
}
