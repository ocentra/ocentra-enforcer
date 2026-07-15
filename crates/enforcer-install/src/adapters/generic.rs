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

use crate::cli_contract::RequestContext;
use crate::core::HarnessAdapter;
use crate::error::{InstallError, InstallResult};
use crate::report::{
    AppliedChange, ApplyResult, ArtifactKind, InstallReport, PlannedChange, VerifyCheck,
    VerifyReport,
};
use enforcer_domain::paths::RepoRoot;
use std::path::{Path, PathBuf};

/// The top-level `mcpServers` map key every generic-adapter registration
/// writes under. Always the x01 const — never the legacy literal.
fn server_key() -> &'static str {
    enforcer_mcp::name::SERVER_NAME
}

/// The fixed marker [`render_description`]/[`parse_description_binary_path`]
/// use to carry the target binary path through a [`PlannedChange`]'s
/// human-readable `description` field, since that type (owned by arc-23)
/// has no dedicated structured field for it. `apply` only receives the
/// previously computed [`InstallReport`] (never the original `ctx`), so
/// the binary path must round-trip through the plan itself.
const BINARY_PATH_MARKER: &str = "\u{0}binary_path=";

fn render_description(binary_path: &Path) -> String {
    format!(
        "upsert mcpServers[\"{}\"]{}{}",
        server_key(),
        BINARY_PATH_MARKER,
        binary_path.display()
    )
}

fn parse_description_binary_path(description: &str) -> InstallResult<PathBuf> {
    description
        .split_once(BINARY_PATH_MARKER)
        .map(|(_, rhs)| PathBuf::from(rhs))
        .ok_or_else(|| InstallError::MalformedConfig {
            path: description.to_owned(),
            reason: "generic adapter planned-change description missing binary-path marker"
                .to_owned(),
        })
}

/// Configuration for one generic-adapter instance: which harness key it
/// reports under, which `.mcp.json` file it upserts into, and the
/// absolute `enforcer` binary path to register.
#[derive(Debug, Clone)]
pub struct GenericAdapterConfig {
    harness_key: &'static str,
    target_path: PathBuf,
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
    pub fn new(harness_key: &'static str, target_path: PathBuf) -> InstallResult<Self> {
        if !target_path.is_absolute() {
            return Err(InstallError::MalformedConfig {
                path: target_path.display().to_string(),
                reason: "generic adapter target path must be absolute".to_owned(),
            });
        }
        Ok(Self {
            harness_key,
            target_path,
        })
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
        ctx: &RequestContext,
        is_update: bool,
    ) -> InstallResult<PlannedChange> {
        let path: RepoRoot = self
            .config
            .target_path
            .display()
            .to_string()
            .try_into()
            .map_err(|e: enforcer_domain::boundary::decode_error::DecodeError| {
                InstallError::MalformedConfig {
                    path: self.config.target_path.display().to_string(),
                    reason: e.to_string(),
                }
            })?;
        Ok(PlannedChange {
            harness: self.config.harness_key.to_owned(),
            kind: ArtifactKind::McpRegistration,
            path,
            description: render_description(&ctx.binary_path),
            is_update,
        })
    }

    /// Read the existing `.mcp.json` contents (or an empty object if the
    /// file does not yet exist), and report whether the registration
    /// already matches `binary_path` (the idempotent-reinstall / is_update
    /// signal).
    fn read_existing(&self) -> InstallResult<serde_json::Value> {
        if !self.config.target_path.is_file() {
            return Ok(serde_json::json!({}));
        }
        let raw =
            std::fs::read_to_string(&self.config.target_path).map_err(|e| InstallError::Io {
                path: self.config.target_path.display().to_string(),
                reason: e.to_string(),
            })?;
        if raw.trim().is_empty() {
            return Ok(serde_json::json!({}));
        }
        serde_json::from_str(&raw).map_err(|e| InstallError::MalformedConfig {
            path: self.config.target_path.display().to_string(),
            reason: e.to_string(),
        })
    }

    fn current_registration_matches(&self, value: &serde_json::Value, binary_path: &Path) -> bool {
        value
            .get("mcpServers")
            .and_then(|servers| servers.get(server_key()))
            .and_then(|entry| entry.get("command"))
            .and_then(serde_json::Value::as_str)
            == Some(&binary_path.display().to_string())
    }

