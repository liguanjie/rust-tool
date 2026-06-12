use super::audit::{FindingSeverity, SecurityFinding};
use super::governance::SecurityCase;
use super::store::MemoMetadata;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum SecurityAssetType {
    Url,
    ApiEndpoint,
    Secret,
    Service,
    Database,
    Dependency,
    Environment,
    DataType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityAsset {
    pub id: String,
    pub asset_type: SecurityAssetType,
    pub name: String,
    pub aliases: Vec<String>,
    pub tags: Vec<String>,
    pub source_doc_ids: Vec<String>,
    pub linked_secret_keys: Vec<String>,
    pub linked_case_ids: Vec<String>,
    pub last_seen_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityAssetDetail {
    pub asset: SecurityAsset,
    pub documents: Vec<MemoMetadata>,
    pub findings: Vec<SecurityFinding>,
    pub cases: Vec<SecurityCase>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum SecurityGraphNodeType {
    Document,
    Asset,
    Finding,
    Secret,
    Case,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum SecurityGraphEdgeType {
    DocumentAsset,
    AssetFinding,
    AssetSecret,
    FindingCase,
    AssetCase,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityGraphNode {
    pub id: String,
    pub node_type: SecurityGraphNodeType,
    pub label: String,
    pub severity: Option<FindingSeverity>,
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityGraphEdge {
    pub id: String,
    pub edge_type: SecurityGraphEdgeType,
    pub from: String,
    pub to: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityAssetGraph {
    pub nodes: Vec<SecurityGraphNode>,
    pub edges: Vec<SecurityGraphEdge>,
}

pub fn extract_assets(
    doc_id: &str,
    title: &str,
    markdown: &str,
    last_seen_at: i64,
) -> Vec<SecurityAsset> {
    let mut assets = Vec::new();
    let mut seen = HashSet::new();

    for token in tokenize(markdown) {
        if token.starts_with("{{secret:") && token.ends_with("}}") {
            let key = token
                .trim_start_matches("{{secret:")
                .trim_end_matches("}}")
                .trim();
            push_asset(
                &mut assets,
                &mut seen,
                SecurityAssetType::Secret,
                key,
                doc_id,
                Vec::new(),
                vec![key.to_string()],
                last_seen_at,
            );
        } else if let Some(database) = normalize_database_asset(&token) {
            push_asset(
                &mut assets,
                &mut seen,
                SecurityAssetType::Database,
                &database,
                doc_id,
                Vec::new(),
                Vec::new(),
                last_seen_at,
            );
        } else if token.starts_with("http://") || token.starts_with("https://") {
            let normalized = normalize_url_asset(&token);
            push_asset(
                &mut assets,
                &mut seen,
                SecurityAssetType::Url,
                &normalized,
                doc_id,
                Vec::new(),
                Vec::new(),
                last_seen_at,
            );
        } else if looks_like_api_path(&token) {
            push_asset(
                &mut assets,
                &mut seen,
                SecurityAssetType::ApiEndpoint,
                &token,
                doc_id,
                Vec::new(),
                Vec::new(),
                last_seen_at,
            );
        } else if let Some((asset_type, name)) = prefixed_asset_from_token(&token) {
            push_asset(
                &mut assets,
                &mut seen,
                asset_type,
                &name,
                doc_id,
                Vec::new(),
                Vec::new(),
                last_seen_at,
            );
        }
    }

    extract_labeled_assets(markdown, &mut assets, &mut seen, doc_id, last_seen_at);

    if !title.trim().is_empty() {
        push_asset(
            &mut assets,
            &mut seen,
            SecurityAssetType::Service,
            title.trim(),
            doc_id,
            Vec::new(),
            Vec::new(),
            last_seen_at,
        );
    }

    assets
}

pub fn merge_assets(asset_sets: Vec<Vec<SecurityAsset>>) -> Vec<SecurityAsset> {
    let mut merged: HashMap<String, SecurityAsset> = HashMap::new();

    for asset in asset_sets.into_iter().flatten() {
        let entry = merged
            .entry(asset.id.clone())
            .or_insert_with(|| SecurityAsset {
                id: asset.id.clone(),
                asset_type: asset.asset_type.clone(),
                name: asset.name.clone(),
                aliases: asset.aliases.clone(),
                tags: asset.tags.clone(),
                source_doc_ids: Vec::new(),
                linked_secret_keys: Vec::new(),
                linked_case_ids: Vec::new(),
                last_seen_at: asset.last_seen_at,
            });

        for doc_id in asset.source_doc_ids {
            if !entry.source_doc_ids.contains(&doc_id) {
                entry.source_doc_ids.push(doc_id);
            }
        }
        for secret_key in asset.linked_secret_keys {
            if !entry.linked_secret_keys.contains(&secret_key) {
                entry.linked_secret_keys.push(secret_key);
            }
        }
        for alias in asset.aliases {
            if !entry.aliases.contains(&alias) {
                entry.aliases.push(alias);
            }
        }
        for tag in asset.tags {
            if !entry.tags.contains(&tag) {
                entry.tags.push(tag);
            }
        }
        entry.last_seen_at = entry.last_seen_at.max(asset.last_seen_at);
    }

    let mut assets = merged.into_values().collect::<Vec<_>>();
    assets.sort_by(|left, right| {
        asset_rank(&left.asset_type)
            .cmp(&asset_rank(&right.asset_type))
            .then_with(|| left.name.cmp(&right.name))
    });
    assets
}

pub fn find_asset<'a>(
    assets: &'a [SecurityAsset],
    asset_id: Option<&str>,
    query: Option<&str>,
) -> Option<&'a SecurityAsset> {
    let asset_id = asset_id.map(str::trim).filter(|value| !value.is_empty());
    let query = query.map(str::trim).filter(|value| !value.is_empty());
    if asset_id.is_none() && query.is_none() {
        return None;
    }
    if let Some(asset_id) = asset_id {
        if let Some(asset) = assets.iter().find(|asset| asset.id == asset_id) {
            return Some(asset);
        }
    }
    let query = query?.to_ascii_lowercase();
    assets.iter().find(|asset| {
        asset.name.to_ascii_lowercase().contains(&query)
            || asset
                .linked_secret_keys
                .iter()
                .any(|key| key.to_ascii_lowercase().contains(&query))
            || asset
                .aliases
                .iter()
                .any(|alias| alias.to_ascii_lowercase().contains(&query))
            || asset
                .tags
                .iter()
                .any(|tag| tag.to_ascii_lowercase().contains(&query))
    })
}

pub fn build_asset_detail(
    asset: SecurityAsset,
    documents: &[MemoMetadata],
    findings: &[SecurityFinding],
    cases: &[SecurityCase],
) -> SecurityAssetDetail {
    let source_doc_ids = asset.source_doc_ids.iter().collect::<HashSet<_>>();
    let documents = documents
        .iter()
        .filter(|doc| source_doc_ids.contains(&doc.id))
        .cloned()
        .collect::<Vec<_>>();
    let findings = findings
        .iter()
        .filter(|finding| asset_matches_finding(&asset, finding))
        .cloned()
        .collect::<Vec<_>>();
    let finding_ids = findings
        .iter()
        .map(|finding| finding.id.as_str())
        .collect::<HashSet<_>>();
    let cases = cases
        .iter()
        .filter(|case| {
            (asset.asset_type == SecurityAssetType::Service
                && asset.source_doc_ids.contains(&case.source_doc_id))
                || case
                    .source_finding_id
                    .as_deref()
                    .is_some_and(|finding_id| finding_ids.contains(finding_id))
                || asset.linked_case_ids.contains(&case.id)
        })
        .cloned()
        .collect::<Vec<_>>();

    SecurityAssetDetail {
        asset,
        documents,
        findings,
        cases,
    }
}

pub fn build_asset_graph(
    assets: &[SecurityAsset],
    documents: &[MemoMetadata],
    findings: &[SecurityFinding],
    cases: &[SecurityCase],
    focus_asset_id: Option<&str>,
) -> SecurityAssetGraph {
    let selected_assets = assets
        .iter()
        .filter(|asset| {
            focus_asset_id
                .map(|asset_id| asset.id == asset_id)
                .unwrap_or(true)
        })
        .take(12)
        .collect::<Vec<_>>();
    let document_by_id = documents
        .iter()
        .map(|document| (document.id.as_str(), document))
        .collect::<HashMap<_, _>>();
    let mut graph = GraphBuilder::default();

    for asset in selected_assets {
        let asset_node_id = graph.add_node(SecurityGraphNode {
            id: graph_asset_node_id(&asset.id),
            node_type: SecurityGraphNodeType::Asset,
            label: asset.name.clone(),
            severity: None,
            status: Some(format!("{:?}", asset.asset_type)),
        });

        for doc_id in &asset.source_doc_ids {
            if let Some(document) = document_by_id.get(doc_id.as_str()) {
                let document_node_id = graph.add_node(SecurityGraphNode {
                    id: graph_document_node_id(&document.id),
                    node_type: SecurityGraphNodeType::Document,
                    label: document.title.clone(),
                    severity: None,
                    status: Some(document.file_name.clone()),
                });
                graph.add_edge(
                    SecurityGraphEdgeType::DocumentAsset,
                    &document_node_id,
                    &asset_node_id,
                    "引用资产",
                );
            }
        }

        for secret_key in &asset.linked_secret_keys {
            let secret_node_id = graph.add_node(SecurityGraphNode {
                id: graph_secret_node_id(secret_key),
                node_type: SecurityGraphNodeType::Secret,
                label: format!("{{{{secret:{secret_key}}}}}"),
                severity: None,
                status: Some("placeholder".to_string()),
            });
            graph.add_edge(
                SecurityGraphEdgeType::AssetSecret,
                &asset_node_id,
                &secret_node_id,
                "使用 Secret",
            );
        }

        let related_findings = findings
            .iter()
            .filter(|finding| asset_matches_finding(asset, finding))
            .collect::<Vec<_>>();
        let related_finding_ids = related_findings
            .iter()
            .map(|finding| finding.id.as_str())
            .collect::<HashSet<_>>();

        for finding in related_findings {
            let finding_node_id = graph.add_node(SecurityGraphNode {
                id: graph_finding_node_id(&finding.id),
                node_type: SecurityGraphNodeType::Finding,
                label: format!("L{} · {}", finding.line_start, finding.title),
                severity: Some(finding.severity.clone()),
                status: Some(finding.status.as_storage().to_string()),
            });
            graph.add_edge(
                SecurityGraphEdgeType::AssetFinding,
                &asset_node_id,
                &finding_node_id,
                "关联风险",
            );
        }

        for case in cases.iter().filter(|case| {
            (asset.asset_type == SecurityAssetType::Service
                && asset.source_doc_ids.contains(&case.source_doc_id))
                || case
                    .source_finding_id
                    .as_deref()
                    .is_some_and(|finding_id| related_finding_ids.contains(finding_id))
                || asset.linked_case_ids.contains(&case.id)
        }) {
            let case_node_id = graph.add_node(SecurityGraphNode {
                id: graph_case_node_id(&case.id),
                node_type: SecurityGraphNodeType::Case,
                label: case.title.clone(),
                severity: Some(case.severity.clone()),
                status: Some(case.status.as_action().to_string()),
            });
            if let Some(finding_id) = case.source_finding_id.as_deref() {
                let finding_node_id = graph_finding_node_id(finding_id);
                if graph.has_node(&finding_node_id) {
                    graph.add_edge(
                        SecurityGraphEdgeType::FindingCase,
                        &finding_node_id,
                        &case_node_id,
                        "治理记录",
                    );
                    continue;
                }
            }
            graph.add_edge(
                SecurityGraphEdgeType::AssetCase,
                &asset_node_id,
                &case_node_id,
                "治理记录",
            );
        }
    }

    graph.finish()
}

#[derive(Default)]
struct GraphBuilder {
    nodes: Vec<SecurityGraphNode>,
    edges: Vec<SecurityGraphEdge>,
    node_ids: HashSet<String>,
    edge_ids: HashSet<String>,
}

impl GraphBuilder {
    fn add_node(&mut self, node: SecurityGraphNode) -> String {
        let id = node.id.clone();
        if self.node_ids.insert(id.clone()) {
            self.nodes.push(node);
        }
        id
    }

    fn has_node(&self, node_id: &str) -> bool {
        self.node_ids.contains(node_id)
    }

    fn add_edge(&mut self, edge_type: SecurityGraphEdgeType, from: &str, to: &str, label: &str) {
        let id = format!("edge-{}-{}-{:?}", from, to, edge_type);
        if !self.edge_ids.insert(id.clone()) {
            return;
        }
        self.edges.push(SecurityGraphEdge {
            id,
            edge_type,
            from: from.to_string(),
            to: to.to_string(),
            label: label.to_string(),
        });
    }

    fn finish(mut self) -> SecurityAssetGraph {
        self.nodes.sort_by(|left, right| left.id.cmp(&right.id));
        self.edges.sort_by(|left, right| left.id.cmp(&right.id));
        SecurityAssetGraph {
            nodes: self.nodes,
            edges: self.edges,
        }
    }
}

fn push_asset(
    assets: &mut Vec<SecurityAsset>,
    seen: &mut HashSet<String>,
    asset_type: SecurityAssetType,
    name: &str,
    doc_id: &str,
    aliases: Vec<String>,
    linked_secret_keys: Vec<String>,
    last_seen_at: i64,
) {
    let name = name
        .trim()
        .trim_matches(|ch: char| {
            matches!(
                ch,
                '"' | '\'' | '`' | ',' | ';' | ')' | '(' | ']' | '。' | '，' | '；' | '：'
            )
        })
        .to_string();
    if name.is_empty() {
        return;
    }

    let id = stable_asset_id(&asset_type, &name);
    if !seen.insert(id.clone()) {
        return;
    }

    assets.push(SecurityAsset {
        id,
        asset_type,
        name,
        aliases,
        tags: Vec::new(),
        source_doc_ids: vec![doc_id.to_string()],
        linked_secret_keys,
        linked_case_ids: Vec::new(),
        last_seen_at,
    });
}

fn asset_matches_finding(asset: &SecurityAsset, finding: &SecurityFinding) -> bool {
    if asset.asset_type == SecurityAssetType::Service
        && asset.source_doc_ids.contains(&finding.doc_id)
    {
        return true;
    }
    let haystack = format!(
        "{}\n{}\n{}\n{}",
        finding.title, finding.detail, finding.evidence, finding.recommendation
    )
    .to_ascii_lowercase();
    haystack.contains(&asset.name.to_ascii_lowercase())
        || asset
            .linked_secret_keys
            .iter()
            .any(|key| haystack.contains(&key.to_ascii_lowercase()))
}

fn graph_document_node_id(id: &str) -> String {
    format!("document:{id}")
}

fn graph_asset_node_id(id: &str) -> String {
    format!("asset:{id}")
}

fn graph_finding_node_id(id: &str) -> String {
    format!("finding:{id}")
}

fn graph_case_node_id(id: &str) -> String {
    format!("case:{id}")
}

fn graph_secret_node_id(secret_key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(secret_key.as_bytes());
    let hash = hasher.finalize();
    format!("secret:{}", hex_prefix(&hash, 12))
}

fn tokenize(markdown: &str) -> Vec<String> {
    markdown
        .split(|ch: char| {
            ch.is_whitespace() || matches!(ch, '<' | '>' | '"' | '\'' | '`' | ',' | ';')
        })
        .filter(|token| !token.trim().is_empty())
        .map(|token| {
            token
                .trim_matches(|ch: char| matches!(ch, ')' | '(' | '[' | ']' | '.'))
                .trim_matches(|ch: char| matches!(ch, '。' | '，' | '；' | '：'))
                .to_string()
        })
        .collect()
}

fn normalize_url_asset(value: &str) -> String {
    let trimmed = value.trim_end_matches(['.', ',', ';', ')']);
    let Some((scheme, rest)) = trimmed.split_once("://") else {
        return trimmed.to_string();
    };
    let host = rest.split('/').next().unwrap_or(rest);
    format!("{scheme}://{host}")
}

fn normalize_database_asset(value: &str) -> Option<String> {
    let trimmed = value.trim_end_matches(['.', ',', ';', ')']);
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("jdbc:postgresql://") {
        return Some(normalize_database_endpoint(
            "jdbc:postgresql",
            &trimmed["jdbc:postgresql://".len()..],
        ));
    }
    if lower.starts_with("jdbc:mysql://") {
        return Some(normalize_database_endpoint(
            "jdbc:mysql",
            &trimmed["jdbc:mysql://".len()..],
        ));
    }
    for scheme in ["postgresql", "postgres", "mysql", "mongodb", "redis"] {
        let prefix = format!("{scheme}://");
        if lower.starts_with(&prefix) {
            return Some(normalize_database_endpoint(
                scheme,
                &trimmed[prefix.len()..],
            ));
        }
    }
    None
}

fn normalize_database_endpoint(scheme: &str, rest: &str) -> String {
    let without_credentials = rest.rsplit('@').next().unwrap_or(rest);
    let endpoint = without_credentials
        .split(['?', '#'])
        .next()
        .unwrap_or(without_credentials)
        .trim_end_matches('/');
    if endpoint.is_empty() {
        scheme.to_string()
    } else {
        format!("{scheme}://{endpoint}")
    }
}

fn prefixed_asset_from_token(value: &str) -> Option<(SecurityAssetType, String)> {
    let (prefix, rest) = value.split_once(':')?;
    let rest = rest.trim_matches(|ch: char| matches!(ch, '/' | ',' | ';' | ')' | '('));
    if rest.is_empty() {
        return None;
    }
    let asset_type = match prefix.to_ascii_lowercase().as_str() {
        "npm" | "crate" | "pip" | "maven" | "go" | "dependency" | "dep" => {
            SecurityAssetType::Dependency
        }
        "env" | "environment" => SecurityAssetType::Environment,
        "datatype" | "data" | "pii" => SecurityAssetType::DataType,
        "db" | "database" => SecurityAssetType::Database,
        _ => return None,
    };
    Some((asset_type, rest.to_string()))
}

fn extract_labeled_assets(
    markdown: &str,
    assets: &mut Vec<SecurityAsset>,
    seen: &mut HashSet<String>,
    doc_id: &str,
    last_seen_at: i64,
) {
    for line in markdown.lines() {
        let Some((label, value)) = split_labeled_line(line) else {
            continue;
        };
        let Some(asset_type) = asset_type_from_label(&label) else {
            continue;
        };
        for item in split_label_values(&value) {
            push_asset(
                assets,
                seen,
                asset_type.clone(),
                &item,
                doc_id,
                Vec::new(),
                Vec::new(),
                last_seen_at,
            );
        }
    }
}

fn split_labeled_line(line: &str) -> Option<(String, String)> {
    let clean = line
        .trim()
        .trim_start_matches(['-', '*'])
        .trim()
        .trim_start_matches(|ch: char| ch.is_ascii_digit() || ch == '.')
        .trim();
    let (label, value) = clean.split_once('：').or_else(|| clean.split_once(':'))?;
    let label = label.trim().trim_matches(['#', '*']).trim().to_string();
    let value = value.trim().to_string();
    if label.is_empty() || value.is_empty() {
        None
    } else {
        Some((label, value))
    }
}

fn asset_type_from_label(label: &str) -> Option<SecurityAssetType> {
    let label = label.to_ascii_lowercase();
    if label.contains("database")
        || label == "db"
        || label.contains("数据库")
        || label.contains("数据存储")
    {
        Some(SecurityAssetType::Database)
    } else if label.contains("dependency")
        || label.contains("dependencies")
        || label.contains("package")
        || label.contains("依赖")
        || label.contains("第三方组件")
    {
        Some(SecurityAssetType::Dependency)
    } else if label.contains("environment")
        || label == "env"
        || label.contains("部署环境")
        || label.contains("运行环境")
        || label == "环境"
    {
        Some(SecurityAssetType::Environment)
    } else if label.contains("data type")
        || label.contains("datatype")
        || label.contains("business data")
        || label.contains("pii")
        || label.contains("数据类型")
        || label.contains("业务数据")
        || label.contains("敏感数据")
        || label.contains("个人信息")
    {
        Some(SecurityAssetType::DataType)
    } else {
        None
    }
}

fn split_label_values(value: &str) -> Vec<String> {
    value
        .split(|ch: char| matches!(ch, ',' | '，' | '、' | ';' | '；' | '\n' | '\t' | '|'))
        .flat_map(|part| part.split(" / "))
        .map(|part| {
            part.trim()
                .trim_matches(['`', '"', '\'', '[', ']', '(', ')'])
        })
        .filter(|part| !part.is_empty())
        .map(str::to_string)
        .collect()
}

fn looks_like_api_path(value: &str) -> bool {
    if !value.starts_with('/') || value.len() < 4 {
        return false;
    }
    let lower = value.to_ascii_lowercase();
    lower.starts_with("/api/")
        || lower.starts_with("/v1/")
        || lower.starts_with("/v2/")
        || lower.starts_with("/oauth/")
        || lower.starts_with("/auth/")
}

fn stable_asset_id(asset_type: &SecurityAssetType, name: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(format!("{asset_type:?}").as_bytes());
    hasher.update(name.to_ascii_lowercase().as_bytes());
    let hash = hasher.finalize();
    format!("asset-{}", hex_prefix(&hash, 12))
}

fn asset_rank(asset_type: &SecurityAssetType) -> u8 {
    match asset_type {
        SecurityAssetType::Service => 0,
        SecurityAssetType::ApiEndpoint => 1,
        SecurityAssetType::Database => 2,
        SecurityAssetType::Dependency => 3,
        SecurityAssetType::Environment => 4,
        SecurityAssetType::DataType => 5,
        SecurityAssetType::Url => 6,
        SecurityAssetType::Secret => 7,
    }
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
    use super::super::audit::{FindingKind, FindingSeverity, FindingStatus};
    use super::super::governance::{
        SecurityCase, SecurityCaseEvent, SecurityCaseStatus, SecurityCaseType,
    };
    use super::*;

    #[test]
    fn extracts_urls_api_paths_and_secret_placeholders() {
        let assets = extract_assets(
            "doc-1",
            "支付服务",
            "回调 https://pay.example.com/cb\nPOST /api/payments\nsecret {{secret:payToken}}\n数据库：PostgreSQL 主库\n依赖：npm:jsonwebtoken、crate:axum\n部署环境：prod / staging\n数据类型：手机号、订单地址\n连接 postgresql://user:secret@db.internal:5432/payments",
            123,
        );

        assert!(assets
            .iter()
            .any(|asset| asset.asset_type == SecurityAssetType::Url));
        assert!(assets
            .iter()
            .any(|asset| asset.asset_type == SecurityAssetType::ApiEndpoint));
        assert!(assets
            .iter()
            .any(|asset| asset.asset_type == SecurityAssetType::Secret));
        assert!(assets
            .iter()
            .any(|asset| asset.asset_type == SecurityAssetType::Service));
        assert!(assets
            .iter()
            .any(|asset| asset.asset_type == SecurityAssetType::Database
                && asset.name.contains("PostgreSQL")));
        assert!(assets
            .iter()
            .any(|asset| asset.asset_type == SecurityAssetType::Database
                && asset.name == "postgresql://db.internal:5432/payments"));
        assert!(assets
            .iter()
            .any(|asset| asset.asset_type == SecurityAssetType::Dependency
                && asset.name == "npm:jsonwebtoken"));
        assert!(assets.iter().any(
            |asset| asset.asset_type == SecurityAssetType::Environment && asset.name == "prod"
        ));
        assert!(
            assets
                .iter()
                .any(|asset| asset.asset_type == SecurityAssetType::DataType
                    && asset.name == "手机号")
        );
        assert!(!assets.iter().any(|asset| asset.name.contains("secret")));
    }

    #[test]
    fn builds_relationship_graph_without_secret_plaintext() {
        let documents = vec![MemoMetadata {
            id: "doc-1".to_string(),
            file_name: "pay.md".to_string(),
            title: "支付服务".to_string(),
            summary: "支付服务审计".to_string(),
            updated_at: 100,
        }];
        let assets = extract_assets(
            "doc-1",
            "支付服务",
            "回调 https://pay.example.com/cb\nsecret {{secret:payToken}}",
            100,
        );
        let finding = SecurityFinding {
            id: "finding-1".to_string(),
            doc_id: "doc-1".to_string(),
            line_start: 2,
            line_end: 2,
            severity: FindingSeverity::Critical,
            kind: FindingKind::HardcodedSecret,
            title: "硬编码密钥泄露".to_string(),
            detail: "发现疑似明文密钥。".to_string(),
            evidence: "payToken: sk-...cdef".to_string(),
            recommendation: "移入 KDBX。".to_string(),
            status: FindingStatus::Open,
        };
        let case = SecurityCase {
            id: "case-1".to_string(),
            case_type: SecurityCaseType::Risk,
            title: "硬编码密钥泄露".to_string(),
            severity: FindingSeverity::Critical,
            status: SecurityCaseStatus::Open,
            source_doc_id: "doc-1".to_string(),
            source_finding_id: Some("finding-1".to_string()),
            linked_assets: Vec::new(),
            owner: None,
            due_at: None,
            accepted_until: None,
            rationale: None,
            impact_scope: None,
            compensating_controls: None,
            reviewer: None,
            created_at: 100,
            updated_at: 100,
            events: vec![SecurityCaseEvent {
                event_type: "findingDetected".to_string(),
                summary: "发现风险".to_string(),
                created_at: 100,
            }],
        };

        let graph = build_asset_graph(
            &assets,
            &documents,
            std::slice::from_ref(&finding),
            &[case],
            None,
        );
        let graph_json = serde_json::to_string(&graph).expect("graph json");

        assert!(graph
            .nodes
            .iter()
            .any(|node| node.node_type == SecurityGraphNodeType::Document));
        assert!(graph
            .nodes
            .iter()
            .any(|node| node.node_type == SecurityGraphNodeType::Finding));
        assert!(graph
            .nodes
            .iter()
            .any(|node| node.node_type == SecurityGraphNodeType::Case));
        assert!(graph
            .edges
            .iter()
            .any(|edge| edge.edge_type == SecurityGraphEdgeType::FindingCase));
        assert!(graph_json.contains("{{secret:payToken}}"));
        assert!(!graph_json.contains("sk-test-1234567890abcdef"));
    }
}
