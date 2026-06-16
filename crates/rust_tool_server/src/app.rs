use axum::{
    routing::{get, post},
    Router,
};
use http::{header, HeaderValue, Method};
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    trace::TraceLayer,
};

use crate::routes::{clash_party, health, osv_scanner, tools, workbench};
use crate::static_files;

#[derive(Clone)]
pub struct AppState {}

pub fn build_app() -> Router {
    let state = AppState {};
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
        .route(
            "/api/tools/osv-scanner/install-status",
            get(osv_scanner::install_status),
        )
        .route(
            "/api/tools/osv-scanner/scan/preview",
            post(osv_scanner::preview_scan),
        )
        .route("/api/tools/osv-scanner/scan", post(osv_scanner::scan))
        .route(
            "/api/tools/osv-scanner/export/preview",
            post(osv_scanner::preview_export),
        )
        .route("/api/tools/osv-scanner/export", post(osv_scanner::export))
        .route("/api/tools/osv-scanner/ignore", post(osv_scanner::ignore))
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
        .route("/api/workbench/scripts", get(workbench::get_scripts))
        .route("/api/workbench/scripts/execute", post(workbench::run_script))
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
    use rust_tool_core::check_osv_scanner_installed;
    use serde_json::{json, Value};
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};
    use tower::ServiceExt;

    static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn test_app_build() {
        let _app = build_app();
    }

    #[tokio::test]
    async fn osv_install_status_route_returns_json() {
        let response = build_app()
            .oneshot(
                Request::builder()
                    .uri("/api/tools/osv-scanner/install-status")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let value: Value = serde_json::from_slice(&body).unwrap();
        assert!(value.get("installed").and_then(Value::as_bool).is_some());
        assert!(value.get("message").and_then(Value::as_str).is_some());
    }

    #[tokio::test]
    async fn osv_preview_scan_route_uses_standard_error_shape() {
        let (status, value) = post_osv_json(
            "/api/tools/osv-scanner/scan/preview",
            json!({
                "projectPath": "/definitely/not/a/rusttool/project",
                "options": {
                    "recursive": true
                }
            }),
        )
        .await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(
            value
                .get("error")
                .and_then(|error| error.get("code"))
                .and_then(Value::as_str),
            Some("invalid_project_path")
        );
        assert!(value
            .get("error")
            .and_then(|error| error.get("message"))
            .and_then(Value::as_str)
            .is_some());
    }

    #[tokio::test]
    async fn osv_export_preview_route_uses_standard_error_shape() {
        let (status, value) = post_osv_json(
            "/api/tools/osv-scanner/export/preview",
            json!({
                "projectPath": "/definitely/not/a/rusttool/project",
                "options": {
                    "recursive": true
                },
                "format": "json",
                "outputPath": "/private/tmp/rusttool-osv-route-test.json"
            }),
        )
        .await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(error_code(&value), Some("invalid_project_path"));
    }

    #[tokio::test]
    async fn osv_ignore_route_uses_standard_error_shape() {
        let (status, value) = post_osv_json(
            "/api/tools/osv-scanner/ignore",
            json!({
                "projectPath": "/definitely/not/a/rusttool/project",
                "vulnerabilityId": "GHSA-test-test-test",
                "reason": "not exploitable"
            }),
        )
        .await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(error_code(&value), Some("invalid_project_path"));
    }

    #[tokio::test]
    async fn osv_scan_route_runs_confirmed_preview_when_scanner_is_available() {
        if !check_osv_scanner_installed().unwrap().installed {
            return;
        }

        let root = unique_temp_dir();
        fs::create_dir_all(&root).unwrap();
        let request = json!({
            "projectPath": root.display().to_string(),
            "options": {
                "recursive": false,
                "allowNoLockfiles": true
            }
        });
        let (preview_status, preview) =
            post_osv_json("/api/tools/osv-scanner/scan/preview", request.clone()).await;
        assert_eq!(preview_status, StatusCode::OK);

        let (scan_status, scan) = post_osv_json(
            "/api/tools/osv-scanner/scan",
            json!({
                "projectPath": root.display().to_string(),
                "options": {
                    "recursive": false,
                    "allowNoLockfiles": true
                },
                "command": preview
            }),
        )
        .await;

        assert_eq!(scan_status, StatusCode::OK);
        assert_eq!(
            scan.get("summary")
                .and_then(|summary| summary.get("totalVulnerabilities"))
                .and_then(Value::as_u64),
            Some(0)
        );
        assert!(scan.get("command").and_then(|command| command.get("argv")).is_some());

        fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn osv_export_route_writes_json_and_html_when_scanner_is_available() {
        if !check_osv_scanner_installed().unwrap().installed {
            return;
        }

        let root = unique_temp_dir();
        fs::create_dir_all(&root).unwrap();

        for (format, filename) in [("json", "report.json"), ("html", "report.html")] {
            let output_path = root.join(filename);
            let preview_request = json!({
                "projectPath": root.display().to_string(),
                "options": {
                    "recursive": false,
                    "allowNoLockfiles": true
                },
                "format": format,
                "outputPath": output_path.display().to_string()
            });
            let (preview_status, preview) =
                post_osv_json("/api/tools/osv-scanner/export/preview", preview_request).await;
            assert_eq!(preview_status, StatusCode::OK);

            let (export_status, export_result) = post_osv_json(
                "/api/tools/osv-scanner/export",
                json!({
                    "projectPath": root.display().to_string(),
                    "options": {
                        "recursive": false,
                        "allowNoLockfiles": true
                    },
                    "format": format,
                    "outputPath": output_path.display().to_string(),
                    "command": preview
                }),
            )
            .await;

            assert_eq!(export_status, StatusCode::OK);
            assert_eq!(
                export_result.get("format").and_then(Value::as_str),
                Some(format)
            );
            assert!(output_path.exists());
        }

        fs::remove_dir_all(root).unwrap();
    }

    async fn post_osv_json(path: &str, body: Value) -> (StatusCode, Value) {
        let response = build_app()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(path)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let value: Value = serde_json::from_slice(&body).unwrap();
        (status, value)
    }

    fn error_code(value: &Value) -> Option<&str> {
        value
            .get("error")
            .and_then(|error| error.get("code"))
            .and_then(Value::as_str)
    }

    fn unique_temp_dir() -> PathBuf {
        let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "rusttool-osv-route-test-{}-{nanos}-{id}",
            std::process::id()
        ))
    }
}
