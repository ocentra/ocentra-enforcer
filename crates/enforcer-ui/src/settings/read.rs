//! GET-only settings read surface: renders the human control-plane view
//! from the arc-03 typed load API. This module performs no writes — every
//! function here is safe to call from a GET handler; mutation lives only
//! in [`crate::settings::write`].
//!
//! camelCase wire casing (locked decision, matches `enforcer-domain`).
//! [`SettingsViewPayload`] derives `ts_rs::TS` so [`crate::ts_export`]
//! regenerates its committed TypeScript binding from here, never
//! hand-written.

use std::path::Path;

use enforcer_config::project_tie::{EnforcerScope, NativeMode, NativeTool, ResolvedProjectTie};

/// One rendered per-rule toggle row: enabled/disabled, effective severity
/// override (if any), and the waiver record (owner + reason + ruleId) when
/// disabled. Mirrors [`enforcer_config::policy::RuleToggle`] at the UI
/// boundary rather than re-deriving its own notion of "disabled".
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct RuleToggleRow {
    /// The rule this row describes, as its wire string (e.g. `"RR-6.1"`).
    pub rule_id: String,
    /// Whether the rule is currently enabled.
    pub enabled: bool,
    /// Effective severity override, lowercase wire form, if one is set.
    pub severity: Option<String>,
    /// Waiver owner, present only when `enabled` is `false`.
    pub waiver_owner: Option<String>,
    /// Waiver reason, present only when `enabled` is `false`.
    pub waiver_reason: Option<String>,
}

/// One resolved native-tool tie row (`cargo`/`tsc`/`ruff`/`dart`/
/// `CFLint`): the effective mode + scope, always present per
/// [`ResolvedProjectTie::tie`]'s total-view guarantee.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct NativeTieRow {
    /// The tool this row describes, lowercase wire form (e.g. `"cargo"`).
    pub tool: String,
    /// Effective mode, lowercase wire form (`"override"`/`"augment"`/
    /// `"both"`).
    pub mode: String,
    /// Effective scope, lowercase wire form (`"scoped"`/`"wholeRepo"`).
    pub scope: String,
}

/// The full settings-view payload the frontend renders: every native-tool
/// tie (always 5 rows, one per recognized tool) plus every declared
/// per-rule toggle. Absence of a `ruleToggles` entry in the underlying
/// config is NOT rendered as a row here — the view shows only rules a
/// human has explicitly touched, matching the "absence means default,
/// never a fabricated row" doctrine; toggling a not-yet-listed rule is a
/// [`crate::settings::write`] operation that adds its row.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct SettingsViewPayload {
    /// Source path this view was loaded from, for display only (e.g.
    /// `".enforce/config"`); never re-parsed by the frontend.
    pub source_path: String,
    /// Every recognized native tool's resolved tie, in stable tool order.
    pub native_ties: Vec<NativeTieRow>,
    /// Every explicitly declared per-rule toggle, in stable (`BTreeMap`)
    /// rule-id order.
    pub rule_toggles: Vec<RuleToggleRow>,
}

fn native_mode_wire(mode: NativeMode) -> &'static str {
    match mode {
        NativeMode::Override => "override",
        NativeMode::Augment => "augment",
        NativeMode::Both => "both",
    }
}

fn enforcer_scope_wire(scope: EnforcerScope) -> &'static str {
    match scope {
        EnforcerScope::Scoped => "scoped",
        EnforcerScope::WholeRepo => "wholeRepo",
    }
}

/// The full set of recognized native tools, in stable order — the same
/// order [`ResolvedProjectTie`] resolves internally, kept here so this
/// read surface never needs a second source of truth for "which tools
/// exist".
const ALL_NATIVE_TOOLS: [NativeTool; 5] = [
    NativeTool::Cargo,
    NativeTool::Tsc,
    NativeTool::Ruff,
    NativeTool::Dart,
    NativeTool::Cflint,
];

fn native_tool_wire(tool: NativeTool) -> &'static str {
    match tool {
        NativeTool::Cargo => "cargo",
        NativeTool::Tsc => "tsc",
        NativeTool::Ruff => "ruff",
        NativeTool::Dart => "dart",
        NativeTool::Cflint => "cflint",
    }
}

/// Render a [`ResolvedProjectTie`] (already loaded/validated by arc-03)
/// into the UI's [`SettingsViewPayload`]. Total mapping: every recognized
/// tool gets a row (never fewer), and every declared rule toggle gets a
/// row — no hardcoded defaults substituted for live config state.
#[must_use]
pub fn render_settings_view(
    source_path: &str,
    resolved: &ResolvedProjectTie,
) -> SettingsViewPayload {
    let native_ties = ALL_NATIVE_TOOLS
        .iter()
        .map(|&tool| {
            let tie = resolved.tie(tool);
            NativeTieRow {
                tool: native_tool_wire(tool).to_owned(),
                mode: native_mode_wire(tie.mode).to_owned(),
                scope: enforcer_scope_wire(tie.scope).to_owned(),
            }
        })
        .collect();

    let rule_toggles = resolved
        .policy
        .rule_toggles
        .iter()
        .map(|(rule_id, toggle)| RuleToggleRow {
            rule_id: rule_id.to_string(),
            enabled: toggle.enabled,
            severity: toggle.severity.and_then(|severity| {
                serde_json::to_value(severity)
                    .ok()
                    .and_then(|v| v.as_str().map(str::to_owned))
            }),
            waiver_owner: toggle.waiver.as_ref().map(|w| w.owner.clone()),
            waiver_reason: toggle.waiver.as_ref().map(|w| w.reason.clone()),
        })
        .collect();

    SettingsViewPayload {
        source_path: source_path.to_owned(),
        native_ties,
        rule_toggles,
    }
}

