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

#[cfg(windows)]
fn platform_script_extension() -> &'static str {
    "ps1"
}

#[cfg(not(windows))]
fn platform_script_extension() -> &'static str {
    "sh"
}

fn is_platform_script_extension(extension: &std::ffi::OsStr) -> bool {
    extension
        .to_string_lossy()
        .eq_ignore_ascii_case(platform_script_extension())
}

fn walk_dir_recursive(
    dir: &Path,
    base_dir: &Path,
    scripts: &mut Vec<ScriptInfo>,
) -> Result<(), String> {
    let entries =
        fs::read_dir(dir).map_err(|e| format!("Failed to read directory {:?}: {}", dir, e))?;

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
                if is_platform_script_extension(extension) {
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

fn script_extension(path: &Path) -> String {
    path.extension()
        .map(|extension| extension.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn build_platform_script_command(script_path: &str) -> Command {
    #[cfg(windows)]
    {
        let mut cmd = Command::new("powershell.exe");
        cmd.args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            script_path,
        ]);
        cmd
    }

    #[cfg(not(windows))]
    {
        let mut cmd = Command::new("bash");
        cmd.arg(script_path);
        cmd
    }
}

pub fn execute_script(script_path: &str, args: Vec<String>) -> Result<ExecutionResult, String> {
    let path = Path::new(script_path);
    if !path.exists() || !path.is_file() {
        return Err(format!("Script not found: {}", script_path));
    }

    let extension = script_extension(path);
    if !extension.eq_ignore_ascii_case(platform_script_extension()) {
        let actual = if extension.is_empty() {
            "none".to_string()
        } else {
            extension
        };
        return Err(format!(
            "Unsupported script extension `{}` on this platform. Expected `.{}`.",
            actual,
            platform_script_extension()
        ));
    }

    let mut cmd = build_platform_script_command(script_path);

    for arg in args {
        cmd.arg(arg);
    }

    let output = cmd
        .output()
        .map_err(|e| format!("Failed to execute script: {}", e))?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir(name: &str) -> PathBuf {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();

        std::env::temp_dir().join(format!("rust-tool-workbench-{}-{}", name, timestamp))
    }

    #[test]
    fn list_scripts_keeps_only_current_platform_scripts() {
        let root = unique_temp_dir("list");
        let codex_dir = root.join("codex");
        fs::create_dir_all(&codex_dir).expect("create test script directory");
        fs::write(
            codex_dir.join("install-to-project.sh"),
            "#!/usr/bin/env bash\n",
        )
        .expect("write shell script");
        fs::write(
            codex_dir.join("install-to-project.ps1"),
            "Write-Output ok\n",
        )
        .expect("write powershell script");

        let scripts = list_scripts(root.to_string_lossy().as_ref()).expect("list scripts");
        let names = scripts
            .into_iter()
            .map(|script| script.name)
            .collect::<Vec<_>>();

        let expected_name = Path::new("codex")
            .join(format!(
                "install-to-project.{}",
                platform_script_extension()
            ))
            .to_string_lossy()
            .into_owned();

        assert_eq!(names, vec![expected_name]);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn execute_script_rejects_non_platform_extension() {
        let root = unique_temp_dir("execute");
        fs::create_dir_all(&root).expect("create test script directory");
        let unsupported_extension = if cfg!(windows) { "sh" } else { "ps1" };
        let script_path = root.join(format!("install-to-project.{unsupported_extension}"));
        fs::write(&script_path, "echo ok\n").expect("write unsupported script");

        let result = execute_script(script_path.to_string_lossy().as_ref(), Vec::new());

        match result {
            Ok(_) => panic!("unsupported script should be rejected"),
            Err(error) => assert!(error.contains("Unsupported script extension")),
        }

        let _ = fs::remove_dir_all(root);
    }
}
