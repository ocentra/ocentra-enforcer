//! c09 — the KiloCode [`crate::core::HarnessAdapter`].
//!
//! # Charter (workpack c09, reference-multiharness-install-matrix —
//! BINDING)
//!
//! KiloCode is a VS Code extension; its MCP registry lives under VS
//! Code's per-user `globalStorage` tree at
//! **`globalStorage/kilocode.kilo-code/settings/mcp_settings.json`**
//! (relative to the VS Code user-data root this adapter is constructed
//! with) — USER/GLOBAL scope, never a per-repo file. This adapter:
//! - Upserts `mcpServers[<x01 SERVER_NAME const>] = { command: <absolute
//!   enforcer binary path>, args: [], env: {} }`, preserving every
//!   unrelated top-level key AND every unrelated `mcpServers` entry via a
//!   `serde_json::Value` merge — never a destructive overwrite of the
//!   whole file.
//! - Backs up the target file before every write
//!   ([`crate::backup::backup_before_write`]).
//! - `verify` re-reads the file and fails closed on malformed JSON.
//!
//! `command` MUST be the absolute path this adapter is constructed with —
//! a relative path cannot resolve from an arbitrary repo cwd
//! (RUST_ARCHITECTURE.md "Global-install scope contract").
//!
//! # `vscode_user_data_dir` vs c02's `.kilocode` marker
//!
//! c02 autodetect's `KILOCODE_HOME`/`.kilocode` probe is a lightweight
//! presence signal only; the real KiloCode extension config lives under
//! VS Code's `globalStorage`, a DIFFERENT root this adapter's caller
//! resolves separately (the platform's VS Code user-data directory,
//! e.g. `%APPDATA%/Code/User` on Windows, `~/.config/Code/User` on
//! Linux, `~/Library/Application Support/Code/User` on macOS) and passes
//! in as `vscode_user_data_dir` at construction — this module never
//! re-derives that platform convention itself.

//! BOUNDARY-INVARIANT: adapter configuration is normalized before install decisions.
//!
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::backup::backup_before_write;
use crate::core::HarnessAdapter;
use crate::error::{InstallError, InstallResult};
use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::install_types::ArtifactKind;
use enforcer_domain::install_types::{
    AppliedInstallChange, ApplyResult, ChangeDisposition, CheckStatus, CheckSubject, InstallReport,
    InstallReportText, InstallVerifyCheck, InstallVerifyReport, PlannedInstallChange,
};
use enforcer_domain::install_types::{InstallBinaryPath, InstallRequestContext, InstallRootPath};
use enforcer_domain::paths::RepoRoot;
use enforcer_domain::mcp_types::SERVER_NAME;

/// This adapter's registration key, matching [`crate::report::HarnessKey`].
/// The KiloCode [`HarnessAdapter`]. Rooted at a `vscode_user_data_dir`
/// (VS Code's per-user data root, the parent of `globalStorage/`) and a
/// `binary_path` fixed at construction, so `plan`/`apply`/`verify` all
/// compute against the exact same target. Tests point
/// `vscode_user_data_dir` at an isolated temp-dir fixture instead of the
/// real VS Code user-data root.
#[derive(Debug, Clone)]
pub struct KiloCodeAdapter {
    vscode_user_data_dir: InstallRootPath,
    binary_path: InstallBinaryPath,
}

impl KiloCodeAdapter {
    /// Build an adapter rooted at `vscode_user_data_dir`, registering
    /// `binary_path` as the MCP server command.
    pub fn try_new(
        vscode_user_data_dir: PathBuf,
        binary_path: PathBuf,
    ) -> Result<Self, DecodeError> {
        Ok(Self {
            vscode_user_data_dir: InstallRootPath::try_from(vscode_user_data_dir)?,
            binary_path: InstallBinaryPath::try_from(binary_path)?,
        })
    }

    /// `globalStorage/kilocode.kilo-code/settings/mcp_settings.json` —
    /// the native KiloCode MCP registry, per the workpack's
    /// reference-format table.
    #[must_use]
    pub fn config_path(&self) -> PathBuf {
        self.vscode_user_data_dir
            .as_path()
            .join("globalStorage")
            .join("kilocode.kilo-code")
            .join("settings")
            .join("mcp_settings.json")
    }

    fn io_err(path: &Path, e: impl std::fmt::Display) -> InstallError {
        InstallError::Io {
            path: path.display().to_string(),
            reason: e.to_string(),
        }
    }

