use rust_tool_core::{convert_vless_to_yaml, ConvertOptions, OutputMode, TemplateMode};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::Manager;

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
struct ConvertVlessRequest {
    input: String,
    mode: Option<VlessOutputMode>,
    template: Option<VlessTemplateMode>,
    proxy_name: Option<String>,
}

#[derive(Debug, Serialize)]
struct ConvertVlessResponse {
    yaml: String,
}

#[derive(Debug, Serialize)]
struct SaveYamlResponse {
    path: String,
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
        },
    )
    .map(|yaml| ConvertVlessResponse { yaml })
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
        .invoke_handler(tauri::generate_handler![
            convert_vless_to_mihomo,
            save_yaml_file
        ])
        .run(tauri::generate_context!())
        .expect("failed to run RustTool desktop app");
}

fn sanitize_yaml_filename(filename: &str) -> String {
    let trimmed = filename.trim();
    let raw_name = if trimmed.is_empty() { "mihomo" } else { trimmed };
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
