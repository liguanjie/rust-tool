use rusqlite::{params, Connection, OptionalExtension};
use rust_tool_core::{
    convert_vless_to_yaml, ConvertOptions, OutputMode, TemplateMode, TransitGroupType,
    TransitProviderOptions, TransitProxyOptions,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::Manager;

mod workbench;

const VLESS_TOOL_SETTINGS_KEY: &str = "toolbox.vless_to_mihomo.settings";

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
fn get_vless_tool_settings(app: tauri::AppHandle) -> Result<VlessToolSettings, String> {
    let conn = open_app_db(&app)?;
    ensure_app_settings_schema(&conn)?;
    let value = conn
        .query_row(
            "SELECT value FROM app_settings WHERE key = ?1",
            params![VLESS_TOOL_SETTINGS_KEY],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| format!("读取 VLESS 工具配置失败: {error}"))?;

    match value {
        Some(value) => serde_json::from_str(&value)
            .map_err(|error| format!("解析 VLESS 工具配置失败: {error}")),
        None => Ok(VlessToolSettings::default()),
    }
}

#[tauri::command]
fn save_vless_tool_settings(
    app: tauri::AppHandle,
    settings: VlessToolSettings,
) -> Result<VlessToolSettings, String> {
    let conn = open_app_db(&app)?;
    ensure_app_settings_schema(&conn)?;
    let value = serde_json::to_string(&settings)
        .map_err(|error| format!("序列化 VLESS 工具配置失败: {error}"))?;
    conn.execute(
        "INSERT INTO app_settings (key, value, updated_at)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
        params![VLESS_TOOL_SETTINGS_KEY, value, now_text()],
    )
    .map_err(|error| format!("保存 VLESS 工具配置失败: {error}"))?;

    Ok(settings)
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

#[tauri::command]
async fn get_workbench_config(app: tauri::AppHandle) -> Result<workbench::WorkbenchConfig, String> {
    tauri::async_runtime::spawn_blocking(move || workbench::get_config(&app))
        .await
        .map_err(|error| format!("异步执行失败: {error}"))?
}

#[tauri::command]
fn get_workbench_platform() -> workbench::WorkbenchPlatform {
    workbench::get_platform()
}

#[tauri::command]
async fn save_workbench_config(
    app: tauri::AppHandle,
    config: workbench::WorkbenchConfig,
) -> Result<workbench::WorkbenchConfig, String> {
    tauri::async_runtime::spawn_blocking(move || workbench::save_config(&app, config))
        .await
        .map_err(|error| format!("异步执行失败: {error}"))?
}

#[tauri::command]
async fn detect_docker(app: tauri::AppHandle) -> Result<workbench::DockerDetection, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let detection = workbench::detect_docker();
        let found =
            !detection.docker_desktop_path.is_empty() || !detection.docker_cli_path.is_empty();
        workbench::record_operation(
            &app,
            "Docker",
            "自动侦测",
            if found { "success" } else { "warn" },
            if found {
                "已自动侦测 Docker 路径"
            } else {
                "未自动侦测到 Docker 路径"
            },
            "",
        );
        detection
    })
    .await
    .map_err(|error| format!("异步执行失败: {error}"))
}

#[tauri::command]
async fn detect_clash_party(
    app: tauri::AppHandle,
) -> Result<workbench::ClashPartyDetection, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let detection = workbench::detect_clash_party();
        let found =
            !detection.clash_party_path.is_empty() || !detection.clash_party_data_dir.is_empty();
        workbench::record_operation(
            &app,
            "Clash Party",
            "自动侦测",
            if found { "success" } else { "warn" },
            if found {
                "已自动侦测 Clash Party 配置"
            } else {
                "未自动侦测到 Clash Party 配置"
            },
            "",
        );
        detection
    })
    .await
    .map_err(|error| format!("异步执行失败: {error}"))
}

