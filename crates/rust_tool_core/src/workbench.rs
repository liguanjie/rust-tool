use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScriptInfo {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExecutionResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub success: bool,
}

fn walk_dir_recursive(dir: &Path, base_dir: &Path, scripts: &mut Vec<ScriptInfo>) -> Result<(), String> {
    let entries = fs::read_dir(dir).map_err(|e| format!("Failed to read directory {:?}: {}", dir, e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let path = entry.path();
        if path.is_dir() {
            let file_name = path.file_name().unwrap_or_default().to_string_lossy();
            if !file_name.starts_with('.') && file_name != "node_modules" && file_name != "target" {
                walk_dir_recursive(&path, base_dir, scripts)?;
            }
        } else if path.is_file() {
            if let Some(extension) = path.extension() {
                if extension == "sh" {
                    if let Ok(relative) = path.strip_prefix(base_dir) {
                        scripts.push(ScriptInfo {
                            name: relative.to_string_lossy().into_owned(),
                            path: path.to_string_lossy().into_owned(),
                        });
                    }
                }
            }
        }
    }
    Ok(())
}

pub fn list_scripts(dir: &str) -> Result<Vec<ScriptInfo>, String> {
    let path = Path::new(dir);
    if !path.exists() || !path.is_dir() {
        return Err(format!("Directory not found: {}", dir));
    }

    let mut scripts = Vec::new();
    walk_dir_recursive(path, path, &mut scripts)?;

    scripts.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(scripts)
}

pub fn execute_script(script_path: &str, args: Vec<String>) -> Result<ExecutionResult, String> {
    let path = Path::new(script_path);
    if !path.exists() || !path.is_file() {
        return Err(format!("Script not found: {}", script_path));
    }

    let mut cmd = Command::new("bash");
    cmd.arg(script_path);
    
    for arg in args {
        cmd.arg(arg);
    }

    let output = cmd.output().map_err(|e| format!("Failed to execute script: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let success = output.status.success();
    let exit_code = output.status.code().unwrap_or(-1);

    Ok(ExecutionResult {
        stdout,
        stderr,
        exit_code,
        success,
    })
}
