use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use serde_yaml::Value as YamlValue;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{env, fs};
use tauri::{AppHandle, Manager};

const SETTINGS_KEY: &str = "windows_workbench.config";
const OPERATION_LOG_RETENTION_SECONDS: u64 = 7 * 24 * 60 * 60;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct WorkbenchConfig {
    pub docker_desktop_path: String,
    pub docker_cli_path: String,
    pub clash_party_path: String,
    pub clash_party_data_dir: String,
    pub clash_party_api_url: String,
    pub clash_party_api_secret: String,
    pub sub2api_start_script: String,
    pub sub2api_stop_script: String,
    pub sub2api_upgrade_script: String,
    pub sub2api_working_dir: String,
    pub sub2api_health_url: String,
    pub sub2api_login_url: String,
    pub sub2api_username: String,
    pub sub2api_password: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DockerDetection {
    pub docker_desktop_path: String,
    pub docker_cli_path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DockerStatus {
    pub desktop_configured: bool,
    pub cli_configured: bool,
    pub desktop_running: bool,
    pub cli_available: bool,
    pub engine_running: bool,
    pub version: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClashPartyDetection {
    pub clash_party_path: String,
    pub clash_party_data_dir: String,
    pub clash_party_api_url: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClashPartyStatus {
    pub configured: bool,
    pub running: bool,
    pub path: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClashPartySubscription {
    pub id: String,
    pub name: String,
    pub profile_type: String,
    pub path: String,
    pub active: bool,
    pub node_count: usize,
    pub group_count: usize,
    pub updated_at: String,
    pub used_bytes: Option<u64>,
    pub total_bytes: Option<u64>,
    pub expire_at: Option<u64>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ClashPartyNode {
    pub name: String,
    pub node_type: String,
    pub server: String,
    pub port: Option<u16>,
    pub delay: Option<i64>,
    pub active: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClashPartyProxyGroup {
    pub name: String,
    pub group_type: String,
    pub selected: String,
    pub nodes: Vec<ClashPartyNode>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClashPartyManagerState {
    pub data_dir: String,
    pub profile_index_path: String,
    pub api_url: String,
    pub active_subscription_id: String,
    pub subscriptions: Vec<ClashPartySubscription>,
    pub groups: Vec<ClashPartyProxyGroup>,
    pub api_available: bool,
    pub message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClashPartySwitchResult {
    pub ok: bool,
    pub message: String,
}

#[derive(Debug, Deserialize)]
struct ClashProfileIndex {
    #[serde(default)]
    items: Vec<ClashProfileItem>,
    #[serde(default)]
    current: String,
}

#[derive(Debug, Deserialize)]
struct ClashProfileItem {
    id: String,
    name: String,
    #[serde(default, rename = "type")]
    profile_type: String,
    #[serde(default)]
    updated: Option<i64>,
    #[serde(default)]
    extra: Option<ClashProfileExtra>,
}

#[derive(Debug, Deserialize)]
struct ClashProfileExtra {
    #[serde(default)]
    upload: Option<u64>,
    #[serde(default)]
    download: Option<u64>,
    #[serde(default)]
    total: Option<u64>,
    #[serde(default)]
    expire: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Sub2apiTask {
    Start,
    Stop,
    Upgrade,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkbenchPathKind {
    Executable,
    Script,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskRun {
    pub id: i64,
    pub task_key: String,
    pub status: String,
    pub started_at: String,
    pub finished_at: String,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthStatus {
    pub ok: bool,
    pub message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationLog {
    pub id: i64,
    pub module: String,
    pub action: String,
    pub status: String,
    pub message: String,
    pub detail: String,
    pub created_at: String,
}

pub fn get_config(app: &AppHandle) -> Result<WorkbenchConfig, String> {
    let conn = open_db(app)?;
    ensure_schema(&conn)?;
    read_config(&conn)
}

pub fn save_config(
    app: &AppHandle,
    mut config: WorkbenchConfig,
) -> Result<WorkbenchConfig, String> {
    let result: Result<WorkbenchConfig, String> = (|| {
        normalize_config_defaults(&mut config);
        validate_optional_executable(&config.docker_desktop_path, "Docker Desktop 路径")?;
        validate_optional_executable(&config.docker_cli_path, "docker CLI 路径")?;
        validate_optional_executable(&config.clash_party_path, "Clash Party 路径")?;
        validate_optional_directory(&config.clash_party_data_dir, "Clash Party 数据目录")?;
        validate_optional_script(&config.sub2api_start_script, "sub2api 启动脚本")?;
        validate_optional_script(&config.sub2api_stop_script, "sub2api 停止脚本")?;
        validate_optional_script(&config.sub2api_upgrade_script, "sub2api 升级脚本")?;
        validate_optional_directory(&config.sub2api_working_dir, "sub2api 工作目录")?;

        let conn = open_db(app)?;
        ensure_schema(&conn)?;
        let value =
            serde_json::to_string(&config).map_err(|error| format!("配置序列化失败: {error}"))?;
        conn.execute(
            "INSERT INTO app_settings (key, value, updated_at)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
            params![SETTINGS_KEY, value, now_text()],
        )
        .map_err(|error| format!("保存配置失败: {error}"))?;

        Ok(config)
    })();

    match &result {
        Ok(_) => log_operation(app, "工作台", "保存配置", "success", "工作台配置已保存", ""),
        Err(error) => log_operation(
            app,
            "工作台",
            "保存配置",
            "failed",
            "工作台配置保存失败",
            error,
        ),
    }

    result
}

pub fn detect_docker() -> DockerDetection {
    DockerDetection {
        docker_desktop_path: first_existing_path(&[
            r"C:\Program Files\Docker\Docker\Docker Desktop.exe",
        ])
        .unwrap_or_default(),
        docker_cli_path: first_existing_path(&[
            r"C:\Program Files\Docker\Docker\resources\bin\docker.exe",
        ])
        .or_else(find_docker_in_path)
        .unwrap_or_default(),
    }
}

pub fn detect_clash_party() -> ClashPartyDetection {
    ClashPartyDetection {
        clash_party_path: detect_clash_party_path().unwrap_or_default(),
        clash_party_data_dir: detect_clash_party_data_dir().unwrap_or_default(),
        clash_party_api_url: default_clash_party_api_url(),
    }
}

pub fn select_workbench_file(kind: WorkbenchPathKind) -> Result<Option<String>, String> {
    let (title, filter) = match kind {
        WorkbenchPathKind::Executable => {
            ("选择程序文件", "程序文件 (*.exe)|*.exe|所有文件 (*.*)|*.*")
        }
        WorkbenchPathKind::Script => (
            "选择脚本或程序",
            "脚本或程序 (*.bat;*.cmd;*.ps1;*.exe)|*.bat;*.cmd;*.ps1;*.exe|所有文件 (*.*)|*.*",
        ),
    };
    let script = format!(
        "Add-Type -AssemblyName System.Windows.Forms; \
         $dialog = New-Object System.Windows.Forms.OpenFileDialog; \
         $dialog.Title = '{}'; \
         $dialog.Filter = '{}'; \
         if ($dialog.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {{ \
           [Console]::Out.Write($dialog.FileName) \
         }}",
        escape_powershell_single_quoted(title),
        escape_powershell_single_quoted(filter),
    );

    run_selection_dialog(&script)
}

pub fn select_workbench_directory() -> Result<Option<String>, String> {
    let script = "Add-Type -AssemblyName System.Windows.Forms; \
         $dialog = New-Object System.Windows.Forms.FolderBrowserDialog; \
         $dialog.Description = '选择工作目录'; \
         $dialog.ShowNewFolderButton = $false; \
         if ($dialog.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) { \
           [Console]::Out.Write($dialog.SelectedPath) \
         }";

    run_selection_dialog(script)
}

pub fn get_docker_status(app: &AppHandle) -> Result<DockerStatus, String> {
    let result: Result<DockerStatus, String> = (|| {
        let config = get_config(app)?;
        let cli_configured = !config.docker_cli_path.trim().is_empty();
        let desktop_configured = !config.docker_desktop_path.trim().is_empty();
        let desktop_running = is_docker_desktop_running();

        if !cli_configured {
            return Ok(DockerStatus {
                desktop_configured,
                cli_configured,
                desktop_running,
                cli_available: false,
                engine_running: false,
                version: String::new(),
                message: "未配置 docker CLI 路径".to_string(),
            });
        }

        let output = Command::new(&config.docker_cli_path)
            .arg("--version")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|error| format!("执行 docker --version 失败: {error}"))?;

        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let cli_available = output.status.success();

        if !cli_available {
            let message = if stderr.is_empty() {
                "docker CLI 检测失败".to_string()
            } else {
                stderr
            };
            return Ok(DockerStatus {
                desktop_configured,
                cli_configured,
                desktop_running,
                cli_available,
                engine_running: false,
                version: stdout,
                message,
            });
        }

        let engine_output = Command::new(&config.docker_cli_path)
            .args(["info", "--format", "{{.ServerVersion}}"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|error| format!("执行 docker info 失败: {error}"))?;

        let engine_stdout = String::from_utf8_lossy(&engine_output.stdout)
            .trim()
            .to_string();
        let engine_stderr = String::from_utf8_lossy(&engine_output.stderr)
            .trim()
            .to_string();
        let engine_running = engine_output.status.success();
        let version = if engine_running && !engine_stdout.is_empty() {
            format!("{stdout}; Engine {engine_stdout}")
        } else {
            stdout
        };
        let message = if engine_running {
            if engine_stdout.is_empty() {
                "Docker Engine 运行中".to_string()
            } else {
                format!("Docker Engine 运行中: {engine_stdout}")
            }
        } else {
            let detail = if !engine_stderr.is_empty() {
                engine_stderr
            } else if !engine_stdout.is_empty() {
                engine_stdout
            } else {
                "docker info 未返回可用状态".to_string()
            };
            if desktop_running {
                format!("Docker Desktop 已启动，但 Engine 尚未就绪: {detail}")
            } else {
                format!("Docker Engine 未运行: {detail}")
            }
        };

        Ok(DockerStatus {
            desktop_configured,
            cli_configured,
            desktop_running,
            cli_available,
            engine_running,
            version,
            message,
        })
    })();

    match &result {
        Ok(status) => log_operation(
            app,
            "Docker",
            "检测",
            if status.engine_running {
                "success"
            } else {
                "warn"
            },
            &status.message,
            &format!(
                "desktop_running={}, cli_available={}, version={}",
                status.desktop_running, status.cli_available, status.version
            ),
        ),
        Err(error) => log_operation(app, "Docker", "检测", "failed", "Docker 检测失败", error),
    }

    result
}

pub fn start_docker(app: &AppHandle) -> Result<TaskRun, String> {
    let config = get_config(app)?;
    validate_required_executable(&config.docker_desktop_path, "Docker Desktop 路径")?;
    spawn_process(
        app,
        "docker.start",
        &config.docker_desktop_path,
        &[],
        None,
        false,
    )
}

pub fn stop_docker(app: &AppHandle) -> Result<TaskRun, String> {
    let script = format!(
        "{}\nWrite-Output 'Docker stop command sent.'",
        docker_stop_commands()
    );
    let args = [
        "-NoProfile",
        "-ExecutionPolicy",
        "Bypass",
        "-Command",
        script.as_str(),
    ];
    run_process(app, "docker.stop", "powershell", &args, None, false)
}

pub fn restart_docker(app: &AppHandle) -> Result<TaskRun, String> {
    let config = get_config(app)?;
    validate_required_executable(&config.docker_desktop_path, "Docker Desktop 路径")?;
    let script = format!(
        "{}\nStart-Sleep -Seconds 2\nStart-Process -FilePath {}\nWrite-Output 'Docker restart command sent.'",
        docker_stop_commands(),
        powershell_single_quoted(&config.docker_desktop_path)
    );
    let args = [
        "-NoProfile",
        "-ExecutionPolicy",
        "Bypass",
        "-Command",
        script.as_str(),
    ];
    run_process(app, "docker.restart", "powershell", &args, None, false)
}

pub fn get_clash_party_status(app: &AppHandle) -> Result<ClashPartyStatus, String> {
    let result: Result<ClashPartyStatus, String> = (|| {
        let config = get_config(app)?;
        let path = config.clash_party_path.trim().to_string();
        let configured = !path.is_empty();
        let running = is_clash_party_running();
        let message = if running {
            "Clash Party 正在运行".to_string()
        } else if configured {
            "Clash Party 未运行".to_string()
        } else {
            "未配置 Clash Party 程序路径".to_string()
        };

        Ok(ClashPartyStatus {
            configured,
            running,
            path,
            message,
        })
    })();

    match &result {
        Ok(status) => log_operation(
            app,
            "Clash Party",
            "检测",
            if status.running { "success" } else { "warn" },
            &status.message,
            &status.path,
        ),
        Err(error) => log_operation(
            app,
            "Clash Party",
            "检测",
            "failed",
            "Clash Party 检测失败",
            error,
        ),
    }

    result
}

pub fn start_clash_party(app: &AppHandle) -> Result<TaskRun, String> {
    let config = get_config(app)?;
    validate_required_executable(&config.clash_party_path, "Clash Party 路径")?;
    spawn_process(
        app,
        "clash_party.start",
        &config.clash_party_path,
        &[],
        None,
        false,
    )
}

pub fn stop_clash_party(app: &AppHandle) -> Result<TaskRun, String> {
    let script = format!(
        "{}\nWrite-Output 'Clash Party exit command sent.'",
        clash_party_stop_commands()
    );
    let args = [
        "-NoProfile",
        "-ExecutionPolicy",
        "Bypass",
        "-Command",
        script.as_str(),
    ];
    run_process(app, "clash_party.stop", "powershell", &args, None, false)
}

pub fn get_clash_party_manager_state(app: &AppHandle) -> Result<ClashPartyManagerState, String> {
    let result: Result<ClashPartyManagerState, String> = (|| {
        let config = get_config(app)?;
        let data_dir = resolve_clash_party_data_dir(&config)?;
        let profile_index_path = data_dir.join("profile.yaml");
        let profile_index = read_clash_profile_index(&profile_index_path)?;
        let subscriptions = build_clash_subscriptions(&data_dir, &profile_index);
        let runtime_result = get_clash_party_runtime_groups(&config);
        let (groups, api_available, message) = match runtime_result {
            Ok(groups) => {
                let message = if groups.is_empty() {
                    "已读取订阅；Mihomo API 可访问，但未返回可切换代理组".to_string()
                } else {
                    format!(
                        "已读取 {} 个订阅和 {} 个运行时代理组",
                        subscriptions.len(),
                        groups.len()
                    )
                };
                (groups, true, message)
            }
            Err(error) => (
                Vec::new(),
                false,
                format!(
                    "已读取 {} 个订阅；运行时 API 未连接: {error}",
                    subscriptions.len()
                ),
            ),
        };

        Ok(ClashPartyManagerState {
            data_dir: data_dir.display().to_string(),
            profile_index_path: profile_index_path.display().to_string(),
            api_url: normalized_clash_party_api_url(&config),
            active_subscription_id: profile_index.current,
            subscriptions,
            groups,
            api_available,
            message,
        })
    })();

    match &result {
        Ok(state) => log_operation(
            app,
            "Clash Party",
            "刷新管理",
            if state.api_available {
                "success"
            } else {
                "warn"
            },
            &state.message,
            &format!(
                "subscriptions={}, groups={}, active_subscription={}",
                state.subscriptions.len(),
                state.groups.len(),
                state.active_subscription_id
            ),
        ),
        Err(error) => log_operation(
            app,
            "Clash Party",
            "刷新管理",
            "failed",
            "Clash Party 管理状态读取失败",
            error,
        ),
    }

    result
}

pub fn switch_clash_party_subscription(
    app: &AppHandle,
    subscription_id: String,
) -> Result<ClashPartySwitchResult, String> {
    let result: Result<ClashPartySwitchResult, String> = (|| {
        let config = get_config(app)?;
        let data_dir = resolve_clash_party_data_dir(&config)?;
        let profile_index_path = data_dir.join("profile.yaml");
        let profile_index = read_clash_profile_index(&profile_index_path)?;
        let exists = profile_index
            .items
            .iter()
            .any(|item| item.id == subscription_id);
        if !exists {
            return Err("订阅不存在，请先刷新订阅列表".to_string());
        }

        let profile_path = clash_profile_path(&data_dir, &subscription_id);
        if !profile_path.exists() {
            return Err(format!("订阅配置文件不存在: {}", profile_path.display()));
        }

        let body = serde_json::json!({
            "path": profile_path.display().to_string(),
        });
        let response = call_clash_party_api(&config, "PUT", "/configs", Some(body))?;
        Ok(ClashPartySwitchResult {
            ok: true,
            message: if response.is_empty() {
                format!("已请求切换订阅: {subscription_id}")
            } else {
                response
            },
        })
    })();

    match &result {
        Ok(result) => log_operation(
            app,
            "Clash Party",
            "切换订阅",
            "success",
            &result.message,
            &subscription_id,
        ),
        Err(error) => log_operation(
            app,
            "Clash Party",
            "切换订阅",
            "failed",
            "Clash Party 订阅切换失败",
            error,
        ),
    }

    result
}

pub fn switch_clash_party_node(
    app: &AppHandle,
    group_name: String,
    node_name: String,
) -> Result<ClashPartySwitchResult, String> {
    let result: Result<ClashPartySwitchResult, String> = (|| {
        if group_name.trim().is_empty() || node_name.trim().is_empty() {
            return Err("代理组和节点名称不能为空".to_string());
        }

        let config = get_config(app)?;
        let path = format!("/proxies/{}", encode_url_path_segment(&group_name));
        let body = serde_json::json!({
            "name": node_name,
        });
        let response = call_clash_party_api(&config, "PUT", &path, Some(body))?;
        Ok(ClashPartySwitchResult {
            ok: true,
            message: if response.is_empty() {
                format!("已请求将 {group_name} 切换到 {node_name}")
            } else {
                response
            },
        })
    })();

    match &result {
        Ok(result) => log_operation(
            app,
            "Clash Party",
            "切换节点",
            "success",
            &result.message,
            &format!("group={group_name}, node={node_name}"),
        ),
        Err(error) => log_operation(
            app,
            "Clash Party",
            "切换节点",
            "failed",
            "Clash Party 节点切换失败",
            error,
        ),
    }

    result
}

pub fn shutdown_windows(app: &AppHandle) -> Result<TaskRun, String> {
    run_process(
        app,
        "system.shutdown",
        "shutdown",
        &["/s", "/t", "10"],
        None,
        false,
    )
}

pub fn run_sub2api_task(app: &AppHandle, task: Sub2apiTask) -> Result<TaskRun, String> {
    let config = get_config(app)?;
    let (task_key, script) = match task {
        Sub2apiTask::Start => ("sub2api.start", config.sub2api_start_script),
        Sub2apiTask::Stop => ("sub2api.stop", config.sub2api_stop_script),
        Sub2apiTask::Upgrade => ("sub2api.upgrade", config.sub2api_upgrade_script),
    };
    validate_required_script(&script, task_key)?;
    let working_dir = if config.sub2api_working_dir.trim().is_empty() {
        Path::new(&script).parent().map(Path::to_path_buf)
    } else {
        Some(PathBuf::from(config.sub2api_working_dir))
    };

    if matches!(task, Sub2apiTask::Start) {
        spawn_process(app, task_key, &script, &[], working_dir.as_deref(), true)
    } else {
        run_process(app, task_key, &script, &[], working_dir.as_deref(), true)
    }
}

pub fn check_sub2api_health(app: &AppHandle) -> Result<HealthStatus, String> {
    let result: Result<HealthStatus, String> = (|| {
        let config = get_config(app)?;
        let url = config.sub2api_health_url.trim();
        if url.is_empty() {
            return Ok(HealthStatus {
                ok: false,
                message: "未配置健康检查地址".to_string(),
            });
        }

        let token = resolve_sub2api_token(&config)?;
        let script = build_sub2api_health_script(url, token.as_deref());
        let output = Command::new("powershell")
            .args(["-NoProfile", "-Command", &script])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|error| format!("健康检查失败: {error}"))?;

        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let ok = output.status.success() && stdout.starts_with('2');
        let message = if ok {
            format!("健康检查通过: HTTP {stdout}")
        } else if !stderr.is_empty() {
            stderr
        } else if !stdout.is_empty() {
            stdout
        } else {
            "健康检查未通过".to_string()
        };

        Ok(HealthStatus { ok, message })
    })();

    match &result {
        Ok(status) => log_operation(
            app,
            "sub2api",
            "健康检查",
            if status.ok { "success" } else { "warn" },
            &status.message,
            "",
        ),
        Err(error) => log_operation(
            app,
            "sub2api",
            "健康检查",
            "failed",
            "sub2api 健康检查失败",
            error,
        ),
    }

    result
}

fn resolve_sub2api_token(config: &WorkbenchConfig) -> Result<Option<String>, String> {
    let username = config.sub2api_username.trim();
    let password = config.sub2api_password.trim();
    if username.is_empty() && password.is_empty() {
        return Ok(None);
    }
    if username.is_empty() || password.is_empty() {
        return Err("sub2api 账号和密码需要同时填写".to_string());
    }

    let login_url = config.sub2api_login_url.trim();
    if login_url.is_empty() {
        return Err("已填写账号密码，但未配置 sub2api 登录地址".to_string());
    }

    let body = serde_json::json!({
        "username": username,
        "password": password,
    })
    .to_string();
    let script = format!(
        "$ErrorActionPreference = 'Stop'; \
         $body = '{}'; \
         $response = Invoke-RestMethod -Method Post -UseBasicParsing -Uri '{}' -ContentType 'application/json' -Body $body -TimeoutSec 8; \
         $token = $response.token; \
         if (-not $token) {{ $token = $response.access_token }}; \
         if (-not $token) {{ $token = $response.accessToken }}; \
         if (-not $token -and $response.data) {{ $token = $response.data.token }}; \
         if (-not $token -and $response.data) {{ $token = $response.data.access_token }}; \
         if (-not $token -and $response.data) {{ $token = $response.data.accessToken }}; \
         if (-not $token) {{ throw '登录成功但响应中没有 token' }}; \
         [Console]::Out.Write($token)",
        escape_powershell_single_quoted(&body),
        escape_powershell_single_quoted(login_url),
    );

    let output = Command::new("powershell")
        .args(["-NoProfile", "-Command", &script])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|error| format!("sub2api 登录失败: {error}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if !output.status.success() {
        return Err(if stderr.is_empty() {
            "sub2api 登录失败".to_string()
        } else {
            format!("sub2api 登录失败: {stderr}")
        });
    }
    if stdout.is_empty() {
        return Err("sub2api 登录未返回 token".to_string());
    }

    Ok(Some(stdout))
}

fn build_sub2api_health_script(url: &str, token: Option<&str>) -> String {
    match token {
        Some(token) => format!(
            "try {{ \
               $headers = @{{ Authorization = 'Bearer {}' }}; \
               (Invoke-WebRequest -UseBasicParsing -Uri '{}' -Headers $headers -TimeoutSec 5).StatusCode \
             }} catch {{ $_.Exception.Message }}",
            escape_powershell_single_quoted(token),
            escape_powershell_single_quoted(url),
        ),
        None => format!(
            "try {{ (Invoke-WebRequest -UseBasicParsing -Uri '{}' -TimeoutSec 5).StatusCode }} catch {{ $_.Exception.Message }}",
            escape_powershell_single_quoted(url),
        ),
    }
}

pub fn list_task_runs(app: &AppHandle, limit: Option<u32>) -> Result<Vec<TaskRun>, String> {
    let conn = open_db(app)?;
    ensure_schema(&conn)?;
    let limit = limit.unwrap_or(20).min(100);
    let mut statement = conn
        .prepare(
            "SELECT id, task_key, status, started_at, finished_at, exit_code, stdout, stderr
             FROM task_runs
             ORDER BY id DESC
             LIMIT ?1",
        )
        .map_err(|error| format!("读取任务记录失败: {error}"))?;
    let rows = statement
        .query_map(params![limit], |row| {
            Ok(TaskRun {
                id: row.get(0)?,
                task_key: row.get(1)?,
                status: row.get(2)?,
                started_at: row.get(3)?,
                finished_at: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
                exit_code: row.get(5)?,
                stdout: row.get(6)?,
                stderr: row.get(7)?,
            })
        })
        .map_err(|error| format!("读取任务记录失败: {error}"))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("读取任务记录失败: {error}"))
}

pub fn list_operation_logs(
    app: &AppHandle,
    limit: Option<u32>,
) -> Result<Vec<OperationLog>, String> {
    let conn = open_db(app)?;
    ensure_schema(&conn)?;
    purge_old_operation_logs(&conn)?;
    let limit = limit.unwrap_or(80).min(300);
    let mut statement = conn
        .prepare(
            "SELECT id, module, action, status, message, detail, created_at
             FROM operation_logs
             ORDER BY id DESC
             LIMIT ?1",
        )
        .map_err(|error| format!("读取操作日志失败: {error}"))?;
    let rows = statement
        .query_map(params![limit], |row| {
            Ok(OperationLog {
                id: row.get(0)?,
                module: row.get(1)?,
                action: row.get(2)?,
                status: row.get(3)?,
                message: row.get(4)?,
                detail: row.get(5)?,
                created_at: row.get(6)?,
            })
        })
        .map_err(|error| format!("读取操作日志失败: {error}"))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("读取操作日志失败: {error}"))
}

pub fn record_operation(
    app: &AppHandle,
    module: &str,
    action: &str,
    status: &str,
    message: &str,
    detail: &str,
) {
    log_operation(app, module, action, status, message, detail);
}

fn run_process(
    app: &AppHandle,
    task_key: &str,
    program: &str,
    args: &[&str],
    working_dir: Option<&Path>,
    use_shell_for_scripts: bool,
) -> Result<TaskRun, String> {
    let conn = open_db(app)?;
    ensure_schema(&conn)?;
    let started_at = now_text();
    let run_id = insert_task_run(&conn, task_key, "running", &started_at)?;
    log_operation(
        app,
        operation_module_for_task(task_key),
        operation_action_for_task(task_key),
        "running",
        &format!("{}开始执行", operation_action_for_task(task_key)),
        program,
    );

    let path = PathBuf::from(program);
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    let mut command = if use_shell_for_scripts && matches!(extension.as_str(), "bat" | "cmd") {
        let mut command = Command::new("cmd");
        command.args(["/C", program]);
        command
    } else if use_shell_for_scripts && extension == "ps1" {
        let mut command = Command::new("powershell");
        command.args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File", program]);
        command
    } else {
        let mut command = Command::new(program);
        command.args(args);
        command
    };

    if let Some(working_dir) = working_dir {
        command.current_dir(working_dir);
    }

    let output = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();
    let finished_at = now_text();

    match output {
        Ok(output) => {
            let stdout = limit_text(&String::from_utf8_lossy(&output.stdout), 16_000);
            let stderr = limit_text(&String::from_utf8_lossy(&output.stderr), 16_000);
            let exit_code = output.status.code();
            let status = if output.status.success() {
                "success"
            } else {
                "failed"
            };
            update_task_run(
                &conn,
                run_id,
                status,
                &finished_at,
                exit_code,
                &stdout,
                &stderr,
            )?;
            log_operation(
                app,
                operation_module_for_task(task_key),
                operation_action_for_task(task_key),
                status,
                if output.status.success() {
                    "命令执行完成"
                } else {
                    "命令执行失败"
                },
                &task_detail(program, &stdout, &stderr),
            );
            Ok(TaskRun {
                id: run_id,
                task_key: task_key.to_string(),
                status: status.to_string(),
                started_at,
                finished_at,
                exit_code,
                stdout,
                stderr,
            })
        }
        Err(error) => {
            let stderr = format!("执行失败: {error}");
            update_task_run(&conn, run_id, "failed", &finished_at, None, "", &stderr)?;
            log_operation(
                app,
                operation_module_for_task(task_key),
                operation_action_for_task(task_key),
                "failed",
                "命令启动失败",
                &task_detail(program, "", &stderr),
            );
            Err(stderr)
        }
    }
}

fn spawn_process(
    app: &AppHandle,
    task_key: &str,
    program: &str,
    args: &[&str],
    working_dir: Option<&Path>,
    use_shell_for_scripts: bool,
) -> Result<TaskRun, String> {
    let conn = open_db(app)?;
    ensure_schema(&conn)?;
    let started_at = now_text();
    let finished_at = started_at.clone();
    let path = PathBuf::from(program);
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    let mut command = if use_shell_for_scripts && matches!(extension.as_str(), "bat" | "cmd") {
        let mut command = Command::new("cmd");
        command.args(["/C", program]);
        command
    } else if use_shell_for_scripts && extension == "ps1" {
        let mut command = Command::new("powershell");
        command.args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File", program]);
        command
    } else {
        let mut command = Command::new(program);
        command.args(args);
        command
    };

    if let Some(working_dir) = working_dir {
        command.current_dir(working_dir);
    }

    let spawn_result = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();

    match spawn_result {
        Ok(_) => {
            conn.execute(
                "INSERT INTO task_runs (task_key, status, started_at, finished_at, stdout)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    task_key,
                    "started",
                    started_at,
                    finished_at,
                    "后台启动命令已发出"
                ],
            )
            .map_err(|error| format!("记录任务失败: {error}"))?;
            log_operation(
                app,
                operation_module_for_task(task_key),
                operation_action_for_task(task_key),
                "started",
                "后台启动命令已发出",
                program,
            );
            Ok(TaskRun {
                id: conn.last_insert_rowid(),
                task_key: task_key.to_string(),
                status: "started".to_string(),
                started_at,
                finished_at,
                exit_code: None,
                stdout: "后台启动命令已发出".to_string(),
                stderr: String::new(),
            })
        }
        Err(error) => {
            let stderr = format!("启动失败: {error}");
            conn.execute(
                "INSERT INTO task_runs (task_key, status, started_at, finished_at, stderr)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![task_key, "failed", started_at, finished_at, stderr],
            )
            .map_err(|error| format!("记录任务失败: {error}"))?;
            log_operation(
                app,
                operation_module_for_task(task_key),
                operation_action_for_task(task_key),
                "failed",
                "后台启动失败",
                &task_detail(program, "", &stderr),
            );
            Err(stderr)
        }
    }
}

fn operation_module_for_task(task_key: &str) -> &'static str {
    match task_key.split('.').next().unwrap_or_default() {
        "docker" => "Docker",
        "clash_party" => "Clash Party",
        "sub2api" => "sub2api",
        "system" => "系统",
        _ => "工作台",
    }
}

fn operation_action_for_task(task_key: &str) -> &'static str {
    match task_key {
        "docker.start" => "启动 Docker",
        "docker.stop" => "停止 Docker",
        "docker.restart" => "重启 Docker",
        "clash_party.start" => "启动 Clash Party",
        "clash_party.stop" => "退出 Clash Party",
        "sub2api.start" => "启动 sub2api",
        "sub2api.stop" => "停止 sub2api",
        "sub2api.upgrade" => "升级 sub2api",
        "system.shutdown" => "关机",
        _ => "执行命令",
    }
}

fn task_detail(program: &str, stdout: &str, stderr: &str) -> String {
    let mut detail = format!("program: {program}");
    if !stdout.trim().is_empty() {
        detail.push_str("\nstdout:\n");
        detail.push_str(stdout.trim());
    }
    if !stderr.trim().is_empty() {
        detail.push_str("\nstderr:\n");
        detail.push_str(stderr.trim());
    }
    detail
}

fn open_db(app: &AppHandle) -> Result<Connection, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("无法定位应用数据目录: {error}"))?;
    fs::create_dir_all(&data_dir).map_err(|error| format!("创建应用数据目录失败: {error}"))?;
    Connection::open(data_dir.join("rusttool.sqlite"))
        .map_err(|error| format!("打开本地数据库失败: {error}"))
}

fn ensure_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS app_settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS task_runs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            task_key TEXT NOT NULL,
            status TEXT NOT NULL,
            started_at TEXT NOT NULL,
            finished_at TEXT,
            exit_code INTEGER,
            stdout TEXT NOT NULL DEFAULT '',
            stderr TEXT NOT NULL DEFAULT ''
        );

        CREATE TABLE IF NOT EXISTS operation_logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            module TEXT NOT NULL,
            action TEXT NOT NULL,
            status TEXT NOT NULL,
            message TEXT NOT NULL,
            detail TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL
        );",
    )
    .map_err(|error| format!("初始化本地数据库失败: {error}"))
}

fn read_config(conn: &Connection) -> Result<WorkbenchConfig, String> {
    let value = conn
        .query_row(
            "SELECT value FROM app_settings WHERE key = ?1",
            params![SETTINGS_KEY],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| format!("读取配置失败: {error}"))?;

    let mut config = match value {
        Some(value) => {
            serde_json::from_str(&value).map_err(|error| format!("解析配置失败: {error}"))?
        }
        None => WorkbenchConfig {
            sub2api_health_url: "http://127.0.0.1:9999/v1/models".to_string(),
            sub2api_login_url: "http://127.0.0.1:9999/api/auth/login".to_string(),
            ..WorkbenchConfig::default()
        },
    };
    normalize_config_defaults(&mut config);
    Ok(config)
}

fn normalize_config_defaults(config: &mut WorkbenchConfig) {
    if config.sub2api_health_url.trim().is_empty() {
        config.sub2api_health_url = "http://127.0.0.1:9999/v1/models".to_string();
    }
    if config.sub2api_login_url.trim().is_empty() {
        config.sub2api_login_url = "http://127.0.0.1:9999/api/auth/login".to_string();
    }
    if config.clash_party_api_url.trim().is_empty() {
        config.clash_party_api_url = default_clash_party_api_url();
    }
    if config.clash_party_data_dir.trim().is_empty() {
        config.clash_party_data_dir = detect_clash_party_data_dir().unwrap_or_default();
    }
}

fn insert_task_run(
    conn: &Connection,
    task_key: &str,
    status: &str,
    started_at: &str,
) -> Result<i64, String> {
    conn.execute(
        "INSERT INTO task_runs (task_key, status, started_at) VALUES (?1, ?2, ?3)",
        params![task_key, status, started_at],
    )
    .map_err(|error| format!("记录任务失败: {error}"))?;
    Ok(conn.last_insert_rowid())
}

fn update_task_run(
    conn: &Connection,
    id: i64,
    status: &str,
    finished_at: &str,
    exit_code: Option<i32>,
    stdout: &str,
    stderr: &str,
) -> Result<(), String> {
    conn.execute(
        "UPDATE task_runs
         SET status = ?1, finished_at = ?2, exit_code = ?3, stdout = ?4, stderr = ?5
         WHERE id = ?6",
        params![status, finished_at, exit_code, stdout, stderr, id],
    )
    .map_err(|error| format!("更新任务记录失败: {error}"))?;
    Ok(())
}

fn log_operation(
    app: &AppHandle,
    module: &str,
    action: &str,
    status: &str,
    message: &str,
    detail: &str,
) {
    let Ok(conn) = open_db(app) else {
        return;
    };
    if ensure_schema(&conn).is_err() {
        return;
    }
    let _ = insert_operation_log(&conn, module, action, status, message, detail);
}

fn insert_operation_log(
    conn: &Connection,
    module: &str,
    action: &str,
    status: &str,
    message: &str,
    detail: &str,
) -> Result<(), String> {
    purge_old_operation_logs(conn)?;
    conn.execute(
        "INSERT INTO operation_logs (module, action, status, message, detail, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            module,
            action,
            status,
            limit_text(message, 2_000),
            limit_text(detail, 8_000),
            now_text()
        ],
    )
    .map_err(|error| format!("记录操作日志失败: {error}"))?;
    Ok(())
}

fn purge_old_operation_logs(conn: &Connection) -> Result<(), String> {
    let cutoff = current_epoch_seconds().saturating_sub(OPERATION_LOG_RETENTION_SECONDS);
    conn.execute(
        "DELETE FROM operation_logs WHERE CAST(created_at AS INTEGER) < ?1",
        params![cutoff.to_string()],
    )
    .map_err(|error| format!("清理过期操作日志失败: {error}"))?;
    Ok(())
}

fn validate_optional_executable(value: &str, label: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Ok(());
    }
    validate_required_executable(value, label)
}

