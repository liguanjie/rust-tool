@echo off
setlocal
cd /d "%~dp0.."
call "%~dp0env.bat"

echo [RustTool] Starting Rust API server...
cargo run -p rust_tool_server -- %*

endlocal
