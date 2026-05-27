use axum::{http::StatusCode, Json};
use rust_tool_core::{
    check_clash_party_api, check_clash_party_node, default_clash_party_api_url,
    default_clash_party_delay_test_url, default_clash_party_delay_timeout_ms,
    detect_clash_party_data_dir, get_clash_party_manager_state, switch_clash_party_node,
    switch_clash_party_subscription, ClashPartyApiHealth, ClashPartyConfig, ClashPartyManagerState,
    ClashPartyNodeCheckResult, ClashPartySwitchResult,
};
use serde::Deserialize;

use crate::routes::tools::{ErrorBody, ErrorResponse};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwitchSubscriptionRequest {
    subscription_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwitchNodeRequest {
    group_name: String,
    node_name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckNodeRequest {
    node_name: String,
}

pub async fn state() -> Result<Json<ClashPartyManagerState>, (StatusCode, Json<ErrorResponse>)> {
    get_clash_party_manager_state(&server_clash_party_config())
        .map(Json)
        .map_err(internal_error)
}

pub async fn health() -> Json<ClashPartyApiHealth> {
    Json(check_clash_party_api(&server_clash_party_config()))
}

pub async fn switch_subscription(
    Json(request): Json<SwitchSubscriptionRequest>,
) -> Result<Json<ClashPartySwitchResult>, (StatusCode, Json<ErrorResponse>)> {
    switch_clash_party_subscription(&server_clash_party_config(), &request.subscription_id)
        .map(Json)
        .map_err(bad_request)
}

pub async fn switch_node(
    Json(request): Json<SwitchNodeRequest>,
) -> Result<Json<ClashPartySwitchResult>, (StatusCode, Json<ErrorResponse>)> {
    switch_clash_party_node(
        &server_clash_party_config(),
        &request.group_name,
        &request.node_name,
    )
    .map(Json)
    .map_err(bad_request)
}

pub async fn check_node(
    Json(request): Json<CheckNodeRequest>,
) -> Result<Json<ClashPartyNodeCheckResult>, (StatusCode, Json<ErrorResponse>)> {
    check_clash_party_node(&server_clash_party_config(), &request.node_name)
        .map(Json)
        .map_err(bad_request)
}

fn server_clash_party_config() -> ClashPartyConfig {
    ClashPartyConfig {
        data_dir: env_trimmed("RUSTTOOL_CLASH_PARTY_DATA_DIR")
            .or_else(detect_clash_party_data_dir)
            .unwrap_or_default(),
        api_url: env_trimmed("RUSTTOOL_CLASH_PARTY_API_URL")
            .unwrap_or_else(default_clash_party_api_url),
        api_secret: env_trimmed("RUSTTOOL_CLASH_PARTY_API_SECRET").unwrap_or_default(),
        delay_test_url: env_trimmed("RUSTTOOL_CLASH_PARTY_DELAY_TEST_URL")
            .unwrap_or_else(default_clash_party_delay_test_url),
        delay_timeout_ms: env_trimmed("RUSTTOOL_CLASH_PARTY_DELAY_TIMEOUT_MS")
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or_else(default_clash_party_delay_timeout_ms),
    }
}

fn env_trimmed(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn bad_request(error: String) -> (StatusCode, Json<ErrorResponse>) {
    api_error(StatusCode::BAD_REQUEST, "clash_party_request_failed", error)
}

fn internal_error(error: String) -> (StatusCode, Json<ErrorResponse>) {
    api_error(
        StatusCode::INTERNAL_SERVER_ERROR,
        "clash_party_unavailable",
        error,
    )
}

fn api_error(
    status: StatusCode,
    code: &'static str,
    message: String,
) -> (StatusCode, Json<ErrorResponse>) {
    (
        status,
        Json(ErrorResponse {
            error: ErrorBody { code, message },
        }),
    )
}
