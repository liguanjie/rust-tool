use axum::{
    routing::{get, post},
    Router,
};
use http::{header, HeaderValue, Method};
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    trace::TraceLayer,
};

use crate::routes::{clash_party, health, tools, workbench};
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

    #[test]
    fn test_app_build() {
        let _app = build_app();
    }
}
