use super::assets::{SecurityAsset, SecurityAssetType};
use super::audit::{FindingSeverity, SecurityFinding};
use super::governance::{AuditEvent, SecurityCase, SecurityCaseStatus};
use super::history::DocumentRiskDiff;
use super::standards::{self, ChecklistItem, StandardEntry};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SecurityReportScope {
    All,
    Document,
    Asset,
    Tags,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityReportRequest {
    pub scope: Option<SecurityReportScope>,
    pub doc_id: Option<String>,
    pub asset_id: Option<String>,
    pub query: Option<String>,
    pub tags: Option<Vec<String>>,
    pub since_days: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityReport {
    pub id: String,
    pub file_name: String,
    pub path: String,
    pub markdown: String,
    pub summary: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SafeShareRequest {
    pub doc_id: String,
    pub markdown: Option<String>,
    pub include_audit: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SafeShareExport {
    pub id: String,
    pub file_name: String,
    pub path: String,
    pub markdown: String,
    pub summary: String,
    pub redacted_secret_count: usize,
    pub finding_count: usize,
    pub created_at: i64,
}

pub fn render_safe_share_markdown(
    title: &str,
    file_name: &str,
    redacted_markdown: &str,
    redacted_secret_count: usize,
    findings: &[SecurityFinding],
    include_audit: bool,
    created_at: i64,
) -> String {
    let mut output = String::new();
    output.push_str(&format!("# {}\n\n", title.trim()));
    output.push_str("## 分享说明\n\n");
    output.push_str(&format!("- 来源文件：{}\n", file_name));
    output.push_str(&format!("- 导出时间：{}\n", created_at));
    output.push_str(&format!("- 自动脱敏：{} 处\n", redacted_secret_count));
    output.push_str("- Secret 明文：不包含\n\n");
    output.push_str("## 文档内容\n\n");
    output.push_str(redacted_markdown.trim());
    output.push_str("\n\n");

    if include_audit {
        output.push_str("## 安全审计摘要\n\n");
        if findings.is_empty() {
            output.push_str("暂无规则风险。\n");
        } else {
            output.push_str(&format!("- 风险发现：{} 个\n\n", findings.len()));
            for finding in findings {
                output.push_str(&format!(
                    "- L{} · {} · {}：{}\n",
                    finding.line_start,
                    severity_label(&finding.severity),
                    finding.title,
                    finding.evidence
                ));
            }
        }
    }

    output
}

pub fn render_security_report(
    report_title: &str,
    scope_summary: &str,
    findings: &[SecurityFinding],
    assets: &[SecurityAsset],
    cases: &[SecurityCase],
    events: &[AuditEvent],
    checklist: &[ChecklistItem],
    standard_entries: &[StandardEntry],
    risk_diffs: &[DocumentRiskDiff],
    created_at: i64,
) -> String {
    let standards_by_id = standard_entries
        .iter()
        .map(|entry| (entry.id.as_str(), entry))
        .collect::<std::collections::HashMap<_, _>>();
    let open_cases = cases
        .iter()
        .filter(|case| {
            matches!(
                case.status,
                SecurityCaseStatus::Open
                    | SecurityCaseStatus::Acknowledged
                    | SecurityCaseStatus::Fixing
                    | SecurityCaseStatus::Reviewing
                    | SecurityCaseStatus::Reopened
            )
        })
        .count();
    let accepted_cases = cases
        .iter()
        .filter(|case| case.status == SecurityCaseStatus::Accepted)
        .collect::<Vec<_>>();
    let critical = findings
        .iter()
        .filter(|finding| finding.severity == FindingSeverity::Critical)
        .count();
    let warning = findings
        .iter()
        .filter(|finding| finding.severity == FindingSeverity::Warning)
        .count();

    let mut report = String::new();
    report.push_str(&format!("# {}\n\n", report_title));
    report.push_str(&format!("- 报告范围：{}\n", scope_summary));
    report.push_str(&format!("- 生成时间：{}\n", created_at));
    report.push_str(&format!("- 风险发现：{} 个\n", findings.len()));
    report.push_str(&format!("- 高危 / 警告：{} / {}\n", critical, warning));
    report.push_str(&format!("- 未关闭治理项：{} 个\n", open_cases));
    report.push_str(&format!("- 已接受风险：{} 个\n", accepted_cases.len()));
    report.push_str(&format!("- 安全资产：{} 个\n\n", assets.len()));

    report.push_str("## 风险发现\n\n");
    if findings.is_empty() {
        report.push_str("暂无规则风险。\n\n");
    } else {
        for finding in findings {
            report.push_str(&format!(
                "### L{} · {} · {}\n\n",
                finding.line_start,
                severity_label(&finding.severity),
                finding.title
            ));
            report.push_str(&format!("- 文档：{}\n", finding.doc_id));
            report.push_str(&format!("- 状态：{:?}\n", finding.status));
            report.push_str(&format!("- 证据摘要：{}\n", finding.evidence));
            report.push_str(&format!("- 建议：{}\n\n", finding.recommendation));
            let standard_titles = standards::standard_ids_for_finding_kind(&finding.kind)
                .into_iter()
                .filter_map(|id| standards_by_id.get(id).map(|entry| entry.title.as_str()))
                .collect::<Vec<_>>();
            if !standard_titles.is_empty() {
                report.push_str(&format!("规范依据：{}\n\n", standard_titles.join("、")));
            }
        }
    }

    report.push_str("## 版本风险变化\n\n");
    if risk_diffs.is_empty() {
        report.push_str("暂无可用的版本风险变化。\n\n");
    } else {
        for diff in risk_diffs {
            report.push_str(&format!("### 文档 {}\n\n", diff.doc_id));
            if diff.previous_saved_at.is_none() {
                report.push_str(&format!(
                    "- 已建立风险基线，当前风险 {} 个。\n\n",
                    diff.summary.current_total
                ));
            } else {
                report.push_str(&format!(
                    "- 新增 / 修复 / 移动 / 等级变化：{} / {} / {} / {}\n",
                    diff.summary.added,
                    diff.summary.resolved,
                    diff.summary.moved,
                    diff.summary.severity_changed
                ));
                report.push_str(&format!(
                    "- 上次快照：{}，当前快照：{}\n",
                    diff.previous_saved_at
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "无".to_string()),
                    diff.current_saved_at
                ));
                for item in diff.added.iter().take(5) {
                    report.push_str(&format!(
                        "- 新增：L{} · {} · {}\n",
                        item.line_start, item.title, item.evidence
                    ));
                }
                for item in diff.resolved.iter().take(5) {
                    report.push_str(&format!(
                        "- 修复：L{} · {} · {}\n",
                        item.line_start, item.title, item.evidence
                    ));
                }
                report.push('\n');
            }
        }
    }

    report.push_str("## 安全 Checklist\n\n");
    if checklist.is_empty() {
        report.push_str("暂无 checklist。\n\n");
    } else {
        for item in checklist {
            report.push_str(&format!(
                "- [{}] {} · {}{}\n",
                checklist_status_label(&item.status),
                item.title,
                if item.recommended { "推荐" } else { "常规" },
                item.note
                    .as_deref()
                    .map(|note| format!(" · {note}"))
                    .unwrap_or_default()
            ));
        }
        report.push('\n');
    }

    report.push_str("## 安全例外台账\n\n");
    let exception_cases = cases
        .iter()
        .filter(|case| case.accepted_until.is_some())
        .collect::<Vec<_>>();
    if exception_cases.is_empty() {
        report.push_str("暂无安全例外。\n\n");
    } else {
        for case in exception_cases {
            report.push_str(&format!(
                "### {} · {}\n\n",
                status_label(&case.status),
                case.title
            ));
            report.push_str(&format!("- 案件 ID：{}\n", case.id));
            report.push_str(&format!(
                "- 接受有效期：{}\n",
                case.accepted_until.as_deref().unwrap_or("未设置")
            ));
            report.push_str(&format!(
                "- 接受原因：{}\n",
                case.rationale.as_deref().unwrap_or("未填写")
            ));
            report.push_str(&format!(
                "- 影响范围：{}\n",
                case.impact_scope.as_deref().unwrap_or("未填写")
            ));
            report.push_str(&format!(
                "- 补偿控制：{}\n",
                case.compensating_controls.as_deref().unwrap_or("未填写")
            ));
            report.push_str(&format!(
                "- 复核人：{}\n",
                case.reviewer.as_deref().unwrap_or("未填写")
            ));
            report.push_str(&format!(
                "- 负责人：{}\n\n",
                case.owner.as_deref().unwrap_or("未分配")
            ));
        }
    }

    report.push_str("## 风险治理项\n\n");
    if cases.is_empty() {
        report.push_str("暂无治理项。\n\n");
    } else {
        for case in cases {
            report.push_str(&format!(
                "### {} · {}\n\n",
                status_label(&case.status),
                case.title
            ));
            report.push_str(&format!("- 案件 ID：{}\n", case.id));
            report.push_str(&format!("- 等级：{}\n", severity_label(&case.severity)));
            report.push_str(&format!(
                "- 负责人：{}\n",
                case.owner.as_deref().unwrap_or("未分配")
            ));
            report.push_str(&format!(
                "- 截止日：{}\n",
                case.due_at.as_deref().unwrap_or("未设置")
            ));
            if let Some(accepted_until) = &case.accepted_until {
                report.push_str(&format!("- 接受有效期：{}\n", accepted_until));
            }
            if let Some(rationale) = &case.rationale {
                report.push_str(&format!("- 处置依据：{}\n", rationale));
            }
            if let Some(impact_scope) = &case.impact_scope {
                report.push_str(&format!("- 影响范围：{}\n", impact_scope));
            }
            if let Some(compensating_controls) = &case.compensating_controls {
                report.push_str(&format!("- 补偿控制：{}\n", compensating_controls));
            }
            if let Some(reviewer) = &case.reviewer {
                report.push_str(&format!("- 复核人：{}\n", reviewer));
            }
            report.push('\n');
        }
    }

    report.push_str("## 安全资产\n\n");
    if assets.is_empty() {
        report.push_str("暂无安全资产索引。\n\n");
    } else {
        for asset in assets.iter().take(30) {
            report.push_str(&format!(
                "- {} · {} · 文档 {}\n",
                asset_type_label(&asset.asset_type),
                asset.name,
                asset.source_doc_ids.join(", ")
            ));
        }
        report.push('\n');
    }

    report.push_str("## 最近治理事件\n\n");
    if events.is_empty() {
        report.push_str("暂无治理事件。\n");
    } else {
        for event in events.iter().rev().take(12) {
            report.push_str(&format!(
                "- {} · {} · {}\n",
                event.created_at, event.event_type, event.summary
            ));
        }
    }

    report
}

fn severity_label(severity: &FindingSeverity) -> &'static str {
    match severity {
        FindingSeverity::Critical => "高危",
        FindingSeverity::Warning => "警告",
        FindingSeverity::Info => "提示",
    }
}

fn status_label(status: &SecurityCaseStatus) -> &'static str {
    match status {
        SecurityCaseStatus::Open => "待确认",
        SecurityCaseStatus::Acknowledged => "已确认",
        SecurityCaseStatus::Accepted => "已接受",
        SecurityCaseStatus::Fixing => "修复中",
        SecurityCaseStatus::Fixed => "已修复",
        SecurityCaseStatus::Reviewing => "待复核",
        SecurityCaseStatus::Closed => "已关闭",
        SecurityCaseStatus::Reopened => "已重开",
    }
}

fn asset_type_label(asset_type: &SecurityAssetType) -> &'static str {
    match asset_type {
        SecurityAssetType::Service => "服务",
        SecurityAssetType::ApiEndpoint => "接口",
        SecurityAssetType::Url => "URL",
        SecurityAssetType::Secret => "Secret 占位符",
        SecurityAssetType::Database => "数据库",
        SecurityAssetType::Dependency => "依赖",
        SecurityAssetType::Environment => "部署环境",
        SecurityAssetType::DataType => "数据类型",
    }
}

fn checklist_status_label(status: &standards::ChecklistStatus) -> &'static str {
    match status {
        standards::ChecklistStatus::Open => "待办",
        standards::ChecklistStatus::Done => "完成",
        standards::ChecklistStatus::Waived => "不适用",
    }
}
