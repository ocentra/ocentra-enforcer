//! Report/result/check types shared by every [`crate::core::HarnessAdapter`]
//! implementation. An adapter never invents its own ad-hoc return shape —
//! `plan`/`apply`/`verify` each return one of these types so `enforcer-cli`
//! (arc-22) and the UI (arc-24) render every harness uniformly.
//!
//! BOUNDARY-INVARIANT: transport DTOs in this module are converted into
//! branded `enforcer-domain` report types with fallible `TryFrom`
//! implementations. Invalid harness ids, report text, paths, and other raw
//! strings are rejected during conversion and never enter domain workflows.

use enforcer_domain::{
    ids::HarnessId,
    install_types::{
        AppliedInstallChange, ApplyResult, ArtifactKind, ChangeDisposition, CheckStatus,
        CheckSubject, InstallReport, InstallReportText, InstallVerifyCheck, InstallVerifyReport,
        PlannedInstallChange, PluginPublishContract, SkillAsset, SkillAssetManifest,
        SkillAssetPath,
    },
    paths::RepoRoot,
};
use serde::{Deserialize, Serialize};

/// A single harness adapter's identity, for reporting which harness a
/// [`PlannedChangeDto`]/[`AppliedChangeDto`]/[`VerifyCheckDto`] belongs to. Adapters
/// register under this key (e.g. `"claude"`, `"codex"`) — never a
/// hardcoded display string scattered across report-rendering code.
/// One change a [`crate::core::HarnessAdapter::plan`] proposes to make.
/// Carries enough detail to render a dry-run diff without touching disk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlannedChangeDto {
    /// Which harness this change is for.
    pub harness: String,
    /// What kind of artifact this change targets.
    #[serde(with = "crate::boundary::install_type_wire::artifact_kind")]
    pub kind: ArtifactKind,
    /// Absolute path of the file this change reads/writes. Branded over
    /// [`RepoRoot`] (parse-at-boundary) rather than a bare `String` — an
    /// adapter cannot construct a [`PlannedChangeDto`] pointing at a relative
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
/// zero or more [`PlannedChangeDto`]s plus any planning-time problems that
/// stop short of a hard error (e.g. a detected-but-unreadable optional
/// config that the adapter will skip rather than fail on).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallReportDto {
    /// Every change this run would make (or, for `--dry-run`, would have
    /// made).
    pub planned_changes: Vec<PlannedChangeDto>,
    /// Non-fatal planning warnings (e.g. a harness detected but its config
    /// directory is unwritable — reported, not silently skipped).
    pub warnings: Vec<String>,
}

impl InstallReportDto {
    /// True when this plan would leave the filesystem untouched (nothing
    /// to do) — the expected shape of a second `install` run's plan once
    /// the first has already applied everything (idempotent re-install).
    #[must_use]
    pub fn is_noop(&self) -> bool {
        self.planned_changes.is_empty()
    }
}

/// The outcome of one [`PlannedChangeDto`] after
/// [`crate::core::HarnessAdapter::apply`] executes it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppliedChangeDto {
    /// The change that was executed.
    pub change: PlannedChangeDto,
    /// Whether the write succeeded.
    pub succeeded: bool,
    /// Path to the pre-write backup, if [`crate::backup`] created one for
    /// this change's target file. Branded over [`RepoRoot`], matching
    /// [`PlannedChangeDto::path`].
    pub backup_path: Option<RepoRoot>,
}

/// The full result a [`crate::core::HarnessAdapter::apply`] call returns.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyResultDto {
    /// Outcome of every applied change, in the order the plan listed them.
    pub applied: Vec<AppliedChangeDto>,
}

impl ApplyResultDto {
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
pub struct VerifyCheckDto {
    /// Which harness this check is for.
    pub harness: String,
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
pub struct VerifyReportDto {
    /// Every check performed.
    pub checks: Vec<VerifyCheckDto>,
}

impl VerifyReportDto {
    /// True when every check passed.
    #[must_use]
    pub fn all_passed(&self) -> bool {
        self.checks.iter().all(|c| c.passed)
    }
}

/// One skill asset a [`SkillAssetManifestDto`] declares. The legacy
/// `scripts/validate-codex-assets.mjs` hardcoded these paths for the Codex
/// adapter alone; here an adapter (or a caller building a manifest for
/// tests) supplies its own asset list, so the check itself stays
/// harness-neutral (RUST_ARCHITECTURE.md skill-asset fold-in, WAVE 6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillAssetDto {
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
pub struct PluginPublishContractDto {
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
pub struct SkillAssetManifestDto {
    /// Every skill asset that must exist on disk.
    pub assets: Vec<SkillAssetDto>,
    /// Zero or more plugin-manifest field/value contracts that must hold.
    pub plugin_contracts: Vec<PluginPublishContractDto>,
}

impl TryFrom<PlannedChangeDto> for PlannedInstallChange {
    type Error = enforcer_domain::boundary::decode_error::DecodeError;