fn validate_required_executable(value: &str, label: &str) -> Result<(), String> {
    let path = Path::new(value.trim());
    if !path.exists() {
        return Err(format!("{label}不存在"));
    }
    if !path.is_file() {
        return Err(format!("{label}不是文件"));
    }
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if extension != "exe" {
        return Err(format!("{label}必须是 .exe 文件"));
    }
    Ok(())
}

fn validate_optional_script(value: &str, label: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Ok(());
    }
    validate_required_script(value, label)
}

fn validate_required_script(value: &str, label: &str) -> Result<(), String> {
    let path = Path::new(value.trim());
    if !path.exists() {
        return Err(format!("{label}不存在"));
    }
    if !path.is_file() {
        return Err(format!("{label}不是文件"));
    }
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if !matches!(extension.as_str(), "bat" | "cmd" | "ps1" | "exe") {
        return Err(format!("{label}只支持 .bat/.cmd/.ps1/.exe"));
    }
    Ok(())
}

fn validate_optional_directory(value: &str, label: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Ok(());
    }
    let path = Path::new(value.trim());
    if !path.exists() {
        return Err(format!("{label}不存在"));
    }
    if !path.is_dir() {
        return Err(format!("{label}不是目录"));
    }
    Ok(())
}

fn resolve_clash_party_data_dir(config: &WorkbenchConfig) -> Result<PathBuf, String> {
    let configured = config.clash_party_data_dir.trim();
    if !configured.is_empty() {
        let path = PathBuf::from(configured);
        if path.join("profile.yaml").exists() {
            return Ok(path);
        }
        return Err(format!(
            "Clash Party 数据目录无效，未找到 profile.yaml: {}",
            path.display()
        ));
    }

    detect_clash_party_data_dir()
        .map(PathBuf::from)
        .ok_or_else(|| {
            "未找到 Clash Party 数据目录，请在配置中选择 mihomo-party 数据目录".to_string()
        })
}

