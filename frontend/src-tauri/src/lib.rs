use rust_tool_core::{
    convert_vless_to_yaml, ConvertOptions, OutputMode, TemplateMode, TransitGroupType,
    TransitProviderOptions, TransitProxyOptions,
};
use rust_tool_core::workbench::{execute_script, list_scripts, ExecutionResult, ScriptInfo};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::Manager;

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
struct DesktopSettings {
    vless_to_mihomo: VlessToolSettings,
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
            save_yaml_file,
            get_vless_tool_settings,
            save_vless_tool_settings,
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