    fn try_from(value: PlannedChangeDto) -> Result<Self, Self::Error> {
        Ok(Self {
            harness: HarnessId::try_from(value.harness)?,
            kind: value.kind,
            path: value.path,
            description: InstallReportText::try_from(value.description)?,
            disposition: if value.is_update {
                ChangeDisposition::Update
            } else {
                ChangeDisposition::Create
            },
        })
    }
}

impl From<PlannedInstallChange> for PlannedChangeDto {
    fn from(value: PlannedInstallChange) -> Self {
        Self {
            harness: value.harness.to_string(),
            kind: value.kind,
            path: value.path,
            description: value.description.as_str().to_owned(),
            is_update: matches!(value.disposition, ChangeDisposition::Update),
        }
    }
}

impl TryFrom<InstallReportDto> for InstallReport {
    type Error = enforcer_domain::boundary::decode_error::DecodeError;

    fn try_from(value: InstallReportDto) -> Result<Self, Self::Error> {
        Ok(Self {
            planned_changes: value
                .planned_changes
                .into_iter()
                .map(PlannedInstallChange::try_from)
                .collect::<Result<_, _>>()?,
            warnings: value
                .warnings
                .into_iter()
                .map(InstallReportText::try_from)
                .collect::<Result<_, _>>()?,
        })
    }
}

impl From<InstallReport> for InstallReportDto {
    fn from(value: InstallReport) -> Self {
        Self {
            planned_changes: value
                .planned_changes
                .into_iter()
                .map(PlannedChangeDto::from)
                .collect(),
            warnings: value
                .warnings
                .into_iter()
                .map(|value| value.as_str().to_owned())
                .collect(),
        }
    }
}

impl TryFrom<AppliedChangeDto> for AppliedInstallChange {
    type Error = enforcer_domain::boundary::decode_error::DecodeError;

    fn try_from(value: AppliedChangeDto) -> Result<Self, Self::Error> {
        Ok(Self {
            change: value.change.try_into()?,
            status: if value.succeeded {
                CheckStatus::Passed
            } else {
                CheckStatus::Failed
            },
            backup_path: value.backup_path,
        })
    }
}

impl From<AppliedInstallChange> for AppliedChangeDto {
    fn from(value: AppliedInstallChange) -> Self {
        Self {
            change: value.change.into(),
            succeeded: matches!(value.status, CheckStatus::Passed),
            backup_path: value.backup_path,
        }
    }
}

impl TryFrom<ApplyResultDto> for ApplyResult {
    type Error = enforcer_domain::boundary::decode_error::DecodeError;

    fn try_from(value: ApplyResultDto) -> Result<Self, Self::Error> {
        Ok(Self {
            applied: value
                .applied
                .into_iter()
                .map(AppliedInstallChange::try_from)
                .collect::<Result<_, _>>()?,
        })
    }
}

impl From<ApplyResult> for ApplyResultDto {
    fn from(value: ApplyResult) -> Self {
        Self {
            applied: value
                .applied
                .into_iter()
                .map(AppliedChangeDto::from)
                .collect(),
        }
    }
}

impl TryFrom<VerifyCheckDto> for InstallVerifyCheck {
    type Error = enforcer_domain::boundary::decode_error::DecodeError;

    fn try_from(value: VerifyCheckDto) -> Result<Self, Self::Error> {
        Ok(Self {
            subject: CheckSubject::Harness(HarnessId::try_from(value.harness)?),
            name: InstallReportText::try_from(value.name)?,
            status: if value.passed {
                CheckStatus::Passed
            } else {
                CheckStatus::Failed
            },
            detail: InstallReportText::try_from(value.detail)?,
        })
    }
}

impl From<InstallVerifyCheck> for VerifyCheckDto {
    fn from(value: InstallVerifyCheck) -> Self {
        let harness = match value.subject {
            CheckSubject::Harness(harness) => harness.as_str().to_owned(),
            CheckSubject::SkillAsset(asset) => asset.as_str().to_owned(),
        };
        Self {
            harness,
            name: value.name.as_str().to_owned(),
            passed: matches!(value.status, CheckStatus::Passed),
            detail: value.detail.as_str().to_owned(),
        }
    }
}

impl TryFrom<VerifyReportDto> for InstallVerifyReport {
    type Error = enforcer_domain::boundary::decode_error::DecodeError;

