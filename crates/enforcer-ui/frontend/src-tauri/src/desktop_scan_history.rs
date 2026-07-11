use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::ScanFindingPayload;

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DesktopReportPayload {
    pub(crate) ok: bool,
    pub(crate) scope: String,
    pub(crate) violations: Vec<ScanFindingPayload>,
    pub(crate) warnings: Vec<ScanFindingPayload>,
    pub(crate) waived: Vec<ScanFindingPayload>,
    pub(crate) total_count: usize,
    pub(crate) runtime: String,
    pub(crate) persistence: String,
    pub(crate) generated_at: String,
    #[serde(default)]
    pub(crate) run_id: String,
    #[serde(default)]
    pub(crate) target_label: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DesktopScanHistoryEntry {
    pub(crate) run_id: String,
    pub(crate) generated_at: String,
    pub(crate) scope: String,
    pub(crate) total_count: usize,
    pub(crate) blocking_count: usize,
    pub(crate) warning_count: usize,
    pub(crate) waived_count: usize,
    pub(crate) runtime: String,
    pub(crate) persistence: String,
}

#[tauri::command]
pub(crate) fn load_cached_scan(root: String) -> Result<Option<DesktopReportPayload>, String> {
    let root_path = PathBuf::from(&root);
    if !root_path.is_dir() {
        return Err(format!("project root is not a directory: {root}"));
    }
    let cache_path = desktop_report_cache_path(&root_path);
    if !cache_path.is_file() {
        return Ok(None);
    }
    let bytes = std::fs::read(&cache_path).map_err(|error| {
        format!(
            "cannot read desktop report cache at {}: {error}",
            cache_path.display()
        )
    })?;
    serde_json::from_slice(&bytes).map(Some).map_err(|error| {
        format!(
            "cannot decode desktop report cache at {}: {error}",
            cache_path.display()
        )
    })
}

pub(crate) fn desktop_report_cache_path(root: &Path) -> PathBuf {
    root.join(".enforce")
        .join("ui")
        .join("desktop-scan-report.json")
}

pub(crate) fn desktop_scan_history_dir(root: &Path) -> PathBuf {
    root.join(".enforce").join("ui").join("scan-runs")
}

pub(crate) fn desktop_scan_run_path(root: &Path, run_id: &str) -> PathBuf {
    desktop_scan_history_dir(root).join(format!("{run_id}.json"))
}

pub(crate) fn desktop_scan_run_id() -> String {
    let milliseconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis());
    format!("desktop-scan-{milliseconds}")
}

pub(crate) fn persist_desktop_report(
    root: &Path,
    report: &DesktopReportPayload,
) -> Result<(), String> {
    let cache_path = desktop_report_cache_path(root);
    let history_path = desktop_scan_run_path(root, &report.run_id);
    if let Some(parent) = cache_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create desktop report cache directory: {error}"))?;
    }
    if let Some(parent) = history_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create desktop scan history directory: {error}"))?;
    }
    let payload = serde_json::to_vec_pretty(report)
        .map_err(|error| format!("cannot encode desktop report cache: {error}"))?;
    std::fs::write(&cache_path, &payload).map_err(|error| {
        format!(
            "cannot persist desktop report cache at {}: {error}",
            cache_path.display()
        )
    })?;
    std::fs::write(&history_path, payload).map_err(|error| {
        format!(
            "cannot persist desktop scan history at {}: {error}",
            history_path.display()
        )
    })
}

#[tauri::command]
pub(crate) fn load_desktop_scan_history(
    root: String,
) -> Result<Vec<DesktopScanHistoryEntry>, String> {
    let root_path = PathBuf::from(&root);
    if !root_path.is_dir() {
        return Err(format!("project root is not a directory: {root}"));
    }
    let history_dir = desktop_scan_history_dir(&root_path);
    if !history_dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(&history_dir)
        .map_err(|error| format!("cannot read desktop scan history: {error}"))?
    {
        let entry =
            entry.map_err(|error| format!("cannot read desktop scan history entry: {error}"))?;
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
            continue;
        }
        let bytes = std::fs::read(&path).map_err(|error| {
            format!(
                "cannot read desktop scan history at {}: {error}",
                path.display()
            )
        })?;
        let report: DesktopReportPayload = serde_json::from_slice(&bytes).map_err(|error| {
            format!(
                "cannot decode desktop scan history at {}: {error}",
                path.display()
            )
        })?;
        if report.run_id.is_empty() {
            continue;
        }
        entries.push(DesktopScanHistoryEntry {
            run_id: report.run_id,
            generated_at: report.generated_at,
            scope: report.scope,
            total_count: report.total_count,
            blocking_count: report.violations.len(),
            warning_count: report.warnings.len(),
            waived_count: report.waived.len(),
            runtime: report.runtime,
            persistence: report.persistence,
        });
    }
    entries.sort_by(|left, right| right.run_id.cmp(&left.run_id));
    entries.truncate(50);
    Ok(entries)
}

#[tauri::command]
pub(crate) fn load_desktop_scan_run(
    root: String,
    run_id: String,
) -> Result<DesktopReportPayload, String> {
    if !run_id.starts_with("desktop-scan-")
        || !run_id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-')
    {
        return Err("invalid desktop scan run id".to_owned());
    }
    let root_path = PathBuf::from(&root);
    if !root_path.is_dir() {
        return Err(format!("project root is not a directory: {root}"));
    }
    let path = desktop_scan_run_path(&root_path, &run_id);
    let bytes = std::fs::read(&path).map_err(|error| {
        format!(
            "cannot read desktop scan run at {}: {error}",
            path.display()
        )
    })?;
    serde_json::from_slice(&bytes).map_err(|error| {
        format!(
            "cannot decode desktop scan run at {}: {error}",
            path.display()
        )
    })
}
