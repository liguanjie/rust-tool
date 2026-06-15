use axum::{
    routing::{get, post},
    Router,
};
use http::{header, HeaderValue, Method};
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    trace::TraceLayer,
};

use crate::routes::{clash_party, health, memo, tools};
use crate::static_files;
use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(default)]
pub struct LocalConfig {
    #[serde(alias = "customDataDir")]
    pub custom_data_dir: Option<String>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Clone)]
pub struct AppState {
    pub memo_manager: Arc<rust_tool_core::memo::MemoManager>,
    pub default_data_dir: PathBuf,
    pub active_data_dir: PathBuf,
}

pub fn get_default_base_dir() -> PathBuf {
    if let Ok(dir_str) = std::env::var("RUSTTOOL_DATA_DIR") {
        PathBuf::from(dir_str)
    } else if cfg!(windows) {
        std::env::var("APPDATA")
            .map(PathBuf::from)
            .map(|base| base.join("rust-tool"))
            .unwrap_or_else(|_| PathBuf::from(".").join("memos_data"))
    } else if cfg!(target_os = "macos") {
        std::env::var("HOME")
            .map(PathBuf::from)
            .map(|home| {
                home.join("Library")
                    .join("Application Support")
                    .join("rust-tool")
            })
            .unwrap_or_else(|_| PathBuf::from(".").join("memos_data"))
    } else if let Ok(app_data) = std::env::var("APPDATA") {
        PathBuf::from(app_data).join("rust-tool")
    } else if cfg!(unix) {
        std::env::var("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|_| std::env::var("HOME").map(|home| PathBuf::from(home).join(".local/share")))
            .map(|base| base.join("rust-tool"))
            .unwrap_or_else(|_| PathBuf::from(".").join("memos_data"))
    } else {
        PathBuf::from(".").join("memos_data")
    }
}

pub fn get_local_config_path(default_dir: &std::path::Path) -> PathBuf {
    default_dir.join("config.json")
}

pub fn read_local_config(default_dir: &std::path::Path) -> LocalConfig {
    let config_path = default_dir.join("config.json");
    if config_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&config_path) {
            if let Ok(cfg) = serde_json::from_str::<LocalConfig>(&content) {
                return cfg;
            }
        }
    }
    LocalConfig::default()
}

pub fn write_local_config(
    default_dir: &std::path::Path,
    config: &LocalConfig,
) -> Result<(), String> {
    std::fs::create_dir_all(default_dir)
        .map_err(|error| format!("Failed to create config directory: {error:?}"))?;
    let mut merged = read_local_config(default_dir);
    merged.custom_data_dir = config.custom_data_dir.clone();
    let config_path = get_local_config_path(default_dir);
    let json = serde_json::to_string_pretty(&merged)
        .map_err(|error| format!("Failed to serialize local config: {error:?}"))?;
    std::fs::write(&config_path, json)
        .map_err(|error| format!("Failed to write local config: {error:?}"))
}

pub fn resolve_memo_data_dir(default_dir: &std::path::Path) -> PathBuf {
    let cfg = read_local_config(default_dir);
    if let Some(custom) = cfg.custom_data_dir {
        if !custom.trim().is_empty() {
            return PathBuf::from(custom);
        }
    }
    default_dir.to_path_buf()
}

fn get_memo_data_dir() -> PathBuf {
    let default_dir = get_default_base_dir();
    resolve_memo_data_dir(&default_dir)
}

pub fn build_app() -> Router {
    let default_data_dir = get_default_base_dir();
    let data_dir = get_memo_data_dir();
    let memo_manager = Arc::new(
        rust_tool_core::memo::MemoManager::new(&data_dir)
            .expect("Failed to initialize MemoManager"),
    );
    let state = AppState {
        memo_manager,
        default_data_dir,
        active_data_dir: data_dir,
    };

    build_app_with_state(state)
}

