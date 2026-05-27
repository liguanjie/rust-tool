use rusqlite::{params, Connection, OptionalExtension};
use rust_tool_core::{
    check_clash_party_node as check_core_clash_party_node, default_clash_party_delay_test_url,
    default_clash_party_delay_timeout_ms,
    get_clash_party_manager_state as get_core_clash_party_manager_state,
    switch_clash_party_node as switch_core_clash_party_node,
    switch_clash_party_subscription as switch_core_clash_party_subscription, ClashPartyConfig,
};
pub use rust_tool_core::{
    ClashPartyManagerState, ClashPartyNodeCheckResult, ClashPartySwitchResult,
};
use serde::{Deserialize, Serialize};
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{env, fs};
use tauri::{AppHandle, Manager};

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

const SETTINGS_KEY: &str = "windows_workbench.config";
const OPERATION_LOG_RETENTION_SECONDS: u64 = 7 * 24 * 60 * 60;
const OPERATION_LOG_PAGE_SIZE: u32 = 50;

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
    pub sub2api_api_key: String,
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationLogPage {
    pub logs: Vec<OperationLog>,
    pub page: u32,
    pub page_size: u32,
    pub total: u64,
    pub total_pages: u32,
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
    let script = "$shell = New-Object -ComObject Shell.Application; \
         $folder = $shell.BrowseForFolder(0, '选择工作目录', 0x00000040); \
         if ($null -ne $folder) { \
           [Console]::Out.Write($folder.Self.Path) \
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

        let output = hidden_command(&config.docker_cli_path)
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
                "docker CLI 无法执行，请确认配置的是 docker.exe，而不是 Docker Desktop.exe。"
                    .to_string()
            } else {
                format!(
                    "docker CLI 无法执行，请确认 docker.exe 路径是否正确。{}",
                    friendly_detail_suffix(&stderr)
                )
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

        let engine_output = hidden_command(&config.docker_cli_path)
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
            friendly_docker_engine_message(desktop_running, &detail)
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
    match spawn_process(
        app,
        "clash_party.start",
        &config.clash_party_path,
        &[],
        None,
        false,
    ) {
        Ok(run) => Ok(run),
        Err(error) if error.contains("os error 740") || error.contains("请求的操作需要提升") => {
            run_process_elevated(app, "clash_party.start", &config.clash_party_path)
        }
        Err(error) => Err(error),
    }
}

