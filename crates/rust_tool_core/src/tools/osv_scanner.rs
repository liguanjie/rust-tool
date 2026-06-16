use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use thiserror::Error;
use toml_edit::{value, ArrayOfTables, DocumentMut, Item, Table};

const OSV_SCANNER_BINARY: &str = "osv-scanner";
const COMMAND_HISTORY_STDERR_LIMIT: usize = 2_000;
const MAX_REASON_LENGTH: usize = 512;
const MAX_VULNERABILITY_ID_LENGTH: usize = 128;
const MAX_ADVANCED_ARGS: usize = 16;
const MAX_REPEATABLE_ARGS: usize = 32;

#[derive(Debug, Error)]
pub enum OsvScannerError {
    #[error("未检测到可用的 osv-scanner，请先安装后再扫描")]
    NotInstalled,
    #[error("项目路径无效: {reason}")]
    InvalidProjectPath { reason: String },
    #[error("命令被拒绝: {0}")]
    CommandRejected(String),
    #[error("扫描失败: {0}")]
    ScanFailed(String),
    #[error("解析 OSV 报告失败: {0}")]
    ReportParseFailed(String),
    #[error("导出报告失败: {0}")]
    ExportFailed(String),
    #[error("报告格式无效")]
    InvalidReportFormat,
    #[error("更新忽略规则失败: {0}")]
    IgnoreUpdateFailed(String),
    #[error("文件操作失败: {0}")]
    Io(String),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OsvInstallStatus {
    pub installed: bool,
    pub binary_path: Option<String>,
    pub version: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum OsvCommandKind {
    Scan,
    Export,
    Fix,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum OsvCommandStatus {
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum OsvReportFormat {
    Json,
    Html,
}

impl OsvReportFormat {
    fn as_osv_format(self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Html => "html",
        }
    }

    fn extension(self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Html => "html",
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct OsvScanOptions {
    pub recursive: bool,
    pub no_ignore: bool,
    pub include_git_root: bool,
    pub config_path: Option<String>,
    pub lockfiles: Vec<String>,
    pub experimental_excludes: Vec<String>,
    pub allow_no_lockfiles: bool,
    pub all_packages: bool,
    pub all_vulns: bool,
    pub offline: bool,
    pub offline_vulnerabilities: bool,
    pub no_resolve: bool,
    pub advanced_args: Vec<String>,
}

impl OsvScanOptions {
    pub fn default_source_scan() -> Self {
        Self {
            recursive: true,
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OsvScanCommandRequest {
    pub project_path: String,
    #[serde(default = "OsvScanOptions::default_source_scan")]
    pub options: OsvScanOptions,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OsvScanRequest {
    pub project_path: String,
    #[serde(default = "OsvScanOptions::default_source_scan")]
    pub options: OsvScanOptions,
    pub command: OsvCommandPreview,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OsvReportExportCommandRequest {
    pub project_path: String,
    #[serde(default = "OsvScanOptions::default_source_scan")]
    pub options: OsvScanOptions,
    pub format: OsvReportFormat,
    pub output_path: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OsvReportExportRequest {
    pub project_path: String,
    #[serde(default = "OsvScanOptions::default_source_scan")]
    pub options: OsvScanOptions,
    pub format: OsvReportFormat,
    pub output_path: String,
    pub command: OsvCommandPreview,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OsvIgnoreRequest {
    pub project_path: String,
    pub vulnerability_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OsvCommandEditableOptions {
    pub recursive: bool,
    pub no_ignore: bool,
    pub include_git_root: bool,
    pub config_path: Option<String>,
    pub lockfiles: Vec<String>,
    pub experimental_excludes: Vec<String>,
    pub allow_no_lockfiles: bool,
    pub all_packages: bool,
    pub all_vulns: bool,
    pub offline: bool,
    pub offline_vulnerabilities: bool,
    pub no_resolve: bool,
    pub advanced_args: Vec<String>,
}

impl From<OsvScanOptions> for OsvCommandEditableOptions {
    fn from(value: OsvScanOptions) -> Self {
        Self {
            recursive: value.recursive,
            no_ignore: value.no_ignore,
            include_git_root: value.include_git_root,
            config_path: value.config_path,
            lockfiles: value.lockfiles,
            experimental_excludes: value.experimental_excludes,
            allow_no_lockfiles: value.allow_no_lockfiles,
            all_packages: value.all_packages,
            all_vulns: value.all_vulns,
            offline: value.offline,
            offline_vulnerabilities: value.offline_vulnerabilities,
            no_resolve: value.no_resolve,
            advanced_args: value.advanced_args,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OsvCommandPreview {
    pub kind: OsvCommandKind,
    pub binary: String,
    pub working_dir: String,
    pub argv: Vec<String>,
    pub display_command: String,
    pub locked_args: Vec<String>,
    pub editable_options: OsvCommandEditableOptions,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OsvCommandExecutionRecord {
    pub id: String,
    pub kind: OsvCommandKind,
    pub project_path: String,
    pub working_dir: String,
    pub argv: Vec<String>,
    pub display_command: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub duration_ms: Option<u64>,
    pub exit_code: Option<i32>,
    pub status: OsvCommandStatus,
    pub summary: String,
    pub stderr_excerpt: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OsvPackageInfo {
    pub name: String,
    pub version: Option<String>,
    pub ecosystem: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum OsvSeverity {
    Critical,
    High,
    Medium,
    Low,
    Unknown,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OsvVulnerabilityFinding {
    pub id: String,
    pub aliases: Vec<String>,
    pub summary: Option<String>,
    pub details: Option<String>,
    pub package: OsvPackageInfo,
    pub severity: OsvSeverity,
    pub affected_paths: Vec<String>,
    pub fixed_versions: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OsvSeverityCounts {
    pub critical: u32,
    pub high: u32,
    pub medium: u32,
    pub low: u32,
    pub unknown: u32,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OsvScanSummary {
    pub total_vulnerabilities: u32,
    pub severity_counts: OsvSeverityCounts,
    pub highest_severity: OsvSeverity,
    pub health_score: u32,
    pub message: String,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OsvScanResult {
    pub project_path: String,
    pub vulnerabilities: Vec<OsvVulnerabilityFinding>,
    pub summary: OsvScanSummary,
    pub command: OsvCommandExecutionRecord,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OsvReportExportResult {
    pub format: OsvReportFormat,
    pub output_path: String,
    pub command: OsvCommandExecutionRecord,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OsvIgnoreResult {
    pub project_path: String,
    pub config_path: String,
    pub vulnerability_id: String,
    pub added: bool,
    pub message: String,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OsvFixResult {
    pub project_path: String,
    pub command: OsvCommandExecutionRecord,
    pub message: String,
}

#[derive(Debug)]
struct PreparedProject {
    path: PathBuf,
    display_path: String,
}

pub fn check_osv_scanner_installed() -> Result<OsvInstallStatus, OsvScannerError> {
    let Some(binary_path) = find_osv_scanner_binary() else {
        return Ok(OsvInstallStatus {
            installed: false,
            binary_path: None,
            version: None,
            message: "未检测到 osv-scanner，请先安装后再使用漏洞扫描。".to_string(),
        });
    };

    let version = Command::new(&binary_path)
        .arg("--version")
        .output()
        .ok()
        .and_then(|output| {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if stdout.is_empty() {
                None
            } else {
                Some(stdout)
            }
        });

    Ok(OsvInstallStatus {
        installed: true,
        binary_path: Some(binary_path.display().to_string()),
        version,
        message: "已检测到 osv-scanner。".to_string(),
    })
}

pub fn build_scan_command(
    request: OsvScanCommandRequest,
) -> Result<OsvCommandPreview, OsvScannerError> {
    let project = prepare_project_path(&request.project_path)?;
    let binary = require_osv_scanner_binary()?;
    let mut argv = base_scan_argv(&binary, OsvReportFormat::Json);
    append_scan_options(&mut argv, &request.options)?;
    argv.push(".".to_string());

    Ok(command_preview(
        OsvCommandKind::Scan,
        binary,
        project,
        argv,
        request.options,
        vec!["scan".to_string(), "source".to_string(), "--format json".to_string()],
    ))
}

pub fn scan_project(request: OsvScanRequest) -> Result<OsvScanResult, OsvScannerError> {
    let expected = build_scan_command(OsvScanCommandRequest {
        project_path: request.project_path.clone(),
        options: request.options,
    })?;
    ensure_confirmed_command_matches(&request.command, &expected)?;

    let mut execution = execute_preview(&expected);
    let stdout = execution
        .stdout
        .as_ref()
        .ok_or_else(|| OsvScannerError::ScanFailed("osv-scanner 没有返回 JSON 输出".to_string()))?;
    let vulnerabilities = parse_osv_report(stdout)?;
    execution.success = true;
    let summary = build_scan_summary(&vulnerabilities);
    let message = if vulnerabilities.is_empty() {
        "未发现已知漏洞。".to_string()
    } else {
        format!("发现 {} 个漏洞。", vulnerabilities.len())
    };
    let command = execution.into_record(message);

    Ok(OsvScanResult {
        project_path: expected.working_dir,
        vulnerabilities,
        summary,
        command,
    })
}

pub fn build_export_command(
    request: OsvReportExportCommandRequest,
) -> Result<OsvCommandPreview, OsvScannerError> {
    let project = prepare_project_path(&request.project_path)?;
    let binary = require_osv_scanner_binary()?;
    let output_path = validate_export_output_path(&request.output_path, request.format)?;
    let mut argv = base_scan_argv(&binary, request.format);
    append_scan_options(&mut argv, &request.options)?;
    argv.push("--output-file".to_string());
    argv.push(output_path.display().to_string());
    argv.push(".".to_string());

    Ok(command_preview(
        OsvCommandKind::Export,
        binary,
        project,
        argv,
        request.options,
        vec![
            "scan".to_string(),
            "source".to_string(),
            format!("--format {}", request.format.as_osv_format()),
            "--output-file".to_string(),
        ],
    ))
}

pub fn export_report(
    request: OsvReportExportRequest,
) -> Result<OsvReportExportResult, OsvScannerError> {
    let expected = build_export_command(OsvReportExportCommandRequest {
        project_path: request.project_path.clone(),
        options: request.options.clone(),
        format: request.format,
        output_path: request.output_path.clone(),
    })?;
    ensure_confirmed_command_matches(&request.command, &expected)?;

    let output_path = validate_export_output_path(&request.output_path, request.format)?;
    let mut execution = execute_preview(&expected);
    if output_path.exists() {
        execution.success = true;
    }
    if !execution.success {
        return Err(OsvScannerError::ExportFailed(
            execution.stderr_excerpt().unwrap_or_else(|| {
                "osv-scanner 导出命令执行失败，未返回可读错误。".to_string()
            }),
        ));
    }

    let command = execution.into_record(format!(
        "已导出 {} 报告。",
        request.format.as_osv_format().to_ascii_uppercase()
    ));

    Ok(OsvReportExportResult {
        format: request.format,
        output_path: output_path.display().to_string(),
        command,
    })
}

pub fn ignore_vulnerability(
    project_path: &str,
    vulnerability_id: &str,
    reason: &str,
) -> Result<OsvIgnoreResult, OsvScannerError> {
    let project = prepare_project_path(project_path)?;
    let id = validate_vulnerability_id(vulnerability_id)?;
    let reason = validate_ignore_reason(reason)?;
    let config_path = project.path.join("osv-scanner.toml");
    let content = if config_path.exists() {
        fs::read_to_string(&config_path).map_err(|error| {
            OsvScannerError::IgnoreUpdateFailed(format!("读取配置失败: {error}"))
        })?
    } else {
        String::new()
    };

    let mut document = content.parse::<DocumentMut>().map_err(|error| {
        OsvScannerError::IgnoreUpdateFailed(format!("解析 osv-scanner.toml 失败: {error}"))
    })?;

    ensure_ignored_vulns_array(&mut document)?;
    let array = document["IgnoredVulns"]
        .as_array_of_tables_mut()
        .ok_or_else(|| {
            OsvScannerError::IgnoreUpdateFailed(
                "IgnoredVulns 配置不是数组表，无法自动更新。".to_string(),
            )
        })?;

    if array.iter().any(|table| {
        table
            .get("id")
            .and_then(Item::as_str)
            .map(|value| value == id)
            .unwrap_or(false)
    }) {
        return Ok(OsvIgnoreResult {
            project_path: project.display_path,
            config_path: config_path.display().to_string(),
            vulnerability_id: id,
            added: false,
            message: "忽略规则已存在，未重复写入。".to_string(),
        });
    }

    let mut table = Table::new();
    table["id"] = value(id.clone());
    table["reason"] = value(reason);
    array.push(table);

    fs::write(&config_path, document.to_string()).map_err(|error| {
        OsvScannerError::IgnoreUpdateFailed(format!("写入 osv-scanner.toml 失败: {error}"))
    })?;

    Ok(OsvIgnoreResult {
        project_path: project.display_path,
        config_path: config_path.display().to_string(),
        vulnerability_id: id,
        added: true,
        message: "已写入忽略规则。".to_string(),
    })
}

pub fn apply_fix(_path: &str) -> Result<OsvFixResult, OsvScannerError> {
    Err(OsvScannerError::CommandRejected(
        "一键修复属于第二阶段能力，当前版本尚未启用。".to_string(),
    ))
}

fn command_preview(
    kind: OsvCommandKind,
    binary: PathBuf,
    project: PreparedProject,
    argv: Vec<String>,
    options: OsvScanOptions,
    locked_args: Vec<String>,
) -> OsvCommandPreview {
    let working_dir = project.display_path;
    let display_command = display_command(&working_dir, &argv);
    OsvCommandPreview {
        kind,
        binary: binary.display().to_string(),
        working_dir,
        argv,
        display_command,
        locked_args,
        editable_options: options.into(),
        warnings: Vec::new(),
    }
}

fn base_scan_argv(binary: &Path, format: OsvReportFormat) -> Vec<String> {
    vec![
        binary.display().to_string(),
        "scan".to_string(),
        "source".to_string(),
        "--format".to_string(),
        format.as_osv_format().to_string(),
    ]
}

fn append_scan_options(
    argv: &mut Vec<String>,
    options: &OsvScanOptions,
) -> Result<(), OsvScannerError> {
    if options.lockfiles.len() > MAX_REPEATABLE_ARGS {
        return Err(OsvScannerError::CommandRejected(
            "lockfile 参数数量过多。".to_string(),
        ));
    }
    if options.experimental_excludes.len() > MAX_REPEATABLE_ARGS {
        return Err(OsvScannerError::CommandRejected(
            "exclude 参数数量过多。".to_string(),
        ));
    }

    if options.recursive {
        argv.push("--recursive".to_string());
    }
    if options.no_ignore {
        argv.push("--no-ignore".to_string());
    }
    if options.include_git_root {
        argv.push("--include-git-root".to_string());
    }
    if let Some(config_path) = optional_trimmed(&options.config_path) {
        validate_cli_value(config_path, "config")?;
        argv.push("--config".to_string());
        argv.push(config_path.to_string());
    }
    for lockfile in &options.lockfiles {
        let value = validate_non_empty_cli_value(lockfile, "lockfile")?;
        argv.push("--lockfile".to_string());
        argv.push(value.to_string());
    }
    for exclude in &options.experimental_excludes {
        let value = validate_non_empty_cli_value(exclude, "experimental-exclude")?;
        argv.push("--experimental-exclude".to_string());
        argv.push(value.to_string());
    }
    if options.allow_no_lockfiles {
        argv.push("--allow-no-lockfiles".to_string());
    }
    if options.all_packages {
        argv.push("--all-packages".to_string());
    }
    if options.all_vulns {
        argv.push("--all-vulns".to_string());
    }
    if options.offline {
        argv.push("--offline".to_string());
    }
    if options.offline_vulnerabilities {
        argv.push("--offline-vulnerabilities".to_string());
    }
    if options.no_resolve {
        argv.push("--no-resolve".to_string());
    }
    append_advanced_args(argv, &options.advanced_args)?;

    Ok(())
}

fn append_advanced_args(argv: &mut Vec<String>, args: &[String]) -> Result<(), OsvScannerError> {
    if args.len() > MAX_ADVANCED_ARGS {
        return Err(OsvScannerError::CommandRejected(
            "高级参数数量过多。".to_string(),
        ));
    }

    for arg in args {
        let arg = validate_non_empty_cli_value(arg, "advanced arg")?;
        if !matches!(
            arg,
            "--no-ignore"
                | "--include-git-root"
                | "--allow-no-lockfiles"
                | "--all-packages"
                | "--all-vulns"
                | "--offline"
                | "--offline-vulnerabilities"
                | "--no-resolve"
        ) {
            return Err(OsvScannerError::CommandRejected(format!(
                "高级参数不在 allowlist 中: {arg}"
            )));
        }
        argv.push(arg.to_string());
    }

    Ok(())
}

fn ensure_confirmed_command_matches(
    actual: &OsvCommandPreview,
    expected: &OsvCommandPreview,
) -> Result<(), OsvScannerError> {
    if actual.kind != expected.kind
        || actual.working_dir != expected.working_dir
        || actual.argv != expected.argv
    {
        return Err(OsvScannerError::CommandRejected(
            "确认命令与当前结构化参数不一致，请重新生成预览。".to_string(),
        ));
    }
    Ok(())
}

struct CommandExecution {
    preview: OsvCommandPreview,
    project_path: String,
    started_at: String,
    finished_at: String,
    duration_ms: u64,
    exit_code: Option<i32>,
    success: bool,
    stdout: Option<Vec<u8>>,
    stderr: Vec<u8>,
}

impl CommandExecution {
    fn stderr_excerpt(&self) -> Option<String> {
        let text = String::from_utf8_lossy(&self.stderr).trim().to_string();
        if text.is_empty() {
            None
        } else {
            Some(truncate_chars(&text, COMMAND_HISTORY_STDERR_LIMIT))
        }
    }

    fn into_record(self, summary: String) -> OsvCommandExecutionRecord {
        let stderr_excerpt = self.stderr_excerpt();
        OsvCommandExecutionRecord {
            id: command_record_id(),
            kind: self.preview.kind,
            project_path: self.project_path,
            working_dir: self.preview.working_dir,
            argv: self.preview.argv,
            display_command: self.preview.display_command,
            started_at: self.started_at,
            finished_at: Some(self.finished_at),
            duration_ms: Some(self.duration_ms),
            exit_code: self.exit_code,
            status: if self.success {
                OsvCommandStatus::Succeeded
            } else {
                OsvCommandStatus::Failed
            },
            summary,
            stderr_excerpt,
        }
    }
}

fn execute_preview(preview: &OsvCommandPreview) -> CommandExecution {
    let started_at = now_millis_string();
    let started = Instant::now();
    let output = Command::new(&preview.argv[0])
        .args(&preview.argv[1..])
        .current_dir(&preview.working_dir)
        .output();
    let duration_ms = started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
    let finished_at = now_millis_string();

    match output {
        Ok(output) => CommandExecution {
            preview: preview.clone(),
            project_path: preview.working_dir.clone(),
            started_at,
            finished_at,
            duration_ms,
            exit_code: output.status.code(),
            success: output.status.success(),
            stdout: Some(output.stdout),
            stderr: output.stderr,
        },
        Err(error) => CommandExecution {
            preview: preview.clone(),
            project_path: preview.working_dir.clone(),
            started_at,
            finished_at,
            duration_ms,
            exit_code: None,
            success: false,
            stdout: None,
            stderr: error.to_string().into_bytes(),
        },
    }
}

fn parse_osv_report(stdout: &[u8]) -> Result<Vec<OsvVulnerabilityFinding>, OsvScannerError> {
    let value: Value = serde_json::from_slice(stdout)
        .map_err(|error| OsvScannerError::ReportParseFailed(error.to_string()))?;
    let results = match value.get("results") {
        Some(Value::Array(results)) => results,
        Some(Value::Null) => return Ok(Vec::new()),
        Some(_) => {
            return Err(OsvScannerError::ReportParseFailed(
                "OSV JSON 中 results 字段类型无效。".to_string(),
            ));
        }
        None => {
            return Err(OsvScannerError::ReportParseFailed(
                "OSV JSON 缺少 results 字段。".to_string(),
            ));
        }
    };
    let mut findings = Vec::new();

    for result in results {
        let source = extract_source_path(result);
        for package_entry in result
            .get("packages")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let package = extract_package_info(package_entry);
            for vulnerability in package_entry
                .get("vulnerabilities")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                if let Some(id) = vulnerability.get("id").and_then(Value::as_str) {
                    findings.push(OsvVulnerabilityFinding {
                        id: id.to_string(),
                        aliases: string_array(vulnerability.get("aliases")),
                        summary: vulnerability
                            .get("summary")
                            .and_then(Value::as_str)
                            .map(ToString::to_string),
                        details: vulnerability
                            .get("details")
                            .and_then(Value::as_str)
                            .map(ToString::to_string),
                        package: package.clone(),
                        severity: extract_severity(vulnerability),
                        affected_paths: source.to_vec(),
                        fixed_versions: extract_fixed_versions(vulnerability),
                    });
                }
            }
        }
    }

    Ok(findings)
}

fn extract_source_path(result: &Value) -> Vec<String> {
    let mut paths = Vec::new();
    if let Some(path) = result
        .get("source")
        .and_then(|source| source.get("path"))
        .and_then(Value::as_str)
    {
        paths.push(path.to_string());
    }
    if let Some(path) = result.get("path").and_then(Value::as_str) {
        paths.push(path.to_string());
    }
    paths.sort();
    paths.dedup();
    paths
}

fn extract_package_info(package_entry: &Value) -> OsvPackageInfo {
    let package = package_entry.get("package").unwrap_or(package_entry);
    OsvPackageInfo {
        name: package
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string(),
        version: package
            .get("version")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        ecosystem: package
            .get("ecosystem")
            .and_then(Value::as_str)
            .map(ToString::to_string),
    }
}

fn extract_severity(vulnerability: &Value) -> OsvSeverity {
    if let Some(severity) = vulnerability
        .get("database_specific")
        .and_then(|value| value.get("severity"))
        .and_then(Value::as_str)
    {
        return severity_from_label(severity);
    }

    vulnerability
        .get("severity")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.get("score").and_then(Value::as_str))
        .map(severity_from_cvss_score)
        .min()
        .unwrap_or(OsvSeverity::Unknown)
}

fn severity_from_label(value: &str) -> OsvSeverity {
    match value.trim().to_ascii_lowercase().as_str() {
        "critical" => OsvSeverity::Critical,
        "high" => OsvSeverity::High,
        "medium" | "moderate" => OsvSeverity::Medium,
        "low" => OsvSeverity::Low,
        _ => OsvSeverity::Unknown,
    }
}

fn severity_from_cvss_score(value: &str) -> OsvSeverity {
    let lower = value.to_ascii_lowercase();
    if lower.contains("critical") {
        OsvSeverity::Critical
    } else if lower.contains("high") {
        OsvSeverity::High
    } else if lower.contains("medium") || lower.contains("moderate") {
        OsvSeverity::Medium
    } else if lower.contains("low") {
        OsvSeverity::Low
    } else if let Ok(score) = value.trim().parse::<f64>() {
        severity_from_numeric_cvss(score)
    } else if let Some(score) = cvss_v3_base_score(value) {
        severity_from_numeric_cvss(score)
    } else {
        OsvSeverity::Unknown
    }
}

fn severity_from_numeric_cvss(score: f64) -> OsvSeverity {
    if score >= 9.0 {
        OsvSeverity::Critical
    } else if score >= 7.0 {
        OsvSeverity::High
    } else if score >= 4.0 {
        OsvSeverity::Medium
    } else if score > 0.0 {
        OsvSeverity::Low
    } else {
        OsvSeverity::Unknown
    }
}

fn cvss_v3_base_score(vector: &str) -> Option<f64> {
    let vector = vector.trim();
    if !(vector.starts_with("CVSS:3.0/") || vector.starts_with("CVSS:3.1/")) {
        return None;
    }

    let mut av = None;
    let mut ac = None;
    let mut pr = None;
    let mut ui = None;
    let mut scope = None;
    let mut c = None;
    let mut i = None;
    let mut a = None;

    for metric in vector.split('/').skip(1) {
        let (name, value) = metric.split_once(':')?;
        match name {
            "AV" => {
                av = match value {
                    "N" => Some(0.85),
                    "A" => Some(0.62),
                    "L" => Some(0.55),
                    "P" => Some(0.20),
                    _ => return None,
                };
            }
            "AC" => {
                ac = match value {
                    "L" => Some(0.77),
                    "H" => Some(0.44),
                    _ => return None,
                };
            }
            "PR" => {
                pr = match value {
                    "N" => Some(("N", 0.85, 0.85)),
                    "L" => Some(("L", 0.62, 0.68)),
                    "H" => Some(("H", 0.27, 0.50)),
                    _ => return None,
                };
            }
            "UI" => {
                ui = match value {
                    "N" => Some(0.85),
                    "R" => Some(0.62),
                    _ => return None,
                };
            }
            "S" => {
                scope = match value {
                    "U" => Some(false),
                    "C" => Some(true),
                    _ => return None,
                };
            }
            "C" => c = cvss_impact_weight(value),
            "I" => i = cvss_impact_weight(value),
            "A" => a = cvss_impact_weight(value),
            _ => {}
        }
    }

    let scope_changed = scope?;
    let pr = pr?;
    let pr_weight = if scope_changed { pr.2 } else { pr.1 };
    let impact_sub_score = 1.0 - (1.0 - c?) * (1.0 - i?) * (1.0 - a?);
    let impact = if scope_changed {
        7.52 * (impact_sub_score - 0.029) - 3.25 * (impact_sub_score - 0.02_f64).powf(15.0)
    } else {
        6.42 * impact_sub_score
    };
    if impact <= 0.0 {
        return Some(0.0);
    }

    let exploitability = 8.22 * av? * ac? * pr_weight * ui?;
    let score = if scope_changed {
        1.08 * (impact + exploitability)
    } else {
        impact + exploitability
    };
    Some(round_up_one_decimal(score.min(10.0)))
}

fn cvss_impact_weight(value: &str) -> Option<f64> {
    match value {
        "H" => Some(0.56),
        "L" => Some(0.22),
        "N" => Some(0.0),
        _ => None,
    }
}

fn round_up_one_decimal(value: f64) -> f64 {
    (value * 10.0).ceil() / 10.0
}

fn extract_fixed_versions(vulnerability: &Value) -> Vec<String> {
    let mut versions = Vec::new();
    for affected in vulnerability
        .get("affected")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        for range in affected
            .get("ranges")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            for event in range
                .get("events")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                if let Some(fixed) = event.get("fixed").and_then(Value::as_str) {
                    versions.push(fixed.to_string());
                }
            }
        }
    }
    versions.sort();
    versions.dedup();
    versions
}

fn string_array(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(ToString::to_string)
        .collect()
}

fn build_scan_summary(vulnerabilities: &[OsvVulnerabilityFinding]) -> OsvScanSummary {
    let mut counts = OsvSeverityCounts::default();
    let mut highest = OsvSeverity::Unknown;

    for vulnerability in vulnerabilities {
        highest = highest.min(vulnerability.severity);
        match vulnerability.severity {
            OsvSeverity::Critical => counts.critical += 1,
            OsvSeverity::High => counts.high += 1,
            OsvSeverity::Medium => counts.medium += 1,
            OsvSeverity::Low => counts.low += 1,
            OsvSeverity::Unknown => counts.unknown += 1,
        }
    }

    if vulnerabilities.is_empty() {
        highest = OsvSeverity::Unknown;
    }

    let health_score = health_score(&counts);
    let total_vulnerabilities = vulnerabilities.len() as u32;
    let message = if total_vulnerabilities == 0 {
        "未发现已知漏洞。".to_string()
    } else {
        format!("发现 {total_vulnerabilities} 个漏洞，健康分 {health_score}。")
    };

    OsvScanSummary {
        total_vulnerabilities,
        severity_counts: counts,
        highest_severity: highest,
        health_score,
        message,
    }
}

fn health_score(counts: &OsvSeverityCounts) -> u32 {
    let penalty = counts.critical * 30
        + counts.high * 20
        + counts.medium * 10
        + counts.low * 5
        + counts.unknown * 3;
    100_u32.saturating_sub(penalty.min(100))
}

fn find_osv_scanner_binary() -> Option<PathBuf> {
    let paths = std::env::var_os("PATH")?;
    for path in std::env::split_paths(&paths) {
        let candidate = path.join(OSV_SCANNER_BINARY);
        if candidate.is_file() {
            return Some(candidate);
        }
        #[cfg(windows)]
        {
            let candidate = path.join("osv-scanner.exe");
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

fn require_osv_scanner_binary() -> Result<PathBuf, OsvScannerError> {
    find_osv_scanner_binary().ok_or(OsvScannerError::NotInstalled)
}

fn prepare_project_path(path: &str) -> Result<PreparedProject, OsvScannerError> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err(OsvScannerError::InvalidProjectPath {
            reason: "路径不能为空".to_string(),
        });
    }
    if trimmed.len() > 4096 {
        return Err(OsvScannerError::InvalidProjectPath {
            reason: "路径过长".to_string(),
        });
    }
    let path = PathBuf::from(trimmed);
    let canonical = path
        .canonicalize()
        .map_err(|error| OsvScannerError::InvalidProjectPath {
            reason: format!("无法解析路径: {error}"),
        })?;
    if !canonical.is_dir() {
        return Err(OsvScannerError::InvalidProjectPath {
            reason: "目标不是目录".to_string(),
        });
    }

    Ok(PreparedProject {
        display_path: canonical.display().to_string(),
        path: canonical,
    })
}

fn validate_export_output_path(
    output_path: &str,
    format: OsvReportFormat,
) -> Result<PathBuf, OsvScannerError> {
    let trimmed = output_path.trim();
    if trimmed.is_empty() {
        return Err(OsvScannerError::ExportFailed(
            "导出路径不能为空。".to_string(),
        ));
    }
    validate_cli_value(trimmed, "output-file")?;
    let path = PathBuf::from(trimmed);
    if !path.is_absolute() {
        return Err(OsvScannerError::ExportFailed(
            "导出路径必须是绝对路径。".to_string(),
        ));
    }
    if path.exists() {
        return Err(OsvScannerError::ExportFailed(
            "导出文件已存在，请选择新的文件名。".to_string(),
        ));
    }
    let file_name = path.file_name().ok_or_else(|| {
        OsvScannerError::ExportFailed("导出路径必须包含文件名。".to_string())
    })?;
    let parent = path.parent().ok_or_else(|| {
        OsvScannerError::ExportFailed("导出路径必须包含父目录。".to_string())
    })?;
    if !parent.is_dir() {
        return Err(OsvScannerError::ExportFailed(
            "导出目录不存在。".to_string(),
        ));
    }
    let extension = path.extension().and_then(OsStr::to_str).unwrap_or_default();
    if !extension.eq_ignore_ascii_case(format.extension()) {
        return Err(OsvScannerError::InvalidReportFormat);
    }
    if is_sensitive_project_file(&path) {
        return Err(OsvScannerError::ExportFailed(
            "导出路径不能覆盖源码锁文件或 OSV 配置文件。".to_string(),
        ));
    }

    let parent = parent.canonicalize().map_err(|error| {
        OsvScannerError::ExportFailed(format!("无法解析导出目录: {error}"))
    })?;
    Ok(parent.join(file_name))
}

fn is_sensitive_project_file(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(OsStr::to_str),
        Some("Cargo.lock")
            | Some("package-lock.json")
            | Some("pnpm-lock.yaml")
            | Some("yarn.lock")
            | Some("go.sum")
            | Some("osv-scanner.toml")
    )
}

fn validate_cli_value<'a>(value: &'a str, name: &str) -> Result<&'a str, OsvScannerError> {
    if value.chars().any(char::is_control) {
        return Err(OsvScannerError::CommandRejected(format!(
            "{name} 包含控制字符。"
        )));
    }
    Ok(value)
}

fn validate_non_empty_cli_value<'a>(
    value: &'a str,
    name: &str,
) -> Result<&'a str, OsvScannerError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(OsvScannerError::CommandRejected(format!(
            "{name} 不能为空。"
        )));
    }
    if has_shell_control(value) || looks_like_env_assignment(value) {
        return Err(OsvScannerError::CommandRejected(format!(
            "{name} 包含不允许的 shell 片段。"
        )));
    }
    validate_cli_value(value, name)
}

fn has_shell_control(value: &str) -> bool {
    value
        .chars()
        .any(|character| matches!(character, '|' | '&' | ';' | '<' | '>' | '`' | '$' | '(' | ')'))
}

fn looks_like_env_assignment(value: &str) -> bool {
    let Some((name, _)) = value.split_once('=') else {
        return false;
    };
    !name.is_empty()
        && name
            .chars()
            .all(|character| character.is_ascii_uppercase() || character == '_')
}

fn optional_trimmed(value: &Option<String>) -> Option<&str> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn display_command(working_dir: &str, argv: &[String]) -> String {
    let args = argv
        .iter()
        .map(|arg| shell_quote(arg))
        .collect::<Vec<_>>()
        .join(" ");
    format!("cd {} && {args}", shell_quote(working_dir))
}

fn shell_quote(value: &str) -> String {
    if value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '/' | '.' | '_' | '-' | ':'))
    {
        return value.to_string();
    }
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn now_millis_string() -> String {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_millis().to_string(),
        Err(_) => "0".to_string(),
    }
}

fn command_record_id() -> String {
    format!("osv-{}", now_millis_string())
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let truncated = chars.by_ref().take(max_chars).collect::<String>();
    if chars.next().is_some() {
        format!("{truncated}...")
    } else {
        truncated
    }
}

fn validate_vulnerability_id(id: &str) -> Result<String, OsvScannerError> {
    let id = id.trim();
    if id.is_empty() {
        return Err(OsvScannerError::IgnoreUpdateFailed(
            "漏洞 ID 不能为空。".to_string(),
        ));
    }
    if id.len() > MAX_VULNERABILITY_ID_LENGTH {
        return Err(OsvScannerError::IgnoreUpdateFailed(
            "漏洞 ID 过长。".to_string(),
        ));
    }
    if !id
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.' | ':'))
    {
        return Err(OsvScannerError::IgnoreUpdateFailed(
            "漏洞 ID 包含非法字符。".to_string(),
        ));
    }
    Ok(id.to_string())
}

fn validate_ignore_reason(reason: &str) -> Result<String, OsvScannerError> {
    let reason = reason.trim();
    if reason.is_empty() {
        return Err(OsvScannerError::IgnoreUpdateFailed(
            "忽略原因不能为空。".to_string(),
        ));
    }
    if reason.len() > MAX_REASON_LENGTH {
        return Err(OsvScannerError::IgnoreUpdateFailed(
            "忽略原因过长。".to_string(),
        ));
    }
    if reason.chars().any(char::is_control) {
        return Err(OsvScannerError::IgnoreUpdateFailed(
            "忽略原因包含控制字符。".to_string(),
        ));
    }
    Ok(reason.to_string())
}

fn ensure_ignored_vulns_array(document: &mut DocumentMut) -> Result<(), OsvScannerError> {
    if !document.as_table().contains_key("IgnoredVulns") {
        document["IgnoredVulns"] = Item::ArrayOfTables(ArrayOfTables::new());
        return Ok(());
    }

    if document["IgnoredVulns"].is_array_of_tables() {
        Ok(())
    } else {
        Err(OsvScannerError::IgnoreUpdateFailed(
            "IgnoredVulns 配置不是数组表，无法自动更新。".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn builds_default_scan_preview_with_json_locked() {
        let project = std::env::current_dir().unwrap();
        let preview = build_scan_command(OsvScanCommandRequest {
            project_path: project.display().to_string(),
            options: OsvScanOptions::default_source_scan(),
        })
        .unwrap();

        assert_eq!(preview.kind, OsvCommandKind::Scan);
        assert!(preview.argv.contains(&"--format".to_string()));
        assert!(preview.argv.contains(&"json".to_string()));
        assert!(preview.argv.contains(&"--recursive".to_string()));
        assert_eq!(preview.argv.last().unwrap(), ".");
    }

    #[test]
    fn rejects_unlisted_advanced_args() {
        let project = std::env::current_dir().unwrap();
        let error = build_scan_command(OsvScanCommandRequest {
            project_path: project.display().to_string(),
            options: OsvScanOptions {
                advanced_args: vec!["--serve".to_string()],
                ..OsvScanOptions::default_source_scan()
            },
        })
        .unwrap_err();

        assert!(matches!(error, OsvScannerError::CommandRejected(_)));
    }

    #[test]
    fn parses_osv_json_findings() {
        let json = br#"{
          "results": [{
            "source": {"path": "Cargo.lock"},
            "packages": [{
              "package": {"name": "demo", "version": "1.0.0", "ecosystem": "crates.io"},
              "vulnerabilities": [{
                "id": "RUSTSEC-2024-0001",
                "summary": "demo vuln",
                "database_specific": {"severity": "HIGH"},
                "aliases": ["CVE-2024-0001"],
                "affected": [{"ranges": [{"events": [{"introduced": "0"}, {"fixed": "1.0.1"}]}]}]
              }]
            }]
          }]
        }"#;

        let findings = parse_osv_report(json).unwrap();
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].id, "RUSTSEC-2024-0001");
        assert_eq!(findings[0].severity, OsvSeverity::High);
        assert_eq!(findings[0].fixed_versions, vec!["1.0.1"]);
    }

    #[test]
    fn parses_cvss_vector_severity() {
        let json = br#"{
          "results": [{
            "source": {"path": "package-lock.json"},
            "packages": [{
              "package": {"name": "demo", "version": "1.0.0", "ecosystem": "npm"},
              "vulnerabilities": [{
                "id": "GHSA-test-test-test",
                "severity": [{
                  "type": "CVSS_V3",
                  "score": "CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H"
                }]
              }]
            }]
          }]
        }"#;

        let findings = parse_osv_report(json).unwrap();
        assert_eq!(findings[0].severity, OsvSeverity::Critical);
    }

    #[test]
    fn rejects_json_without_results_array() {
        let error = parse_osv_report(br#"{"error":"temporary unavailable"}"#).unwrap_err();

        assert!(matches!(error, OsvScannerError::ReportParseFailed(_)));
    }

    #[test]
    fn computes_health_score_from_severity_counts() {
        let score = health_score(&OsvSeverityCounts {
            critical: 1,
            high: 1,
            medium: 1,
            low: 1,
            unknown: 1,
        });
        assert_eq!(score, 32);
    }

    #[test]
    fn writes_ignore_rule_without_duplicates() {
        let root = unique_temp_dir();
        fs::create_dir_all(&root).unwrap();

        let first = ignore_vulnerability(
            root.to_str().unwrap(),
            "GHSA-xxxx-yyyy-zzzz",
            "Only used in local tests",
        )
        .unwrap();
        let second = ignore_vulnerability(
            root.to_str().unwrap(),
            "GHSA-xxxx-yyyy-zzzz",
            "Only used in local tests",
        )
        .unwrap();

        assert!(first.added);
        assert!(!second.added);
        let content = fs::read_to_string(root.join("osv-scanner.toml")).unwrap();
        assert_eq!(content.matches("[[IgnoredVulns]]").count(), 1);
        assert!(content.contains("id = \"GHSA-xxxx-yyyy-zzzz\""));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn export_preview_rejects_extension_mismatch() {
        let project = std::env::current_dir().unwrap();
        let output = std::env::temp_dir().join("rusttool-osv-report.txt");
        let error = build_export_command(OsvReportExportCommandRequest {
            project_path: project.display().to_string(),
            options: OsvScanOptions::default_source_scan(),
            format: OsvReportFormat::Html,
            output_path: output.display().to_string(),
        })
        .unwrap_err();

        assert!(matches!(error, OsvScannerError::InvalidReportFormat));
    }

    #[test]
    fn export_preview_rejects_relative_output_path() {
        let project = std::env::current_dir().unwrap();
        let error = build_export_command(OsvReportExportCommandRequest {
            project_path: project.display().to_string(),
            options: OsvScanOptions::default_source_scan(),
            format: OsvReportFormat::Json,
            output_path: "report.json".to_string(),
        })
        .unwrap_err();

        assert!(matches!(error, OsvScannerError::ExportFailed(_)));
    }

    #[test]
    fn export_preview_trims_and_normalizes_output_path() {
        let project = std::env::current_dir().unwrap();
        let output = std::env::temp_dir().join(format!(
            "rusttool-osv-normalized-{}.json",
            NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let preview = build_export_command(OsvReportExportCommandRequest {
            project_path: project.display().to_string(),
            options: OsvScanOptions::default_source_scan(),
            format: OsvReportFormat::Json,
            output_path: format!(" {} ", output.display()),
        })
        .unwrap();

        let output_index = preview
            .argv
            .iter()
            .position(|arg| arg == "--output-file")
            .unwrap()
            + 1;
        let expected = output
            .parent()
            .unwrap()
            .canonicalize()
            .unwrap()
            .join(output.file_name().unwrap());
        assert_eq!(preview.argv[output_index], expected.display().to_string());
    }

    #[test]
    fn scans_empty_fixture_with_real_osv_scanner_when_available() {
        if !check_osv_scanner_installed().unwrap().installed {
            return;
        }

        let root = unique_temp_dir();
        fs::create_dir_all(&root).unwrap();
        let options = OsvScanOptions {
            recursive: false,
            allow_no_lockfiles: true,
            ..OsvScanOptions::default()
        };
        let preview = build_scan_command(OsvScanCommandRequest {
            project_path: root.display().to_string(),
            options: options.clone(),
        })
        .unwrap();

        let result = scan_project(OsvScanRequest {
            project_path: root.display().to_string(),
            options,
            command: preview,
        })
        .unwrap();

        assert_eq!(result.summary.total_vulnerabilities, 0);
        assert_eq!(result.summary.health_score, 100);
        assert_eq!(result.command.status, OsvCommandStatus::Succeeded);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn exports_json_and_html_with_real_osv_scanner_when_available() {
        if !check_osv_scanner_installed().unwrap().installed {
            return;
        }

        let root = unique_temp_dir();
        fs::create_dir_all(&root).unwrap();
        let options = OsvScanOptions {
            recursive: false,
            allow_no_lockfiles: true,
            ..OsvScanOptions::default()
        };

        for (format, filename) in [
            (OsvReportFormat::Json, "report.json"),
            (OsvReportFormat::Html, "report.html"),
        ] {
            let output_path = root.join(filename);
            let preview = build_export_command(OsvReportExportCommandRequest {
                project_path: root.display().to_string(),
                options: options.clone(),
                format,
                output_path: output_path.display().to_string(),
            })
            .unwrap();
            let result = export_report(OsvReportExportRequest {
                project_path: root.display().to_string(),
                options: options.clone(),
                format,
                output_path: output_path.display().to_string(),
                command: preview,
            })
            .unwrap();

            assert_eq!(result.format, format);
            assert!(output_path.exists());
            assert_eq!(result.command.status, OsvCommandStatus::Succeeded);
        }

        fs::remove_dir_all(root).unwrap();
    }

    fn unique_temp_dir() -> PathBuf {
        let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "rusttool-osv-test-{}-{nanos}-{id}",
            std::process::id()
        ))
    }
}