    fn repo_root(path: &Path) -> InstallResult<RepoRoot> {
        path.display().to_string().try_into().map_err(
            |e: enforcer_domain::boundary::decode_error::DecodeError| {
                InstallError::MalformedConfig {
                    path: path.display().to_string(),
                    reason: e.to_string(),
                }
            },
        )
    }

    fn read_config(&self) -> InstallResult<Value> {
        let path = self.config_path();
        if !path.is_file() {
            return Ok(Value::Object(Map::new()));
        }
        let raw = std::fs::read_to_string(&path).map_err(|e| Self::io_err(&path, e))?;
        if raw.trim().is_empty() {
            return Ok(Value::Object(Map::new()));
        }
        serde_json::from_str(&raw).map_err(|e| InstallError::MalformedConfig {
            path: path.display().to_string(),
            reason: format!("not valid JSON: {e}"),
        })
    }

    fn write_config(&self, root: &Value) -> InstallResult<()> {
        let path = self.config_path();
        let rendered = serde_json::to_string_pretty(root).map_err(|e| InstallError::Io {
            path: path.display().to_string(),
            reason: e.to_string(),
        })?;
        std::fs::write(&path, rendered).map_err(|e| Self::io_err(&path, e))
    }

    fn desired_entry(binary_path: &Path) -> Value {
        serde_json::json!({
            "command": binary_path.display().to_string(),
            "args": [],
            "env": {},
        })
    }

    fn entry_matches(existing_root: &Value, desired: &Value) -> bool {
        existing_root
            .get("mcpServers")
            .and_then(|servers| servers.get(SERVER_NAME))
            == Some(desired)
    }

    fn merge_mcp_server(existing_root: &mut Value, desired: Value) {
        if !existing_root.is_object() {
            *existing_root = Value::Object(Map::new());
        }
        let Some(root) = existing_root.as_object_mut() else {
            return;
        };
        let servers = root
            .entry("mcpServers")
            .or_insert_with(|| Value::Object(Map::new()));
        if !servers.is_object() {
            *servers = Value::Object(Map::new());
        }
        if let Some(servers_map) = servers.as_object_mut() {
            servers_map.insert(SERVER_NAME.to_owned(), desired);
        }
    }
}

impl HarnessAdapter for KiloCodeAdapter {
    fn harness_key(&self) -> enforcer_domain::ids::HarnessId {
        enforcer_domain::ids::BuiltInHarness::KiloCode.id()
    }

    fn plan(&self, ctx: &InstallRequestContext) -> InstallResult<InstallReport> {
        let existing = self.read_config()?;
        let desired = Self::desired_entry(self.binary_path.as_path());
        if Self::entry_matches(&existing, &desired) {
            return Ok(InstallReport {
                planned_changes: vec![],
                warnings: vec![],
            });
        }
        let config_path = self.config_path();
        let is_update = config_path.is_file();
        Ok(InstallReport {
            planned_changes: vec![PlannedInstallChange {
                harness: self.harness_key(),
                kind: ArtifactKind::McpRegistration,
                path: Self::repo_root(&config_path)?,
                description: InstallReportText::try_from(format!(
                    "upsert mcpServers[\"{SERVER_NAME}\"] in \
                     globalStorage/kilocode.kilo-code/settings/mcp_settings.json \
                     (user/global scope), binary_path={}",
                    ctx.binary_path.as_path().display()
                ))?,
                disposition: if is_update {
                    ChangeDisposition::Update
                } else {
                    ChangeDisposition::Create
                },
            }],
            warnings: vec![],
        })
    }

    fn apply(&self, report: &InstallReport) -> InstallResult<ApplyResult> {
        let mut applied = Vec::with_capacity(report.planned_changes.len());
        for change in &report.planned_changes {
            let target = PathBuf::from(change.path.as_str());
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent).map_err(|e| Self::io_err(parent, e))?;
            }
            let backup_path = backup_before_write(&target)?;

            let mut existing = self.read_config()?;
            Self::merge_mcp_server(
                &mut existing,
                Self::desired_entry(self.binary_path.as_path()),
            );
            self.write_config(&existing)?;