#[tauri::command]
async fn select_workbench_file(
    app: tauri::AppHandle,
    kind: workbench::WorkbenchPathKind,
) -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let result = workbench::select_workbench_file(kind);
        match &result {
            Ok(Some(path)) => workbench::record_operation(
                &app,
                "工作台",
                "选择文件",
                "success",
                "已选择本地文件",
                path,
            ),
            Ok(None) => workbench::record_operation(
                &app,
                "工作台",
                "选择文件",
                "cancelled",
                "已取消选择文件",
                "",
            ),
            Err(error) => workbench::record_operation(
                &app,
                "工作台",
                "选择文件",
                "failed",
                "选择文件失败",
                error,
            ),
        }
        result
    })
    .await
    .map_err(|error| format!("异步执行失败: {error}"))?
}

#[tauri::command]
async fn select_workbench_directory(app: tauri::AppHandle) -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let result = workbench::select_workbench_directory();
        match &result {
            Ok(Some(path)) => workbench::record_operation(
                &app,
                "工作台",
                "选择目录",
                "success",
                "已选择本地目录",
                path,
            ),
            Ok(None) => workbench::record_operation(
                &app,
                "工作台",
                "选择目录",
                "cancelled",
                "已取消选择目录",
                "",
            ),
            Err(error) => workbench::record_operation(
                &app,
                "工作台",
                "选择目录",
                "failed",
                "选择目录失败",
                error,
            ),
        }
        result
    })
    .await
    .map_err(|error| format!("异步执行失败: {error}"))?
}

#[tauri::command]
async fn get_docker_status(app: tauri::AppHandle) -> Result<workbench::DockerStatus, String> {
    tauri::async_runtime::spawn_blocking(move || workbench::get_docker_status(&app))
        .await
        .map_err(|error| format!("异步执行失败: {error}"))?
}

#[tauri::command]
async fn start_docker(app: tauri::AppHandle) -> Result<workbench::TaskRun, String> {
    tauri::async_runtime::spawn_blocking(move || workbench::start_docker(&app))
        .await
        .map_err(|error| format!("异步执行失败: {error}"))?
}

#[tauri::command]
async fn stop_docker(app: tauri::AppHandle) -> Result<workbench::TaskRun, String> {
    tauri::async_runtime::spawn_blocking(move || workbench::stop_docker(&app))
        .await
        .map_err(|error| format!("异步执行失败: {error}"))?
}

#[tauri::command]
async fn restart_docker(app: tauri::AppHandle) -> Result<workbench::TaskRun, String> {
    tauri::async_runtime::spawn_blocking(move || workbench::restart_docker(&app))
        .await
        .map_err(|error| format!("异步执行失败: {error}"))?
}

#[tauri::command]
async fn get_clash_party_status(
    app: tauri::AppHandle,
) -> Result<workbench::ClashPartyStatus, String> {
    tauri::async_runtime::spawn_blocking(move || workbench::get_clash_party_status(&app))
        .await
        .map_err(|error| format!("异步执行失败: {error}"))?
}

#[tauri::command]
async fn start_clash_party(app: tauri::AppHandle) -> Result<workbench::TaskRun, String> {
    tauri::async_runtime::spawn_blocking(move || workbench::start_clash_party(&app))
        .await
        .map_err(|error| format!("异步执行失败: {error}"))?
}

#[tauri::command]
async fn stop_clash_party(app: tauri::AppHandle) -> Result<workbench::TaskRun, String> {
    tauri::async_runtime::spawn_blocking(move || workbench::stop_clash_party(&app))
        .await
        .map_err(|error| format!("异步执行失败: {error}"))?
}

#[tauri::command]
async fn get_clash_party_manager_state(
    app: tauri::AppHandle,
) -> Result<workbench::ClashPartyManagerState, String> {
    tauri::async_runtime::spawn_blocking(move || workbench::get_clash_party_manager_state(&app))
        .await
        .map_err(|error| format!("异步执行失败: {error}"))?
}

#[tauri::command]
async fn switch_clash_party_subscription(
    app: tauri::AppHandle,
    subscription_id: String,
) -> Result<workbench::ClashPartySwitchResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        workbench::switch_clash_party_subscription(&app, subscription_id)
    })
    .await
    .map_err(|error| format!("异步执行失败: {error}"))?
}

#[tauri::command]
async fn switch_clash_party_node(
    app: tauri::AppHandle,
    group_name: String,
    node_name: String,
) -> Result<workbench::ClashPartySwitchResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        workbench::switch_clash_party_node(&app, group_name, node_name)
    })
    .await
    .map_err(|error| format!("异步执行失败: {error}"))?
}

