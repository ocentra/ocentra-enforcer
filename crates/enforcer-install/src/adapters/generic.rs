//! c07 — the generic `.mcp.json` writer.
//!
//! # Charter
//!
//! Most harnesses with no bespoke config format (unlike Claude's
//! `~/.claude.json` top-level `mcpServers` map, or Codex's
//! `~/.codex/config.toml` `[mcp_servers.<name>]` table) speak a plain
//! `.mcp.json` file whose top-level `mcpServers` object maps a server name
//! to `{ command, args, env }`. This module is that adapter: it upserts
//! `mcpServers[<x01 SERVER_NAME const>]` into the harness's **USER/GLOBAL**
//! `.mcp.json` at the home path c02's autodetect resolved — never a
//! per-repo project file (RUST_ARCHITECTURE.md "Global-install scope
//! contract") — preserving every unrelated top-level key already in the
//! file (`serde_json::Value` merge, not a destructive overwrite).
//!
//! # Never the legacy literal
//!
//! The registration key is always [`enforcer_mcp::name::SERVER_NAME`]
//! (`"enforcer"`) — this module never writes (or reads for identity) the
//! retired `ocentra-enforcer` string. `command` is always the caller's
//! supplied **absolute** binary path; a relative path is rejected at
//! construction (see [`GenericAdapterConfig::new`]) so a bad call site
//! cannot silently ship a registration a harness can't resolve from an
//! arbitrary cwd.
//!
//! # Purity (fixture-testable)
//!
//! `plan`/`apply`/`verify` operate purely against a `target_path`
//! (typically `<home>/.mcp.json`) handed in by the caller (in production,
//! c02's `DetectedHarness::home_path` joined with `.mcp.json`; in tests, a
//! `tempfile::tempdir()` root) — this module never resolves a harness home
//! itself, and never touches any global/ambient path other than the one
//! `target_path` names.

//! BOUNDARY-INVARIANT: adapter configuration is normalized before install decisions.
//! Negative invalid inputs are rejected by adapter configuration tests.
//!
use crate::core::HarnessAdapter;
use crate::error::{InstallError, InstallResult};
use enforcer_domain::ids::HarnessId;
use enforcer_domain::install_types::ArtifactKind;
use enforcer_domain::install_types::{
    AppliedInstallChange, ApplyResult, ChangeDisposition, CheckStatus, CheckSubject, InstallReport,
    InstallReportText, InstallVerifyCheck, InstallVerifyReport, PlannedInstallChange,
};
use enforcer_domain::install_types::{InstallBinaryPath, InstallRequestContext, InstallTargetPath};
use enforcer_domain::paths::RepoRoot;
use std::path::Path;

/// The top-level `mcpServers` map key every generic-adapter registration
/// writes under. Always the x01 const — never the legacy literal.
fn server_key() -> &'static str {
    enforcer_mcp::name::SERVER_NAME
}

/// Configuration for one generic-adapter instance: which harness key it
/// reports under, which `.mcp.json` file it upserts into, and the
/// absolute `enforcer` binary path to register.
#[derive(Debug, Clone)]
pub struct GenericAdapterConfig {
    harness_key: HarnessId,
    target_path: InstallTargetPath,
    binary_path: InstallBinaryPath,
    server_map_key: &'static str,
}

impl GenericAdapterConfig {
    /// Build a config for a harness that resolves to `target_path` (an
    /// absolute `.mcp.json` file path — typically `<user-home>/.mcp.json`,
    /// never a per-repo project path).
    ///
    /// # Errors
    /// Returns [`InstallError::MalformedConfig`] if `target_path` is not
    /// absolute — a relative target cannot resolve from an arbitrary repo
    /// cwd (RUST_ARCHITECTURE.md), so this is rejected at construction
    /// rather than silently written.
    pub fn new(
        harness_key: HarnessId,
        target_path: InstallTargetPath,
        binary_path: InstallBinaryPath,
    ) -> Self {
        Self {
            harness_key,
            target_path,
            binary_path,
            server_map_key: "mcpServers",
        }
    }

    /// Build a config for a harness whose native settings use a different
    /// top-level server map, such as Zed's `context_servers`.
    #[must_use]
    pub fn new_with_server_map(
        harness_key: HarnessId,
        target_path: InstallTargetPath,
        binary_path: InstallBinaryPath,
        server_map_key: &'static str,
    ) -> Self {
        Self {
            harness_key,
            target_path,
            binary_path,
            server_map_key,
        }
    }
}

