use axum::{http::StatusCode, Json};
use rust_tool_core::{
    build_export_command, build_scan_command, check_osv_scanner_installed, export_report,
    ignore_vulnerability, scan_project, OsvCommandPreview, OsvIgnoreRequest, OsvIgnoreResult,
    OsvInstallStatus, OsvReportExportCommandRequest, OsvReportExportRequest,
    OsvReportExportResult, OsvScanCommandRequest, OsvScanRequest, OsvScanResult,
    OsvScannerError,
};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: ErrorBody,
}

#[derive(Debug, Serialize)]
pub struct ErrorBody {
    pub code: &'static str,
    pub message: String,
}

pub async fn install_status() -> Result<Json<OsvInstallStatus>, (StatusCode, Json<ErrorResponse>)> {
    check_osv_scanner_installed()
        .map(Json)
        .map_err(osv_error_response)
}

pub async fn preview_scan(
    Json(request): Json<OsvScanCommandRequest>,
) -> Result<Json<OsvCommandPreview>, (StatusCode, Json<ErrorResponse>)> {
    build_scan_command(request)
        .map(Json)
        .map_err(osv_error_response)
}

pub async fn scan(
    Json(request): Json<OsvScanRequest>,
) -> Result<Json<OsvScanResult>, (StatusCode, Json<ErrorResponse>)> {
    scan_project(request).map(Json).map_err(osv_error_response)
}

pub async fn preview_export(
    Json(request): Json<OsvReportExportCommandRequest>,
) -> Result<Json<OsvCommandPreview>, (StatusCode, Json<ErrorResponse>)> {
    build_export_command(request)
        .map(Json)
        .map_err(osv_error_response)
}

pub async fn export(
    Json(request): Json<OsvReportExportRequest>,
) -> Result<Json<OsvReportExportResult>, (StatusCode, Json<ErrorResponse>)> {
    export_report(request)
        .map(Json)
        .map_err(osv_error_response)
}

pub async fn ignore(
    Json(request): Json<OsvIgnoreRequest>,
) -> Result<Json<OsvIgnoreResult>, (StatusCode, Json<ErrorResponse>)> {
    ignore_vulnerability(
        &request.project_path,
        &request.vulnerability_id,
        &request.reason,
    )
    .map(Json)
    .map_err(osv_error_response)
}

fn osv_error_response(error: OsvScannerError) -> (StatusCode, Json<ErrorResponse>) {
    let (status, code) = match &error {
        OsvScannerError::NotInstalled => (StatusCode::SERVICE_UNAVAILABLE, "osv_not_installed"),
        OsvScannerError::InvalidProjectPath { .. } => {
            (StatusCode::BAD_REQUEST, "invalid_project_path")
        }
        OsvScannerError::CommandRejected(_) => (StatusCode::BAD_REQUEST, "osv_command_rejected"),
        OsvScannerError::ScanFailed(_) => (StatusCode::BAD_GATEWAY, "osv_scan_failed"),
        OsvScannerError::ReportParseFailed(_) => {
            (StatusCode::BAD_GATEWAY, "osv_report_parse_failed")
        }
        OsvScannerError::ExportFailed(_) => (StatusCode::BAD_REQUEST, "osv_export_failed"),
        OsvScannerError::InvalidReportFormat => (StatusCode::BAD_REQUEST, "invalid_report_format"),
        OsvScannerError::IgnoreUpdateFailed(_) => {
            (StatusCode::BAD_REQUEST, "osv_ignore_update_failed")
        }
        OsvScannerError::Io(_) => (StatusCode::INTERNAL_SERVER_ERROR, "osv_io_failed"),
    };

    (
        status,
        Json(ErrorResponse {
            error: ErrorBody {
                code,
                message: error.to_string(),
            },
        }),
    )
}