fn read_clash_profile_index(path: &Path) -> Result<ClashProfileIndex, String> {
    let content = fs::read_to_string(path).map_err(|error| format!("读取订阅索引失败: {error}"))?;
    serde_yaml::from_str(&content).map_err(|error| format!("解析订阅索引失败: {error}"))
}

fn build_clash_subscriptions(
    data_dir: &Path,
    profile_index: &ClashProfileIndex,
) -> Vec<ClashPartySubscription> {
    profile_index
        .items
        .iter()
        .map(|item| {
            let path = clash_profile_path(data_dir, &item.id);
            let (node_count, group_count) = read_clash_profile_summary(&path).unwrap_or((0, 0));
            let (used_bytes, total_bytes, expire_at) = item
                .extra
                .as_ref()
                .map(|extra| {
                    (
                        extra
                            .upload
                            .unwrap_or(0)
                            .saturating_add(extra.download.unwrap_or(0)),
                        extra.total,
                        extra.expire,
                    )
                })
                .unwrap_or((0, None, None));
            ClashPartySubscription {
                id: item.id.clone(),
                name: item.name.clone(),
                profile_type: if item.profile_type.is_empty() {
                    "unknown".to_string()
                } else {
                    item.profile_type.clone()
                },
                path: path.display().to_string(),
                active: item.id == profile_index.current,
                node_count,
                group_count,
                updated_at: item
                    .updated
                    .map_or_else(String::new, |value| value.to_string()),
                used_bytes: (used_bytes > 0).then_some(used_bytes),
                total_bytes,
                expire_at,
            }
        })
        .collect()
}