pub fn stop_clash_party(app: &AppHandle) -> Result<TaskRun, String> {
    let config = get_config(app)?;
    let configured_path = config.clash_party_path.trim();
    let stop_by_path = if configured_path.is_empty() {
        String::new()
    } else {
        format!(
            "Get-Process | Where-Object {{ $_.Path -eq {} }} | Stop-Process -Force -ErrorAction SilentlyContinue\n",
            powershell_single_quoted(configured_path)
        )
    };
    let script = format!(
        "{}{}\nfor ($i = 0; $i -lt 12; $i++) {{\n  Start-Sleep -Milliseconds 500\n  $running = @({})\n  if ($running.Count -eq 0) {{ Write-Output 'Clash Party exited.'; exit 0 }}\n  $running | Stop-Process -Force -ErrorAction SilentlyContinue\n}}\nWrite-Error 'Clash Party is still running after exit command.'\nexit 1",
        stop_by_path,
        clash_party_stop_commands(),
        clash_party_running_powershell_expression()
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
    let mut runtime_error = String::new();
    let result: Result<ClashPartyManagerState, String> = (|| {
        let config = get_config(app)?;
        get_core_clash_party_manager_state(&clash_party_config_from_workbench(&config))
            .inspect_err(|error| runtime_error = error.clone())
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
            &clash_party_manager_log_detail(state, &runtime_error),
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

fn clash_party_manager_log_detail(state: &ClashPartyManagerState, runtime_error: &str) -> String {
    if runtime_error.is_empty() {
        format!(
            "subscriptions={}, groups={}, active_subscription={}",
            state.subscriptions.len(),
            state.groups.len(),
            state.active_subscription_id
        )
    } else {
        format!(
            "subscriptions={}, groups={}, active_subscription={}, runtime_error={}",
            state.subscriptions.len(),
            state.groups.len(),
            state.active_subscription_id,
            runtime_error
        )
    }
}

pub fn switch_clash_party_subscription(
    app: &AppHandle,
    subscription_id: String,
) -> Result<ClashPartySwitchResult, String> {
    let result: Result<ClashPartySwitchResult, String> = (|| {
        let config = get_config(app)?;
        switch_core_clash_party_subscription(
            &clash_party_config_from_workbench(&config),
            &subscription_id,
        )
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
        let config = get_config(app)?;
        switch_core_clash_party_node(
            &clash_party_config_from_workbench(&config),
            &group_name,
            &node_name,
        )
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

pub fn check_clash_party_node(
    app: &AppHandle,
    node_name: String,
) -> Result<ClashPartyNodeCheckResult, String> {
    let result: Result<ClashPartyNodeCheckResult, String> = (|| {
        let config = get_config(app)?;
        check_core_clash_party_node(&clash_party_config_from_workbench(&config), &node_name)
    })();

    match &result {
        Ok(result) => log_operation(
            app,
            "Clash Party",
            "检测节点",
            if result.available {
                "success"
            } else {
                "failed"
            },
            &result.message,
            &format!("node={node_name}, delay={:?}", result.delay),
        ),
        Err(error) => log_operation(
            app,
            "Clash Party",
            "检测节点",
            "failed",
            "Clash Party 节点检测失败",
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

        let mut ok = true;
        let mut messages = Vec::new();

        if url.is_empty() {
            ok = false;
            messages.push("API 接口未检查：还没有配置健康检查地址。".to_string());
        } else {
            let api_result = check_sub2api_api_health(url, config.sub2api_api_key.trim())?;
            ok &= api_result.ok;
            messages.push(api_result.message);
        }

        if sub2api_login_should_check(&config) {
            let login_result = check_sub2api_login(&config)?;
            ok &= login_result.ok;
            messages.push(login_result.message);
        }

        Ok(HealthStatus {
            ok,
            message: messages.join(" "),
        })
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

struct Sub2apiCheckResult {
    ok: bool,
    message: String,
}

fn check_sub2api_api_health(url: &str, api_key: &str) -> Result<Sub2apiCheckResult, String> {
    if sub2api_health_requires_api_key(url) && api_key.is_empty() {
        return Ok(Sub2apiCheckResult {
            ok: false,
            message: "API 接口未检查：/v1 接口需要 API Key，请在 sub2api 配置里填写后再检测。"
                .to_string(),
        });
    }

    let script = build_sub2api_health_script(url, (!api_key.is_empty()).then_some(api_key));
    let output = hidden_command("powershell")
        .args(["-NoProfile", "-Command", &script])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|error| format!("健康检查失败: {error}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let probe = parse_sub2api_probe(&stdout, &stderr);
    let ok = output.status.success()
        && probe
            .status_code
            .is_some_and(|code| (200..300).contains(&code));

    Ok(Sub2apiCheckResult {
        ok,
        message: if ok {
            format!(
                "API 接口正常（HTTP {}）。",
                probe.status_code.unwrap_or(200)
            )
        } else {
            format!(
                "API 接口未通过：{}",
                describe_sub2api_probe_failure(Sub2apiProbeTarget::Api, url, &probe)
            )
        },
    })
}

fn sub2api_login_should_check(config: &WorkbenchConfig) -> bool {
    !config.sub2api_username.trim().is_empty() || !config.sub2api_password.trim().is_empty()
}

fn check_sub2api_login(config: &WorkbenchConfig) -> Result<Sub2apiCheckResult, String> {
    let login_url = config.sub2api_login_url.trim();
    let email = config.sub2api_username.trim();
    let password = config.sub2api_password.trim();

    if login_url.is_empty() || email.is_empty() || password.is_empty() {
        return Ok(Sub2apiCheckResult {
            ok: false,
            message: "后台登录未检查：登录地址、邮箱和密码需要同时填写；如果暂时不想校验后台登录，可以先清空邮箱和密码。".to_string(),
        });
    }

    let body = serde_json::json!({
        "email": email,
        "password": password,
    })
    .to_string();
    let script = format!(
        "{} \
         try {{ \
           $body = '{}'; \
           $response = Invoke-WebRequest -Method Post -UseBasicParsing -Uri '{}' -ContentType 'application/json' -Body $body -TimeoutSec 8; \
           [Console]::Out.Write('HTTP_STATUS=' + [int]$response.StatusCode) \
         }} catch {{ Write-Sub2apiProbeError $_ }}",
        sub2api_probe_powershell_preamble(),
        escape_powershell_single_quoted(&body),
        escape_powershell_single_quoted(login_url),
    );

    let output = hidden_command("powershell")
        .args(["-NoProfile", "-Command", &script])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|error| format!("后台登录检查失败: {error}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let probe = parse_sub2api_probe(&stdout, &stderr);
    let ok = output.status.success()
        && probe
            .status_code
            .is_some_and(|code| (200..300).contains(&code));

    Ok(Sub2apiCheckResult {
        ok,
        message: if ok {
            format!(
                "后台登录正常（HTTP {}）。",
                probe.status_code.unwrap_or(200)
            )
        } else {
            format!(
                "后台登录未通过：{}",
                describe_sub2api_probe_failure(Sub2apiProbeTarget::Login, login_url, &probe)
            )
        },
    })
}

fn build_sub2api_health_script(url: &str, token: Option<&str>) -> String {
    match token {
        Some(token) => format!(
            "{} \
             try {{ \
               $headers = @{{ Authorization = 'Bearer {}' }}; \
               $response = Invoke-WebRequest -UseBasicParsing -Uri '{}' -Headers $headers -TimeoutSec 5; \
               [Console]::Out.Write('HTTP_STATUS=' + [int]$response.StatusCode) \
             }} catch {{ Write-Sub2apiProbeError $_ }}",
            sub2api_probe_powershell_preamble(),
            escape_powershell_single_quoted(token),
            escape_powershell_single_quoted(url),
        ),
        None => format!(
            "{} \
             try {{ \
               $response = Invoke-WebRequest -UseBasicParsing -Uri '{}' -TimeoutSec 5; \
               [Console]::Out.Write('HTTP_STATUS=' + [int]$response.StatusCode) \
             }} catch {{ Write-Sub2apiProbeError $_ }}",
            sub2api_probe_powershell_preamble(),
            escape_powershell_single_quoted(url),
        ),
    }
}

#[derive(Default)]
struct Sub2apiProbe {
    status_code: Option<u16>,
    error_kind: String,
    error_message: String,
}

enum Sub2apiProbeTarget {
    Api,
    Login,
}

fn sub2api_probe_powershell_preamble() -> &'static str {
    "$ErrorActionPreference = 'Stop'; \
     $utf8 = New-Object System.Text.UTF8Encoding($false); \
     [Console]::OutputEncoding = $utf8; \
     $OutputEncoding = $utf8; \
     function Write-Sub2apiProbeError($record) { \
       $status = $null; \
       if ($record.Exception.Response) { \
         try { $status = [int]$record.Exception.Response.StatusCode } catch { } \
       }; \
       if ($null -ne $status) { [Console]::Error.Write('HTTP_STATUS=' + $status + \"`n\") }; \
       [Console]::Error.Write('ERROR_KIND=' + $record.Exception.GetType().FullName + \"`n\"); \
       [Console]::Error.Write('ERROR_MESSAGE=' + $record.Exception.Message); \
       exit 1 \
     };"
}

fn parse_sub2api_probe(stdout: &str, stderr: &str) -> Sub2apiProbe {
    let mut probe = Sub2apiProbe::default();
    let mut raw_messages = Vec::new();

    for line in stdout.lines().chain(stderr.lines()).map(str::trim) {
        if line.is_empty() {
            continue;
        }
        if let Some(value) = line.strip_prefix("HTTP_STATUS=") {
            probe.status_code = value.trim().parse::<u16>().ok();
        } else if let Some(value) = line.strip_prefix("ERROR_KIND=") {
            probe.error_kind = value.trim().to_string();
        } else if let Some(value) = line.strip_prefix("ERROR_MESSAGE=") {
            probe.error_message = value.trim().to_string();
        } else {
            raw_messages.push(line.to_string());
        }
    }

    if probe.status_code.is_none() {
        probe.status_code = stdout.trim().parse::<u16>().ok();
    }
    if probe.error_message.is_empty() && !raw_messages.is_empty() {
        probe.error_message = raw_messages.join(" ");
    }

    probe
}

fn describe_sub2api_probe_failure(
    target: Sub2apiProbeTarget,
    url: &str,
    probe: &Sub2apiProbe,
) -> String {
    if let Some(status_code) = probe.status_code {
        return match (target, status_code) {
            (Sub2apiProbeTarget::Api, 401 | 403) => {
                "服务有响应，但 API Key 没通过鉴权。请检查 sub2api 配置里的 API Key 是否正确。"
                    .to_string()
            }
            (Sub2apiProbeTarget::Login, 401 | 403) => {
                "服务有响应，但邮箱或密码没有通过验证。请检查后台登录信息。".to_string()
            }
            (Sub2apiProbeTarget::Api, 404) => {
                "服务有响应，但健康检查接口不存在。请确认地址是否类似 http://127.0.0.1:9999/v1/models。"
                    .to_string()
            }
            (Sub2apiProbeTarget::Login, 404) => {
                "服务有响应，但登录接口不存在。请确认登录地址是否类似 http://127.0.0.1:9999/api/auth/login。"
                    .to_string()
            }
            (_, 405) => "服务有响应，但当前接口不接受这类请求。请检查配置的接口地址。".to_string(),
            (_, 429) => "服务有响应，但请求过于频繁。请稍等一会儿再检测。".to_string(),
            (_, 500..=599) => {
                format!("sub2api 已响应，但服务端返回 HTTP {status_code}。请查看 sub2api 后台日志。")
            }
            (_, code) => format!("服务有响应，但返回 HTTP {code}。请检查地址和鉴权配置。"),
        };
    }

    let error_message = probe.error_message.trim();
    let normalized = error_message.to_ascii_lowercase();
    if normalized.contains("actively refused")
        || normalized.contains("connection refused")
        || normalized.contains("unable to connect")
        || normalized.contains("no connection could be made")
        || normalized.contains("无法连接")
        || normalized.contains("由于目标计算机积极拒绝")
    {
        return format!("没有连接到 sub2api 服务。请先启动 sub2api，并确认它正在监听 {url}。");
    }
    if normalized.contains("timed out")
        || normalized.contains("timeout")
        || normalized.contains("超时")
    {
        return "请求超时。sub2api 可能正在启动、卡住，或当前地址不可达。".to_string();
    }
    if normalized.contains("invalid uri")
        || normalized.contains("not a valid")
        || normalized.contains("无效")
    {
        return "健康检查地址格式不正确，请确认以 http:// 或 https:// 开头。".to_string();
    }
    if normalized.contains("name resolution")
        || normalized.contains("no such host")
        || normalized.contains("无法解析")
    {
        return "地址里的域名无法解析。请检查主机名或改用 127.0.0.1。".to_string();
    }
    if normalized.contains("certificate")
        || normalized.contains("ssl")
        || normalized.contains("tls")
        || normalized.contains("证书")
    {
        return "HTTPS 证书校验失败。请检查证书配置，或在本机服务场景下改用 http:// 地址。"
            .to_string();
    }

    format!(
        "请求没有拿到有效响应。请确认 sub2api 已启动，并检查地址是否正确。{}",
        friendly_detail_suffix(error_message)
    )
}

fn sub2api_health_requires_api_key(url: &str) -> bool {
    let normalized = url.trim().to_ascii_lowercase();
    normalized.contains("/v1/") || normalized.ends_with("/v1")
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
    page: Option<u32>,
    query: Option<String>,
) -> Result<OperationLogPage, String> {
    let conn = open_db(app)?;
    ensure_schema(&conn)?;
    purge_old_operation_logs(&conn)?;
    let requested_page = page.unwrap_or(1).max(1);
    let query = query.unwrap_or_default();
    let keyword = query.trim();

    let total: u64 = if keyword.is_empty() {
        conn.query_row("SELECT COUNT(*) FROM operation_logs", [], |row| {
            row.get::<_, u64>(0)
        })
        .map_err(|error| format!("统计操作日志失败: {error}"))?
    } else {
        let pattern = format!("%{}%", keyword);
        conn.query_row(
            "SELECT COUNT(*)
             FROM operation_logs
             WHERE module LIKE ?1 OR action LIKE ?1 OR status LIKE ?1 OR message LIKE ?1 OR detail LIKE ?1 OR created_at LIKE ?1",
            params![pattern],
            |row| row.get::<_, u64>(0),
        )
        .map_err(|error| format!("统计操作日志失败: {error}"))?
    };

    let total_pages = if total == 0 {
        1
    } else {
        ((total + u64::from(OPERATION_LOG_PAGE_SIZE) - 1) / u64::from(OPERATION_LOG_PAGE_SIZE))
            .min(u64::from(u32::MAX)) as u32
    };
    let page = requested_page.min(total_pages).max(1);
    let offset = (page - 1) * OPERATION_LOG_PAGE_SIZE;

    let logs = if keyword.is_empty() {
        let mut statement = conn
            .prepare(
                "SELECT id, module, action, status, message, detail, created_at
                 FROM operation_logs
                 ORDER BY id DESC
                 LIMIT ?1 OFFSET ?2",
            )
            .map_err(|error| format!("读取操作日志失败: {error}"))?;
        let rows = statement
            .query_map(
                params![OPERATION_LOG_PAGE_SIZE, offset],
                operation_log_from_row,
            )
            .map_err(|error| format!("读取操作日志失败: {error}"))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("读取操作日志失败: {error}"))?
    } else {
        let pattern = format!("%{}%", keyword);
        let mut statement = conn
            .prepare(
                "SELECT id, module, action, status, message, detail, created_at
                 FROM operation_logs
                 WHERE module LIKE ?1 OR action LIKE ?1 OR status LIKE ?1 OR message LIKE ?1 OR detail LIKE ?1 OR created_at LIKE ?1
                 ORDER BY id DESC
                 LIMIT ?2 OFFSET ?3",
            )
            .map_err(|error| format!("读取操作日志失败: {error}"))?;
        let rows = statement
            .query_map(
                params![pattern, OPERATION_LOG_PAGE_SIZE, offset],
                operation_log_from_row,
            )
            .map_err(|error| format!("读取操作日志失败: {error}"))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("读取操作日志失败: {error}"))?
    };

    Ok(OperationLogPage {
        logs,
        page,
        page_size: OPERATION_LOG_PAGE_SIZE,
        total,
        total_pages,
    })
}

fn operation_log_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<OperationLog> {
    Ok(OperationLog {
        id: row.get(0)?,
        module: row.get(1)?,
        action: row.get(2)?,
        status: row.get(3)?,
        message: row.get(4)?,
        detail: row.get(5)?,
        created_at: row.get(6)?,
    })
}

pub fn clear_operation_logs(app: &AppHandle) -> Result<u64, String> {
    let conn = open_db(app)?;
    ensure_schema(&conn)?;
    let deleted = conn
        .execute("DELETE FROM operation_logs", [])
        .map_err(|error| format!("清理操作日志失败: {error}"))?;
    log_operation(
        app,
        "工作台",
        "清理日志",
        "success",
        "操作日志已清理",
        &format!("deleted={deleted}"),
    );
    Ok(deleted as u64)
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

fn hidden_command(program: &str) -> Command {
    let mut command = Command::new(program);
    hide_command_window(&mut command);
    command
}

fn hide_command_window(command: &mut Command) {
    #[cfg(windows)]
    {
        command.creation_flags(CREATE_NO_WINDOW);
    }
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
        let mut command = hidden_command("cmd");
        command.args(["/C", program]);
        command
    } else if use_shell_for_scripts && extension == "ps1" {
        let mut command = hidden_command("powershell");
        command.args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File", program]);
        command
    } else {
        let mut command = hidden_command(program);
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
        let mut command = hidden_command("cmd");
        command.args(["/C", program]);
        command
    } else if use_shell_for_scripts && extension == "ps1" {
        let mut command = hidden_command("powershell");
        command.args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File", program]);
        command
    } else {
        let mut command = hidden_command(program);
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

fn run_process_elevated(app: &AppHandle, task_key: &str, program: &str) -> Result<TaskRun, String> {
    let script = format!(
        "Start-Process -FilePath {} -Verb RunAs; Write-Output 'UAC elevation request sent.'",
        powershell_single_quoted(program)
    );
    let args = [
        "-NoProfile",
        "-ExecutionPolicy",
        "Bypass",
        "-Command",
        script.as_str(),
    ];
    run_process(app, task_key, "powershell", &args, None, false)
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

fn clash_party_config_from_workbench(config: &WorkbenchConfig) -> ClashPartyConfig {
    ClashPartyConfig {
        data_dir: config.clash_party_data_dir.clone(),
        api_url: config.clash_party_api_url.clone(),
        api_secret: config.clash_party_api_secret.clone(),
        delay_test_url: default_clash_party_delay_test_url(),
        delay_timeout_ms: default_clash_party_delay_timeout_ms(),
    }
}

fn first_existing_path(paths: &[&str]) -> Option<String> {
    paths
        .iter()
        .copied()
        .find(|path| Path::new(path).exists())
        .map(ToOwned::to_owned)
}

fn find_docker_in_path() -> Option<String> {
    let output = hidden_command("where")
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
    "http://127.0.0.1:9998".to_string()
}

fn find_executable_in_path(name: &str) -> Option<String> {
    let output = hidden_command("where")
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
        "Stop-Process -Name 'mihomo' -Force -ErrorAction SilentlyContinue",
    ]
    .join("\n")
}

fn clash_party_running_powershell_expression() -> &'static str {
    "Get-Process -ErrorAction SilentlyContinue | Where-Object { @('Clash Party','clash-party','Mihomo Party','mihomo-party','mihomo') -contains $_.ProcessName }"
}

fn powershell_single_quoted(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn is_windows_process_running(image_name: &str) -> bool {
    let output = hidden_command("tasklist")
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
    let output = hidden_command("powershell")
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

fn friendly_docker_engine_message(desktop_running: bool, detail: &str) -> String {
    let normalized = detail.to_ascii_lowercase();
    let hint = if normalized.contains("dockerdesktoplinuxengine")
        || normalized.contains("pipe")
        || normalized.contains("daemon")
        || normalized.contains("cannot find the file specified")
        || normalized.contains("system cannot find")
    {
        if desktop_running {
            "Docker Desktop 已启动，但 Docker Engine 还没有准备好。请等几十秒后再检测；如果一直如此，尝试重启 Docker Desktop。"
        } else {
            "Docker Engine 未运行。请先启动 Docker Desktop，等状态稳定后再检测。"
        }
    } else if normalized.contains("permission")
        || normalized.contains("access is denied")
        || normalized.contains("拒绝访问")
    {
        "当前用户没有访问 Docker Engine 的权限。请确认 Docker Desktop 已允许当前账号使用，必要时以管理员身份启动。"
    } else if desktop_running {
        "Docker Desktop 已启动，但 Docker Engine 暂时不可用。请稍等后重试。"
    } else {
        "Docker Engine 未运行。请先启动 Docker Desktop。"
    };

    format!("{hint}{}", friendly_detail_suffix(detail))
}

fn friendly_detail_suffix(detail: &str) -> String {
    let trimmed = detail.trim();
    if trimmed.is_empty() {
        String::new()
    } else {
        format!(" 原始信息：{}", limit_text(trimmed, 240))
    }
}

fn escape_powershell_single_quoted(value: &str) -> String {
    value.replace('\'', "''")
}
