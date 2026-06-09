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

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct LocalConfig {
    pub custom_data_dir: Option<String>,
}

#[derive(Clone)]
pub struct AppState {
    pub memo_manager: Arc<rust_tool_core::memo::MemoManager>,
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
            .or_else(|_| {
                std::env::var("HOME").map(|home| PathBuf::from(home).join(".local/share"))
            })
            .map(|base| base.join("rust-tool"))
            .unwrap_or_else(|_| PathBuf::from(".").join("memos_data"))
    } else {
        PathBuf::from(".").join("memos_data")
    }
}

fn get_memo_data_dir() -> PathBuf {
    let default_dir = get_default_base_dir();
    let config_path = default_dir.join("config.json");
    if config_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&config_path) {
            if let Ok(cfg) = serde_json::from_str::<LocalConfig>(&content) {
                if let Some(custom) = cfg.custom_data_dir {
                    if !custom.trim().is_empty() {
                        return PathBuf::from(custom);
                    }
                }
            }
        }
    }
    default_dir
}

pub fn build_app() -> Router {
    let data_dir = get_memo_data_dir();
    let memo_manager = Arc::new(
        rust_tool_core::memo::MemoManager::new(&data_dir)
            .expect("Failed to initialize MemoManager"),
    );
    let state = AppState { memo_manager };

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
        .route("/api/memo/test-connection", post(memo::test_connection))
        .route("/api/memo/list", get(memo::list_documents))
        .route("/api/memo/doc/:id", get(memo::get_document))
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
        build_app_with_state(AppState { memo_manager })
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
        let app = test_app(&data_dir);

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
}