/// The generic `.mcp.json` adapter. One instance per harness that speaks
/// the plain `.mcp.json` shape (c02 autodetect decides which harnesses
/// route here versus a bespoke adapter such as c03/c06).
#[derive(Debug, Clone)]
pub struct GenericAdapter {
    config: GenericAdapterConfig,
}

impl GenericAdapter {
    /// Build an adapter over `config`.
    #[must_use]
    pub fn new(config: GenericAdapterConfig) -> Self {
        Self { config }
    }

    fn planned_change(
        &self,
        ctx: &InstallRequestContext,
        disposition: ChangeDisposition,
    ) -> InstallResult<PlannedInstallChange> {
        let path: RepoRoot = self
            .config
            .target_path
            .as_path()
            .display()
            .to_string()
            .try_into()
            .map_err(|e: enforcer_domain::boundary::decode_error::DecodeError| {
                InstallError::MalformedConfig {
                    path: self.config.target_path.as_path().display().to_string(),
                    reason: e.to_string(),
                }
            })?;
        Ok(PlannedInstallChange {
            harness: self.config.harness_key.clone(),
            kind: ArtifactKind::McpRegistration,
            path,
            description: InstallReportText::try_from(format!(
                "upsert {}[\"{}\"] with binary `{}`",
                self.config.server_map_key,
                server_key(),
                ctx.binary_path.as_path().display()
            ))?,
            disposition,
        })
    }

    /// Read the existing `.mcp.json` contents (or an empty object if the
    /// file does not yet exist), and report whether the registration
    /// already matches `binary_path` (the idempotent-reinstall / is_update
    /// signal).
    fn read_existing(&self) -> InstallResult<serde_json::Value> {
        if !self.config.target_path.as_path().is_file() {
            return Ok(serde_json::json!({}));
        }
        let raw = std::fs::read_to_string(self.config.target_path.as_path()).map_err(|e| {
            InstallError::Io {
                path: self.config.target_path.as_path().display().to_string(),
                reason: e.to_string(),
            }
        })?;
        if raw.trim().is_empty() {
            return Ok(serde_json::json!({}));
        }
        serde_json::from_str(&raw).map_err(|e| InstallError::MalformedConfig {
            path: self.config.target_path.as_path().display().to_string(),
            reason: e.to_string(),
        })
    }

    fn current_registration_matches(&self, value: &serde_json::Value, binary_path: &Path) -> bool {
        value
            .get(self.config.server_map_key)
            .and_then(|servers| servers.get(server_key()))
            .and_then(|entry| entry.get("command"))
            .and_then(serde_json::Value::as_str)
            == Some(&binary_path.display().to_string())
    }

    fn upsert(
        value: &mut serde_json::Value,
        binary_path: &Path,
        server_map_key: &'static str,
    ) -> InstallResult<()> {
        if !value.is_object() {
            *value = serde_json::json!({});
        }
        let Some(root) = value.as_object_mut() else {
            return Err(InstallError::MalformedConfig {
                path: binary_path.display().to_string(),
                reason: "expected the `.mcp.json` root to be a JSON object".to_owned(),
            });
        };
        let servers = root
            .entry(server_map_key)
            .or_insert_with(|| serde_json::json!({}));
        if !servers.is_object() {
            *servers = serde_json::json!({});
        }
        let Some(servers_map) = servers.as_object_mut() else {
            return Err(InstallError::MalformedConfig {
                path: binary_path.display().to_string(),
                reason: format!("expected `{server_map_key}` to be a JSON object"),
            });
        };
        servers_map.insert(
            server_key().to_owned(),
            serde_json::json!({
                "command": binary_path.display().to_string(),
                "args": [],
                "env": {},
            }),
        );
        Ok(())
    }
}

impl HarnessAdapter for GenericAdapter {
    fn harness_key(&self) -> HarnessId {
        self.config.harness_key.clone()
    }

    fn plan(&self, ctx: &InstallRequestContext) -> InstallResult<InstallReport> {
        if ctx.binary_path != self.config.binary_path {
            return Err(InstallError::MalformedConfig {
                path: self.config.target_path.as_path().display().to_string(),
                reason: "request binary path does not match the generic adapter configuration"
                    .to_owned(),
            });
        }
        let existing = self.read_existing()?;
        let already_matches =
            self.current_registration_matches(&existing, ctx.binary_path.as_path());
        if already_matches {
            return Ok(InstallReport {
                planned_changes: vec![],
                warnings: vec![],
            });
        }
        let disposition = if existing
            .get(self.config.server_map_key)
            .and_then(|servers| servers.get(server_key()))
            .is_some()
        {
            ChangeDisposition::Update
        } else {
            ChangeDisposition::Create
        };
        Ok(InstallReport {
            planned_changes: vec![self.planned_change(ctx, disposition)?],
            warnings: vec![],
        })
    }

