use super::assets::SecurityAsset;
use super::audit::{FindingSeverity, FindingStatus, SecurityFinding};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SecurityCaseType {
    Risk,
    Exception,
    Review,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SecurityCaseStatus {
    Open,
    Acknowledged,
    Accepted,
    Fixing,
    Fixed,
    Reviewing,
    Closed,
    Reopened,
}

impl SecurityCaseStatus {
    pub fn from_action(value: &str) -> Option<Self> {
        match value.trim() {
            "open" => Some(Self::Open),
            "acknowledged" => Some(Self::Acknowledged),
            "accepted" => Some(Self::Accepted),
            "fixing" => Some(Self::Fixing),
            "fixed" => Some(Self::Fixed),
            "reviewing" => Some(Self::Reviewing),
            "closed" => Some(Self::Closed),
            "reopened" => Some(Self::Reopened),
            _ => None,
        }
    }

    pub fn as_action(&self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Acknowledged => "acknowledged",
            Self::Accepted => "accepted",
            Self::Fixing => "fixing",
            Self::Fixed => "fixed",
            Self::Reviewing => "reviewing",
            Self::Closed => "closed",
            Self::Reopened => "reopened",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityCaseEvent {
    pub event_type: String,
    pub summary: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityCase {
    pub id: String,
    pub case_type: SecurityCaseType,
    pub title: String,
    pub severity: FindingSeverity,
    pub status: SecurityCaseStatus,
    pub source_doc_id: String,
    pub source_finding_id: Option<String>,
    pub linked_assets: Vec<String>,
    pub owner: Option<String>,
    pub due_at: Option<String>,
    pub accepted_until: Option<String>,
    pub rationale: Option<String>,
    pub impact_scope: Option<String>,
    pub compensating_controls: Option<String>,
    pub reviewer: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub events: Vec<SecurityCaseEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditEvent {
    pub id: String,
    pub event_type: String,
    pub actor: String,
    pub target_id: String,
    pub summary: String,
    pub created_at: i64,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GovernanceRiskSummary {
    pub total: usize,
    pub open: usize,
    pub critical: usize,
    pub warning: usize,
    pub info: usize,
    pub reviewing: usize,
    pub ignored: usize,
    pub accepted: usize,
    pub expiring_soon: usize,
    pub expired_acceptances: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GovernanceAssetSummary {
    pub total: usize,
    pub services: usize,
    pub api_endpoints: usize,
    pub urls: usize,
    pub secrets: usize,
    pub databases: usize,
    pub dependencies: usize,
    pub environments: usize,
    pub data_types: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GovernanceActivity {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GovernanceSummary {
    pub risk_summary: GovernanceRiskSummary,
    pub asset_summary: GovernanceAssetSummary,
    pub recent_findings: Vec<SecurityFinding>,
    pub recent_assets: Vec<SecurityAsset>,
    pub recent_activities: Vec<GovernanceActivity>,
}

pub fn build_cases_from_findings(findings: &[SecurityFinding], now: i64) -> Vec<SecurityCase> {
    findings
        .iter()
        .filter(|finding| finding.status != FindingStatus::Ignored)
        .map(|finding| SecurityCase {
            id: format!("case-{}", finding.id.trim_start_matches("finding-")),
            case_type: SecurityCaseType::Risk,
            title: finding.title.clone(),
            severity: finding.severity.clone(),
            status: status_from_finding(&finding.status),
            source_doc_id: finding.doc_id.clone(),
            source_finding_id: Some(finding.id.clone()),
            linked_assets: Vec::new(),
            owner: None,
            due_at: None,
            accepted_until: None,
            rationale: None,
            impact_scope: None,
            compensating_controls: None,
            reviewer: None,
            created_at: now,
            updated_at: now,
            events: vec![SecurityCaseEvent {
                event_type: "findingDetected".to_string(),
                summary: format!("发现风险：{}", finding.title),
                created_at: now,
            }],
        })
        .collect()
}

pub fn build_governance_summary(
    findings: Vec<SecurityFinding>,
    assets: Vec<SecurityAsset>,
    cases: &[SecurityCase],
    events: &[AuditEvent],
    now: i64,
) -> GovernanceSummary {
    let mut risk_summary = GovernanceRiskSummary {
        total: cases.len(),
        ..GovernanceRiskSummary::default()
    };

    for case in cases {
        match case.severity {
            FindingSeverity::Critical => risk_summary.critical += 1,
            FindingSeverity::Warning => risk_summary.warning += 1,
            FindingSeverity::Info => risk_summary.info += 1,
        }
        match case.status {
            SecurityCaseStatus::Open
            | SecurityCaseStatus::Acknowledged
            | SecurityCaseStatus::Fixing
            | SecurityCaseStatus::Reopened => risk_summary.open += 1,
            SecurityCaseStatus::Reviewing => risk_summary.reviewing += 1,
            SecurityCaseStatus::Accepted => {
                risk_summary.accepted += 1;
                risk_summary.ignored += 1;
            }
            SecurityCaseStatus::Fixed | SecurityCaseStatus::Closed => {}
        }
        match acceptance_expiry_state(case.accepted_until.as_deref(), now) {
            AcceptanceExpiryState::Expired
                if !matches!(
                    case.status,
                    SecurityCaseStatus::Closed | SecurityCaseStatus::Fixed
                ) =>
            {
                risk_summary.expired_acceptances += 1;
            }
            AcceptanceExpiryState::ExpiringSoon if case.status == SecurityCaseStatus::Accepted => {
                risk_summary.expiring_soon += 1;
            }
            _ => {}
        }
    }

    let mut asset_summary = GovernanceAssetSummary {
        total: assets.len(),
        ..GovernanceAssetSummary::default()
    };
    for asset in &assets {
        match asset.asset_type {
            super::assets::SecurityAssetType::Service => asset_summary.services += 1,
            super::assets::SecurityAssetType::ApiEndpoint => asset_summary.api_endpoints += 1,
            super::assets::SecurityAssetType::Url => asset_summary.urls += 1,
            super::assets::SecurityAssetType::Secret => asset_summary.secrets += 1,
            super::assets::SecurityAssetType::Database => asset_summary.databases += 1,
            super::assets::SecurityAssetType::Dependency => asset_summary.dependencies += 1,
            super::assets::SecurityAssetType::Environment => asset_summary.environments += 1,
            super::assets::SecurityAssetType::DataType => asset_summary.data_types += 1,
        }
    }

    let mut recent_findings = findings;
    recent_findings.sort_by(|left, right| {
        severity_rank(&right.severity)
            .cmp(&severity_rank(&left.severity))
            .then_with(|| left.line_start.cmp(&right.line_start))
    });
    recent_findings.truncate(8);

    let mut recent_assets = assets;
    recent_assets.truncate(8);

    let mut recent_activities = events
        .iter()
        .rev()
        .take(5)
        .map(|event| GovernanceActivity {
            id: event.id.clone(),
            title: event.summary.clone(),
            summary: event.event_type.clone(),
            created_at: event.created_at,
        })
        .collect::<Vec<_>>();
    if recent_activities.is_empty() {
        recent_activities = recent_findings
            .iter()
            .take(5)
            .map(|finding| GovernanceActivity {
                id: format!("activity-{}", finding.id),
                title: finding.title.clone(),
                summary: format!(
                    "{} · L{}",
                    severity_label(&finding.severity),
                    finding.line_start
                ),
                created_at: now,
            })
            .collect();
    }

    GovernanceSummary {
        risk_summary,
        asset_summary,
        recent_findings,
        recent_assets,
        recent_activities,
    }
}

pub fn sync_cases_with_findings(
    existing_cases: Vec<SecurityCase>,
    findings: &[SecurityFinding],
    now: i64,
) -> (Vec<SecurityCase>, Vec<AuditEvent>) {
    let mut cases_by_finding = HashMap::new();
    let mut standalone_cases = Vec::new();
    for case in existing_cases {
        if let Some(finding_id) = case.source_finding_id.clone() {
            cases_by_finding.insert(finding_id, case);
        } else {
            standalone_cases.push(case);
        }
    }
    let mut active_finding_ids = HashSet::new();
    let mut next_cases = standalone_cases;
    let mut events = Vec::new();

    for finding in findings
        .iter()
        .filter(|finding| finding.status != FindingStatus::Ignored)
    {
        active_finding_ids.insert(finding.id.clone());
        let (mut case, was_new) = match cases_by_finding.remove(&finding.id) {
            Some(case) => (case, false),
            None => (create_case_from_finding(finding, now), true),
        };
        expire_accepted_case_if_needed(&mut case, now, &mut events);

        if !was_new
            && matches!(
                case.status,
                SecurityCaseStatus::Closed | SecurityCaseStatus::Fixed
            )
        {
            case.status = SecurityCaseStatus::Reopened;
            case.updated_at = now;
            push_case_event(&mut case, "caseReopened", "风险再次出现，已自动重开。", now);
            events.push(audit_event(
                "caseReopened",
                "system",
                &case.id,
                &format!("风险再次出现：{}", case.title),
                now,
                serde_json::json!({ "findingId": finding.id }),
            ));
        }

        case.title = finding.title.clone();
        case.severity = finding.severity.clone();
        case.source_doc_id = finding.doc_id.clone();
        if was_new {
            events.push(audit_event(
                "caseCreated",
                "system",
                &case.id,
                &format!("创建风险治理项：{}", case.title),
                now,
                serde_json::json!({ "findingId": finding.id }),
            ));
        }
        next_cases.push(case);
    }

    for (_, mut case) in cases_by_finding {
        expire_accepted_case_if_needed(&mut case, now, &mut events);
        if matches!(
            case.case_type,
            SecurityCaseType::Risk | SecurityCaseType::Review
        ) && !matches!(
            case.status,
            SecurityCaseStatus::Closed | SecurityCaseStatus::Fixed | SecurityCaseStatus::Accepted
        ) && case
            .source_finding_id
            .as_ref()
            .is_some_and(|finding_id| !active_finding_ids.contains(finding_id))
        {
            case.status = SecurityCaseStatus::Fixed;
            case.updated_at = now;
            push_case_event(
                &mut case,
                "findingResolved",
                "当前扫描中风险已消失，等待复核关闭。",
                now,
            );
            events.push(audit_event(
                "findingResolved",
                "system",
                &case.id,
                &format!("风险已从扫描结果中消失：{}", case.title),
                now,
                serde_json::json!({ "caseId": case.id }),
            ));
        }
        next_cases.push(case);
    }

    next_cases.sort_by(|left, right| {
        status_rank(&left.status)
            .cmp(&status_rank(&right.status))
            .then_with(|| severity_rank(&right.severity).cmp(&severity_rank(&left.severity)))
            .then_with(|| right.updated_at.cmp(&left.updated_at))
    });
    (next_cases, events)
}

pub fn transition_case(
    case: &mut SecurityCase,
    status: SecurityCaseStatus,
    owner: Option<String>,
    due_at: Option<String>,
    rationale: Option<String>,
    actor: &str,
    now: i64,
) -> AuditEvent {
    case.status = status.clone();
    case.owner = owner.or_else(|| case.owner.clone());
    case.due_at = due_at.or_else(|| case.due_at.clone());
    case.rationale = rationale.or_else(|| case.rationale.clone());
    case.updated_at = now;
    let summary = format!("{} -> {}", case.title, status.as_action());
    push_case_event(case, "caseStatusChanged", &summary, now);
    audit_event(
        "caseStatusChanged",
        actor,
        &case.id,
        &summary,
        now,
        serde_json::json!({
            "status": status.as_action(),
            "owner": case.owner.clone(),
            "dueAt": case.due_at.clone(),
            "rationale": case.rationale.clone(),
        }),
    )
}

pub fn accept_case(
    case: &mut SecurityCase,
    rationale: String,
    accepted_until: String,
    impact_scope: String,
    compensating_controls: String,
    reviewer: String,
    owner: Option<String>,
    actor: &str,
    now: i64,
) -> AuditEvent {
    case.status = SecurityCaseStatus::Accepted;
    case.owner = owner.or_else(|| case.owner.clone());
    case.accepted_until = Some(accepted_until.clone());
    case.rationale = Some(rationale.clone());
    case.impact_scope = Some(impact_scope.clone());
    case.compensating_controls = Some(compensating_controls.clone());
    case.reviewer = Some(reviewer.clone());
    case.updated_at = now;
    let summary = format!("风险已接受至 {}：{}", accepted_until, case.title);
    push_case_event(case, "caseAccepted", &summary, now);
    audit_event(
        "caseAccepted",
        actor,
        &case.id,
        &summary,
        now,
        serde_json::json!({
            "acceptedUntil": accepted_until,
            "rationale": rationale,
            "impactScope": impact_scope,
            "compensatingControls": compensating_controls,
            "reviewer": reviewer,
            "owner": case.owner.clone(),
        }),
    )
}

pub fn create_case_from_finding(finding: &SecurityFinding, now: i64) -> SecurityCase {
    SecurityCase {
        id: format!("case-{}", finding.id.trim_start_matches("finding-")),
        case_type: SecurityCaseType::Risk,
        title: finding.title.clone(),
        severity: finding.severity.clone(),
        status: status_from_finding(&finding.status),
        source_doc_id: finding.doc_id.clone(),
        source_finding_id: Some(finding.id.clone()),
        linked_assets: Vec::new(),
        owner: None,
        due_at: None,
        accepted_until: None,
        rationale: None,
        impact_scope: None,
        compensating_controls: None,
        reviewer: None,
        created_at: now,
        updated_at: now,
        events: vec![SecurityCaseEvent {
            event_type: "findingDetected".to_string(),
            summary: format!("发现风险：{}", finding.title),
            created_at: now,
        }],
    }
}

pub fn audit_event(
    event_type: &str,
    actor: &str,
    target_id: &str,
    summary: &str,
    created_at: i64,
    metadata: serde_json::Value,
) -> AuditEvent {
    AuditEvent {
        id: Uuid::new_v4().to_string(),
        event_type: event_type.to_string(),
        actor: actor.to_string(),
        target_id: target_id.to_string(),
        summary: summary.to_string(),
        created_at,
        metadata,
    }
}

pub fn parse_date_days(value: &str) -> Option<i64> {
    let mut parts = value.trim().split('-');
    let year = parts.next()?.parse::<i32>().ok()?;
    let month = parts.next()?.parse::<u32>().ok()?;
    let day = parts.next()?.parse::<u32>().ok()?;
    if parts.next().is_some() || month == 0 || month > 12 {
        return None;
    }
    if day == 0 || day > days_in_month(year, month) {
        return None;
    }
    Some(days_from_civil(year, month, day))
}

fn expire_accepted_case_if_needed(case: &mut SecurityCase, now: i64, events: &mut Vec<AuditEvent>) {
    if case.status != SecurityCaseStatus::Accepted {
        return;
    }
    if acceptance_expiry_state(case.accepted_until.as_deref(), now)
        != AcceptanceExpiryState::Expired
    {
        return;
    }

    case.status = SecurityCaseStatus::Reopened;
    case.updated_at = now;
    let accepted_until = case.accepted_until.clone().unwrap_or_default();
    let summary = format!("风险接受已到期，已重新进入待处理队列：{}", case.title);
    push_case_event(case, "riskAcceptanceExpired", &summary, now);
    events.push(audit_event(
        "riskAcceptanceExpired",
        "system",
        &case.id,
        &summary,
        now,
        serde_json::json!({
            "acceptedUntil": accepted_until,
            "caseId": case.id,
        }),
    ));
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AcceptanceExpiryState {
    None,
    Active,
    ExpiringSoon,
    Expired,
}

fn acceptance_expiry_state(accepted_until: Option<&str>, now: i64) -> AcceptanceExpiryState {
    let Some(accepted_until_days) = accepted_until.and_then(parse_date_days) else {
        return AcceptanceExpiryState::None;
    };
    let today_days = now.div_euclid(86_400);
    let days_remaining = accepted_until_days - today_days;
    if days_remaining < 0 {
        AcceptanceExpiryState::Expired
    } else if days_remaining <= 14 {
        AcceptanceExpiryState::ExpiringSoon
    } else {
        AcceptanceExpiryState::Active
    }
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn days_from_civil(mut year: i32, month: u32, day: u32) -> i64 {
    year -= i32::from(month <= 2);
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let month_prime = month as i32 + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * month_prime + 2) / 5 + day as i32 - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    i64::from(era * 146_097 + day_of_era - 719_468)
}

fn push_case_event(case: &mut SecurityCase, event_type: &str, summary: &str, created_at: i64) {
    case.events.push(SecurityCaseEvent {
        event_type: event_type.to_string(),
        summary: summary.to_string(),
        created_at,
    });
}

fn status_from_finding(status: &FindingStatus) -> SecurityCaseStatus {
    match status {
        FindingStatus::Open => SecurityCaseStatus::Open,
        FindingStatus::Fixed => SecurityCaseStatus::Fixed,
        FindingStatus::Ignored => SecurityCaseStatus::Closed,
        FindingStatus::Reviewing => SecurityCaseStatus::Reviewing,
    }
}

fn severity_rank(severity: &FindingSeverity) -> u8 {
    match severity {
        FindingSeverity::Critical => 3,
        FindingSeverity::Warning => 2,
        FindingSeverity::Info => 1,
    }
}

fn status_rank(status: &SecurityCaseStatus) -> u8 {
    match status {
        SecurityCaseStatus::Open | SecurityCaseStatus::Reopened => 0,
        SecurityCaseStatus::Acknowledged | SecurityCaseStatus::Fixing => 1,
        SecurityCaseStatus::Reviewing => 2,
        SecurityCaseStatus::Accepted => 3,
        SecurityCaseStatus::Fixed => 4,
        SecurityCaseStatus::Closed => 5,
    }
}

fn severity_label(severity: &FindingSeverity) -> &'static str {
    match severity {
        FindingSeverity::Critical => "高危",
        FindingSeverity::Warning => "警告",
        FindingSeverity::Info => "提示",
    }
}

#[cfg(test)]
mod tests {
    use super::super::audit::{FindingKind, FindingStatus};
    use super::*;

    fn test_now() -> i64 {
        parse_date_days("2026-06-11").expect("test date") * 86_400
    }

    fn finding() -> SecurityFinding {
        SecurityFinding {
            id: "finding-1".to_string(),
            doc_id: "doc-1".to_string(),
            line_start: 10,
            line_end: 10,
            severity: FindingSeverity::Critical,
            kind: FindingKind::HardcodedSecret,
            title: "硬编码密钥泄露".to_string(),
            detail: "发现疑似明文密钥。".to_string(),
            evidence: "api_key: sk-...cdef".to_string(),
            recommendation: "移入 KDBX。".to_string(),
            status: FindingStatus::Open,
        }
    }

    #[test]
    fn expired_accepted_case_reopens_and_records_event() {
        let now = test_now();
        let mut case = create_case_from_finding(&finding(), now - 10);
        case.status = SecurityCaseStatus::Accepted;
        case.accepted_until = Some("2026-06-10".to_string());
        case.rationale = Some("等待窗口".to_string());
        case.impact_scope = Some("支付回调".to_string());
        case.compensating_controls = Some("监控告警".to_string());
        case.reviewer = Some("sec-lead".to_string());

        let (cases, events) = sync_cases_with_findings(vec![case], &[finding()], now);

        assert_eq!(cases[0].status, SecurityCaseStatus::Reopened);
        assert!(cases[0]
            .events
            .iter()
            .any(|event| event.event_type == "riskAcceptanceExpired"));
        assert!(events
            .iter()
            .any(|event| event.event_type == "riskAcceptanceExpired"));
    }

    #[test]
    fn governance_summary_counts_expiring_acceptances() {
        let now = test_now();
        let mut case = create_case_from_finding(&finding(), now - 10);
        case.status = SecurityCaseStatus::Accepted;
        case.accepted_until = Some("2026-06-20".to_string());
        let summary = build_governance_summary(Vec::new(), Vec::new(), &[case], &[], now);

        assert_eq!(summary.risk_summary.accepted, 1);
        assert_eq!(summary.risk_summary.expiring_soon, 1);
        assert_eq!(summary.risk_summary.expired_acceptances, 0);
    }
}