    fn try_from(value: VerifyReportDto) -> Result<Self, Self::Error> {
        Ok(Self {
            checks: value
                .checks
                .into_iter()
                .map(InstallVerifyCheck::try_from)
                .collect::<Result<_, _>>()?,
        })
    }
}

impl From<InstallVerifyReport> for VerifyReportDto {
    fn from(value: InstallVerifyReport) -> Self {
        Self {
            checks: value.checks.into_iter().map(VerifyCheckDto::from).collect(),
        }
    }
}

impl TryFrom<SkillAssetDto> for SkillAsset {
    type Error = enforcer_domain::boundary::decode_error::DecodeError;

    fn try_from(value: SkillAssetDto) -> Result<Self, Self::Error> {
        Ok(Self {
            name: InstallReportText::try_from(value.skill_name)?,
            path: SkillAssetPath::from(std::path::PathBuf::from(value.asset_path)),
        })
    }
}

impl TryFrom<PluginPublishContractDto> for PluginPublishContract {
    type Error = enforcer_domain::boundary::decode_error::DecodeError;

    fn try_from(value: PluginPublishContractDto) -> Result<Self, Self::Error> {
        Ok(Self {
            manifest_path: SkillAssetPath::from(std::path::PathBuf::from(value.manifest_path)),
            field: InstallReportText::try_from(value.field)?,
            expected_value: InstallReportText::try_from(value.expected_value)?,
        })
    }
}

impl TryFrom<SkillAssetManifestDto> for SkillAssetManifest {
    type Error = enforcer_domain::boundary::decode_error::DecodeError;

