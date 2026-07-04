//! Report/result/check types shared by every [`crate::core::HarnessAdapter`]
//! implementation. An adapter never invents its own ad-hoc return shape —
//! `plan`/`apply`/`verify` each return one of these types so `enforcer-cli`
//! (arc-22) and the UI (arc-24) render every harness uniformly.

use enforcer_domain::paths::RepoRoot;
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
    /// Absolute path of the file this change reads/writes. Branded over
    /// [`RepoRoot`] (parse-at-boundary) rather than a bare `String` — an
    /// adapter cannot construct a [`PlannedChange`] pointing at a relative
    /// path, matching the "absolute path, never relative" contract every
    /// adapter's MCP registration must honor (RUST_ARCHITECTURE.md).
    pub path: RepoRoot,
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
    /// this change's target file. Branded over [`RepoRoot`], matching
    /// [`PlannedChange::path`].
    pub backup_path: Option<RepoRoot>,
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

/// One skill asset a [`SkillAssetManifest`] declares. The legacy
/// `scripts/validate-codex-assets.mjs` hardcoded these paths for the Codex
/// adapter alone; here an adapter (or a caller building a manifest for
/// tests) supplies its own asset list, so the check itself stays
/// harness-neutral (RUST_ARCHITECTURE.md skill-asset fold-in, WAVE 6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillAsset {
    /// Short name identifying the skill this asset belongs to (e.g.
    /// `"ocentra-enforcer"`).
    pub skill_name: String,
    /// Absolute or repo-relative path to the declared `SKILL.md` (or other
    /// skill asset) that must exist on disk.
    pub asset_path: String,
}

/// The publish-contract assertion the legacy `.mjs` validator made for
/// Codex specifically (`plugin.skills === "./skills/"` in
/// `.codex-plugin/plugin.json`). Kept as an adapter-provided manifest field
/// rather than a hardcoded literal in the check, so a future adapter with a
/// different plugin-manifest shape can supply its own expectation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginPublishContract {
    /// Path to the plugin manifest file (e.g. `.codex-plugin/plugin.json`).
    pub manifest_path: String,
    /// The dotted JSON field expected to hold `expected_value` (e.g.
    /// `"skills"`).
    pub field: String,
    /// The value `field` must equal for the publish contract to hold (e.g.
    /// `"./skills/"`).
    pub expected_value: String,
}

/// A caller/adapter-provided manifest of skill assets + the plugin publish
/// contract to check. This is the harness-neutral replacement for the
/// hardcoded Codex canonical/legacy skill paths the retired `.mjs` script
/// baked in: any adapter (or test fixture) builds one of these to describe
/// ITS assets, and [`crate::core::run_skill_asset_checks`] evaluates it
/// uniformly.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillAssetManifest {
    /// Every skill asset that must exist on disk.
    pub assets: Vec<SkillAsset>,
    /// Zero or more plugin-manifest field/value contracts that must hold.
    pub plugin_contracts: Vec<PluginPublishContract>,
}

#[cfg(test)]
mod tests {
    use super::{
        AppliedChange, ApplyResult, ArtifactKind, InstallReport, PlannedChange,
        PluginPublishContract, RepoRoot, SkillAsset, SkillAssetManifest, VerifyCheck, VerifyReport,
    };

    fn sample_change(is_update: bool) -> Result<PlannedChange, Box<dyn std::error::Error>> {
        let path: RepoRoot = "/home/user/.claude.json".to_owned().try_into()?;
        Ok(PlannedChange {
            harness: "claude".to_owned(),
            kind: ArtifactKind::McpRegistration,
            path,
            description: "add enforcer to mcpServers".to_owned(),
            is_update,
        })
    }

    #[test]
    fn empty_report_is_noop() {
        assert!(InstallReport::default().is_noop());
    }

    #[test]
    fn report_with_changes_is_not_noop() -> Result<(), Box<dyn std::error::Error>> {
        let report = InstallReport {
            planned_changes: vec![sample_change(false)?],
            warnings: vec![],
        };
        assert!(!report.is_noop());
        Ok(())
    }

    #[test]
    fn apply_result_all_succeeded_true_when_every_change_ok(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let result = ApplyResult {
            applied: vec![AppliedChange {
                change: sample_change(false)?,
                succeeded: true,
                backup_path: None,
            }],
        };
        assert!(result.all_succeeded());
        Ok(())
    }

    #[test]
    fn apply_result_all_succeeded_false_when_any_change_failed(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let backup_path: RepoRoot = "/home/user/.claude.json.bak".to_owned().try_into()?;
        let result = ApplyResult {
            applied: vec![
                AppliedChange {
                    change: sample_change(false)?,
                    succeeded: true,
                    backup_path: None,
                },
                AppliedChange {
                    change: sample_change(true)?,
                    succeeded: false,
                    backup_path: Some(backup_path),
                },
            ],
        };
        assert!(!result.all_succeeded());
        Ok(())
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
        let change = sample_change(true)?;
        let wire = serde_json::to_string(&change)?;
        let back: PlannedChange = serde_json::from_str(&wire)?;
        assert_eq!(back, change);
        Ok(())
    }

    #[test]
    fn skill_asset_manifest_round_trips_through_json() -> Result<(), Box<dyn std::error::Error>> {
        let manifest = SkillAssetManifest {
            assets: vec![SkillAsset {
                skill_name: "ocentra-enforcer".to_owned(),
                asset_path: "skills/ocentra-enforcer/SKILL.md".to_owned(),
            }],
            plugin_contracts: vec![PluginPublishContract {
                manifest_path: ".codex-plugin/plugin.json".to_owned(),
                field: "skills".to_owned(),
                expected_value: "./skills/".to_owned(),
            }],
        };
        let wire = serde_json::to_string(&manifest)?;
        assert!(wire.contains("\"skillName\""));
        assert!(wire.contains("\"expectedValue\""));
        let back: SkillAssetManifest = serde_json::from_str(&wire)?;
        assert_eq!(back, manifest);
        Ok(())
    }

    #[test]
    fn empty_skill_asset_manifest_has_no_assets_or_contracts() {
        let manifest = SkillAssetManifest::default();
        assert!(manifest.assets.is_empty());
        assert!(manifest.plugin_contracts.is_empty());
    }
}
