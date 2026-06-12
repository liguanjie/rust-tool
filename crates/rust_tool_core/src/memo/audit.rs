use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum FindingSeverity {
    Critical,
    Warning,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum FindingKind {
    HardcodedSecret,
    WeakJwt,
    InsecureLink,
    SensitiveOperation,
    GovernanceGap,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum FindingStatus {
    Open,
    Fixed,
    Ignored,
    Reviewing,
}

impl FindingStatus {
    pub fn from_storage(value: &str) -> Self {
        match value {
            "fixed" => Self::Fixed,
            "ignored" => Self::Ignored,
            "reviewing" => Self::Reviewing,
            _ => Self::Open,
        }
    }

    pub fn as_storage(&self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Fixed => "fixed",
            Self::Ignored => "ignored",
            Self::Reviewing => "reviewing",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityFinding {
    pub id: String,
    pub doc_id: String,
    pub line_start: usize,
    pub line_end: usize,
    pub severity: FindingSeverity,
    pub kind: FindingKind,
    pub title: String,
    pub detail: String,
    pub evidence: String,
    pub recommendation: String,
    pub status: FindingStatus,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditSummary {
    pub total: usize,
    pub critical: usize,
    pub warning: usize,
    pub info: usize,
    pub open: usize,
    pub ignored: usize,
    pub fixed: usize,
    pub reviewing: usize,
    pub last_scanned_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditScanResponse {
    pub findings: Vec<SecurityFinding>,
    pub summary: AuditSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditFixPreview {
    pub patch_markdown: String,
    pub explanation: String,
}

pub fn scan_markdown(
    doc_id: &str,
    markdown: &str,
    stored_statuses: &HashMap<String, String>,
    scanned_at: i64,
) -> AuditScanResponse {
    let mut findings = Vec::new();

    for (line_index, line) in markdown.lines().enumerate() {
        let line_number = line_index + 1;
        findings.extend(scan_line(doc_id, line_number, line));
    }

    for finding in &mut findings {
        if let Some(status) = stored_statuses.get(&finding.id) {
            finding.status = FindingStatus::from_storage(status);
        }
    }

    AuditScanResponse {
        summary: summarize_findings(&findings, scanned_at),
        findings,
    }
}

pub fn summarize_findings(findings: &[SecurityFinding], scanned_at: i64) -> AuditSummary {
    let mut summary = AuditSummary {
        total: findings.len(),
        last_scanned_at: scanned_at,
        ..AuditSummary::default()
    };

    for finding in findings {
        match finding.severity {
            FindingSeverity::Critical => summary.critical += 1,
            FindingSeverity::Warning => summary.warning += 1,
            FindingSeverity::Info => summary.info += 1,
        }
        match finding.status {
            FindingStatus::Open => summary.open += 1,
            FindingStatus::Fixed => summary.fixed += 1,
            FindingStatus::Ignored => summary.ignored += 1,
            FindingStatus::Reviewing => summary.reviewing += 1,
        }
    }

    summary
}

pub fn preview_fix(doc_id: &str, markdown: &str, finding_id: &str) -> Option<AuditFixPreview> {
    let response = scan_markdown(doc_id, markdown, &HashMap::new(), 0);
    let finding = response
        .findings
        .into_iter()
        .find(|finding| finding.id == finding_id || finding.id.ends_with(finding_id))?;

    let mut lines = markdown.lines().map(str::to_string).collect::<Vec<_>>();
    let line_index = finding.line_start.saturating_sub(1);
    let line = lines.get(line_index)?.clone();

    let (next_line, explanation) = match finding.kind {
        FindingKind::HardcodedSecret => {
            let replacement_key =
                secret_placeholder_key(&line).unwrap_or_else(|| "managedSecret".to_string());
            let next = replace_secret_value(&line, &replacement_key)?;
            (
                next,
                format!(
                    "将第 {} 行的疑似明文密钥替换为 {{{{secret:{}}}}} 占位符；保存时请把真实值写入本地 KDBX 密码库。",
                    finding.line_start, replacement_key
                ),
            )
        }
        FindingKind::WeakJwt => (
            line.replace("HS256", "RS256").replace("hs256", "RS256"),
            format!(
                "将第 {} 行的弱 JWT 签名算法建议替换为非对称算法 RS256。",
                finding.line_start
            ),
        ),
        FindingKind::InsecureLink => (
            line.replace("http://", "https://"),
            format!(
                "将第 {} 行的明文 HTTP 链接候选改为 HTTPS。",
                finding.line_start
            ),
        ),
        FindingKind::SensitiveOperation => (
            format!("{line} （需补充审批、有效期和回滚方案）"),
            format!(
                "为第 {} 行的高风险运维动作补充治理要求。",
                finding.line_start
            ),
        ),
        FindingKind::GovernanceGap => (
            format!("{line} （需补充负责人、到期时间和复核记录）"),
            format!(
                "为第 {} 行的治理例外补充负责人、到期和复核信息。",
                finding.line_start
            ),
        ),
    };

    lines[line_index] = next_line;
    Some(AuditFixPreview {
        patch_markdown: lines.join("\n"),
        explanation,
    })
}

fn scan_line(doc_id: &str, line_number: usize, line: &str) -> Vec<SecurityFinding> {
    let mut findings = Vec::new();
    let lower = line.to_ascii_lowercase();

    if let Some(evidence) = detect_hardcoded_secret(line, &lower) {
        findings.push(build_finding(
            doc_id,
            line_number,
            FindingSeverity::Critical,
            FindingKind::HardcodedSecret,
            "硬编码密钥泄露",
            "发现疑似明文密钥、Token、密码或私钥片段。",
            evidence,
            "将真实值移入本地 KDBX 密码库或环境变量，文档中只保留 secret 占位符。",
        ));
    }

    if lower.contains("hs256") || lower.contains("alg\":\"none") || lower.contains("alg: none") {
        findings.push(build_finding(
            doc_id,
            line_number,
            FindingSeverity::Warning,
            FindingKind::WeakJwt,
            "JWT 签名弱点",
            "文档中出现 HS256 或 none 等需要复核的 JWT 签名配置。",
            compact_evidence(line),
            "优先使用 RS256 / ES256，并明确密钥强度、轮换和验证策略。",
        ));
    }

    if lower.contains("http://") {
        findings.push(build_finding(
            doc_id,
            line_number,
            FindingSeverity::Warning,
            FindingKind::InsecureLink,
            "第三方 HTTP 链接",
            "发现明文 HTTP 链接，可能导致传输内容被窃听或篡改。",
            mask_urls(line),
            "改用 HTTPS；如必须使用内网明文协议，需要记录网络边界和补偿控制。",
        ));
    }

    if contains_any(
        &lower,
        &[
            "跳过鉴权",
            "关闭校验",
            "禁用认证",
            "disable auth",
            "skip auth",
            "bypass auth",
        ],
    ) {
        findings.push(build_finding(
            doc_id,
            line_number,
            FindingSeverity::Critical,
            FindingKind::SensitiveOperation,
            "高风险安全开关",
            "发现跳过鉴权、关闭校验或绕过认证的描述。",
            compact_evidence(line),
            "补充审批依据、影响范围、有效期、监控和回滚方案。",
        ));
    }

    if contains_any(
        &lower,
        &[
            "风险接受",
            "临时例外",
            "安全例外",
            "accepted risk",
            "temporary exception",
        ],
    ) {
        findings.push(build_finding(
            doc_id,
            line_number,
            FindingSeverity::Info,
            FindingKind::GovernanceGap,
            "安全例外缺少治理信息",
            "发现风险接受或安全例外描述，需要明确治理字段。",
            compact_evidence(line),
            "记录接受原因、负责人、到期时间、补偿控制和复核计划。",
        ));
    }

    findings
}

fn build_finding(
    doc_id: &str,
    line_number: usize,
    severity: FindingSeverity,
    kind: FindingKind,
    title: &str,
    detail: &str,
    evidence: String,
    recommendation: &str,
) -> SecurityFinding {
    let id = stable_finding_id(doc_id, line_number, &kind, &evidence);
    SecurityFinding {
        id,
        doc_id: doc_id.to_string(),
        line_start: line_number,
        line_end: line_number,
        severity,
        kind,
        title: title.to_string(),
        detail: detail.to_string(),
        evidence,
        recommendation: recommendation.to_string(),
        status: FindingStatus::Open,
    }
}

fn stable_finding_id(
    doc_id: &str,
    line_number: usize,
    kind: &FindingKind,
    evidence: &str,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(doc_id.as_bytes());
    hasher.update(line_number.to_string().as_bytes());
    hasher.update(format!("{kind:?}").as_bytes());
    hasher.update(evidence.as_bytes());
    let hash = hasher.finalize();
    format!("finding-{}", hex_prefix(&hash, 12))
}

fn detect_hardcoded_secret(line: &str, lower: &str) -> Option<String> {
    if lower.contains("{{secret:") {
        return None;
    }
    if contains_any(
        lower,
        &["private key", "begin rsa private key", "begin private key"],
    ) {
        return Some("private-key: ********".to_string());
    }
    if is_non_secret_inventory_line(line) {
        return None;
    }

    let secret_keywords = [
        "api_key",
        "apikey",
        "api key",
        "access_key",
        "secret",
        "token",
        "password",
        "passwd",
        "pwd",
        "密钥",
        "密码",
        "令牌",
    ];
    if !contains_any(lower, &secret_keywords) {
        return detect_token_like_value(line);
    }

    let value = value_after_separator(line)?;
    let trimmed = value.trim_matches(|ch: char| {
        ch == '"' || ch == '\'' || ch == '`' || ch == ';' || ch.is_whitespace()
    });
    if trimmed.len() < 8 || looks_like_placeholder(trimmed) {
        return None;
    }

    Some(format!("{}: {}", secret_label(line), mask_secret(trimmed)))
}

fn detect_token_like_value(line: &str) -> Option<String> {
    for word in line.split(|ch: char| {
        ch.is_whitespace() || matches!(ch, '"' | '\'' | '`' | ',' | ';' | ')' | '(')
    }) {
        let trimmed = word.trim();
        if trimmed.len() >= 24
            && trimmed.chars().any(|ch| ch.is_ascii_digit())
            && trimmed.chars().any(|ch| ch.is_ascii_alphabetic())
            && trimmed
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | ':'))
        {
            return Some(format!("token-like: {}", mask_secret(trimmed)));
        }
    }
    None
}

fn value_after_separator(line: &str) -> Option<&str> {
    for separator in ["=", ":", "："] {
        if let Some((_, value)) = line.split_once(separator) {
            return Some(value);
        }
    }
    None
}

fn secret_label(line: &str) -> String {
    let label = line
        .split(|ch: char| matches!(ch, '=' | ':' | '：'))
        .next()
        .unwrap_or("secret")
        .trim()
        .trim_start_matches(['-', '*'])
        .trim();
    if label.is_empty() {
        "secret".to_string()
    } else {
        label.chars().take(40).collect()
    }
}

fn is_non_secret_inventory_line(line: &str) -> bool {
    let Some((label, _)) = line.split_once('：').or_else(|| line.split_once(':')) else {
        return false;
    };
    let label = label.trim().trim_start_matches(['-', '*']).trim();
    let lower = label.to_ascii_lowercase();
    contains_any(
        &lower,
        &[
            "dependency",
            "dependencies",
            "package",
            "packages",
            "data type",
            "datatype",
            "business data",
            "environment",
            "依赖",
            "第三方组件",
            "数据类型",
            "业务数据",
            "敏感数据",
            "部署环境",
            "运行环境",
        ],
    ) || lower == "env"
        || label == "环境"
}

fn secret_placeholder_key(line: &str) -> Option<String> {
    let raw = line
        .split(|ch: char| matches!(ch, '=' | ':' | '：'))
        .next()?
        .trim()
        .trim_start_matches(['-', '*'])
        .trim();
    let mut key = String::new();
    let mut upper_next = false;
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() {
            if key.is_empty() {
                key.push(ch.to_ascii_lowercase());
            } else if upper_next {
                key.push(ch.to_ascii_uppercase());
                upper_next = false;
            } else {
                key.push(ch);
            }
        } else {
            upper_next = true;
        }
    }
    if key.is_empty() {
        None
    } else {
        Some(key)
    }
}

fn replace_secret_value(line: &str, key: &str) -> Option<String> {
    for separator in ["=", ":", "："] {
        if let Some((left, _)) = line.split_once(separator) {
            return Some(format!(
                "{}{} {{{{secret:{}}}}}",
                left.trim_end(),
                separator,
                key
            ));
        }
    }
    None
}

fn looks_like_placeholder(value: &str) -> bool {
    value.contains("{{secret:")
        || value.contains("${")
        || value.contains("<")
        || value.eq_ignore_ascii_case("password")
}

fn mask_secret(value: &str) -> String {
    let chars = value.chars().collect::<Vec<_>>();
    if chars.len() <= 8 {
        return "********".to_string();
    }
    let prefix = chars.iter().take(3).collect::<String>();
    let suffix = chars
        .iter()
        .rev()
        .take(4)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<String>();
    format!("{prefix}...{suffix}")
}

fn mask_urls(line: &str) -> String {
    line.split_whitespace()
        .map(|part| {
            if part.starts_with("http://") {
                let without_scheme = part.trim_start_matches("http://");
                let host = without_scheme.split('/').next().unwrap_or(without_scheme);
                format!("http://{host}/...")
            } else {
                part.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn compact_evidence(line: &str) -> String {
    let compact = line.trim();
    if compact.chars().count() <= 96 {
        compact.to_string()
    } else {
        format!("{}...", compact.chars().take(93).collect::<String>())
    }
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

fn hex_prefix(bytes: &[u8], chars: usize) -> String {
    let mut output = String::new();
    for byte in bytes {
        output.push_str(&format!("{byte:02x}"));
        if output.len() >= chars {
            output.truncate(chars);
            break;
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_masks_hardcoded_secret() {
        let markdown = "api_key = \"sk-test-1234567890abcdef\"";
        let response = scan_markdown("doc-1", markdown, &HashMap::new(), 123);

        assert_eq!(response.summary.critical, 1);
        assert_eq!(response.findings[0].kind, FindingKind::HardcodedSecret);
        assert!(response.findings[0].evidence.contains("sk-...cdef"));
        assert!(!response.findings[0].evidence.contains("1234567890ab"));
    }

    #[test]
    fn detects_jwt_and_http_risks() {
        let markdown = "jwt alg HS256\ncallback http://api.example.com/hook";
        let response = scan_markdown("doc-1", markdown, &HashMap::new(), 123);

        assert_eq!(response.summary.warning, 2);
        assert!(response
            .findings
            .iter()
            .any(|finding| finding.kind == FindingKind::WeakJwt));
        assert!(response
            .findings
            .iter()
            .any(|finding| finding.kind == FindingKind::InsecureLink));
    }

    #[test]
    fn ignores_dependency_inventory_with_token_in_package_name() {
        let markdown = "依赖：npm:jsonwebtoken、crate:axum\n数据类型：access token、手机号";
        let response = scan_markdown("doc-1", markdown, &HashMap::new(), 123);

        assert_eq!(response.summary.total, 0);
    }

    #[test]
    fn applies_stored_status() {
        let markdown = "token = \"abcdef1234567890\"";
        let first = scan_markdown("doc-1", markdown, &HashMap::new(), 123);
        let statuses = HashMap::from([(first.findings[0].id.clone(), "ignored".to_string())]);
        let second = scan_markdown("doc-1", markdown, &statuses, 124);

        assert_eq!(second.findings[0].status, FindingStatus::Ignored);
        assert_eq!(second.summary.ignored, 1);
    }

    #[test]
    fn previews_http_fix_without_saving() {
        let markdown = "callback http://api.example.com/hook";
        let scan = scan_markdown("__preview__", markdown, &HashMap::new(), 0);
        let preview = preview_fix("__preview__", markdown, &scan.findings[0].id).expect("preview");

        assert!(preview
            .patch_markdown
            .contains("https://api.example.com/hook"));
    }
}
