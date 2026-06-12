use super::assets::{SecurityAsset, SecurityAssetType};
use super::audit::{FindingKind, SecurityFinding};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

const GLOBAL_SCOPE: &str = "__global__";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StandardEntry {
    pub id: String,
    pub category: String,
    pub title: String,
    pub description: String,
    pub controls: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ChecklistStatus {
    Open,
    Done,
    Waived,
}

impl ChecklistStatus {
    pub fn from_action(value: &str) -> Option<Self> {
        match value.trim() {
            "open" => Some(Self::Open),
            "done" => Some(Self::Done),
            "waived" => Some(Self::Waived),
            _ => None,
        }
    }

    pub fn as_action(&self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Done => "done",
            Self::Waived => "waived",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChecklistStatusRecord {
    pub item_id: String,
    pub status: ChecklistStatus,
    pub note: Option<String>,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChecklistItem {
    pub id: String,
    pub title: String,
    pub description: String,
    pub standard_ids: Vec<String>,
    pub recommended: bool,
    pub status: ChecklistStatus,
    pub note: Option<String>,
    pub evidence: Vec<String>,
    pub updated_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StandardsChecklistResponse {
    pub doc_id: Option<String>,
    pub items: Vec<ChecklistItem>,
    pub standards: Vec<StandardEntry>,
    pub updated_at: i64,
}

pub fn builtin_standards() -> Vec<StandardEntry> {
    vec![
        StandardEntry {
            id: "RT-SEC-SECRET-01".to_string(),
            category: "密钥管理".to_string(),
            title: "密钥不得明文进入文档或代码".to_string(),
            description: "生产密钥、API Key、Token、私钥和密码必须托管在受控密钥系统或本地 KDBX，文档仅保留占位符。".to_string(),
            controls: vec![
                "文档中使用 {{secret:key}} 占位符。".to_string(),
                "密钥轮换和访问范围需要可追溯。".to_string(),
            ],
        },
        StandardEntry {
            id: "RT-SEC-AUTH-02".to_string(),
            category: "认证与会话".to_string(),
            title: "认证令牌必须使用强签名与轮换策略".to_string(),
            description: "JWT、会话令牌和服务间认证配置需要明确算法、密钥强度、轮换周期和验证边界。".to_string(),
            controls: vec![
                "避免 none、弱 HMAC 密钥或未说明密钥来源的 HS256。".to_string(),
                "优先说明 RS256 / ES256 等非对称签名策略。".to_string(),
            ],
        },
        StandardEntry {
            id: "RT-SEC-TRANSPORT-01".to_string(),
            category: "传输安全".to_string(),
            title: "第三方与跨边界通信必须加密".to_string(),
            description: "外部回调、第三方接口和跨网络边界通信默认使用 HTTPS 或受控专线；例外必须记录补偿控制。".to_string(),
            controls: vec![
                "第三方 URL 默认使用 HTTPS。".to_string(),
                "内网明文协议需要说明边界、鉴权和监控。".to_string(),
            ],
        },
        StandardEntry {
            id: "RT-SEC-CHANGE-01".to_string(),
            category: "变更治理".to_string(),
            title: "高风险安全开关必须有审批、期限和回滚".to_string(),
            description: "关闭校验、跳过鉴权、临时白名单等高风险变更必须记录影响范围、负责人、有效期和回滚方案。".to_string(),
            controls: vec![
                "补充负责人和到期复核时间。".to_string(),
                "记录审批依据、监控指标和回滚步骤。".to_string(),
            ],
        },
        StandardEntry {
            id: "RT-SEC-ASSET-01".to_string(),
            category: "资产清单".to_string(),
            title: "服务、接口和 Secret 引用需要进入安全资产清单".to_string(),
            description: "安全评审文档中的服务、API、URL 和 secret 占位符应当可搜索、可关联风险、可复核。".to_string(),
            controls: vec![
                "记录服务和接口出现在哪些文档中。".to_string(),
                "将资产与风险、secret 占位符建立关联。".to_string(),
            ],
        },
        StandardEntry {
            id: "OWASP-AUTH".to_string(),
            category: "OWASP 类别".to_string(),
            title: "认证、会话和访问控制风险".to_string(),
            description: "认证绕过、弱令牌、缺少校验和权限边界不清都应进入认证与访问控制审查。".to_string(),
            controls: vec!["复核认证边界、签名算法、权限验证和失败策略。".to_string()],
        },
        StandardEntry {
            id: "OWASP-CONFIG".to_string(),
            category: "OWASP 类别".to_string(),
            title: "安全配置与敏感信息暴露".to_string(),
            description: "明文密钥、不安全协议、临时绕过校验和配置漂移都属于安全配置审查重点。".to_string(),
            controls: vec!["检查配置来源、加密传输、密钥托管和默认安全开关。".to_string()],
        },
    ]
}

pub fn standard_ids_for_finding_kind(kind: &FindingKind) -> Vec<&'static str> {
    match kind {
        FindingKind::HardcodedSecret => vec!["RT-SEC-SECRET-01", "OWASP-CONFIG"],
        FindingKind::WeakJwt => vec!["RT-SEC-AUTH-02", "OWASP-AUTH"],
        FindingKind::InsecureLink => vec!["RT-SEC-TRANSPORT-01", "OWASP-CONFIG"],
        FindingKind::SensitiveOperation => vec!["RT-SEC-CHANGE-01", "OWASP-CONFIG"],
        FindingKind::GovernanceGap => vec!["RT-SEC-CHANGE-01", "RT-SEC-ASSET-01"],
    }
}

pub fn checklist_status_key(doc_id: Option<&str>, item_id: &str) -> String {
    format!("{}:{item_id}", doc_scope(doc_id))
}

pub fn build_checklist(
    doc_id: Option<&str>,
    findings: &[SecurityFinding],
    assets: &[SecurityAsset],
    statuses: &HashMap<String, ChecklistStatusRecord>,
    updated_at: i64,
) -> StandardsChecklistResponse {
    let mut items = checklist_templates();
    let finding_kinds = findings
        .iter()
        .map(|finding| finding.kind.clone())
        .collect::<HashSet<_>>();
    let has_api_assets = assets
        .iter()
        .any(|asset| asset.asset_type == SecurityAssetType::ApiEndpoint);
    let has_secret_assets = assets
        .iter()
        .any(|asset| asset.asset_type == SecurityAssetType::Secret);

    for item in &mut items {
        item.recommended = match item.id.as_str() {
            "secret-management" => {
                has_secret_assets || finding_kinds.contains(&FindingKind::HardcodedSecret)
            }
            "auth-token-review" => finding_kinds.contains(&FindingKind::WeakJwt),
            "transport-security" => finding_kinds.contains(&FindingKind::InsecureLink),
            "change-exception-control" => {
                finding_kinds.contains(&FindingKind::SensitiveOperation)
                    || finding_kinds.contains(&FindingKind::GovernanceGap)
            }
            "asset-inventory" => !assets.is_empty() || has_api_assets,
            _ => false,
        };
        item.evidence = evidence_for_item(&item.id, findings, assets);
        if let Some(record) = status_record_for_item(doc_id, &item.id, statuses) {
            item.status = record.status.clone();
            item.note = record.note.clone();
            item.updated_at = Some(record.updated_at);
        }
    }

    items.sort_by(|left, right| {
        right
            .recommended
            .cmp(&left.recommended)
            .then_with(|| status_rank(&left.status).cmp(&status_rank(&right.status)))
            .then_with(|| left.title.cmp(&right.title))
    });

    StandardsChecklistResponse {
        doc_id: doc_id.map(str::to_string),
        items,
        standards: builtin_standards(),
        updated_at,
    }
}

fn checklist_templates() -> Vec<ChecklistItem> {
    vec![
        ChecklistItem {
            id: "secret-management".to_string(),
            title: "密钥托管与轮换".to_string(),
            description:
                "确认文档中的密钥均已替换为 secret 占位符，并记录轮换、权限范围和保管位置。"
                    .to_string(),
            standard_ids: vec!["RT-SEC-SECRET-01".to_string(), "OWASP-CONFIG".to_string()],
            recommended: false,
            status: ChecklistStatus::Open,
            note: None,
            evidence: Vec::new(),
            updated_at: None,
        },
        ChecklistItem {
            id: "auth-token-review".to_string(),
            title: "认证与令牌签名复核".to_string(),
            description: "复核 JWT、会话令牌和服务间认证的签名算法、密钥强度、过期策略和验证边界。"
                .to_string(),
            standard_ids: vec!["RT-SEC-AUTH-02".to_string(), "OWASP-AUTH".to_string()],
            recommended: false,
            status: ChecklistStatus::Open,
            note: None,
            evidence: Vec::new(),
            updated_at: None,
        },
        ChecklistItem {
            id: "transport-security".to_string(),
            title: "传输安全复核".to_string(),
            description: "确认第三方链接、回调地址和跨边界流量使用 HTTPS 或具备明确补偿控制。"
                .to_string(),
            standard_ids: vec![
                "RT-SEC-TRANSPORT-01".to_string(),
                "OWASP-CONFIG".to_string(),
            ],
            recommended: false,
            status: ChecklistStatus::Open,
            note: None,
            evidence: Vec::new(),
            updated_at: None,
        },
        ChecklistItem {
            id: "change-exception-control".to_string(),
            title: "高风险变更与例外治理".to_string(),
            description:
                "为关闭校验、跳过鉴权、临时白名单等例外补齐审批、影响范围、补偿控制和到期复核。"
                    .to_string(),
            standard_ids: vec!["RT-SEC-CHANGE-01".to_string()],
            recommended: false,
            status: ChecklistStatus::Open,
            note: None,
            evidence: Vec::new(),
            updated_at: None,
        },
        ChecklistItem {
            id: "asset-inventory".to_string(),
            title: "资产清单与关联关系".to_string(),
            description:
                "确认服务、接口、URL 和 secret 占位符已经进入资产清单，并能关联到风险与文档。"
                    .to_string(),
            standard_ids: vec!["RT-SEC-ASSET-01".to_string()],
            recommended: false,
            status: ChecklistStatus::Open,
            note: None,
            evidence: Vec::new(),
            updated_at: None,
        },
    ]
}

fn evidence_for_item(
    item_id: &str,
    findings: &[SecurityFinding],
    assets: &[SecurityAsset],
) -> Vec<String> {
    let mut evidence = Vec::new();
    match item_id {
        "secret-management" => {
            push_finding_evidence(&mut evidence, findings, FindingKind::HardcodedSecret);
            for asset in assets
                .iter()
                .filter(|asset| asset.asset_type == SecurityAssetType::Secret)
                .take(3)
            {
                evidence.push(format!("Secret 占位符：{}", asset.name));
            }
        }
        "auth-token-review" => {
            push_finding_evidence(&mut evidence, findings, FindingKind::WeakJwt);
        }
        "transport-security" => {
            push_finding_evidence(&mut evidence, findings, FindingKind::InsecureLink);
        }
        "change-exception-control" => {
            push_finding_evidence(&mut evidence, findings, FindingKind::SensitiveOperation);
            push_finding_evidence(&mut evidence, findings, FindingKind::GovernanceGap);
        }
        "asset-inventory" => {
            for asset in assets.iter().take(5) {
                evidence.push(format!(
                    "{}：{}",
                    asset_type_label(&asset.asset_type),
                    asset.name
                ));
            }
        }
        _ => {}
    }
    evidence.truncate(5);
    evidence
}

fn push_finding_evidence(
    evidence: &mut Vec<String>,
    findings: &[SecurityFinding],
    kind: FindingKind,
) {
    for finding in findings
        .iter()
        .filter(|finding| finding.kind == kind)
        .take(3)
    {
        evidence.push(format!("L{}：{}", finding.line_start, finding.title));
    }
}

fn asset_type_label(asset_type: &SecurityAssetType) -> &'static str {
    match asset_type {
        SecurityAssetType::Service => "服务",
        SecurityAssetType::ApiEndpoint => "接口",
        SecurityAssetType::Url => "URL",
        SecurityAssetType::Secret => "Secret",
        SecurityAssetType::Database => "数据库",
        SecurityAssetType::Dependency => "依赖",
        SecurityAssetType::Environment => "部署环境",
        SecurityAssetType::DataType => "数据类型",
    }
}

fn doc_scope(doc_id: Option<&str>) -> String {
    doc_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(GLOBAL_SCOPE)
        .to_string()
}

fn status_record_for_item<'a>(
    doc_id: Option<&str>,
    item_id: &str,
    statuses: &'a HashMap<String, ChecklistStatusRecord>,
) -> Option<&'a ChecklistStatusRecord> {
    if let Some(record) = statuses.get(&checklist_status_key(doc_id, item_id)) {
        return Some(record);
    }
    if doc_id.is_some() {
        return None;
    }
    let suffix = format!(":{item_id}");
    statuses
        .iter()
        .filter(|(key, _)| key.ends_with(&suffix))
        .map(|(_, record)| record)
        .max_by_key(|record| record.updated_at)
}

fn status_rank(status: &ChecklistStatus) -> u8 {
    match status {
        ChecklistStatus::Open => 0,
        ChecklistStatus::Done => 1,
        ChecklistStatus::Waived => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_finding_kind_maps_to_standards() {
        for kind in [
            FindingKind::HardcodedSecret,
            FindingKind::WeakJwt,
            FindingKind::InsecureLink,
            FindingKind::SensitiveOperation,
            FindingKind::GovernanceGap,
        ] {
            assert!(!standard_ids_for_finding_kind(&kind).is_empty());
        }
    }

    #[test]
    fn checklist_recommends_items_from_findings_and_assets() {
        let findings = vec![SecurityFinding {
            id: "finding-1".to_string(),
            doc_id: "doc-1".to_string(),
            line_start: 3,
            line_end: 3,
            severity: super::super::audit::FindingSeverity::Critical,
            kind: FindingKind::HardcodedSecret,
            title: "硬编码密钥泄露".to_string(),
            detail: "detail".to_string(),
            evidence: "sk-...abcd".to_string(),
            recommendation: "move secret".to_string(),
            status: super::super::audit::FindingStatus::Open,
        }];
        let assets = vec![SecurityAsset {
            id: "asset-1".to_string(),
            asset_type: SecurityAssetType::ApiEndpoint,
            name: "/api/login".to_string(),
            aliases: Vec::new(),
            tags: Vec::new(),
            source_doc_ids: vec!["doc-1".to_string()],
            linked_secret_keys: Vec::new(),
            linked_case_ids: Vec::new(),
            last_seen_at: 100,
        }];

        let checklist = build_checklist(Some("doc-1"), &findings, &assets, &HashMap::new(), 101);

        assert!(checklist.items.iter().any(|item| {
            item.id == "secret-management" && item.recommended && !item.evidence.is_empty()
        }));
        assert!(checklist
            .items
            .iter()
            .any(|item| item.id == "asset-inventory" && item.recommended));
    }

    #[test]
    fn global_checklist_uses_latest_document_status() {
        let mut statuses = HashMap::new();
        statuses.insert(
            checklist_status_key(Some("doc-1"), "secret-management"),
            ChecklistStatusRecord {
                item_id: "secret-management".to_string(),
                status: ChecklistStatus::Done,
                note: Some("已轮换".to_string()),
                updated_at: 100,
            },
        );

        let checklist = build_checklist(None, &[], &[], &statuses, 101);
        let item = checklist
            .items
            .iter()
            .find(|item| item.id == "secret-management")
            .expect("secret checklist item");

        assert_eq!(item.status, ChecklistStatus::Done);
        assert_eq!(item.note.as_deref(), Some("已轮换"));
    }
}
