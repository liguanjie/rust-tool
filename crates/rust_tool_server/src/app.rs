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