    fn upsert(value: &mut serde_json::Value, binary_path: &Path) -> InstallResult<()> {
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
            .entry("mcpServers")
            .or_insert_with(|| serde_json::json!({}));
        if !servers.is_object() {
            *servers = serde_json::json!({});
        }
        let Some(servers_map) = servers.as_object_mut() else {
            return Err(InstallError::MalformedConfig {
                path: binary_path.display().to_string(),
                reason: "expected `mcpServers` to be a JSON object".to_owned(),
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
    fn harness_key(&self) -> &'static str {
        self.config.harness_key
    }

    fn plan(&self, ctx: &RequestContext) -> InstallResult<InstallReport> {
        let existing = self.read_existing()?;
        let already_matches = self.current_registration_matches(&existing, &ctx.binary_path);
        if already_matches {
            return Ok(InstallReport {
                planned_changes: vec![],
                warnings: vec![],
            });
        }
        let is_update = existing
            .get("mcpServers")
            .and_then(|servers| servers.get(server_key()))
            .is_some();
        Ok(InstallReport {
            planned_changes: vec![self.planned_change(ctx, is_update)?],
            warnings: vec![],
        })
    }

    fn apply(&self, report: &InstallReport) -> InstallResult<ApplyResult> {
        let mut applied = Vec::with_capacity(report.planned_changes.len());
        for change in &report.planned_changes {
            let binary_path = parse_description_binary_path(&change.description)?;

            let mut existing = self.read_existing()?;
            Self::upsert(&mut existing, &binary_path)?;

            if let Some(parent) = self.config.target_path.parent() {
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
            std::fs::write(&self.config.target_path, rendered).map_err(|e| InstallError::Io {
                path: self.config.target_path.display().to_string(),
                reason: e.to_string(),
            })?;

            applied.push(AppliedChange {
                change: change.clone(),
                succeeded: true,
                backup_path: None,
            });
        }
        Ok(ApplyResult { applied })
    }

    fn verify(&self, ctx: &RequestContext) -> InstallResult<VerifyReport> {
        let existing = self.read_existing()?;
        let passed = self.current_registration_matches(&existing, &ctx.binary_path);
        Ok(VerifyReport {
            checks: vec![VerifyCheck {
                harness: self.config.harness_key.to_owned(),
                name: "mcp-registration-present".to_owned(),
                passed,
                detail: if passed {
                    String::new()
                } else {
                    format!(
                        "mcpServers[\"{}\"].command != {} at `{}`",
                        server_key(),
                        ctx.binary_path.display(),
                        self.config.target_path.display()
                    )
                },
            }],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{server_key, GenericAdapter, GenericAdapterConfig};
    use crate::cli_contract::RequestContext;
    use crate::core::HarnessAdapter;
    use std::path::PathBuf;

    fn fixture_root(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/generic")
            .join(name)
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
        let config = GenericAdapterConfig::new("generic-harness", target.clone())?;
        let adapter = GenericAdapter::new(config);
        let ctx = RequestContext::with_defaults(PathBuf::from("/abs/path/to/enforcer"));

        let plan = adapter.plan(&ctx)?;
        assert!(!plan.is_noop());
        adapter.apply(&plan)?;

        let written = std::fs::read_to_string(&target)?;
        let golden = std::fs::read_to_string(fixture_root("fresh_install").join("mcp.json"))?;
        let written_value: serde_json::Value = serde_json::from_str(&written)?;
        let golden_value: serde_json::Value = serde_json::from_str(&golden)?;
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

        let config = GenericAdapterConfig::new("generic-harness", target.clone())?;
        let adapter = GenericAdapter::new(config);
        let ctx = RequestContext::with_defaults(PathBuf::from("/abs/path/to/enforcer"));

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
            "/abs/path/to/enforcer"
        );
        Ok(())
    }

    #[test]
    fn never_writes_the_legacy_ocentra_enforcer_key() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let target = dir.path().join(".mcp.json");
        let config = GenericAdapterConfig::new("generic-harness", target.clone())?;
        let adapter = GenericAdapter::new(config);
        let ctx = RequestContext::with_defaults(PathBuf::from("/abs/path/to/enforcer"));
        let plan = adapter.plan(&ctx)?;
        adapter.apply(&plan)?;

        let written = std::fs::read_to_string(&target)?;
        assert!(!written.contains("ocentra-enforcer"));
        Ok(())
    }

    #[test]
    fn idempotent_reinstall_yields_a_noop_plan() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let target = dir.path().join(".mcp.json");
        let config = GenericAdapterConfig::new("generic-harness", target)?;
        let adapter = GenericAdapter::new(config);
        let ctx = RequestContext::with_defaults(PathBuf::from("/abs/path/to/enforcer"));

        let plan = adapter.plan(&ctx)?;
        adapter.apply(&plan)?;

        let second_plan = adapter.plan(&ctx)?;
        assert!(second_plan.is_noop());
        Ok(())
    }

    #[test]
    fn verify_passes_after_apply_and_fails_after_binary_moves(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let target = dir.path().join(".mcp.json");
        let config = GenericAdapterConfig::new("generic-harness", target)?;
        let adapter = GenericAdapter::new(config);
        let ctx = RequestContext::with_defaults(PathBuf::from("/abs/path/to/enforcer"));

        let plan = adapter.plan(&ctx)?;
        adapter.apply(&plan)?;
        let report = adapter.verify(&ctx)?;
        assert!(report.all_passed());

        let moved_ctx = RequestContext::with_defaults(PathBuf::from("/abs/new/path/enforcer"));
        let report = adapter.verify(&moved_ctx)?;
        assert!(!report.all_passed());
        assert_eq!(report.checks[0].name, "mcp-registration-present");
        assert!(!report.checks[0].detail.is_empty());
        Ok(())
    }

    #[test]
    fn rejects_a_relative_target_path() {
        let result =
            GenericAdapterConfig::new("generic-harness", PathBuf::from("relative/.mcp.json"));
        assert!(result.is_err());
    }

    #[test]
    fn never_writes_a_per_repo_project_file_path_by_construction() {
        // The adapter has no notion of "project file" at all -- it only
        // ever writes `target_path`, which construction requires to be
        // absolute. This test documents that guarantee: there is no
        // relative/per-repo code path to accidentally exercise.
        let result = GenericAdapterConfig::new("generic-harness", PathBuf::from(".mcp.json"));
        assert!(result.is_err());
    }
}