pub fn build_app_with_state(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list([
            HeaderValue::from_static("http://127.0.0.1:5173"),
            HeaderValue::from_static("http://localhost:5173"),
        ]))
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([header::CONTENT_TYPE]);

    Router::new()
        .route("/api/health", get(health::health))
        .route("/api/tools/vless-to-mihomo", post(tools::vless_to_mihomo))
        .route("/api/clash-party/state", get(clash_party::state))
        .route("/api/clash-party/health", get(clash_party::health))
        .route(
            "/api/clash-party/nodes/check",
            post(clash_party::check_node),
        )
        .route(
            "/api/clash-party/subscriptions/switch",
            post(clash_party::switch_subscription),
        )
        .route(
            "/api/clash-party/nodes/switch",
            post(clash_party::switch_node),
        )
        // AI Document Assistant endpoints
        .route("/api/memo/unlock", post(memo::unlock))
        .route("/api/memo/lock", post(memo::lock))
        .route("/api/memo/status", get(memo::status))
        .route("/api/memo/settings", post(memo::update_settings))
        .route("/api/memo/data-dir", get(memo::data_dir))
        .route("/api/memo/data-dir/migrate", post(memo::migrate_data_dir))
        .route("/api/memo/test-connection", post(memo::test_connection))
        .route("/api/memo/list", get(memo::list_documents))
        .route("/api/memo/doc/:id", get(memo::get_document))
        .route("/api/memo/audit/scan", post(memo::audit_scan))
        .route(
            "/api/memo/audit/finding/status",
            post(memo::audit_update_finding_status),
        )
        .route("/api/memo/audit/fix-preview", post(memo::audit_fix_preview))
        .route("/api/memo/audit/redact", post(memo::audit_redact))
        .route("/api/memo/history/doc-diff", post(memo::document_risk_diff))
        .route(
            "/api/memo/governance/summary",
            get(memo::governance_summary),
        )
        .route("/api/memo/governance/cases", get(memo::governance_cases))
        .route("/api/memo/governance/events", get(memo::governance_events))
        .route(
            "/api/memo/governance/case/status",
            post(memo::governance_update_case_status),
        )
        .route(
            "/api/memo/governance/case/accept",
            post(memo::governance_accept_case),
        )
        .route("/api/memo/assets/list", get(memo::security_assets))
        .route("/api/memo/assets/detail", get(memo::security_asset_detail))
        .route("/api/memo/assets/graph", get(memo::security_asset_graph))
        .route("/api/memo/reports/generate", post(memo::generate_report))
        .route("/api/memo/share/export", post(memo::safe_share_export))
        .route("/api/memo/standards/list", get(memo::standards_list))
        .route(
            "/api/memo/standards/checklist",
            get(memo::standards_checklist),
        )
        .route(
            "/api/memo/standards/checklist/status",
            post(memo::standards_update_checklist_status),
        )
        .route("/api/memo/secrets", get(memo::list_secrets))
        .route("/api/memo/secrets/reveal", post(memo::reveal_secret))
        .route(
            "/api/memo/change-master-password",
            post(memo::change_master_password),
        )
        .route("/api/memo/save", post(memo::save_document))
        .route("/api/memo/draft", post(memo::draft_document))
        .route("/api/memo/delete", post(memo::delete_document))
        .route("/api/memo/query", post(memo::query_memos))
        .route("/api/memo/chat", post(memo::chat))
        .route("/api/memo/backup", post(memo::backup_memos))
        .route("/api/memo/restore", post(memo::restore_memos))
        .route("/api/memo/translate-key", post(memo::translate_key))
        .route("/api/memo/tree-state", get(memo::get_tree_state).post(memo::set_tree_state))
        .route("/api/memo/folder/rename", post(memo::rename_folder))
        .route("/api/memo/folder/delete", post(memo::delete_folder))
        .fallback(static_files::serve_static)
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use serde_json::Value;
    use std::time::{SystemTime, UNIX_EPOCH};
    use tower::ServiceExt;

    fn make_test_dir(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("rusttool_server_{}_{}", name, stamp));
        std::fs::create_dir_all(&dir).expect("test temp dir");
        dir
    }

    fn test_app(data_dir: &std::path::Path) -> Router {
        let memo_manager =
            Arc::new(rust_tool_core::memo::MemoManager::new(data_dir).expect("memo manager"));
        build_app_with_state(AppState {
            memo_manager,
            default_data_dir: data_dir.to_path_buf(),
            active_data_dir: data_dir.to_path_buf(),
        })
    }

    async fn response_json(response: axum::response::Response) -> Value {
        let bytes = response
            .into_body()
            .collect()
            .await
            .expect("body")
            .to_bytes();
        serde_json::from_slice(&bytes).expect("json body")
    }

    #[tokio::test]
    async fn memo_routes_return_unauthorized_when_locked() {
        let data_dir = make_test_dir("locked_routes");
        let locked_migration_target = make_test_dir("locked_migration_target");
        let app = test_app(&data_dir);

        let data_dir_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/memo/data-dir")
                    .body(Body::empty())
                    .expect("data dir request"),
            )
            .await
            .expect("data dir response");
        assert_eq!(data_dir_response.status(), StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/memo/list")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let body = response_json(response).await;
        assert_eq!(body["error"]["code"], "vault_locked");

        let test_connection_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/memo/test-connection")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"ollamaUrl":"https://api.openai.com/v1","apiKey":null,"chatModel":"test","embeddingModel":"test","reasoningEffort":"xhigh","disableResponseStorage":true}"#,
                    ))
                    .expect("test connection request"),
            )
            .await
            .expect("test connection response");

        assert_eq!(test_connection_response.status(), StatusCode::UNAUTHORIZED);
        let body = response_json(test_connection_response).await;
        assert_eq!(body["error"]["code"], "vault_locked");

        let migrate_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/memo/data-dir/migrate")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "targetDir": locked_migration_target.to_string_lossy().to_string()
                        })
                        .to_string(),
                    ))
                    .expect("migrate request"),
            )
            .await
            .expect("migrate response");

        assert_eq!(migrate_response.status(), StatusCode::UNAUTHORIZED);
        let body = response_json(migrate_response).await;
        assert_eq!(body["error"]["code"], "vault_locked");

        let chat_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/memo/chat")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"query":"Hi"}"#))
                    .expect("chat request"),
            )
            .await
            .expect("chat response");

        assert_eq!(chat_response.status(), StatusCode::UNAUTHORIZED);
        let body = response_json(chat_response).await;
        assert_eq!(body["error"]["code"], "vault_locked");

        let audit_response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/memo/audit/scan")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"docId":"new","markdown":"api_key = \"sk-test-1234567890abcdef\""}"#,
                    ))
                    .expect("audit scan request"),
            )
            .await
            .expect("audit scan response");

        assert_eq!(audit_response.status(), StatusCode::UNAUTHORIZED);
        let body = response_json(audit_response).await;
        assert_eq!(body["error"]["code"], "vault_locked");

        let _ = std::fs::remove_dir_all(data_dir);
        let _ = std::fs::remove_dir_all(locked_migration_target);
    }

    #[tokio::test]
    async fn memo_routes_work_after_unlock() {
        let data_dir = make_test_dir("unlocked_routes");
        let app = test_app(&data_dir);

        let unlock_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/memo/unlock")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"password":"test-password"}"#))
                    .expect("unlock request"),
            )
            .await
            .expect("unlock response");
        assert_eq!(unlock_response.status(), StatusCode::OK);

        let list_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/memo/list")
                    .body(Body::empty())
                    .expect("list request"),
            )
            .await
            .expect("list response");

        assert_eq!(list_response.status(), StatusCode::OK);
        let body = response_json(list_response).await;
        assert_eq!(body, Value::Array(Vec::new()));

        let save_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/memo/save")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "id": null,
                            "fileName": "security-audit.md",
                            "title": "Security Audit",
                            "markdown": "api_key = \"sk-test-1234567890abcdef\"\nJWT 使用 HS256\n回调 http://api.example.com/hook\n数据库：PostgreSQL 主库\n依赖：npm:jsonwebtoken、crate:axum\n部署环境：prod / staging\n数据类型：手机号、订单地址\n连接 postgresql://user:secret@db.internal:5432/payments",
                            "secrets": {},
                            "summary": "Security audit sample"
                        })
                        .to_string(),
                    ))
                    .expect("save request"),
            )
            .await
            .expect("save response");
        assert_eq!(save_response.status(), StatusCode::OK);
        let saved_doc = response_json(save_response).await;
        let doc_id = saved_doc["id"].as_str().expect("saved doc id").to_string();
        assert_eq!(saved_doc["riskDiff"]["previousSavedAt"], Value::Null);
        assert_eq!(saved_doc["riskDiff"]["summary"]["currentTotal"], 4);

        let audit_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/memo/audit/scan")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "docId": doc_id,
                            "markdown": "api_key = \"sk-test-1234567890abcdef\"\nJWT 使用 HS256\n回调 http://api.example.com/hook\n数据库：PostgreSQL 主库\n依赖：npm:jsonwebtoken、crate:axum\n部署环境：prod / staging\n数据类型：手机号、订单地址\n连接 postgresql://user:secret@db.internal:5432/payments"
                        })
                        .to_string(),
                    ))
                    .expect("audit scan request"),
            )
            .await
            .expect("audit scan response");
        assert_eq!(audit_response.status(), StatusCode::OK);
        let body = response_json(audit_response).await;
        assert_eq!(body["summary"]["critical"], 2);
        assert_eq!(body["summary"]["warning"], 2);

        let redact_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/memo/audit/redact")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "markdown": "api_key = \"sk-test-1234567890abcdef\"\n连接 postgresql://user:secret@db.internal:5432/payments\nAPI Key: {{secret:apiKey}}"
                        })
                        .to_string(),
                    ))
                    .expect("redact request"),
            )
            .await
            .expect("redact response");
        assert_eq!(redact_response.status(), StatusCode::OK);
        let redact = response_json(redact_response).await;
        let redact_markdown = redact["markdown"].as_str().expect("redacted markdown");
        assert!(redact_markdown.contains("{{secret:pending_1}}"));
        assert!(redact_markdown.contains("{{secret:pending_2}}"));
        assert!(redact_markdown.contains("{{secret:apiKey}}"));
        assert!(!redact_markdown.contains("sk-test-1234567890abcdef"));
        assert!(!redact_markdown.contains("user:secret"));
        assert_eq!(redact["redactedSecretCount"], 2);
        assert_eq!(
            redact["secrets"]
                .as_array()
                .expect("redacted secrets")
                .len(),
            2
        );

        let governance_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/memo/governance/summary")
                    .body(Body::empty())
                    .expect("governance request"),
            )
            .await
            .expect("governance response");
        assert_eq!(governance_response.status(), StatusCode::OK);
        let governance = response_json(governance_response).await;
        assert_eq!(governance["assetSummary"]["databases"], 2);
        assert_eq!(governance["assetSummary"]["dependencies"], 2);
        assert_eq!(governance["assetSummary"]["environments"], 2);
        assert_eq!(governance["assetSummary"]["dataTypes"], 2);

        let cases_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/memo/governance/cases")
                    .body(Body::empty())
                    .expect("cases request"),
            )
            .await
            .expect("cases response");
        assert_eq!(cases_response.status(), StatusCode::OK);
        let cases = response_json(cases_response).await;
        let first_case_id = cases
            .as_array()
            .and_then(|items| items.first())
            .and_then(|item| item["id"].as_str())
            .expect("first case id")
            .to_string();

        let status_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/memo/governance/case/status")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "caseId": first_case_id,
                            "status": "fixing",
                            "owner": "sec",
                            "dueAt": "2026-06-30",
                            "rationale": "已进入修复排期"
                        })
                        .to_string(),
                    ))
                    .expect("case status request"),
            )
            .await
            .expect("case status response");
        assert_eq!(status_response.status(), StatusCode::OK);
        let updated_case = response_json(status_response).await;
        assert_eq!(updated_case["status"], "fixing");
        assert_eq!(updated_case["owner"], "sec");

        let incomplete_accept_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/memo/governance/case/accept")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "caseId": updated_case["id"],
                            "rationale": "临时接受",
                            "acceptedUntil": "2999-07-31",
                            "impactScope": "",
                            "compensatingControls": "",
                            "reviewer": "",
                            "owner": "sec"
                        })
                        .to_string(),
                    ))
                    .expect("incomplete case accept request"),
            )
            .await
            .expect("incomplete case accept response");
        assert_eq!(incomplete_accept_response.status(), StatusCode::BAD_REQUEST);

        let accept_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/memo/governance/case/accept")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "caseId": updated_case["id"],
                            "rationale": "第三方迁移窗口未到，临时接受",
                            "acceptedUntil": "2999-07-31",
                            "impactScope": "支付回调接口",
                            "compensatingControls": "增加回调来源监控和告警",
                            "reviewer": "sec-lead",
                            "owner": "sec"
                        })
                        .to_string(),
                    ))
                    .expect("case accept request"),
            )
            .await
            .expect("case accept response");
        assert_eq!(accept_response.status(), StatusCode::OK);
        let accepted_case = response_json(accept_response).await;
        assert_eq!(accepted_case["status"], "accepted");
        assert_eq!(accepted_case["acceptedUntil"], "2999-07-31");
        assert_eq!(accepted_case["impactScope"], "支付回调接口");
        assert_eq!(
            accepted_case["compensatingControls"],
            "增加回调来源监控和告警"
        );
        assert_eq!(accepted_case["reviewer"], "sec-lead");

        let standards_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/memo/standards/list")
                    .body(Body::empty())
                    .expect("standards request"),
            )
            .await
            .expect("standards response");
        assert_eq!(standards_response.status(), StatusCode::OK);
        let standards = response_json(standards_response).await;
        assert!(standards
            .as_array()
            .expect("standards array")
            .iter()
            .any(|standard| standard["id"] == "RT-SEC-SECRET-01"));

        let checklist_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/memo/standards/checklist?docId={doc_id}"))
                    .body(Body::empty())
                    .expect("checklist request"),
            )
            .await
            .expect("checklist response");
        assert_eq!(checklist_response.status(), StatusCode::OK);
        let checklist = response_json(checklist_response).await;
        assert!(checklist["items"]
            .as_array()
            .expect("checklist items")
            .iter()
            .any(|item| item["id"] == "secret-management" && item["recommended"] == true));

        let checklist_status_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/memo/standards/checklist/status")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "docId": doc_id,
                            "itemId": "secret-management",
                            "status": "done",
                            "note": "密钥已移入 KDBX"
                        })
                        .to_string(),
                    ))
                    .expect("checklist status request"),
            )
            .await
            .expect("checklist status response");
        assert_eq!(checklist_status_response.status(), StatusCode::OK);
        let checklist_item = response_json(checklist_status_response).await;
        assert_eq!(checklist_item["status"], "done");
        assert_eq!(checklist_item["note"], "密钥已移入 KDBX");

        let events_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/memo/governance/events")
                    .body(Body::empty())
                    .expect("events request"),
            )
            .await
            .expect("events response");
        assert_eq!(events_response.status(), StatusCode::OK);
        let events = response_json(events_response).await;
        assert!(events
            .as_array()
            .expect("events array")
            .iter()
            .any(|event| event["eventType"] == "caseAccepted"));

        let report_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/memo/reports/generate")
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .expect("report request"),
            )
            .await
            .expect("report response");
        assert_eq!(report_response.status(), StatusCode::OK);
        let report = response_json(report_response).await;
        let report_path = report["path"].as_str().expect("report path");
        assert!(std::path::Path::new(report_path).exists());
        let report_markdown = report["markdown"].as_str().expect("report markdown");
        assert!(report_markdown.contains("安全治理审计报告"));
        assert!(report_markdown.contains("硬编码密钥泄露"));
        assert!(report_markdown.contains("规范依据"));
        assert!(report_markdown.contains("安全 Checklist"));
        assert!(report_markdown.contains("安全例外台账"));
        assert!(report_markdown.contains("案件 ID："));
        assert!(report_markdown.contains("支付回调接口"));
        assert!(report_markdown.contains("增加回调来源监控和告警"));
        assert!(!report_markdown.contains("sk-test-1234567890abcdef"));

        let recent_report_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/memo/reports/generate")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "scope": "all",
                            "sinceDays": 30
                        })
                        .to_string(),
                    ))
                    .expect("recent report request"),
            )
            .await
            .expect("recent report response");
        assert_eq!(recent_report_response.status(), StatusCode::OK);
        let recent_report = response_json(recent_report_response).await;
        let recent_report_markdown = recent_report["markdown"]
            .as_str()
            .expect("recent report markdown");
        assert!(recent_report_markdown.contains("最近 30 天"));
        assert!(recent_report_markdown.contains("硬编码密钥泄露"));
        assert!(recent_report["fileName"]
            .as_str()
            .expect("recent report filename")
            .contains("recent-30"));
        assert!(!recent_report_markdown.contains("sk-test-1234567890abcdef"));

        let tag_report_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/memo/reports/generate")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "scope": "tags",
                            "tags": ["secret", "jwt"]
                        })
                        .to_string(),
                    ))
                    .expect("tag report request"),
            )
            .await
            .expect("tag report response");
        assert_eq!(tag_report_response.status(), StatusCode::OK);
        let tag_report = response_json(tag_report_response).await;
        let tag_report_markdown = tag_report["markdown"]
            .as_str()
            .expect("tag report markdown");
        assert!(tag_report_markdown.contains("标签：secret、jwt"));
        assert!(tag_report_markdown.contains("标签范围安全报告"));
        assert!(tag_report_markdown.contains("硬编码密钥泄露"));
        assert!(tag_report_markdown.contains("JWT 签名弱点"));
        assert!(tag_report["fileName"]
            .as_str()
            .expect("tag report filename")
            .contains("security-report-tags-secret-jwt-"));
        assert!(!tag_report_markdown.contains("sk-test-1234567890abcdef"));

        let safe_share_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/memo/share/export")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "docId": doc_id.clone(),
                            "markdown": "api_key = \"sk-test-1234567890abcdef\"\nJWT 使用 HS256\n回调 http://api.example.com/hook\n连接 postgresql://user:secret@db.internal:5432/payments\nAPI Key: {{secret:apiKey}}",
                            "includeAudit": true
                        })
                        .to_string(),
                    ))
                    .expect("safe share request"),
            )
            .await
            .expect("safe share response");
        assert_eq!(safe_share_response.status(), StatusCode::OK);
        let safe_share = response_json(safe_share_response).await;
        let safe_share_path = safe_share["path"].as_str().expect("safe share path");
        assert!(std::path::Path::new(safe_share_path).exists());
        let safe_share_markdown = safe_share["markdown"]
            .as_str()
            .expect("safe share markdown");
        assert!(safe_share_markdown.contains("分享说明"));
        assert!(safe_share_markdown.contains("Secret 明文：不包含"));
        assert!(safe_share_markdown.contains("安全审计摘要"));
        assert!(safe_share_markdown.contains("{{secret:pending_"));
        assert!(safe_share_markdown.contains("{{secret:apiKey}}"));
        assert!(!safe_share_markdown.contains("sk-test-1234567890abcdef"));
        assert!(!safe_share_markdown.contains("user:secret"));
        assert!(safe_share["redactedSecretCount"].as_u64().unwrap_or(0) >= 2);
        assert!(safe_share["findingCount"].as_u64().unwrap_or(0) >= 1);

        let report_events_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/memo/governance/events")
                    .body(Body::empty())
                    .expect("report events request"),
            )
            .await
            .expect("report events response");
        assert_eq!(report_events_response.status(), StatusCode::OK);
        let report_events = response_json(report_events_response).await;
        assert!(report_events
            .as_array()
            .expect("report events array")
            .iter()
            .any(|event| {
                event["eventType"] == "reportGenerated"
                    && event["metadata"]["caseIds"]
                        .as_array()
                        .is_some_and(|case_ids| !case_ids.is_empty())
                    && event["metadata"]["docIds"]
                        .as_array()
                        .is_some_and(|doc_ids| !doc_ids.is_empty())
            }));
        assert!(report_events
            .as_array()
            .expect("safe share events array")
            .iter()
            .any(|event| {
                event["eventType"] == "safeShareExported"
                    && event["metadata"]["docId"].as_str() == Some(doc_id.as_str())
                    && event["metadata"]["redactedSecretCount"]
                        .as_u64()
                        .unwrap_or(0)
                        >= 2
                    && !event.to_string().contains("sk-test-1234567890abcdef")
                    && !event.to_string().contains("user:secret")
            }));

        let doc_report_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/memo/reports/generate")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "scope": "document",
                            "docId": doc_id.clone()
                        })
                        .to_string(),
                    ))
                    .expect("document report request"),
            )
            .await
            .expect("document report response");
        assert_eq!(doc_report_response.status(), StatusCode::OK);
        let doc_report = response_json(doc_report_response).await;
        let doc_report_markdown = doc_report["markdown"]
            .as_str()
            .expect("document report markdown");
        assert!(doc_report_markdown.contains("单篇文档"));
        assert!(doc_report_markdown.contains("版本风险变化"));
        assert!(doc_report["fileName"]
            .as_str()
            .expect("document report filename")
            .contains("security-report-doc-"));

        let assets_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/memo/assets/list")
                    .body(Body::empty())
                    .expect("assets request"),
            )
            .await
            .expect("assets response");
        assert_eq!(assets_response.status(), StatusCode::OK);
        let assets = response_json(assets_response).await;
        let asset_items = assets.as_array().expect("assets array");
        assert!(asset_items
            .iter()
            .any(|item| item["assetType"] == "database"
                && item["name"] == "postgresql://db.internal:5432/payments"));
        assert!(asset_items
            .iter()
            .any(|item| item["assetType"] == "dependency" && item["name"] == "npm:jsonwebtoken"));
        assert!(asset_items
            .iter()
            .any(|item| item["assetType"] == "environment" && item["name"] == "prod"));
        assert!(asset_items
            .iter()
            .any(|item| item["assetType"] == "dataType" && item["name"] == "手机号"));
        assert!(!assets.to_string().contains("user:secret"));
        let asset_id = assets
            .as_array()
            .and_then(|items| items.iter().find(|item| item["assetType"] == "url"))
            .and_then(|item| item["id"].as_str())
            .expect("url asset id")
            .to_string();

        let asset_detail_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/memo/assets/detail?assetId={asset_id}"))
                    .body(Body::empty())
                    .expect("asset detail request"),
            )
            .await
            .expect("asset detail response");
        assert_eq!(asset_detail_response.status(), StatusCode::OK);
        let asset_detail = response_json(asset_detail_response).await;
        assert_eq!(asset_detail["asset"]["id"], asset_id);
        assert_eq!(asset_detail["documents"][0]["id"], doc_id);
        assert!(asset_detail["findings"]
            .as_array()
            .expect("asset findings")
            .iter()
            .any(|finding| finding["title"] == "第三方 HTTP 链接"));
        assert!(!asset_detail["cases"]
            .as_array()
            .expect("asset cases")
            .is_empty());

        let asset_graph_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/memo/assets/graph?assetId={asset_id}"))
                    .body(Body::empty())
                    .expect("asset graph request"),
            )
            .await
            .expect("asset graph response");
        assert_eq!(asset_graph_response.status(), StatusCode::OK);
        let asset_graph = response_json(asset_graph_response).await;
        assert!(asset_graph["nodes"]
            .as_array()
            .expect("graph nodes")
            .iter()
            .any(|node| node["nodeType"] == "asset"));
        assert!(asset_graph["edges"]
            .as_array()
            .expect("graph edges")
            .iter()
            .any(|edge| edge["edgeType"] == "assetFinding"));
        assert!(!asset_graph.to_string().contains("sk-test-1234567890abcdef"));

        let asset_report_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/memo/reports/generate")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "scope": "asset",
                            "assetId": asset_id.clone()
                        })
                        .to_string(),
                    ))
                    .expect("asset report request"),
            )
            .await
            .expect("asset report response");
        assert_eq!(asset_report_response.status(), StatusCode::OK);
        let asset_report = response_json(asset_report_response).await;
        let asset_report_markdown = asset_report["markdown"]
            .as_str()
            .expect("asset report markdown");
        assert!(asset_report_markdown.contains("资产安全报告"));
        assert!(asset_report_markdown.contains("安全资产："));
        assert!(asset_report_markdown.contains("第三方 HTTP 链接"));
        assert!(!asset_report_markdown.contains("sk-test-1234567890abcdef"));

        let asset_query_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/memo/assets/detail?query=api.example.com")
                    .body(Body::empty())
                    .expect("asset query request"),
            )
            .await
            .expect("asset query response");
        assert_eq!(asset_query_response.status(), StatusCode::OK);

        let safer_markdown =
            "JWT 使用 HS256\n回调 https://api.example.com/hook\nAPI Key: {{secret:apiKey}}";
        let draft_diff_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/memo/history/doc-diff")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "docId": doc_id,
                            "markdown": safer_markdown
                        })
                        .to_string(),
                    ))
                    .expect("doc diff request"),
            )
            .await
            .expect("doc diff response");
        assert_eq!(draft_diff_response.status(), StatusCode::OK);
        let draft_diff = response_json(draft_diff_response).await;
        assert_eq!(draft_diff["summary"]["resolved"], 3);
        assert_eq!(draft_diff["summary"]["currentTotal"], 1);

        let diff_save_response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/memo/save")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "id": doc_id,
                            "fileName": "security-audit.md",
                            "title": "Security Audit",
                            "markdown": safer_markdown,
                            "secrets": { "apiKey": "sk-test-1234567890abcdef" },
                            "summary": "Security audit sample"
                        })
                        .to_string(),
                    ))
                    .expect("diff save request"),
            )
            .await
            .expect("diff save response");
        assert_eq!(diff_save_response.status(), StatusCode::OK);
        let saved_diff_doc = response_json(diff_save_response).await;
        assert_eq!(saved_diff_doc["riskDiff"]["summary"]["resolved"], 3);
        assert_eq!(saved_diff_doc["riskDiff"]["summary"]["currentTotal"], 1);

        let _ = std::fs::remove_dir_all(data_dir);
    }

    #[tokio::test]
    async fn memo_data_dir_migration_copies_data_and_locks_vault() {
        let data_dir = make_test_dir("migration_source");
        let target_dir = make_test_dir("migration_target");
        let app = test_app(&data_dir);

        let unlock_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/memo/unlock")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"password":"test-password"}"#))
                    .expect("unlock request"),
            )
            .await
            .expect("unlock response");
        assert_eq!(unlock_response.status(), StatusCode::OK);

        let save_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/memo/save")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "id": null,
                            "fileName": "server.md",
                            "title": "Server",
                            "markdown": "Password: {{secret:sshPassword}}",
                            "secrets": { "sshPassword": "abc123" },
                            "summary": "Server credentials"
                        })
                        .to_string(),
                    ))
                    .expect("save request"),
            )
            .await
            .expect("save response");
        assert_eq!(save_response.status(), StatusCode::OK);

        let migrate_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/memo/data-dir/migrate")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "targetDir": target_dir.to_string_lossy().to_string()
                        })
                        .to_string(),
                    ))
                    .expect("migrate request"),
            )
            .await
            .expect("migrate response");
        assert_eq!(migrate_response.status(), StatusCode::OK);
        let body = response_json(migrate_response).await;
        assert_eq!(body["ok"], true);

        assert!(target_dir.join("documents/server.md").exists());
        assert!(target_dir.join("config.json").exists());
        assert!(target_dir.join("secrets.kdbx").exists());
        assert!(target_dir.join(".rusttool-migration-backups").exists());
        assert!(
            std::fs::read_dir(target_dir.join(".rusttool-migration-backups"))
                .expect("backup dir")
                .any(|entry| entry
                    .expect("backup entry")
                    .path()
                    .extension()
                    .map(|extension| extension == "zip")
                    .unwrap_or(false))
        );

        let config_text =
            std::fs::read_to_string(data_dir.join("config.json")).expect("migration config");
        assert!(config_text.contains(&target_dir.to_string_lossy().to_string()));

        let list_response = app
            .oneshot(
                Request::builder()
                    .uri("/api/memo/list")
                    .body(Body::empty())
                    .expect("list request"),
            )
            .await
            .expect("list response");
        assert_eq!(list_response.status(), StatusCode::UNAUTHORIZED);

        let _ = std::fs::remove_dir_all(data_dir);
        let _ = std::fs::remove_dir_all(target_dir);
    }
}