fn clash_profile_path(data_dir: &Path, subscription_id: &str) -> PathBuf {
    data_dir
        .join("profiles")
        .join(format!("{}.yaml", sanitize_path_file_stem(subscription_id)))
}

fn read_clash_profile_summary(path: &Path) -> Result<(usize, usize), String> {
    let content = fs::read_to_string(path).map_err(|error| error.to_string())?;
    let yaml: YamlValue = serde_yaml::from_str(&content).map_err(|error| error.to_string())?;
    let node_count = yaml_sequence_len(&yaml, "proxies");
    let group_count = yaml_sequence_len(&yaml, "proxy-groups");
    Ok((node_count, group_count))
}

fn yaml_sequence_len(value: &YamlValue, key: &str) -> usize {
    value
        .get(key)
        .and_then(YamlValue::as_sequence)
        .map(Vec::len)
        .unwrap_or_default()
}

fn get_clash_party_runtime_groups(
    config: &WorkbenchConfig,
) -> Result<Vec<ClashPartyProxyGroup>, String> {
    let text = call_clash_party_api(config, "GET", "/proxies", None)?;
    let value: JsonValue = serde_json::from_str(&text)
        .map_err(|error| format!("解析 Mihomo 代理列表失败: {error}"))?;
    let proxies = value
        .get("proxies")
        .and_then(JsonValue::as_object)
        .ok_or_else(|| "Mihomo API 未返回 proxies 字段".to_string())?;

    let mut proxy_detail = HashMap::new();
    for (name, proxy) in proxies {
        proxy_detail.insert(name.clone(), proxy.clone());
    }

    let mut groups = Vec::new();
    for (name, proxy) in proxies {
        let Some(all) = proxy.get("all").and_then(JsonValue::as_array) else {
            continue;
        };
        if all.is_empty() {
            continue;
        }
        let selected = proxy
            .get("now")
            .and_then(JsonValue::as_str)
            .unwrap_or_default()
            .to_string();
        let group_type = proxy
            .get("type")
            .and_then(JsonValue::as_str)
            .unwrap_or("Selector")
            .to_string();
        let nodes = all
            .iter()
            .filter_map(JsonValue::as_str)
            .map(|node_name| {
                let detail = proxy_detail.get(node_name);
                ClashPartyNode {
                    name: node_name.to_string(),
                    node_type: detail
                        .and_then(|value| value.get("type"))
                        .and_then(JsonValue::as_str)
                        .unwrap_or("unknown")
                        .to_string(),
                    server: detail
                        .and_then(|value| value.get("server"))
                        .and_then(JsonValue::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    port: detail
                        .and_then(|value| value.get("port"))
                        .and_then(JsonValue::as_u64)
                        .and_then(|value| u16::try_from(value).ok()),
                    delay: read_proxy_delay(detail),
                    active: node_name == selected,
                }
            })
            .collect();
        groups.push(ClashPartyProxyGroup {
            name: name.clone(),
            group_type,
            selected,
            nodes,
        });
    }

    groups.sort_by_key(|group| group.name.to_ascii_lowercase());
    Ok(groups)
}

fn read_proxy_delay(detail: Option<&JsonValue>) -> Option<i64> {
    detail
        .and_then(|value| value.get("history"))
        .and_then(JsonValue::as_array)
        .and_then(|history| history.last())
        .and_then(|item| item.get("delay"))
        .and_then(JsonValue::as_i64)
        .filter(|delay| *delay >= 0)
}

fn call_clash_party_api(
    config: &WorkbenchConfig,
    method: &str,
    path: &str,
    body: Option<JsonValue>,
) -> Result<String, String> {
    let base_url = normalized_clash_party_api_url(config);
    if !base_url.starts_with("http://") && !base_url.starts_with("https://") {
        return Err("Clash Party API 地址必须以 http:// 或 https:// 开头".to_string());
    }
    let url = format!(
        "{}/{}",
        base_url.trim_end_matches('/'),
        path.trim_start_matches('/')
    );
    let headers = if config.clash_party_api_secret.trim().is_empty() {
        String::new()
    } else {
        format!(
            "$headers = @{{ Authorization = 'Bearer {}' }};",
            escape_powershell_single_quoted(config.clash_party_api_secret.trim())
        )
    };
    let headers_arg = if headers.is_empty() {
        ""
    } else {
        "-Headers $headers"
    };
    let body_script = body.map_or_else(String::new, |value| {
        format!(
            "$body = '{}';",
            escape_powershell_single_quoted(&value.to_string())
        )
    });
    let body_arg = if body_script.is_empty() {
        ""
    } else {
        "-ContentType 'application/json' -Body $body"
    };
    let script = format!(
        "$ErrorActionPreference = 'Stop'; \
         {}; \
         {}; \
         $response = Invoke-RestMethod -Method {} -UseBasicParsing -Uri '{}' {} {} -TimeoutSec 6; \
         if ($null -ne $response) {{ $response | ConvertTo-Json -Depth 30 -Compress }}",
        headers,
        body_script,
        method,
        escape_powershell_single_quoted(&url),
        headers_arg,
        body_arg
    );

    let output = Command::new("powershell")
        .args(["-NoProfile", "-Command", &script])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|error| format!("调用 Clash Party API 失败: {error}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if output.status.success() {
        Ok(stdout)
    } else if stderr.is_empty() {
        Err("Clash Party API 请求失败".to_string())
    } else {
        Err(stderr)
    }
}

fn normalized_clash_party_api_url(config: &WorkbenchConfig) -> String {
    let url = config.clash_party_api_url.trim();
    if url.is_empty() {
        default_clash_party_api_url()
    } else {
        url.trim_end_matches('/').to_string()
    }
}

fn encode_url_path_segment(value: &str) -> String {
    value
        .bytes()
        .flat_map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                vec![byte as char]
            }
            _ => format!("%{byte:02X}").chars().collect(),
        })
        .collect()
}