            applied.push(AppliedInstallChange {
                change: change.clone(),
                status: CheckStatus::Passed,
                backup_path: backup_path.map(|p| Self::repo_root(&p)).transpose()?,
            });
        }
        Ok(ApplyResult { applied })
    }

    fn verify(&self, ctx: &InstallRequestContext) -> InstallResult<InstallVerifyReport> {
        let root = self.read_config()?;
        let entry = root.get("mcpServers").and_then(|s| s.get(SERVER_NAME));
        let check = match entry {
            None => InstallVerifyCheck {
                subject: CheckSubject::Harness(self.harness_key()),
                name: InstallReportText::try_from("mcp-registration-present".to_owned())?,
                status: CheckStatus::Failed,
                detail: InstallReportText::try_from(format!(
                    "mcpServers.{SERVER_NAME} missing from `{}`",
                    self.config_path().display()
                ))?,
            },
            Some(entry) => {
                let command = entry.get("command").and_then(Value::as_str);
                let expected = ctx.binary_path.as_path().display().to_string();
                match command {
                    Some(c) if c == expected => InstallVerifyCheck {
                        subject: CheckSubject::Harness(self.harness_key()),
                        name: InstallReportText::try_from("mcp-registration-present".to_owned())?,
                        status: CheckStatus::Passed,
                        detail: InstallReportText::try_from(String::new())?,
                    },
                    Some(c) => InstallVerifyCheck {
                        subject: CheckSubject::Harness(self.harness_key()),
                        name: InstallReportText::try_from("mcp-registration-present".to_owned())?,
                        status: CheckStatus::Failed,
                        detail: InstallReportText::try_from(format!(
                            "mcpServers.{SERVER_NAME}.command = `{c}`, expected `{expected}`"
                        ))?,
                    },
                    None => InstallVerifyCheck {
                        subject: CheckSubject::Harness(self.harness_key()),
                        name: InstallReportText::try_from("mcp-registration-present".to_owned())?,
                        status: CheckStatus::Failed,
                        detail: InstallReportText::try_from(format!(
                            "mcpServers.{SERVER_NAME} has no `command` field"
                        ))?,
                    },
                }
            }
        };
        Ok(InstallVerifyReport {
            checks: vec![check],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{InstallError, KiloCodeAdapter, SERVER_NAME};
    use crate::core::HarnessAdapter;
    use enforcer_domain::boundary::decode_error::DecodeError;
    use enforcer_domain::install_types::InstallRequestContext;
    use serde_json::Value;
    use std::fs;
    use std::path::Path;

    fn ctx(binary: &Path) -> Result<InstallRequestContext, DecodeError> {
        InstallRequestContext::try_with_defaults(binary.to_path_buf())
    }

    #[test]
    fn harness_key_is_kilocode() -> Result<(), Box<dyn std::error::Error>> {
        let home = tempfile::tempdir()?;
        let adapter =
            KiloCodeAdapter::try_new(home.path().to_path_buf(), home.path().join("enforcer"))?;
        assert_eq!(adapter.harness_key().as_str(), "kilocode");
        Ok(())
    }

    #[test]
    fn fresh_install_apply_then_verify_all_green() -> Result<(), Box<dyn std::error::Error>> {
        let home = tempfile::tempdir()?;
        let binary = home.path().join("bin").join("enforcer");
        let adapter = KiloCodeAdapter::try_new(home.path().to_path_buf(), binary.clone())?;

        let plan = adapter.plan(&ctx(&binary)?)?;
        assert_eq!(plan.planned_changes.len(), 1);
        let applied = adapter.apply(&plan)?;
        assert!(applied.applied.iter().all(|change| matches!(
            change.status,
            enforcer_domain::install_types::CheckStatus::Passed
        )));

        let verify = adapter.verify(&ctx(&binary)?)?;
        assert!(
            verify.checks.iter().all(|check| matches!(
                check.status,
                enforcer_domain::install_types::CheckStatus::Passed
            )),
            "expected all checks to pass: {verify:?}"
        );

        let second_plan = adapter.plan(&ctx(&binary)?)?;
        assert!(
            second_plan.planned_changes.is_empty(),
            "second apply must be idempotent"
        );
        Ok(())
    }

    #[test]
    fn second_apply_is_byte_identical() -> Result<(), Box<dyn std::error::Error>> {
        let home = tempfile::tempdir()?;
        let binary = home.path().join("bin").join("enforcer");
        let adapter = KiloCodeAdapter::try_new(home.path().to_path_buf(), binary.clone())?;

        let plan = adapter.plan(&ctx(&binary)?)?;
        adapter.apply(&plan)?;
        let first_bytes = fs::read(adapter.config_path())?;

        let mut existing = adapter.read_config()?;
        KiloCodeAdapter::merge_mcp_server(&mut existing, KiloCodeAdapter::desired_entry(&binary));
        adapter.write_config(&existing)?;
        let second_bytes = fs::read(adapter.config_path())?;
        assert_eq!(first_bytes, second_bytes);
        Ok(())
    }

    #[test]
    fn upsert_preserves_unrelated_keys_and_servers() -> Result<(), Box<dyn std::error::Error>> {
        let home = tempfile::tempdir()?;
        let binary = home.path().join("bin").join("enforcer");
        let config_dir = home
            .path()
            .join("globalStorage")
            .join("kilocode.kilo-code")
            .join("settings");
        fs::create_dir_all(&config_dir)?;
        fs::write(
            config_dir.join("mcp_settings.json"),
            serde_json::to_string_pretty(&serde_json::json!({
                "unrelatedTopLevelKey": "keep-me",
                "mcpServers": {
                    "other-server": { "command": "/abs/other" }
                }
            }))?,
        )?;

        let adapter = KiloCodeAdapter::try_new(home.path().to_path_buf(), binary.clone())?;
        let plan = adapter.plan(&ctx(&binary)?)?;
        adapter.apply(&plan)?;

        let written: Value = serde_json::from_str(&fs::read_to_string(adapter.config_path())?)?;
        assert_eq!(written["unrelatedTopLevelKey"], "keep-me");
        assert_eq!(
            written["mcpServers"]["other-server"]["command"],
            "/abs/other"
        );
        assert_eq!(
            written["mcpServers"][SERVER_NAME]["command"],
            binary.display().to_string()
        );
        Ok(())
    }

    #[test]
    fn verify_fails_when_entry_missing_or_renamed() -> Result<(), Box<dyn std::error::Error>> {
        let home = tempfile::tempdir()?;
        let binary = home.path().join("bin").join("enforcer");
        let adapter = KiloCodeAdapter::try_new(home.path().to_path_buf(), binary.clone())?;

        let report = adapter.verify(&ctx(&binary)?)?;
        assert!(!report.checks.iter().all(|check| matches!(
            check.status,
            enforcer_domain::install_types::CheckStatus::Passed
        )));

        let plan = adapter.plan(&ctx(&binary)?)?;
        adapter.apply(&plan)?;
        let mut root = adapter.read_config()?;
        if let Some(servers) = root.get_mut("mcpServers").and_then(Value::as_object_mut) {
            if let Some(entry) = servers.remove(SERVER_NAME) {
                servers.insert("enforcer-renamed".to_owned(), entry);
            }
        }
        adapter.write_config(&root)?;
        let report = adapter.verify(&ctx(&binary)?)?;
        assert!(!report.checks.iter().all(|check| matches!(
            check.status,
            enforcer_domain::install_types::CheckStatus::Passed
        )));
        assert_eq!(report.checks[0].name.as_str(), "mcp-registration-present");
        Ok(())
    }

    #[test]
    fn malformed_config_is_a_detected_plan_failure() -> Result<(), Box<dyn std::error::Error>> {
        let home = tempfile::tempdir()?;
        let binary = home.path().join("bin").join("enforcer");
        let config_dir = home
            .path()
            .join("globalStorage")
            .join("kilocode.kilo-code")
            .join("settings");
        fs::create_dir_all(&config_dir)?;
        fs::write(config_dir.join("mcp_settings.json"), "{ not valid json")?;
        let adapter = KiloCodeAdapter::try_new(home.path().to_path_buf(), binary.clone())?;
        let result = adapter.plan(&ctx(&binary)?);
        assert!(matches!(result, Err(InstallError::MalformedConfig { .. })));
        Ok(())
    }

    #[test]
    fn verify_fails_closed_on_malformed_config() -> Result<(), Box<dyn std::error::Error>> {
        let home = tempfile::tempdir()?;
        let binary = home.path().join("bin").join("enforcer");
        let config_dir = home
            .path()
            .join("globalStorage")
            .join("kilocode.kilo-code")
            .join("settings");
        fs::create_dir_all(&config_dir)?;
        fs::write(config_dir.join("mcp_settings.json"), "{ not valid json")?;
        let adapter = KiloCodeAdapter::try_new(home.path().to_path_buf(), binary.clone())?;
        let result = adapter.verify(&ctx(&binary)?);
        assert!(matches!(result, Err(InstallError::MalformedConfig { .. })));
        Ok(())
    }

    #[test]
    fn never_writes_the_legacy_ocentra_enforcer_key() -> Result<(), Box<dyn std::error::Error>> {
        let home = tempfile::tempdir()?;
        let binary = home.path().join("bin").join("enforcer");
        let adapter = KiloCodeAdapter::try_new(home.path().to_path_buf(), binary.clone())?;
        let plan = adapter.plan(&ctx(&binary)?)?;
        adapter.apply(&plan)?;
        let written = fs::read_to_string(adapter.config_path())?;
        assert!(!written.as_str().contains("ocentra-enforcer"));
        Ok(())
    }
}