/// Load `.enforce/config` at `config_path` through the arc-03 typed load
/// API ([`enforcer_config::project_tie::load_project_tie`]) and render it
/// into a [`SettingsViewPayload`]. The one entry point this module's GET
/// handler calls; never reads or parses the file itself.
///
/// # Errors
/// Returns [`enforcer_config::error::ConfigLoadError`] if the underlying
/// typed load fails (malformed JSON, unknown tool/mode, or a disabled rule
/// with no attributable waiver) — the read surface fails closed rather
/// than rendering a best-effort partial view.
pub fn load_settings_view(
    config_path: &Path,
) -> enforcer_config::error::ConfigResult<SettingsViewPayload> {
    let resolved = enforcer_config::project_tie::load_project_tie(config_path)?;
    Ok(render_settings_view(
        &config_path.display().to_string(),
        &resolved,
    ))
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use enforcer_config::project_tie::{parse_project_tie, ProjectConfig};

    use super::{load_settings_view, render_settings_view};

    /// PASS fixture: a settings read renders the effective config —
    /// native ties for all 5 recognized tools always present, and a
    /// declared rule toggle (with its waiver) shows up as a row.
    #[test]
    fn settings_read_renders_effective_config() -> Result<(), Box<dyn std::error::Error>> {
        let raw = serde_json::json!({
            "native": {
                "cargo": { "mode": "override", "scope": "wholeRepo" }
            },
            "policy": {
                "ruleToggles": {
                    "RR-1.1": {
                        "enabled": false,
                        "waiver": {
                            "ruleId": "RR-1.1",
                            "owner": "platform-team",
                            "reason": "tracked in TICKET-42"
                        }
                    }
                }
            }
        })
        .to_string();
        let resolved = parse_project_tie(&raw, "cfg.json")?;
        let payload = render_settings_view("cfg.json", &resolved);

        assert_eq!(payload.source_path, "cfg.json");
        assert_eq!(payload.native_ties.len(), 5);
        let cargo_row = payload
            .native_ties
            .iter()
            .find(|row| row.tool == "cargo")
            .ok_or("expected a cargo row")?;
        assert_eq!(cargo_row.mode, "override");
        assert_eq!(cargo_row.scope, "wholeRepo");

        assert_eq!(payload.rule_toggles.len(), 1);
        let row = &payload.rule_toggles[0];
        assert_eq!(row.rule_id, "RR-1.1");
        assert!(!row.enabled);
        assert_eq!(row.waiver_owner.as_deref(), Some("platform-team"));
        assert_eq!(row.waiver_reason.as_deref(), Some("tracked in TICKET-42"));
        Ok(())
    }

    /// PASS fixture: absence of `.enforce/config` on disk still renders a
    /// total view (all 5 tools defaulted, zero rule-toggle rows) rather
    /// than erroring or hardcoding a fake row — matches arc-03's
    /// "zero-config projects work" invariant.
    #[test]
    fn settings_read_renders_zero_config_default() -> Result<(), Box<dyn std::error::Error>> {
        let default_resolved = enforcer_config::project_tie::ResolvedProjectTie::resolve(
            &ProjectConfig::default(),
            "<none>",
        )?;
        let payload = render_settings_view("<none>", &default_resolved);
        assert_eq!(payload.native_ties.len(), 5);
        assert!(payload.rule_toggles.is_empty());
        for row in &payload.native_ties {
            assert_eq!(row.mode, "augment");
            assert_eq!(row.scope, "scoped");
        }
        Ok(())
    }

    /// PASS fixture: `load_settings_view` reads a real temp-dir file
    /// through the arc-03 typed load API end to end.
    #[test]
    fn settings_read_loads_from_real_file() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join(".enforce-config.json");
        let mut file = std::fs::File::create(&path)?;
        file.write_all(
            serde_json::json!({
                "native": { "tsc": { "mode": "both" } }
            })
            .to_string()
            .as_bytes(),
        )?;
        drop(file);

        let payload = load_settings_view(&path)?;
        let tsc_row = payload
            .native_ties
            .iter()
            .find(|row| row.tool == "tsc")
            .ok_or("expected a tsc row")?;
        assert_eq!(tsc_row.mode, "both");
        Ok(())
    }

    /// FAIL fixture: a malformed `.enforce/config` (unknown tool key)
    /// fails the read closed, exactly as the arc-03 typed load API
    /// dictates — this module never substitutes a best-effort default for
    /// a load-time error.
    #[test]
    fn settings_read_fails_closed_on_malformed_config() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join(".enforce-config.json");
        std::fs::write(
            &path,
            serde_json::json!({ "native": { "gofmt": { "mode": "augment" } } }).to_string(),
        )?;
        let outcome = load_settings_view(&path);
        assert!(outcome.is_err());
        Ok(())
    }
}