#[tauri::command]
async fn check_clash_party_node(
    app: tauri::AppHandle,
    node_name: String,
) -> Result<workbench::ClashPartyNodeCheckResult, String> {
    tauri::async_runtime::spawn_blocking(move || workbench::check_clash_party_node(&app, node_name))
        .await
        .map_err(|error| format!("异步执行失败: {error}"))?
}

#[tauri::command]
async fn shutdown_windows(app: tauri::AppHandle) -> Result<workbench::TaskRun, String> {
    tauri::async_runtime::spawn_blocking(move || workbench::shutdown_windows(&app))
        .await
        .map_err(|error| format!("异步执行失败: {error}"))?
}

#[tauri::command]
async fn shutdown_system(app: tauri::AppHandle) -> Result<workbench::TaskRun, String> {
    tauri::async_runtime::spawn_blocking(move || workbench::shutdown_system(&app))
        .await
        .map_err(|error| format!("异步执行失败: {error}"))?
}

#[tauri::command]
async fn run_sub2api_task(
    app: tauri::AppHandle,
    task: workbench::Sub2apiTask,
) -> Result<workbench::TaskRun, String> {
    tauri::async_runtime::spawn_blocking(move || workbench::run_sub2api_task(&app, task))
        .await
        .map_err(|error| format!("异步执行失败: {error}"))?
}

#[tauri::command]
async fn check_sub2api_health(app: tauri::AppHandle) -> Result<workbench::HealthStatus, String> {
    tauri::async_runtime::spawn_blocking(move || workbench::check_sub2api_health(&app))
        .await
        .map_err(|error| format!("异步执行失败: {error}"))?
}

#[tauri::command]
async fn list_task_runs(
    app: tauri::AppHandle,
    limit: Option<u32>,
) -> Result<Vec<workbench::TaskRun>, String> {
    tauri::async_runtime::spawn_blocking(move || workbench::list_task_runs(&app, limit))
        .await
        .map_err(|error| format!("异步执行失败: {error}"))?
}

#[tauri::command]
async fn list_operation_logs(
    app: tauri::AppHandle,
    page: Option<u32>,
    query: Option<String>,
) -> Result<workbench::OperationLogPage, String> {
    tauri::async_runtime::spawn_blocking(move || workbench::list_operation_logs(&app, page, query))
        .await
        .map_err(|error| format!("异步执行失败: {error}"))?
}

#[tauri::command]
async fn clear_operation_logs(app: tauri::AppHandle) -> Result<u64, String> {
    tauri::async_runtime::spawn_blocking(move || workbench::clear_operation_logs(&app))
        .await
        .map_err(|error| format!("异步执行失败: {error}"))?
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            convert_vless_to_mihomo,
            save_yaml_file,
            get_vless_tool_settings,
            save_vless_tool_settings,
            get_workbench_config,
            get_workbench_platform,
            save_workbench_config,
            detect_docker,
            detect_clash_party,
            select_workbench_file,
            select_workbench_directory,
            get_docker_status,
            start_docker,
            stop_docker,
            restart_docker,
            get_clash_party_status,
            start_clash_party,
            stop_clash_party,
            get_clash_party_manager_state,
            switch_clash_party_subscription,
            switch_clash_party_node,
            check_clash_party_node,
            shutdown_windows,
            shutdown_system,
            run_sub2api_task,
            check_sub2api_health,
            list_task_runs,
            list_operation_logs,
            clear_operation_logs
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

fn open_app_db(app: &tauri::AppHandle) -> Result<Connection, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("无法定位应用数据目录: {error}"))?;
    fs::create_dir_all(&data_dir).map_err(|error| format!("创建应用数据目录失败: {error}"))?;
    Connection::open(data_dir.join("rusttool.sqlite"))
        .map_err(|error| format!("打开本地数据库失败: {error}"))
}

fn ensure_app_settings_schema(conn: &Connection) -> Result<(), String> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS app_settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )",
        [],
    )
    .map_err(|error| format!("初始化配置表失败: {error}"))?;

    Ok(())
}

fn now_text() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
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
