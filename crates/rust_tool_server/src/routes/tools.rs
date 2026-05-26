use axum::{http::StatusCode, Json};
use rust_tool_core::{convert_vless_to_yaml, ConvertOptions, OutputMode, TemplateMode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VlessOutputMode {
    FullConfig,
    ProxyOnly,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VlessTemplateMode {
    Minimal,
    Standard,
    FullRules,
}

#[derive(Debug, Deserialize)]
pub struct VlessToMihomoRequest {
    input: String,
    mode: Option<VlessOutputMode>,
    template: Option<VlessTemplateMode>,
    proxy_name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct VlessToMihomoResponse {
    yaml: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    error: ErrorBody,
}

#[derive(Debug, Serialize)]
pub struct ErrorBody {
    code: &'static str,
    message: String,
}

pub async fn vless_to_mihomo(
    Json(request): Json<VlessToMihomoRequest>,
) -> Result<Json<VlessToMihomoResponse>, (StatusCode, Json<ErrorResponse>)> {
    let output_mode = match request.mode.unwrap_or(VlessOutputMode::FullConfig) {
        VlessOutputMode::FullConfig => OutputMode::FullConfig,
        VlessOutputMode::ProxyOnly => OutputMode::ProxyOnly,
    };
    let template_mode = match request.template.unwrap_or(VlessTemplateMode::Standard) {
        VlessTemplateMode::Minimal => TemplateMode::Minimal,
        VlessTemplateMode::Standard => TemplateMode::Standard,
        VlessTemplateMode::FullRules => TemplateMode::FullRules,
    };

    convert_vless_to_yaml(
        &request.input,
        ConvertOptions {
            output_mode,
            template_mode,
            proxy_name: request.proxy_name,
        },
    )
        .map(|yaml| Json(VlessToMihomoResponse { yaml }))
        .map_err(|error| {
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: ErrorBody {
                        code: "invalid_vless_url",
                        message: error.to_string(),
                    },
                }),
            )
        })
}