    fn try_from(value: SkillAssetManifestDto) -> Result<Self, Self::Error> {
        Ok(Self {
            assets: value
                .assets
                .into_iter()
                .map(SkillAsset::try_from)
                .collect::<Result<_, _>>()?,
            plugin_contracts: value
                .plugin_contracts
                .into_iter()
                .map(PluginPublishContract::try_from)
                .collect::<Result<_, _>>()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AppliedChangeDto, ApplyResultDto, InstallReportDto, PlannedChangeDto, PlannedInstallChange,
        PluginPublishContractDto, RepoRoot, SkillAssetDto, SkillAssetManifestDto, VerifyCheckDto,
        VerifyReportDto,
    };
    use enforcer_domain::install_types::ArtifactKind;

    fn sample_change(is_update: bool) -> Result<PlannedChangeDto, Box<dyn std::error::Error>> {
        let path: RepoRoot = "/home/user/.claude.json".to_owned().try_into()?;
        Ok(PlannedChangeDto {
            harness: "claude".to_owned(),
            kind: ArtifactKind::McpRegistration,
            path,
            description: "add enforcer to mcpServers".to_owned(),
            is_update,
        })
    }

    #[test]
    fn empty_report_is_noop() {
        assert!(InstallReportDto::default().is_noop());
    }

    #[test]
    fn report_with_changes_is_not_noop() -> Result<(), Box<dyn std::error::Error>> {
        let report = InstallReportDto {
            planned_changes: vec![sample_change(false)?],
            warnings: vec![],
        };
        assert_eq!(report.planned_changes.len(), 1);
        Ok(())
    }

    #[test]
    fn apply_result_all_succeeded_true_when_every_change_ok(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let result = ApplyResultDto {
            applied: vec![AppliedChangeDto {
                change: sample_change(false)?,
                succeeded: true,
                backup_path: None,
            }],
        };
        assert!(result.applied.iter().all(|change| change.succeeded));
        Ok(())
    }

    #[test]
    fn apply_result_all_succeeded_false_when_any_change_failed(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let backup_path: RepoRoot = "/home/user/.claude.json.bak".to_owned().try_into()?;
        let result = ApplyResultDto {
            applied: vec![
                AppliedChangeDto {
                    change: sample_change(false)?,
                    succeeded: true,
                    backup_path: None,
                },
                AppliedChangeDto {
                    change: sample_change(true)?,
                    succeeded: false,
                    backup_path: Some(backup_path),
                },
            ],
        };
        assert!(result.applied.iter().any(|change| !change.succeeded));
        Ok(())
    }

    #[test]
    fn verify_report_all_passed_reflects_every_check() {
        let passing = VerifyReportDto {
            checks: vec![VerifyCheckDto {
                harness: "claude".to_owned(),
                name: "mcp-registration-present".to_owned(),
                passed: true,
                detail: String::new(),
            }],
        };
        assert!(passing.checks.iter().all(|check| check.passed));

        let failing = VerifyReportDto {
            checks: vec![VerifyCheckDto {
                harness: "claude".to_owned(),
                name: "mcp-registration-present".to_owned(),
                passed: false,
                detail: "missing mcpServers.enforcer".to_owned(),
            }],
        };
        assert!(failing.checks.iter().any(|check| !check.passed));
    }

    #[test]
    /// Round-trip proof for every planned-change transport field.
    fn planned_change_round_trip_through_json() -> Result<(), Box<dyn std::error::Error>> {
        let change = sample_change(true)?;
        let wire = serde_json::to_string(&change)?;
        let back: PlannedChangeDto = serde_json::from_str(&wire)?;
        assert_eq!(back, change);
        Ok(())
    }

    #[test]
    fn install_report_dto_round_trip_through_json() -> Result<(), Box<dyn std::error::Error>> {
        let report = InstallReportDto {
            planned_changes: vec![sample_change(false)?],
            warnings: vec!["restart the harness".to_owned()],
        };
        let wire = serde_json::to_string(&report)?;
        let round_trip: InstallReportDto = serde_json::from_str(&wire)?;
        assert_eq!(round_trip, report);
        Ok(())
    }

    #[test]
    fn apply_result_dto_round_trip_through_json() -> Result<(), Box<dyn std::error::Error>> {
        let apply_result = ApplyResultDto {
            applied: vec![AppliedChangeDto {
                change: sample_change(true)?,
                succeeded: true,
                backup_path: Some("/home/user/.claude.json.bak".to_owned().try_into()?),
            }],
        };
        let wire = serde_json::to_string(&apply_result)?;
        let round_trip: ApplyResultDto = serde_json::from_str(&wire)?;
        assert_eq!(round_trip, apply_result);
        Ok(())
    }

    #[test]
    fn verify_report_dto_round_trip_through_json() -> Result<(), Box<dyn std::error::Error>> {
        let verify_report = VerifyReportDto {
            checks: vec![VerifyCheckDto {
                harness: "claude".to_owned(),
                name: "mcp-registration-present".to_owned(),
                passed: false,
                detail: "registration missing".to_owned(),
            }],
        };
        let wire = serde_json::to_string(&verify_report)?;
        let round_trip: VerifyReportDto = serde_json::from_str(&wire)?;
        assert_eq!(round_trip, verify_report);
        Ok(())
    }

    #[test]
    /// Invalid empty harness input is rejected by the DTO conversion.
    fn invalid_relative_planned_change_path_is_rejected() -> Result<(), Box<dyn std::error::Error>>
    {
        let mut change = sample_change(false)?;
        change.harness = String::new();
        let error = PlannedInstallChange::try_from(change)
            .err()
            .ok_or("empty harness identity must be rejected")?;
        assert_eq!(error.path, "harnessId");
        assert_eq!(
            error.reason,
            "expected lowercase kebab-case (e.g. `claude`, `codex`, `kilocode`)"
        );
        Ok(())
    }

    #[test]
    fn skill_asset_manifest_round_trips_through_json() -> Result<(), Box<dyn std::error::Error>> {
        let manifest = SkillAssetManifestDto {
            assets: vec![SkillAssetDto {
                skill_name: "ocentra-enforcer".to_owned(),
                asset_path: "skills/ocentra-enforcer/SKILL.md".to_owned(),
            }],
            plugin_contracts: vec![PluginPublishContractDto {
                manifest_path: ".codex-plugin/plugin.json".to_owned(),
                field: "skills".to_owned(),
                expected_value: "./skills/".to_owned(),
            }],
        };
        let wire = serde_json::to_string(&manifest)?;
        assert!(wire.as_str().contains("\"skillName\""));
        assert!(wire.as_str().contains("\"expectedValue\""));
        let back: SkillAssetManifestDto = serde_json::from_str(&wire)?;
        assert_eq!(back, manifest);
        Ok(())
    }

    #[test]
    fn empty_skill_asset_manifest_has_no_assets_or_contracts() {
        let manifest = SkillAssetManifestDto::default();
        assert!(manifest.assets.is_empty());
        assert!(manifest.plugin_contracts.is_empty());
    }
}
