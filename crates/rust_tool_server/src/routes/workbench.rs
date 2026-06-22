use axum::{extract::Query, Json};
use rust_tool_core::workbench::{execute_script, list_scripts, ExecutionResult, ScriptInfo};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct ListScriptsQuery {
    pub dir: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ExecuteScriptPayload {
    pub path: String,
    pub args: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

pub async fn get_scripts(
    Query(query): Query<ListScriptsQuery>,
) -> Json<ApiResponse<Vec<ScriptInfo>>> {
    let dir = query
        .dir
        .unwrap_or_else(|| "/Users/ben/work/99_codex".to_string());

    match list_scripts(&dir) {
        Ok(scripts) => Json(ApiResponse {
            success: true,
            data: Some(scripts),
            error: None,
        }),
        Err(e) => Json(ApiResponse {
            success: false,
            data: None,
            error: Some(e),
        }),
    }
}

pub async fn run_script(
    Json(payload): Json<ExecuteScriptPayload>,
) -> Json<ApiResponse<ExecutionResult>> {
    let args_str = payload.args.unwrap_or_default();
    let args: Vec<String> = args_str.split_whitespace().map(|s| s.to_string()).collect();

    match execute_script(&payload.path, args) {
        Ok(result) => Json(ApiResponse {
            success: true,
            data: Some(result),
            error: None,
        }),
        Err(e) => Json(ApiResponse {
            success: false,
            data: None,
            error: Some(e),
        }),
    }
}