fn sanitize_path_file_stem(value: &str) -> String {
    value
        .chars()
        .filter(|character| {
            !matches!(
                character,
                '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'
            ) && !character.is_control()
        })
        .collect()
}

fn first_existing_path(paths: &[&str]) -> Option<String> {
    paths
        .iter()
        .copied()
        .find(|path| Path::new(path).exists())
        .map(ToOwned::to_owned)
}

fn find_docker_in_path() -> Option<String> {
    let output = Command::new("where")
        .arg("docker")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && Path::new(line).exists())
        .map(ToOwned::to_owned)
}

fn detect_clash_party_path() -> Option<String> {
    let mut candidates = vec![
        r"C:\Program Files\Clash Party\Clash Party.exe".to_string(),
        r"C:\Program Files\Clash Party\clash-party.exe".to_string(),
        r"C:\Program Files\Mihomo Party\Mihomo Party.exe".to_string(),
        r"C:\Program Files\Mihomo Party\mihomo-party.exe".to_string(),
        r"C:\Program Files (x86)\Clash Party\Clash Party.exe".to_string(),
        r"C:\Program Files (x86)\Clash Party\clash-party.exe".to_string(),
        r"C:\Program Files (x86)\Mihomo Party\Mihomo Party.exe".to_string(),
        r"C:\Program Files (x86)\Mihomo Party\mihomo-party.exe".to_string(),
    ];

    if let Ok(local_app_data) = env::var("LOCALAPPDATA") {
        candidates.extend([
            format!(r"{local_app_data}\Programs\Clash Party\Clash Party.exe"),
            format!(r"{local_app_data}\Programs\Clash Party\clash-party.exe"),
            format!(r"{local_app_data}\Programs\mihomo-party\Mihomo Party.exe"),
            format!(r"{local_app_data}\Programs\mihomo-party\mihomo-party.exe"),
            format!(r"{local_app_data}\Programs\Mihomo Party\Mihomo Party.exe"),
            format!(r"{local_app_data}\Programs\Mihomo Party\mihomo-party.exe"),
            format!(r"{local_app_data}\clash-party\Clash Party.exe"),
            format!(r"{local_app_data}\clash-party\clash-party.exe"),
            format!(r"{local_app_data}\mihomo-party\Mihomo Party.exe"),
            format!(r"{local_app_data}\mihomo-party\mihomo-party.exe"),
        ]);
    }

    candidates
        .iter()
        .map(String::as_str)
        .find(|path| Path::new(path).exists())
        .map(ToOwned::to_owned)
        .or_else(|| find_executable_in_path("clash-party"))
        .or_else(|| find_executable_in_path("Clash Party"))
        .or_else(|| find_executable_in_path("mihomo-party"))
        .or_else(|| find_executable_in_path("Mihomo Party"))
}

