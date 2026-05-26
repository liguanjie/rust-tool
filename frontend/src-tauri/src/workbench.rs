use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager};

const SETTINGS_KEY: &str = "windows_workbench.config";

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct WorkbenchConfig {
    pub docker_desktop_path: String,
    pub docker_cli_path: String,
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

pub fn get_config(app: &AppHandle) -> Result<WorkbenchConfig, String> {
    let conn = open_db(app)?;
    ensure_schema(&conn)?;
    read_config(&conn)
}

pub fn save_config(app: &AppHandle, config: WorkbenchConfig) -> Result<WorkbenchConfig, String> {
    validate_optional_executable(&config.docker_desktop_path, "Docker Desktop 路径")?;
    validate_optional_executable(&config.docker_cli_path, "docker CLI 路径")?;
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
            Err(stderr)
        }
    }
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

    match value {
        Some(value) => {
            serde_json::from_str(&value).map_err(|error| format!("解析配置失败: {error}"))
        }
        None => Ok(WorkbenchConfig {
            sub2api_health_url: "http://127.0.0.1:9999/v1/models".to_string(),
            sub2api_login_url: "http://127.0.0.1:9999/api/auth/login".to_string(),
            ..WorkbenchConfig::default()
        }),
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

fn is_docker_desktop_running() -> bool {
    ["Docker Desktop.exe", "com.docker.backend.exe"]
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
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs())
        .unwrap_or_default();
    seconds.to_string()
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