    fn apply(&self, report: &InstallReport) -> InstallResult<ApplyResult> {
        let mut applied = Vec::with_capacity(report.planned_changes.len());
        for change in &report.planned_changes {
            let mut existing = self.read_existing()?;
            Self::upsert(
                &mut existing,
                self.config.binary_path.as_path(),
                self.config.server_map_key,
            )?;

            if let Some(parent) = self.config.target_path.as_path().parent() {
                std::fs::create_dir_all(parent).map_err(|e| InstallError::Io {
                    path: parent.display().to_string(),
                    reason: e.to_string(),
                })?;
            }
            let rendered =
                serde_json::to_string_pretty(&existing).map_err(|e| InstallError::Io {
                    path: self.config.target_path.display().to_string(),
                    reason: e.to_string(),
                })?;
            std::fs::write(self.config.target_path.as_path(), rendered).map_err(|e| {
                InstallError::Io {
                    path: self.config.target_path.as_path().display().to_string(),
                    reason: e.to_string(),
                }
            })?;

            applied.push(AppliedInstallChange {
                change: change.clone(),
                status: CheckStatus::Passed,
                backup_path: None,
            });
        }
        Ok(ApplyResult { applied })
    }

    fn verify(&self, ctx: &InstallRequestContext) -> InstallResult<InstallVerifyReport> {
        let existing = self.read_existing()?;
        let passed = self.current_registration_matches(&existing, ctx.binary_path.as_path());
        Ok(InstallVerifyReport {
            checks: vec![InstallVerifyCheck {
                subject: CheckSubject::Harness(self.config.harness_key.clone()),
                name: InstallReportText::try_from("mcp-registration-present".to_owned())?,
                status: if passed {
                    CheckStatus::Passed
                } else {
                    CheckStatus::Failed
                },
                detail: InstallReportText::try_from(if passed {
                    String::new()
                } else {
                    format!(
                        "{}[\"{}\"].command != {} at `{}`",
                        self.config.server_map_key,
                        server_key(),
                        ctx.binary_path.as_path().display(),
                        self.config.target_path.as_path().display()
                    )
                })?,
            }],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{server_key, GenericAdapter, GenericAdapterConfig};
    use crate::core::HarnessAdapter;
    use enforcer_domain::ids::HarnessId;
    use enforcer_domain::install_types::{
        InstallBinaryPath, InstallRequestContext, InstallTargetPath,
    };
    use std::path::PathBuf;

    fn fixture_root(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/generic")
            .join(name)
    }

    fn config(
        harness: &str,
        target: PathBuf,
        binary: PathBuf,
    ) -> Result<GenericAdapterConfig, Box<dyn std::error::Error>> {
        Ok(GenericAdapterConfig::new(
            HarnessId::try_from(harness.to_owned())?,
            InstallTargetPath::try_from(target)?,
            InstallBinaryPath::try_from(binary)?,
        ))
    }

    #[test]
    fn server_key_is_the_x01_const_never_the_legacy_literal() {
        assert_eq!(server_key(), "enforcer");
        assert_ne!(server_key(), "ocentra-enforcer");
    }

    #[test]
    fn fresh_install_writes_the_golden_bytes() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let target = dir.path().join(".mcp.json");
        let binary = std::env::temp_dir().join("enforcer");
        let config = config("generic-harness", target.clone(), binary.clone())?;
        let adapter = GenericAdapter::new(config);
        let ctx = InstallRequestContext::try_with_defaults(binary.clone())?;

        let plan = adapter.plan(&ctx)?;
        assert_eq!(plan.planned_changes.len(), 1);
        adapter.apply(&plan)?;

        let written = std::fs::read_to_string(&target)?;
        let golden = std::fs::read_to_string(fixture_root("fresh_install").join("mcp.json"))?;
        let written_value: serde_json::Value = serde_json::from_str(&written)?;
        let mut golden_value: serde_json::Value = serde_json::from_str(&golden)?;
        golden_value["mcpServers"]["enforcer"]["command"] =
            serde_json::Value::String(binary.display().to_string());
        assert_eq!(written_value, golden_value);
        Ok(())
    }

    #[test]
    fn upsert_preserves_unrelated_keys_and_servers() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let target = dir.path().join(".mcp.json");
        std::fs::write(
            &target,
            serde_json::to_string_pretty(&serde_json::json!({
                "someUnrelatedKey": "keep-me",
                "mcpServers": {
                    "other-server": { "command": "/abs/other", "args": [], "env": {} }
                }
            }))?,
        )?;

        let binary = std::env::temp_dir().join("enforcer");
        let config = config("generic-harness", target.clone(), binary.clone())?;
        let adapter = GenericAdapter::new(config);
        let ctx = InstallRequestContext::try_with_defaults(binary.clone())?;

        let plan = adapter.plan(&ctx)?;
        adapter.apply(&plan)?;

        let written: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&target)?)?;
        assert_eq!(written["someUnrelatedKey"], "keep-me");
        assert_eq!(
            written["mcpServers"]["other-server"]["command"],
            "/abs/other"
        );
        assert_eq!(
            written["mcpServers"]["enforcer"]["command"],
            binary.display().to_string()
        );
        Ok(())
    }

    #[test]
    fn never_writes_the_legacy_ocentra_enforcer_key() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let target = dir.path().join(".mcp.json");
        let binary = std::env::temp_dir().join("enforcer");
        let config = config("generic-harness", target.clone(), binary.clone())?;
        let adapter = GenericAdapter::new(config);
        let ctx = InstallRequestContext::try_with_defaults(binary)?;
        let plan = adapter.plan(&ctx)?;
        adapter.apply(&plan)?;

        let written = std::fs::read_to_string(&target)?;
        assert!(!written.as_str().contains("ocentra-enforcer"));
        Ok(())
    }

    #[test]
    fn idempotent_reinstall_yields_a_noop_plan() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let target = dir.path().join(".mcp.json");
        let binary = std::env::temp_dir().join("enforcer");
        let config = config("generic-harness", target, binary.clone())?;
        let adapter = GenericAdapter::new(config);
        let ctx = InstallRequestContext::try_with_defaults(binary)?;

        let plan = adapter.plan(&ctx)?;
        adapter.apply(&plan)?;

        let second_plan = adapter.plan(&ctx)?;
        assert!(second_plan.planned_changes.is_empty());
        Ok(())
    }

    #[test]
    fn verify_passes_after_apply_and_fails_after_binary_moves(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let target = dir.path().join(".mcp.json");
        let binary = std::env::temp_dir().join("enforcer");
        let config = config("generic-harness", target, binary.clone())?;
        let adapter = GenericAdapter::new(config);
        let ctx = InstallRequestContext::try_with_defaults(binary)?;

        let plan = adapter.plan(&ctx)?;
        adapter.apply(&plan)?;
        let report = adapter.verify(&ctx)?;
        assert!(report.checks.iter().all(|check| matches!(
            check.status,
            enforcer_domain::install_types::CheckStatus::Passed
        )));

        let moved_ctx =
            InstallRequestContext::try_with_defaults(std::env::temp_dir().join("new-enforcer"))?;
        let report = adapter.verify(&moved_ctx)?;
        assert!(!report.checks.iter().all(|check| matches!(
            check.status,
            enforcer_domain::install_types::CheckStatus::Passed
        )));
        assert_eq!(report.checks[0].name.as_str(), "mcp-registration-present");
        assert!(!report.checks[0].detail.as_str().is_empty());
        Ok(())
    }

    #[test]
    fn rejects_a_relative_target_path() -> Result<(), Box<dyn std::error::Error>> {
        let result = InstallTargetPath::try_from(PathBuf::from("relative/.mcp.json"));
        let error = result
            .err()
            .ok_or("relative installer target must be rejected")?;
        assert_eq!(error.path, "installTargetPath");
        assert_eq!(error.reason, "must be an absolute filesystem path");
        Ok(())
    }

    #[test]
    fn never_writes_a_per_repo_project_file_path_by_construction(
    ) -> Result<(), Box<dyn std::error::Error>> {
        // The adapter has no notion of "project file" at all -- it only
        // ever writes `target_path`, which construction requires to be
        // absolute. This test documents that guarantee: there is no
        // relative/per-repo code path to accidentally exercise.
        let result = InstallTargetPath::try_from(PathBuf::from(".mcp.json"));
        let error = result
            .err()
            .ok_or("project-relative installer target must be rejected")?;
        assert_eq!(error.path, "installTargetPath");
        assert_eq!(error.reason, "must be an absolute filesystem path");
        Ok(())
    }
}