fn detect_clash_party_data_dir() -> Option<String> {
    let mut candidates = Vec::new();
    if let Ok(app_data) = env::var("APPDATA") {
        candidates.extend([
            format!(r"{app_data}\mihomo-party"),
            format!(r"{app_data}\clash-party"),
            format!(r"{app_data}\clashmi"),
        ]);
    }
    if let Ok(local_app_data) = env::var("LOCALAPPDATA") {
        candidates.extend([
            format!(r"{local_app_data}\mihomo-party"),
            format!(r"{local_app_data}\clash-party"),
            format!(r"{local_app_data}\clashmi"),
        ]);
    }

    candidates
        .iter()
        .map(String::as_str)
        .find(|path| Path::new(path).join("profile.yaml").exists())
        .map(ToOwned::to_owned)
}

fn default_clash_party_api_url() -> String {
    "http://127.0.0.1:9090".to_string()
}

fn find_executable_in_path(name: &str) -> Option<String> {
    let output = Command::new("where")
        .arg(name)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && Path::new(line).exists())
        .map(ToOwned::to_owned)
}

fn is_docker_desktop_running() -> bool {
    ["Docker Desktop.exe", "com.docker.backend.exe"]
        .iter()
        .any(|image_name| is_windows_process_running(image_name))
}

