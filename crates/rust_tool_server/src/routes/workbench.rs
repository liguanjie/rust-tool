use axum::{extract::Query, Json};
use rust_tool_core::workbench::{execute_script, list_scripts, ExecutionResult, ScriptInfo};
use rust_tool_core::{
    get_agent_skills_settings, initialize_database, save_agent_skills_settings, AgentSkillsSettings,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const DEFAULT_DATABASE_FILE_NAME: &str = "rusttool.db";
const DEFAULT_DATA_DIR: &str = "./data";

#[derive(Debug, Deserialize)]
pub struct ListScriptsQuery {
    pub dir: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ExecuteScriptPayload {
    pub path: String,
    pub args: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SaveAgentSkillsSettingsPayload {
    pub settings: AgentSkillsSettings,
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

pub async fn get_agent_skills_settings_route() -> Json<ApiResponse<AgentSkillsSettings>> {
    let database = match initialize_workbench_database().await {
        Ok(database) => database,
        Err(error) => {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(error),
            })
        }
    };

    match get_agent_skills_settings(&database).await {
        Ok(settings) => Json(ApiResponse {
            success: true,
            data: Some(settings),
            error: None,
        }),
        Err(error) => Json(ApiResponse {
            success: false,
            data: None,
            error: Some(error.to_string()),
        }),
    }
}

pub async fn save_agent_skills_settings_route(
    Json(payload): Json<SaveAgentSkillsSettingsPayload>,
) -> Json<ApiResponse<AgentSkillsSettings>> {
    let database = match initialize_workbench_database().await {
        Ok(database) => database,
        Err(error) => {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(error),
            })
        }
    };

    match save_agent_skills_settings(&database, payload.settings).await {
        Ok(settings) => Json(ApiResponse {
            success: true,
            data: Some(settings),
            error: None,
        }),
        Err(error) => Json(ApiResponse {
            success: false,
            data: None,
            error: Some(error.to_string()),
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

async fn initialize_workbench_database() -> Result<rust_tool_core::StorageDatabase, String> {
    initialize_database(workbench_database_path())
        .await
        .map_err(|error| error.to_string())
}

fn workbench_database_path() -> PathBuf {
    let data_dir = std::env::var("RUSTTOOL_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_DATA_DIR));

    data_dir.join(DEFAULT_DATABASE_FILE_NAME)
}
