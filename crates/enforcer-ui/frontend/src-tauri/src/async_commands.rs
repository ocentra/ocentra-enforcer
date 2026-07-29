//! Async Tauri command adapters. Blocking domain and filesystem work remains
//! in the synchronous desktop module and is dispatched through this boundary.

use std::path::{Path, PathBuf};

use super::{
    discover_scan_targets, run_legacy_analysis_sync, run_packaged_scan_sync,
    write_desktop_file_rule_waiver, DesktopFindingWaiverInput, DesktopReportPayload,
    DesktopScanTarget, LegacyAnalysisKind, LegacyAnalysisRunPayload,
};

#[tauri::command]
pub(crate) async fn run_packaged_scan(
    root: String,
    target: Option<DesktopScanTarget>,
) -> Result<DesktopReportPayload, String> {
    tauri::async_runtime::spawn_blocking(move || run_packaged_scan_sync(root, target))
        .await
        .map_err(|error| format!("scan task failed: {error}"))?
}

#[tauri::command]
pub(crate) async fn waive_packaged_finding(
    root: String,
    request: DesktopFindingWaiverInput,
) -> Result<DesktopReportPayload, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let root_path = PathBuf::from(&root);
        if !root_path.is_dir() {
            return Err(format!("project root is not a directory: {root}"));
        }
        write_desktop_file_rule_waiver(&root_path, request)?;
        run_packaged_scan_sync(root, None)
    })
    .await
    .map_err(|error| format!("waiver task failed: {error}"))?
}

#[tauri::command]
pub(crate) async fn load_scan_targets(root: String) -> Result<Vec<DesktopScanTarget>, String> {
    tauri::async_runtime::spawn_blocking(move || discover_scan_targets(Path::new(&root)))
        .await
        .map_err(|error| format!("scan target discovery failed: {error}"))?
}

#[tauri::command]
pub(crate) async fn run_legacy_analysis(
    root: String,
    kind: LegacyAnalysisKind,
) -> Result<LegacyAnalysisRunPayload, String> {
    tauri::async_runtime::spawn_blocking(move || run_legacy_analysis_sync(root, kind))
        .await
        .map_err(|error| format!("analysis task failed: {error}"))?
}
