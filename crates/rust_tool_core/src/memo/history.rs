use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

use super::audit::{FindingKind, FindingSeverity, FindingStatus, SecurityFinding};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentRiskSnapshot {
    pub doc_id: String,
    pub saved_at: i64,
    pub content_hash: String,
    pub findings: Vec<SecurityFinding>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentRiskDiffSummary {
    pub added: usize,
    pub resolved: usize,
    pub severity_changed: usize,
    pub moved: usize,
    pub unchanged: usize,
    pub previous_total: usize,
    pub current_total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentRiskDiffItem {
    pub fingerprint: String,
    pub title: String,
    pub severity: FindingSeverity,
    pub previous_severity: Option<FindingSeverity>,
    pub kind: FindingKind,
    pub line_start: usize,
    pub line_end: usize,
    pub previous_line_start: Option<usize>,
    pub previous_line_end: Option<usize>,
    pub evidence: String,
    pub status: FindingStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentRiskDiff {
    pub doc_id: String,
    pub previous_saved_at: Option<i64>,
    pub current_saved_at: i64,
    pub previous_hash: Option<String>,
    pub current_hash: String,
    pub added: Vec<DocumentRiskDiffItem>,
    pub resolved: Vec<DocumentRiskDiffItem>,
    pub changed: Vec<DocumentRiskDiffItem>,
    pub summary: DocumentRiskDiffSummary,
}

pub fn content_hash(markdown: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(markdown.as_bytes());
    let digest = hasher.finalize();
    digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}

pub fn build_snapshot(
    doc_id: &str,
    saved_at: i64,
    markdown: &str,
    findings: Vec<SecurityFinding>,
) -> DocumentRiskSnapshot {
    DocumentRiskSnapshot {
        doc_id: doc_id.to_string(),
        saved_at,
        content_hash: content_hash(markdown),
        findings,
    }
}

pub fn diff_snapshots(
    doc_id: &str,
    previous: Option<&DocumentRiskSnapshot>,
    current: &DocumentRiskSnapshot,
) -> DocumentRiskDiff {
    let previous_findings = previous
        .map(|snapshot| snapshot.findings.as_slice())
        .unwrap_or(&[]);
    let mut previous_by_fingerprint = HashMap::new();
    for finding in previous_findings {
        previous_by_fingerprint.insert(finding_fingerprint(finding), finding);
    }

    let mut current_by_fingerprint = HashMap::new();
    for finding in &current.findings {
        current_by_fingerprint.insert(finding_fingerprint(finding), finding);
    }

    let mut added = Vec::new();
    let mut changed = Vec::new();
    let mut unchanged = 0;

    for finding in &current.findings {
        let fingerprint = finding_fingerprint(finding);
        match previous_by_fingerprint.get(&fingerprint) {
            Some(previous_finding) => {
                let severity_changed = previous_finding.severity != finding.severity;
                let moved = previous_finding.line_start != finding.line_start
                    || previous_finding.line_end != finding.line_end;
                if severity_changed || moved {
                    changed.push(diff_item(&fingerprint, finding, Some(previous_finding)));
                } else {
                    unchanged += 1;
                }
            }
            None => added.push(diff_item(&fingerprint, finding, None)),
        }
    }

    let mut resolved = Vec::new();
    for finding in previous_findings {
        let fingerprint = finding_fingerprint(finding);
        if !current_by_fingerprint.contains_key(&fingerprint) {
            resolved.push(diff_item(&fingerprint, finding, None));
        }
    }

    let summary = DocumentRiskDiffSummary {
        added: added.len(),
        resolved: resolved.len(),
        severity_changed: changed
            .iter()
            .filter(|item| {
                item.previous_severity
                    .as_ref()
                    .is_some_and(|severity| severity != &item.severity)
            })
            .count(),
        moved: changed
            .iter()
            .filter(|item| {
                item.previous_line_start
                    .is_some_and(|line| line != item.line_start)
                    || item
                        .previous_line_end
                        .is_some_and(|line| line != item.line_end)
            })
            .count(),
        unchanged,
        previous_total: previous_findings.len(),
        current_total: current.findings.len(),
    };

    DocumentRiskDiff {
        doc_id: doc_id.to_string(),
        previous_saved_at: previous.map(|snapshot| snapshot.saved_at),
        current_saved_at: current.saved_at,
        previous_hash: previous.map(|snapshot| snapshot.content_hash.clone()),
        current_hash: current.content_hash.clone(),
        added,
        resolved,
        changed,
        summary,
    }
}

fn finding_fingerprint(finding: &SecurityFinding) -> String {
    let mut hasher = Sha256::new();
    hasher.update(format!("{:?}", finding.kind).as_bytes());
    hasher.update(finding.title.as_bytes());
    hasher.update(finding.evidence.as_bytes());
    let digest = hasher.finalize();
    let suffix = digest
        .iter()
        .take(8)
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("risk-{suffix}")
}

fn diff_item(
    fingerprint: &str,
    finding: &SecurityFinding,
    previous: Option<&SecurityFinding>,
) -> DocumentRiskDiffItem {
    DocumentRiskDiffItem {
        fingerprint: fingerprint.to_string(),
        title: finding.title.clone(),
        severity: finding.severity.clone(),
        previous_severity: previous.map(|finding| finding.severity.clone()),
        kind: finding.kind.clone(),
        line_start: finding.line_start,
        line_end: finding.line_end,
        previous_line_start: previous.map(|finding| finding.line_start),
        previous_line_end: previous.map(|finding| finding.line_end),
        evidence: finding.evidence.clone(),
        status: finding.status.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn finding(line: usize, severity: FindingSeverity, evidence: &str) -> SecurityFinding {
        SecurityFinding {
            id: format!("finding-{line}-{evidence}"),
            doc_id: "doc-1".to_string(),
            line_start: line,
            line_end: line,
            severity,
            kind: FindingKind::InsecureLink,
            title: "第三方 HTTP 链接".to_string(),
            detail: "发现明文 HTTP 链接。".to_string(),
            evidence: evidence.to_string(),
            recommendation: "改用 HTTPS。".to_string(),
            status: FindingStatus::Open,
        }
    }

    #[test]
    fn reports_added_resolved_and_moved_risks() {
        let previous = DocumentRiskSnapshot {
            doc_id: "doc-1".to_string(),
            saved_at: 1,
            content_hash: "old".to_string(),
            findings: vec![
                finding(3, FindingSeverity::Warning, "http://old.example.com"),
                finding(5, FindingSeverity::Warning, "http://moved.example.com"),
            ],
        };
        let current = DocumentRiskSnapshot {
            doc_id: "doc-1".to_string(),
            saved_at: 2,
            content_hash: "new".to_string(),
            findings: vec![
                finding(9, FindingSeverity::Warning, "http://moved.example.com"),
                finding(12, FindingSeverity::Warning, "http://new.example.com"),
            ],
        };

        let diff = diff_snapshots("doc-1", Some(&previous), &current);

        assert_eq!(diff.summary.added, 1);
        assert_eq!(diff.summary.resolved, 1);
        assert_eq!(diff.summary.moved, 1);
        assert_eq!(diff.summary.unchanged, 0);
        assert_eq!(diff.added[0].line_start, 12);
        assert_eq!(diff.resolved[0].line_start, 3);
        assert_eq!(diff.changed[0].previous_line_start, Some(5));
        assert_eq!(diff.changed[0].line_start, 9);
    }
}
