use axum::{
    routing::{get, post},
    Router,
};
use http::{header, HeaderValue, Method};
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    trace::TraceLayer,
};

use crate::routes::{clash_party, health, tools};
use crate::static_files;

pub fn build_app() -> Router {
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
        .fallback(static_files::serve_static)
        .layer(TraceLayer::new_for_http())
        .layer(cors)
}