fn is_clash_party_running() -> bool {
    [
        "Clash Party.exe",
        "clash-party.exe",
        "mihomo-party.exe",
        "Mihomo Party.exe",
    ]
    .iter()
    .any(|image_name| is_windows_process_running(image_name))
}

fn docker_stop_commands() -> String {
    [
        "Stop-Process -Name 'Docker Desktop' -Force -ErrorAction SilentlyContinue",
        "Stop-Process -Name 'com.docker.backend' -Force -ErrorAction SilentlyContinue",
        "Stop-Process -Name 'com.docker.proxy' -Force -ErrorAction SilentlyContinue",
        "Stop-Process -Name 'vpnkit' -Force -ErrorAction SilentlyContinue",
        "Stop-Process -Name 'docker' -Force -ErrorAction SilentlyContinue",
    ]
    .join("\n")
}

fn clash_party_stop_commands() -> String {
    [
        "Stop-Process -Name 'Clash Party' -Force -ErrorAction SilentlyContinue",
        "Stop-Process -Name 'clash-party' -Force -ErrorAction SilentlyContinue",
        "Stop-Process -Name 'Mihomo Party' -Force -ErrorAction SilentlyContinue",
        "Stop-Process -Name 'mihomo-party' -Force -ErrorAction SilentlyContinue",
    ]
    .join("\n")
}

