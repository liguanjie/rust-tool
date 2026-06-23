use rust_tool_core::workbench::{execute_script, list_scripts, ExecutionResult, ScriptInfo};
use rust_tool_core::{
    backup_database, build_export_command, build_scan_command, check_database_health,
    check_osv_scanner_installed,
    clear_agent_execution_history as clear_agent_execution_history_in_db,
    clear_osv_command_history as clear_osv_command_history_in_db, convert_vless_to_yaml,
    database_file_stats, database_storage_diagnostics,
    decode_finalshell_password as decode_finalshell_password_in_core,
    delete_osv_latest_scan_result as delete_osv_latest_scan_result_in_db, diagnose_project,
    export_report, get_agent_skills_settings as get_agent_skills_settings_from_db,
    get_osv_latest_scan_result as get_osv_latest_scan_result_from_db, ignore_vulnerability,
    initialize_database, list_agent_execution_history, list_osv_command_history, list_osv_projects,
    replace_osv_command_history, replace_osv_projects, restore_database_file,
    save_agent_execution_history_record as save_agent_execution_history_record_in_db,
    save_agent_skills_settings as save_agent_skills_settings_in_db,
    save_osv_latest_scan_result as save_osv_latest_scan_result_in_db, scan_project,
    vacuum_database, AgentExecutionHistoryRecord, AgentSkillsSettings, ConvertOptions,
    DatabaseFileStats, DatabaseHealth, DatabaseStorageDiagnostics, OsvCommandExecutionRecord,
    OsvCommandPreview, OsvIgnoreRequest, OsvIgnoreResult, OsvInstallStatus, OsvProjectDiagnostic,
    OsvProjectDiagnosticRequest, OsvProjectRecord, OsvReportExportCommandRequest,
    OsvReportExportRequest, OsvReportExportResult, OsvScanCommandRequest, OsvScanRequest,
    OsvScanResult, OutputMode, StorageDatabase, TemplateMode, TransitGroupType,
    TransitProviderOptions, TransitProxyOptions,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::Manager;

const OSV_COMMAND_HISTORY_LIMIT: usize = 50;
const AGENT_EXECUTION_HISTORY_LIMIT: usize = 50;
const DEFAULT_DATABASE_FILE_NAME: &str = "rusttool.db";
const LEGACY_DATABASE_FILE_NAME: &str = "rusttool.sqlite";

#[tauri::command]
fn get_workbench_scripts(dir: String) -> Result<Vec<ScriptInfo>, String> {
    list_scripts(&dir)
}

#[tauri::command]
fn run_workbench_script(path: String, args: String) -> Result<ExecutionResult, String> {
    let args_vec: Vec<String> = args.split_whitespace().map(|s| s.to_string()).collect();
    execute_script(&path, args_vec)
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum VlessOutputMode {
    FullConfig,
    ProxyOnly,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum VlessTemplateMode {
    Minimal,
    Standard,
    FullRules,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum VlessTransitGroupType {
    Select,
    UrlTest,
    Fallback,
}

#[derive(Debug, Deserialize)]
struct VlessTransitProxyRequest {
    provider_name: String,
    provider_url: Option<String>,
    provider_path: Option<String>,
    group_name: String,
    group_type: Option<VlessTransitGroupType>,
    bypass_domains: Option<Vec<String>>,
    providers: Option<Vec<VlessTransitProviderRequest>>,
}

#[derive(Debug, Deserialize)]
struct VlessTransitProviderRequest {
    provider_name: String,
    provider_url: Option<String>,
    provider_path: Option<String>,
    group_name: String,
}

#[derive(Debug, Deserialize)]
struct ConvertVlessRequest {
    input: String,
    mode: Option<VlessOutputMode>,
    template: Option<VlessTemplateMode>,
    proxy_name: Option<String>,
    direct_domains: Option<Vec<String>>,
    transit_proxy: Option<VlessTransitProxyRequest>,
}

#[derive(Debug, Serialize)]
struct ConvertVlessResponse {
    yaml: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FinalShellPasswordDecodeRequest {
    encrypted_password: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FinalShellPasswordDecodeResponse {
    password: String,
}

#[derive(Debug, Serialize)]
struct SaveYamlResponse {
    path: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
struct VlessToolSettings {
    input: String,
    mode: String,
    template: String,
    download_name: String,
    direct_domains: String,
    transit_enabled: bool,
    transit_provider_url: String,
    transit_provider_name: String,
    transit_provider_path: String,
    transit_group_name: String,
    transit_group_type: String,
    transit_bypass_domains: String,
}

impl Default for VlessToolSettings {
    fn default() -> Self {
        Self {
            input: String::new(),
            mode: "full_config".to_string(),
            template: "full_rules".to_string(),
            download_name: "mihomo".to_string(),
            direct_domains: String::new(),
            transit_enabled: false,
            transit_provider_url: String::new(),
            transit_provider_name: "transit".to_string(),
            transit_provider_path: String::new(),
            transit_group_name: "中转节点组".to_string(),
            transit_group_type: "url_test".to_string(),
            transit_bypass_domains: String::new(),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
struct OsvProjectSettings {
    name: String,
    path: String,
    last_scanned: Option<String>,
    health_score: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
struct OsvScannerSettings {
    projects: Vec<OsvProjectSettings>,
    auto_scan_schedule: String,
    command_history: Vec<OsvCommandExecutionRecord>,
}

impl Default for OsvScannerSettings {
    fn default() -> Self {
        Self {
            projects: Vec::new(),
            auto_scan_schedule: "none".to_string(),
            command_history: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
struct ProgramSettings {
    database_path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProgramSettingsResponse {
    settings: ProgramSettings,
    default_database_path: String,
    effective_database_path: String,
    database_health: DatabaseHealth,
    database_stats: DatabaseFileStats,
    database_diagnostics: DatabaseStorageDiagnostics,
    legacy_database: LegacyDatabaseInfo,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DatabaseBackupResponse {
    backup_path: String,
    database_stats: DatabaseFileStats,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RestoreProgramDatabaseRequest {
    backup_path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DatabaseRestoreResponse {
    safety_backup_path: String,
    state: ProgramSettingsResponse,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LegacyDatabaseInfo {
    path: String,
    exists: bool,
    main_file_size_bytes: u64,
    wal_file_size_bytes: u64,
    shm_file_size_bytes: u64,
    total_size_bytes: u64,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
struct DesktopSettings {
    program: ProgramSettings,
    vless_to_mihomo: VlessToolSettings,
    osv_scanner: OsvScannerSettings,
}

#[tauri::command]
fn convert_vless_to_mihomo(request: ConvertVlessRequest) -> Result<ConvertVlessResponse, String> {
    let output_mode = match request.mode.unwrap_or(VlessOutputMode::FullConfig) {
        VlessOutputMode::FullConfig => OutputMode::FullConfig,
        VlessOutputMode::ProxyOnly => OutputMode::ProxyOnly,
    };
    let template_mode = match request.template.unwrap_or(VlessTemplateMode::Standard) {
        VlessTemplateMode::Minimal => TemplateMode::Minimal,
        VlessTemplateMode::Standard => TemplateMode::Standard,
        VlessTemplateMode::FullRules => TemplateMode::FullRules,
    };

    convert_vless_to_yaml(
        &request.input,
        ConvertOptions {
            output_mode,
            template_mode,
            proxy_name: request.proxy_name,
            direct_domains: request.direct_domains.unwrap_or_default(),
            transit_proxy: request.transit_proxy.map(Into::into),
        },
    )
    .map(|yaml| ConvertVlessResponse { yaml })
    .map_err(|error| error.to_string())
}

#[tauri::command]
fn decode_finalshell_password(
    request: FinalShellPasswordDecodeRequest,
) -> Result<FinalShellPasswordDecodeResponse, String> {
    decode_finalshell_password_in_core(&request.encrypted_password)
        .map(|password| FinalShellPasswordDecodeResponse { password })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn get_vless_tool_settings(app: tauri::AppHandle) -> Result<VlessToolSettings, String> {
    Ok(read_desktop_settings(&app)?.vless_to_mihomo)
}

#[tauri::command]
fn save_vless_tool_settings(
    app: tauri::AppHandle,
    settings: VlessToolSettings,
) -> Result<VlessToolSettings, String> {
    let mut desktop_settings = read_desktop_settings(&app)?;
    desktop_settings.vless_to_mihomo = settings.clone();
    write_desktop_settings(&app, &desktop_settings)?;

    Ok(settings)
}

#[tauri::command]
async fn get_osv_settings(app: tauri::AppHandle) -> Result<OsvScannerSettings, String> {
    let desktop_settings = read_desktop_settings(&app)?;
    let database = initialize_app_database(&app).await?;

    migrate_legacy_osv_settings_if_needed(&database, &desktop_settings.osv_scanner).await?;
    compose_osv_settings_from_storage(&database, desktop_settings.osv_scanner.auto_scan_schedule)
        .await
}

#[tauri::command]
async fn save_osv_settings(
    app: tauri::AppHandle,
    mut settings: OsvScannerSettings,
) -> Result<OsvScannerSettings, String> {
    trim_osv_command_history(&mut settings.command_history);
    let database = initialize_app_database(&app).await?;

    replace_osv_projects(
        &database,
        settings
            .projects
            .clone()
            .into_iter()
            .map(Into::into)
            .collect(),
    )
    .await
    .map_err(|error| error.to_string())?;
    replace_osv_command_history(
        &database,
        settings.command_history.clone(),
        OSV_COMMAND_HISTORY_LIMIT,
    )
    .await
    .map_err(|error| error.to_string())?;

    let saved_settings =
        compose_osv_settings_from_storage(&database, settings.auto_scan_schedule).await?;
    let mut desktop_settings = read_desktop_settings(&app)?;
    desktop_settings.osv_scanner = saved_settings.clone();
    write_desktop_settings(&app, &desktop_settings)?;

    Ok(saved_settings)
}

#[tauri::command]
async fn get_program_settings(app: tauri::AppHandle) -> Result<ProgramSettingsResponse, String> {
    let settings = read_desktop_settings(&app)?.program;
    program_settings_response(&app, settings).await
}

#[tauri::command]
async fn save_program_settings(
    app: tauri::AppHandle,
    settings: ProgramSettings,
) -> Result<ProgramSettingsResponse, String> {
    let settings = normalize_program_settings(settings);
    let mut desktop_settings = read_desktop_settings(&app)?;
    desktop_settings.program = settings.clone();
    write_desktop_settings(&app, &desktop_settings)?;

    program_settings_response(&app, settings).await
}

#[tauri::command]
async fn backup_program_database(app: tauri::AppHandle) -> Result<DatabaseBackupResponse, String> {
    let database = initialize_app_database(&app).await?;
    let backup_path = next_available_path(default_database_backup_path(&app)?);
    let backup_path = backup_database(&database, &backup_path)
        .await
        .map_err(|error| error.to_string())?;
    let database_stats = database_file_stats(database.database_path())
        .await
        .map_err(|error| error.to_string())?;

    Ok(DatabaseBackupResponse {
        backup_path: backup_path.display().to_string(),
        database_stats,
    })
}

#[tauri::command]
async fn compact_program_database(
    app: tauri::AppHandle,
) -> Result<ProgramSettingsResponse, String> {
    let database = initialize_app_database(&app).await?;
    vacuum_database(&database)
        .await
        .map_err(|error| error.to_string())?;
    let settings = read_desktop_settings(&app)?.program;
    program_settings_response(&app, settings).await
}

#[tauri::command]
async fn restore_program_database(
    app: tauri::AppHandle,
    request: RestoreProgramDatabaseRequest,
) -> Result<DatabaseRestoreResponse, String> {
    let restore_source = request.backup_path.trim().to_string();
    let current_database_path = PathBuf::from(effective_database_path(&app)?);
    let database = initialize_app_database(&app).await?;
    let safety_backup_path = next_available_path(default_database_restore_backup_path(&app)?);
    let safety_backup_path = backup_database(&database, &safety_backup_path)
        .await
        .map_err(|error| error.to_string())?;

    database.pool().close().await;

    if let Err(error) = restore_database_file(&current_database_path, &restore_source).await {
        let _ = restore_database_file(&current_database_path, &safety_backup_path).await;
        return Err(format!("恢复数据库失败，已尝试回滚到恢复前备份: {error}"));
    }

    initialize_database(&current_database_path)
        .await
        .map_err(|error| error.to_string())?;
    let settings = read_desktop_settings(&app)?.program;
    let state = program_settings_response(&app, settings).await?;

    Ok(DatabaseRestoreResponse {
        safety_backup_path: safety_backup_path.display().to_string(),
        state,
    })
}

#[tauri::command]
fn open_program_database_directory(app: tauri::AppHandle) -> Result<(), String> {
    let directory = effective_database_directory(&app)?;
    fs::create_dir_all(&directory).map_err(|error| format!("创建数据库目录失败: {error}"))?;
    open_path_in_file_manager(&directory)
}

#[tauri::command]
async fn delete_legacy_program_database(
    app: tauri::AppHandle,
) -> Result<ProgramSettingsResponse, String> {
    delete_legacy_database_files(&app)?;
    let settings = read_desktop_settings(&app)?.program;
    program_settings_response(&app, settings).await
}

#[tauri::command]
async fn clear_program_database_history(
    app: tauri::AppHandle,
) -> Result<ProgramSettingsResponse, String> {
    let database = initialize_app_database(&app).await?;
    clear_agent_execution_history_in_db(&database)
        .await
        .map_err(|error| error.to_string())?;
    clear_osv_command_history_in_db(&database)
        .await
        .map_err(|error| error.to_string())?;

    let mut desktop_settings = read_desktop_settings(&app)?;
    desktop_settings.osv_scanner.command_history.clear();
    let settings = desktop_settings.program.clone();
    write_desktop_settings(&app, &desktop_settings)?;

    program_settings_response(&app, settings).await
}

#[tauri::command]
async fn get_agent_execution_history(
    app: tauri::AppHandle,
) -> Result<Vec<AgentExecutionHistoryRecord>, String> {
    let database = initialize_app_database(&app).await?;
    list_agent_execution_history(&database, AGENT_EXECUTION_HISTORY_LIMIT)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn save_agent_execution_history_record(
    app: tauri::AppHandle,
    record: AgentExecutionHistoryRecord,
) -> Result<Vec<AgentExecutionHistoryRecord>, String> {
    let database = initialize_app_database(&app).await?;
    save_agent_execution_history_record_in_db(&database, record, AGENT_EXECUTION_HISTORY_LIMIT)
        .await
        .map_err(|error| error.to_string())?;

    list_agent_execution_history(&database, AGENT_EXECUTION_HISTORY_LIMIT)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn clear_agent_execution_history(app: tauri::AppHandle) -> Result<(), String> {
    let database = initialize_app_database(&app).await?;
    clear_agent_execution_history_in_db(&database)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn get_agent_skills_settings(app: tauri::AppHandle) -> Result<AgentSkillsSettings, String> {
    let database = initialize_app_database(&app).await?;
    get_agent_skills_settings_from_db(&database)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn save_agent_skills_settings(
    app: tauri::AppHandle,
    settings: AgentSkillsSettings,
) -> Result<AgentSkillsSettings, String> {
    let database = initialize_app_database(&app).await?;
    save_agent_skills_settings_in_db(&database, settings)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn get_osv_latest_scan_result(
    app: tauri::AppHandle,
    project_path: String,
) -> Result<Option<OsvScanResult>, String> {
    let database = initialize_app_database(&app).await?;
    get_osv_latest_scan_result_from_db(&database, project_path)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn save_osv_latest_scan_result(
    app: tauri::AppHandle,
    result: OsvScanResult,
) -> Result<(), String> {
    let database = initialize_app_database(&app).await?;
    save_osv_latest_scan_result_in_db(&database, result)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn delete_osv_latest_scan_result(
    app: tauri::AppHandle,
    project_path: String,
) -> Result<(), String> {
    let database = initialize_app_database(&app).await?;
    delete_osv_latest_scan_result_in_db(&database, project_path)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn check_osv_installed() -> Result<OsvInstallStatus, String> {
    check_osv_scanner_installed().map_err(|error| error.to_string())
}

#[tauri::command]
fn preview_osv_scan_command(request: OsvScanCommandRequest) -> Result<OsvCommandPreview, String> {
    build_scan_command(request).map_err(|error| error.to_string())
}

#[tauri::command]
fn diagnose_osv_project(
    request: OsvProjectDiagnosticRequest,
) -> Result<OsvProjectDiagnostic, String> {
    diagnose_project(request).map_err(|error| error.to_string())
}

#[tauri::command]
fn scan_osv_project(request: OsvScanRequest) -> Result<OsvScanResult, String> {
    scan_project(request).map_err(|error| error.to_string())
}

#[tauri::command]
fn preview_osv_report_export_command(
    request: OsvReportExportCommandRequest,
) -> Result<OsvCommandPreview, String> {
    build_export_command(request).map_err(|error| error.to_string())
}

#[tauri::command]
fn export_osv_report(request: OsvReportExportRequest) -> Result<OsvReportExportResult, String> {
    export_report(request).map_err(|error| error.to_string())
}

#[tauri::command]
fn ignore_osv_vulnerability(request: OsvIgnoreRequest) -> Result<OsvIgnoreResult, String> {
    ignore_vulnerability(
        &request.project_path,
        &request.vulnerability_id,
        &request.reason,
    )
    .map_err(|error| error.to_string())
}

#[tauri::command]
fn save_yaml_file(
    app: tauri::AppHandle,
    filename: String,
    content: String,
) -> Result<SaveYamlResponse, String> {
    if content.trim().is_empty() {
        return Err("没有可下载的 YAML 内容".to_string());
    }

    let download_dir = app
        .path()
        .download_dir()
        .map_err(|error| format!("无法定位下载目录: {error}"))?;
    let path = next_available_path(download_dir.join(sanitize_yaml_filename(&filename)));

    fs::write(&path, content).map_err(|error| format!("写入文件失败: {error}"))?;

    Ok(SaveYamlResponse {
        path: path.display().to_string(),
    })
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            convert_vless_to_mihomo,
            decode_finalshell_password,
            save_yaml_file,
            get_program_settings,
            save_program_settings,
            backup_program_database,
            compact_program_database,
            restore_program_database,
            open_program_database_directory,
            delete_legacy_program_database,
            clear_program_database_history,
            get_agent_execution_history,
            save_agent_execution_history_record,
            clear_agent_execution_history,
            get_agent_skills_settings,
            save_agent_skills_settings,
            get_osv_latest_scan_result,
            save_osv_latest_scan_result,
            delete_osv_latest_scan_result,
            get_vless_tool_settings,
            save_vless_tool_settings,
            get_osv_settings,
            save_osv_settings,
            check_osv_installed,
            preview_osv_scan_command,
            diagnose_osv_project,
            scan_osv_project,
            preview_osv_report_export_command,
            export_osv_report,
            ignore_osv_vulnerability,
            get_workbench_scripts,
            run_workbench_script
        ])
        .run(tauri::generate_context!())
        .expect("failed to run RustTool desktop app");
}

impl From<VlessTransitProxyRequest> for TransitProxyOptions {
    fn from(value: VlessTransitProxyRequest) -> Self {
        Self {
            provider_name: value.provider_name,
            provider_url: value.provider_url,
            provider_path: value.provider_path,
            group_name: value.group_name,
            group_type: match value.group_type.unwrap_or(VlessTransitGroupType::UrlTest) {
                VlessTransitGroupType::Select => TransitGroupType::Select,
                VlessTransitGroupType::UrlTest => TransitGroupType::UrlTest,
                VlessTransitGroupType::Fallback => TransitGroupType::Fallback,
            },
            bypass_domains: value.bypass_domains.unwrap_or_default(),
            providers: value
                .providers
                .unwrap_or_default()
                .into_iter()
                .map(Into::into)
                .collect(),
        }
    }
}

impl From<VlessTransitProviderRequest> for TransitProviderOptions {
    fn from(value: VlessTransitProviderRequest) -> Self {
        Self {
            provider_name: value.provider_name,
            provider_url: value.provider_url,
            provider_path: value.provider_path,
            group_name: value.group_name,
        }
    }
}

impl From<OsvProjectSettings> for OsvProjectRecord {
    fn from(value: OsvProjectSettings) -> Self {
        Self {
            name: value.name,
            path: value.path,
            last_scanned: value.last_scanned,
            health_score: value.health_score,
        }
    }
}

impl From<OsvProjectRecord> for OsvProjectSettings {
    fn from(value: OsvProjectRecord) -> Self {
        Self {
            name: value.name,
            path: value.path,
            last_scanned: value.last_scanned,
            health_score: value.health_score,
        }
    }
}

fn desktop_settings_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("无法定位应用数据目录: {error}"))?;
    fs::create_dir_all(&data_dir).map_err(|error| format!("创建应用数据目录失败: {error}"))?;
    Ok(data_dir.join("rusttool-settings.json"))
}

fn read_desktop_settings(app: &tauri::AppHandle) -> Result<DesktopSettings, String> {
    let path = desktop_settings_path(app)?;
    if !path.exists() {
        return Ok(DesktopSettings::default());
    }
    let content =
        fs::read_to_string(&path).map_err(|error| format!("读取桌面配置失败: {error}"))?;
    serde_json::from_str(&content).map_err(|error| format!("解析桌面配置失败: {error}"))
}

fn write_desktop_settings(
    app: &tauri::AppHandle,
    settings: &DesktopSettings,
) -> Result<(), String> {
    let path = desktop_settings_path(app)?;
    let json = serde_json::to_string_pretty(settings)
        .map_err(|error| format!("序列化桌面配置失败: {error}"))?;
    fs::write(path, json).map_err(|error| format!("保存桌面配置失败: {error}"))
}

async fn program_settings_response(
    app: &tauri::AppHandle,
    settings: ProgramSettings,
) -> Result<ProgramSettingsResponse, String> {
    let default_database_path = default_database_path(app)?;
    let effective_database_path = if settings.database_path.trim().is_empty() {
        default_database_path.clone()
    } else {
        settings.database_path.clone()
    };
    let database_health = check_database_health(&effective_database_path).await;
    let database_stats = database_file_stats(&effective_database_path)
        .await
        .unwrap_or_else(|_| empty_database_file_stats(&effective_database_path));
    let database_diagnostics = database_diagnostics_for_path(&effective_database_path).await;
    let legacy_database = legacy_database_info(app).await?;

    Ok(ProgramSettingsResponse {
        settings,
        default_database_path,
        effective_database_path,
        database_health,
        database_stats,
        database_diagnostics,
        legacy_database,
    })
}

async fn initialize_app_database(app: &tauri::AppHandle) -> Result<StorageDatabase, String> {
    initialize_database(effective_database_path(app)?)
        .await
        .map_err(|error| error.to_string())
}

async fn migrate_legacy_osv_settings_if_needed(
    database: &StorageDatabase,
    legacy_settings: &OsvScannerSettings,
) -> Result<(), String> {
    let stored_projects = list_osv_projects(database)
        .await
        .map_err(|error| error.to_string())?;
    let stored_history = list_osv_command_history(database, OSV_COMMAND_HISTORY_LIMIT)
        .await
        .map_err(|error| error.to_string())?;

    if !stored_projects.is_empty() || !stored_history.is_empty() {
        return Ok(());
    }
    if legacy_settings.projects.is_empty() && legacy_settings.command_history.is_empty() {
        return Ok(());
    }

    replace_osv_projects(
        database,
        legacy_settings
            .projects
            .clone()
            .into_iter()
            .map(Into::into)
            .collect(),
    )
    .await
    .map_err(|error| error.to_string())?;
    replace_osv_command_history(
        database,
        legacy_settings.command_history.clone(),
        OSV_COMMAND_HISTORY_LIMIT,
    )
    .await
    .map_err(|error| error.to_string())
}

async fn compose_osv_settings_from_storage(
    database: &StorageDatabase,
    auto_scan_schedule: String,
) -> Result<OsvScannerSettings, String> {
    let projects = list_osv_projects(database)
        .await
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(Into::into)
        .collect();
    let command_history = list_osv_command_history(database, OSV_COMMAND_HISTORY_LIMIT)
        .await
        .map_err(|error| error.to_string())?;

    Ok(OsvScannerSettings {
        projects,
        auto_scan_schedule,
        command_history,
    })
}

fn effective_database_path(app: &tauri::AppHandle) -> Result<String, String> {
    let settings = read_desktop_settings(app)?.program;
    let default_database_path = default_database_path(app)?;
    if settings.database_path.trim().is_empty() {
        Ok(default_database_path)
    } else {
        Ok(settings.database_path.trim().to_string())
    }
}

fn default_database_path(app: &tauri::AppHandle) -> Result<String, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("无法定位应用数据目录: {error}"))?;
    fs::create_dir_all(&data_dir).map_err(|error| format!("创建应用数据目录失败: {error}"))?;
    Ok(data_dir
        .join(DEFAULT_DATABASE_FILE_NAME)
        .display()
        .to_string())
}

fn default_database_backup_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("无法定位应用数据目录: {error}"))?;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("生成备份文件名失败: {error}"))?
        .as_secs();

    Ok(data_dir
        .join("backups")
        .join(format!("rusttool-backup-{timestamp}.db")))
}

fn default_database_restore_backup_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("无法定位应用数据目录: {error}"))?;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("生成恢复前备份文件名失败: {error}"))?
        .as_secs();

    Ok(data_dir
        .join("backups")
        .join(format!("rusttool-before-restore-{timestamp}.db")))
}

fn effective_database_directory(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let database_path = PathBuf::from(effective_database_path(app)?);
    Ok(database_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from(".")))
}

fn legacy_database_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("无法定位应用数据目录: {error}"))?;
    Ok(data_dir.join(LEGACY_DATABASE_FILE_NAME))
}

async fn legacy_database_info(app: &tauri::AppHandle) -> Result<LegacyDatabaseInfo, String> {
    let path = legacy_database_path(app)?;
    let stats = database_file_stats(&path)
        .await
        .unwrap_or_else(|_| empty_database_file_stats(&path.display().to_string()));
    let exists = stats.total_size_bytes > 0;

    Ok(LegacyDatabaseInfo {
        path: path.display().to_string(),
        exists,
        main_file_size_bytes: stats.main_file_size_bytes,
        wal_file_size_bytes: stats.wal_file_size_bytes,
        shm_file_size_bytes: stats.shm_file_size_bytes,
        total_size_bytes: stats.total_size_bytes,
    })
}

async fn database_diagnostics_for_path(database_path: &str) -> DatabaseStorageDiagnostics {
    let Ok(database) = initialize_database(database_path).await else {
        return empty_database_storage_diagnostics();
    };

    database_storage_diagnostics(&database)
        .await
        .unwrap_or_else(|_| empty_database_storage_diagnostics())
}

fn empty_database_file_stats(database_path: &str) -> DatabaseFileStats {
    DatabaseFileStats {
        database_path: database_path.to_string(),
        main_file_size_bytes: 0,
        wal_file_size_bytes: 0,
        shm_file_size_bytes: 0,
        total_size_bytes: 0,
    }
}

fn empty_database_storage_diagnostics() -> DatabaseStorageDiagnostics {
    DatabaseStorageDiagnostics {
        total_records: 0,
        record_counts: Vec::new(),
    }
}

fn delete_legacy_database_files(app: &tauri::AppHandle) -> Result<(), String> {
    let legacy_path = legacy_database_path(app)?;
    for path in [
        legacy_path.clone(),
        PathBuf::from(format!("{}-wal", legacy_path.display())),
        PathBuf::from(format!("{}-shm", legacy_path.display())),
    ] {
        match fs::remove_file(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(format!("删除旧数据库文件失败 {}: {error}", path.display())),
        }
    }

    Ok(())
}

fn open_path_in_file_manager(path: &Path) -> Result<(), String> {
    let status = if cfg!(target_os = "macos") {
        Command::new("open").arg(path).status()
    } else if cfg!(target_os = "windows") {
        Command::new("explorer").arg(path).status()
    } else {
        Command::new("xdg-open").arg(path).status()
    }
    .map_err(|error| format!("打开数据库目录失败: {error}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("打开数据库目录失败，退出码: {status}"))
    }
}

fn normalize_program_settings(settings: ProgramSettings) -> ProgramSettings {
    ProgramSettings {
        database_path: settings.database_path.trim().to_string(),
    }
}

fn trim_osv_command_history(history: &mut Vec<OsvCommandExecutionRecord>) {
    if history.len() > OSV_COMMAND_HISTORY_LIMIT {
        let keep_from = history.len() - OSV_COMMAND_HISTORY_LIMIT;
        history.drain(0..keep_from);
    }
}

fn sanitize_yaml_filename(filename: &str) -> String {
    let trimmed = filename.trim();
    let raw_name = if trimmed.is_empty() {
        "mihomo"
    } else {
        trimmed
    };
    let safe_name = raw_name
        .chars()
        .map(|character| match character {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            character if character.is_control() => '_',
            character => character,
        })
        .collect::<String>()
        .trim_matches([' ', '.'])
        .to_string();
    let safe_name = if safe_name.is_empty() {
        "mihomo".to_string()
    } else {
        safe_name
    };

    if safe_name.to_ascii_lowercase().ends_with(".yaml")
        || safe_name.to_ascii_lowercase().ends_with(".yml")
    {
        safe_name
    } else {
        format!("{safe_name}.yaml")
    }
}

fn next_available_path(path: PathBuf) -> PathBuf {
    if !path.exists() {
        return path;
    }

    let parent = path.parent().map(Path::to_path_buf).unwrap_or_default();
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("mihomo");
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("yaml");

    for index in 1.. {
        let candidate = parent.join(format!("{stem} ({index}).{extension}"));
        if !candidate.exists() {
            return candidate;
        }
    }

    unreachable!("infinite iterator always returns")
}
