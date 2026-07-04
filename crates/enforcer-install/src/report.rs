//! Report/result/check types shared by every [`crate::core::HarnessAdapter`]
//! implementation. An adapter never invents its own ad-hoc return shape —
//! `plan`/`apply`/`verify` each return one of these types so `enforcer-cli`
//! (arc-22) and the UI (arc-24) render every harness uniformly.

use serde::{Deserialize, Serialize};

/// A single harness adapter's identity, for reporting which harness a
/// [`PlannedChange`]/[`AppliedChange`]/[`VerifyCheck`] belongs to. Adapters
/// register under this key (e.g. `"claude"`, `"codex"`) — never a
/// hardcoded display string scattered across report-rendering code.
pub type HarnessKey = String;

/// What kind of artifact a planned/applied change targets. Mirrors the
/// "BOTH surfaces first-class" split in RUST_ARCHITECTURE.md: a harness's
/// MCP registration is one artifact kind among several the installer can
/// emit for a given target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ArtifactKind {
    /// A harness's user-level MCP server registration (e.g. the
    /// top-level `mcpServers` map in `~/.claude.json`).
    McpRegistration,
    /// A `cargo enforce` alias (or equivalent) for direct/CI/precommit use.
    CargoAlias,
    /// A pre-commit hook that runs the `enforcer` binary.
    PrecommitHook,
    /// A tool-neutral doctrine reference (e.g. an `AGENTS.md` block),
    /// distinct from any single harness's own managed block.
    DoctrineReference,
    /// A harness-specific artifact outside the four above (e.g. Claude's
    /// `CLAUDE.md` managed block, or a PreToolUse/SessionStart hook) —
    /// owned by that harness's own adapter/hook module, not this crate.
    HarnessSpecific,
}

/// One change a [`crate::core::HarnessAdapter::plan`] proposes to make.
/// Carries enough detail to render a dry-run diff without touching disk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlannedChange {
    /// Which harness this change is for.
    pub harness: HarnessKey,
    /// What kind of artifact this change targets.
    pub kind: ArtifactKind,
    /// Absolute path of the file this change reads/writes.
    pub path: String,
    /// Human-readable one-line description of the change (e.g. "add
    /// `enforcer` to `mcpServers`" or "update stale `enforcer` entry").
    pub description: String,
    /// Whether the target already has a registration/artifact that would
    /// be updated (`true`) versus created fresh (`false`) — the idempotent
    /// re-install signal a fixture asserts on.
    pub is_update: bool,
}

/// The full plan a [`crate::core::HarnessAdapter::plan`] call returns:
/// zero or more [`PlannedChange`]s plus any planning-time problems that
/// stop short of a hard error (e.g. a detected-but-unreadable optional
/// config that the adapter will skip rather than fail on).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallReport {
    /// Every change this run would make (or, for `--dry-run`, would have
    /// made).
    pub planned_changes: Vec<PlannedChange>,
    /// Non-fatal planning warnings (e.g. a harness detected but its config
    /// directory is unwritable — reported, not silently skipped).
    pub warnings: Vec<String>,
}

impl InstallReport {
    /// True when this plan would leave the filesystem untouched (nothing
    /// to do) — the expected shape of a second `install` run's plan once
    /// the first has already applied everything (idempotent re-install).
    #[must_use]
    pub fn is_noop(&self) -> bool {
        self.planned_changes.is_empty()
    }
}

/// The outcome of one [`PlannedChange`] after
/// [`crate::core::HarnessAdapter::apply`] executes it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppliedChange {
    /// The change that was executed.
    pub change: PlannedChange,
    /// Whether the write succeeded.
    pub succeeded: bool,
    /// Path to the pre-write backup, if [`crate::backup`] created one for
    /// this change's target file.
    pub backup_path: Option<String>,
}

/// The full result a [`crate::core::HarnessAdapter::apply`] call returns.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyResult {
    /// Outcome of every applied change, in the order the plan listed them.
    pub applied: Vec<AppliedChange>,
}

impl ApplyResult {
    /// True when every applied change succeeded.
    #[must_use]
    pub fn all_succeeded(&self) -> bool {
        self.applied.iter().all(|c| c.succeeded)
    }
}

/// One post-install health check a
/// [`crate::core::HarnessAdapter::verify`] call performs (the same shape
/// `enforcer doctor` renders across harnesses).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifyCheck {
    /// Which harness this check is for.
    pub harness: HarnessKey,
    /// Short check name (e.g. "mcp-registration-present").
    pub name: String,
    /// Whether the check passed.
    pub passed: bool,
    /// Detail shown when the check fails (empty when it passes).
    pub detail: String,
}

/// The full result a [`crate::core::HarnessAdapter::verify`] call returns.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifyReport {
    /// Every check performed.
    pub checks: Vec<VerifyCheck>,
}

impl VerifyReport {
    /// True when every check passed.
    #[must_use]
    pub fn all_passed(&self) -> bool {
        self.checks.iter().all(|c| c.passed)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AppliedChange, ApplyResult, ArtifactKind, InstallReport, PlannedChange, VerifyCheck,
        VerifyReport,
    };

    fn sample_change(is_update: bool) -> PlannedChange {
        PlannedChange {
            harness: "claude".to_owned(),
            kind: ArtifactKind::McpRegistration,
            path: "/home/user/.claude.json".to_owned(),
            description: "add enforcer to mcpServers".to_owned(),
            is_update,
        }
    }

    #[test]
    fn empty_report_is_noop() {
        assert!(InstallReport::default().is_noop());
    }

    #[test]
    fn report_with_changes_is_not_noop() {
        let report = InstallReport {
            planned_changes: vec![sample_change(false)],
            warnings: vec![],
        };
        assert!(!report.is_noop());
    }

    #[test]
    fn apply_result_all_succeeded_true_when_every_change_ok() {
        let result = ApplyResult {
            applied: vec![AppliedChange {
                change: sample_change(false),
                succeeded: true,
                backup_path: None,
            }],
        };
        assert!(result.all_succeeded());
    }

    #[test]
    fn apply_result_all_succeeded_false_when_any_change_failed() {
        let result = ApplyResult {
            applied: vec![
                AppliedChange {
                    change: sample_change(false),
                    succeeded: true,
                    backup_path: None,
                },
                AppliedChange {
                    change: sample_change(true),
                    succeeded: false,
                    backup_path: Some("/home/user/.claude.json.bak".to_owned()),
                },
            ],
        };
        assert!(!result.all_succeeded());
    }

    #[test]
    fn verify_report_all_passed_reflects_every_check() {
        let passing = VerifyReport {
            checks: vec![VerifyCheck {
                harness: "claude".to_owned(),
                name: "mcp-registration-present".to_owned(),
                passed: true,
                detail: String::new(),
            }],
        };
        assert!(passing.all_passed());

        let failing = VerifyReport {
            checks: vec![VerifyCheck {
                harness: "claude".to_owned(),
                name: "mcp-registration-present".to_owned(),
                passed: false,
                detail: "missing mcpServers.enforcer".to_owned(),
            }],
        };
        assert!(!failing.all_passed());
    }

    #[test]
    fn planned_change_round_trips_through_json() -> Result<(), Box<dyn std::error::Error>> {
        let change = sample_change(true);
        let wire = serde_json::to_string(&change)?;
        let back: PlannedChange = serde_json::from_str(&wire)?;
        assert_eq!(back, change);
        Ok(())
    }
}