fn powershell_single_quoted(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn is_windows_process_running(image_name: &str) -> bool {
    let output = Command::new("tasklist")
        .args(["/FI", &format!("IMAGENAME eq {image_name}"), "/NH"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output();
    let Ok(output) = output else {
        return false;
    };
    if !output.status.success() {
        return false;
    }

    String::from_utf8_lossy(&output.stdout)
        .to_ascii_lowercase()
        .contains(&image_name.to_ascii_lowercase())
}

fn run_selection_dialog(script: &str) -> Result<Option<String>, String> {
    let output = Command::new("powershell")
        .args(["-NoProfile", "-STA", "-Command", script])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|error| format!("打开选择窗口失败: {error}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if !output.status.success() {
        return Err(if stderr.is_empty() {
            "选择窗口已异常关闭".to_string()
        } else {
            stderr
        });
    }

    Ok((!stdout.is_empty()).then_some(stdout))
}

fn now_text() -> String {
    current_epoch_seconds().to_string()
}

fn current_epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs())
        .unwrap_or_default()
}

fn limit_text(value: &str, limit: usize) -> String {
    let trimmed = value.trim();
    if trimmed.len() <= limit {
        trimmed.to_string()
    } else {
        format!("{}...", &trimmed[..limit])
    }
}

fn escape_powershell_single_quoted(value: &str) -> String {
    value.replace('\'', "''")
}
