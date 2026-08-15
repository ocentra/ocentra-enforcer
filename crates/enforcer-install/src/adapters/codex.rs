//! c06 — the Codex [`crate::core::HarnessAdapter`].
//!
//! BOUNDARY-INVARIANT: this adapter is the only layer that reads or writes
//! Codex's user-global configuration and agent files. It preserves unrelated
//! configuration and translates that external TOML/YAML/file-system state into
//! typed install reports and errors; no domain policy belongs here.
//! boundaryOwnerNote: `crates/enforcer-install/src/adapters/**` is the owned
//! install-transport boundary because each harness has a distinct user-global
//! configuration format and lifecycle contract.
//!
//! # Charter (workpack c06, RUST_ARCHITECTURE.md "Global-install scope
//! contract" — BINDING)
//!
//! Codex reads its MCP server registry from the user `~/.codex/config.toml`
//! `[mcp_servers.<name>]` table — USER/GLOBAL scope, never a per-repo
//! config. This adapter:
//! - Upserts `[mcp_servers.<SERVER_NAME>]` into `<codex_home>/config.toml`
//!   via `toml_edit` (format-preserving — every unrelated table/comment in
//!   the file survives byte-for-byte), pointing `command` at the
//!   **absolute** installed `enforcer` binary path, with `args = []`, an
//!   `env.OCENTRA_LEDGER_HOME` entry, and `enabled = true`. The registered
//!   MCP server is the `enforcer` binary itself speaking MCP on stdio — no
//!   `node`/`.mjs` shim (unlike the legacy reference `src/codex-install.mjs`,
//!   which pointed `command` at `node` with the `.mjs` server file as an
//!   arg).
//! - Emits `<codex_home>/agents/openai.yaml` — the native Codex agent
//!   descriptor (`display_name`/`short_description`/`default_prompt`/
//!   `allow_implicit_invocation`), keyed to the x01-owned [`SERVER_NAME`]
//!   const, mirroring how c03 emits `.claude/agents/<SERVER_NAME>.md`. This
//!   is a first-class per-adapter emitter — NOT the legacy bulk
//!   `.codex-plugin` publish (`plugin.json "skills": "./skills/"`), which
//!   dragged the whole `skills/` tree along.
//! - Owns the harness-neutral GLOBAL `<codex_home>/AGENTS.md` managed block
//!   bounded by the TRANSITIONAL `<!-- ocentra-enforcer:start -->` /
//!   `<!-- ocentra-enforcer:end -->` markers, kept BYTE-FOR-BYTE identical
//!   to the legacy `.mjs` markers until x03 migrates them — this is
//!   intentionally NOT the shared `crate::managed_block` marker shape
//!   (`<!-- enforcer:managed:begin:<name> -->`), which is the FUTURE
//!   post-x03 shape.
//! - Drops the enforcer user-skill under `<codex_home>/skills/<SERVER_NAME>`.
//! - `verify` re-reads `config.toml` AND the descriptor, fail-closed on
//!   malformed TOML or a missing/corrupt descriptor.
//!
//! `command` MUST be the absolute path this adapter is constructed with
//! ([`CodexAdapter::try_new`]) — a relative path cannot resolve from an
//! arbitrary repo cwd (RUST_ARCHITECTURE.md). As with c03's `ClaudeAdapter`,
//! the binary path is adapter STATE (set once at construction) rather than
//! re-derived per call — `plan`/`verify` both cross-check their
//! `ctx.binary_path` against this same stored value so plan/apply/verify can
//! never silently drift from one another.

use std::path::{Path, PathBuf};

use toml_edit::{value, Array, DocumentMut, Item, Table};

use crate::backup::backup_before_write;
use crate::core::HarnessAdapter;
use crate::error::{InstallError, InstallResult};
use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::install_types::{
    AppliedInstallChange, ApplyResult, ArtifactKind, ChangeDisposition, CheckStatus, CheckSubject,
    InstallBinaryPath, InstallReport, InstallReportText, InstallRequestContext, InstallRootPath,
    InstallVerifyCheck, InstallVerifyReport, PlannedInstallChange,
};
use enforcer_domain::mcp_types::SERVER_NAME;
use enforcer_domain::paths::RepoRoot;

/// Env var this adapter sets on the registered MCP server entry, matching
/// [`crate::adapters::claude::LEDGER_HOME_ENV`] (shared literal, kept local
/// to avoid a cross-adapter dependency edge).
pub const LEDGER_HOME_ENV: &str = "OCENTRA_LEDGER_HOME";

/// The TRANSITIONAL global `AGENTS.md` managed-block markers, kept
/// byte-for-byte identical to the legacy `src/codex-install.mjs`
/// `GLOBAL_AGENTS_START`/`GLOBAL_AGENTS_END` constants until x03 migrates
/// them to the harness-neutral `ocentra-enforcer:managed:*` shape. NEVER the
/// shared `crate::managed_block` marker shape — that is the FUTURE
/// post-x03 shape, and changing this literal early would break every
/// consumer's existing `AGENTS.md` mid-transition.
const GLOBAL_AGENTS_START: &str = "<!-- ocentra-enforcer:start -->";
const GLOBAL_AGENTS_END: &str = "<!-- ocentra-enforcer:end -->";

/// The Codex [`HarnessAdapter`]. Rooted at a `codex_home` directory (the
/// parent of `config.toml`, `agents/openai.yaml`, `skills/`, and
/// `AGENTS.md`) and a `binary_path` (the absolute path to the installed
/// `enforcer` binary) fixed at construction, so `plan`/`apply`/`verify` all
/// compute against the exact same target. Tests point `codex_home` at an
/// isolated temp-dir fixture instead of the real `~/.codex`.
#[derive(Debug, Clone)]
pub struct CodexAdapter {
    /// The Codex home root (`~/.codex` in production, or `$CODEX_HOME`
    /// when set — resolving that env var is c02's job; this adapter takes
    /// the already-resolved directory). In a test this is a temp-dir
    /// fixture root — NEVER the real `~/.codex` in a test.
    codex_home: InstallRootPath,
    /// Absolute path to the installed `enforcer` binary this adapter
    /// registers as `mcp_servers.<SERVER_NAME>.command`.
    binary_path: InstallBinaryPath,
}

impl CodexAdapter {
    /// Build an adapter rooted at `codex_home`, registering `binary_path`
    /// as the MCP server command.
    pub fn try_new(codex_home: PathBuf, binary_path: PathBuf) -> Result<Self, DecodeError> {
        Ok(Self {
            codex_home: InstallRootPath::try_from(codex_home)?,
            binary_path: InstallBinaryPath::try_from(binary_path)?,
        })
    }

    /// `<codex_home>/config.toml` — the `mcp_servers` registry table.
    #[must_use]
    pub fn config_toml_path(&self) -> PathBuf {
        self.codex_home.as_path().join("config.toml")
    }

    /// `<codex_home>/skills/<SERVER_NAME>` — the skill directory this
    /// adapter drops the enforcer skill under.
    #[must_use]
    pub fn skill_dir(&self) -> PathBuf {
        self.codex_home.as_path().join("skills").join(SERVER_NAME)
    }

    /// `<codex_home>/skills/<SERVER_NAME>/SKILL.md`.
    #[must_use]
    pub fn skill_md_path(&self) -> PathBuf {
        self.skill_dir().join("SKILL.md")
    }

    /// `<codex_home>/agents/openai.yaml` — the native Codex agent
    /// descriptor, the analog of Claude's `.claude/agents/<SERVER_NAME>.md`.
    #[must_use]
    pub fn agent_descriptor_path(&self) -> PathBuf {
        self.codex_home.as_path().join("agents").join("openai.yaml")
    }

    /// `<codex_home>/AGENTS.md` — the GLOBAL managed-block doctrine
    /// reference (transitional `ocentra-enforcer` markers).
    #[must_use]
    pub fn global_agents_md_path(&self) -> PathBuf {
        self.codex_home.as_path().join("AGENTS.md")
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

    /// Read `config.toml` as a format-preserving [`DocumentMut`], defaulting
    /// to an empty document when the file does not exist yet (fresh
    /// install). A file that exists but fails to parse is a detected
    /// [`InstallError::MalformedConfig`] — never silently treated as empty.
    fn read_config_toml(&self) -> InstallResult<DocumentMut> {
        let path = self.config_toml_path();
        if !path.is_file() {
            return Ok(DocumentMut::new());
        }
        let raw = std::fs::read_to_string(&path).map_err(|e| Self::io_err(&path, e))?;
        raw.parse::<DocumentMut>()
            .map_err(|e| InstallError::MalformedConfig {
                path: path.display().to_string(),
                reason: format!("not valid TOML: {e}"),
            })
    }

    fn write_config_toml(&self, doc: &DocumentMut) -> InstallResult<()> {
        let path = self.config_toml_path();
        std::fs::write(&path, doc.to_string()).map_err(|e| Self::io_err(&path, e))
    }

    /// Build the desired `[mcp_servers.<SERVER_NAME>]` table: `command`
    /// pointing at the absolute `binary_path`, empty `args` (the binary
    /// itself speaks MCP on stdio — no `.mjs` shim/args), the
    /// `OCENTRA_LEDGER_HOME` env entry, and `enabled = true`.
    fn desired_table(binary_path: &Path) -> Table {
        let mut table = Table::new();
        table.insert("command", value(binary_path.display().to_string()));
        table.insert("args", Item::Value(Array::new().into()));
        let mut env = Table::new();
        env.set_implicit(false);
        env.insert("OCENTRA_LEDGER_HOME", value("${HOME}/.enforcer/ledger"));
        table.insert("env", Item::Table(env));
        table.insert("enabled", value(true));
        table
    }

    /// True when `doc`'s `mcp_servers.<SERVER_NAME>` table already equals
    /// `desired` — the idempotent-reinstall no-op signal.
    fn entry_matches(doc: &DocumentMut, desired: &Table) -> bool {
        doc.get("mcp_servers")
            .and_then(Item::as_table)
            .and_then(|servers| servers.get(SERVER_NAME))
            .and_then(Item::as_table)
            .map(|existing| existing.to_string() == desired.to_string())
            .unwrap_or(false)
    }

    /// Upsert `desired` into `doc`'s `mcp_servers.<SERVER_NAME>` table,
    /// preserving every unrelated top-level table AND every unrelated
    /// `mcp_servers` entry, format-preserving via `toml_edit`.
    fn merge_mcp_server(doc: &mut DocumentMut, desired: Table) {
        if doc.get("mcp_servers").and_then(Item::as_table).is_none() {
            doc.as_table_mut()
                .insert("mcp_servers", Item::Table(Table::new()));
        }
        if let Some(servers) = doc.get_mut("mcp_servers").and_then(Item::as_table_mut) {
            servers.insert(SERVER_NAME, Item::Table(desired));
        }
    }

    /// Remove `mcp_servers.<SERVER_NAME>` from `doc`, preserving every
    /// unrelated table AND every unrelated `mcp_servers` entry — the
    /// uninstall-direction mirror of [`Self::merge_mcp_server`].
    fn remove_mcp_server(doc: &mut DocumentMut) {
        if let Some(servers) = doc.get_mut("mcp_servers").and_then(Item::as_table_mut) {
            servers.remove(SERVER_NAME);
        }
    }

    /// Render the native `agents/openai.yaml` descriptor: `display_name`/
    /// `short_description`/`default_prompt`/`allow_implicit_invocation`,
    /// keyed to [`SERVER_NAME`].
    #[must_use]
    pub fn render_agent_descriptor() -> String {
        format!(
            "display_name: {SERVER_NAME}\n\
             short_description: Mechanical enforcement gate for this repository.\n\
             default_prompt: |\n\
             \x20\x20You are the `{SERVER_NAME}` enforcement agent. Use the\n\
             \x20\x20`mcp__{SERVER_NAME}__*` tools to run mechanical checks, scans, and\n\
             \x20\x20proof status against this repository. Never let a self-review\n\
             \x20\x20substitute for a mechanical gate: treat every claim of\n\
             \x20\x20\"done\"/\"passing\"/\"ready\" as unproven until the `{SERVER_NAME}`\n\
             \x20\x20tools confirm it.\n\
             allow_implicit_invocation: true\n\
             "
        )
    }

    /// Parse-and-validate `agents/openai.yaml`'s content far enough to
    /// confirm it is a genuine descriptor: declares `display_name:` and
    /// `allow_implicit_invocation: true`. Returns the parse failure reason
    /// on anything else — never a silent pass on a corrupt/truncated file.
    fn validate_agent_descriptor(raw: &str) -> Result<(), String> {
        if !raw
            .lines()
            .any(|l| l.trim_start().starts_with("display_name:"))
        {
            return Err("missing `display_name:` field".to_owned());
        }
        if !raw
            .lines()
            .any(|l| l.trim() == "allow_implicit_invocation: true")
        {
            return Err("missing `allow_implicit_invocation: true`".to_owned());
        }
        Ok(())
    }

    /// The GLOBAL `AGENTS.md` managed-block content, kept structurally
    /// equivalent to the legacy `.mjs` `globalAgentsInstructionBlock`
    /// (server name, pack-root-equivalent reference, coordination
    /// pointers) — the transitional-marker body, not the shared
    /// `crate::managed_block` shape.
    fn global_agents_block_body() -> String {
        format!(
            "# Ocentra Enforcer\n\
             \n\
             Use Ocentra Enforcer for project-independent enforcement, coordination, \
             and compact diagnostics.\n\
             MCP server name: `{SERVER_NAME}`.\n\
             \n\
             Before relying on raw terminal output, prefer:\n\
             - `mcp__{SERVER_NAME}__route` for indexed rule routing.\n\
             - `mcp__{SERVER_NAME}__check` / `mcp__{SERVER_NAME}__scan` for hard \
             validation.\n\
             - `mcp__{SERVER_NAME}__run` plus `mcp__{SERVER_NAME}__last_failure` for \
             compact harness diagnostics.\n\
             - `mcp__{SERVER_NAME}__coordination_health` / `claim` / `guard` for \
             lane/mail/exact-file coordination.\n\
             \n\
             Coordination is a harness concern, not a product-repo concern. Live \
             state belongs under the Enforcer install ledger root by default.\n"
        )
    }

    /// Full transitional-marker-bounded block ready to upsert into
    /// `AGENTS.md`, mirroring the legacy `.mjs` `upsertManagedBlock`
    /// contract: markers + body, no trailing content beyond a single
    /// newline.
    fn render_global_agents_block() -> String {
        format!(
            "{GLOBAL_AGENTS_START}\n{}{GLOBAL_AGENTS_END}\n",
            Self::global_agents_block_body()
        )
    }

    /// Upsert the transitional `<!-- ocentra-enforcer:start/end -->`
    /// managed block into `existing`, replacing exactly one prior
    /// begin/end pair (or appending if none is present) — the
    /// `codex.rs`-local equivalent of `crate::managed_block::upsert_block`,
    /// kept on the OLD marker shape rather than migrated early (x03 owns
    /// that migration).
    ///
    /// # Errors
    /// Returns [`InstallError::ManagedBlockInvalid`] if the begin/end
    /// markers appear more than once, or one appears without its
    /// counterpart.
    pub fn upsert_global_agents_block(existing: &str, path: &str) -> InstallResult<String> {
        let begin_count = existing.matches(GLOBAL_AGENTS_START).count();
        let end_count = existing.matches(GLOBAL_AGENTS_END).count();
        let rendered = Self::render_global_agents_block();

        if begin_count == 0 && end_count == 0 {
            if existing.trim().is_empty() {
                return Ok(rendered);
            }
            let sep = if existing.ends_with('\n') {
                "\n"
            } else {
                "\n\n"
            };
            return Ok(format!("{existing}{sep}{rendered}"));
        }

        if begin_count != 1 || end_count != 1 {
            return Err(InstallError::ManagedBlockInvalid {
                path: path.to_owned(),
                marker: "ocentra-enforcer".to_owned(),
                reason: format!(
                    "expected exactly one begin/end marker pair, found {begin_count} begin and {end_count} end"
                ),
            });
        }

        let begin_idx = existing.find(GLOBAL_AGENTS_START).ok_or_else(|| {
            InstallError::ManagedBlockInvalid {
                path: path.to_owned(),
                marker: "ocentra-enforcer".to_owned(),
                reason: "begin marker vanished during re-scan".to_owned(),
            }
        })?;
        let end_idx =
            existing
                .find(GLOBAL_AGENTS_END)
                .ok_or_else(|| InstallError::ManagedBlockInvalid {
                    path: path.to_owned(),
                    marker: "ocentra-enforcer".to_owned(),
                    reason: "end marker vanished during re-scan".to_owned(),
                })?;
        if end_idx < begin_idx {
            return Err(InstallError::ManagedBlockInvalid {
                path: path.to_owned(),
                marker: "ocentra-enforcer".to_owned(),
                reason: "end marker appears before begin marker".to_owned(),
            });
        }

        let before =
            existing
                .get(..begin_idx)
                .ok_or_else(|| InstallError::ManagedBlockInvalid {
                    path: path.to_owned(),
                    marker: "ocentra-enforcer".to_owned(),
                    reason: "begin marker offset is not a UTF-8 boundary".to_owned(),
                })?;
        let after_offset = end_idx + GLOBAL_AGENTS_END.len();
        let after =
            existing
                .get(after_offset..)
                .ok_or_else(|| InstallError::ManagedBlockInvalid {
                    path: path.to_owned(),
                    marker: "ocentra-enforcer".to_owned(),
                    reason: "end marker offset is not a UTF-8 boundary".to_owned(),
                })?;
        let after = after.trim_start_matches('\n');
        Ok(format!("{before}{rendered}{after}"))
    }

    /// The enforcer user-skill body dropped at
    /// `<codex_home>/skills/<SERVER_NAME>/SKILL.md`.
    fn skill_md_content() -> String {
        format!(
            "---\n\
             name: {SERVER_NAME}\n\
             description: Run the {SERVER_NAME} mechanical-enforcement checks (rule \
             checks, scans, proof status) against this repository via its MCP tools \
             (mcp__{SERVER_NAME}__*).\n\
             ---\n\
             \n\
             # {SERVER_NAME}\n\
             \n\
             Use the `mcp__{SERVER_NAME}__*` MCP tools for rule checks, scans, and \
             proof status. See `agents/openai.yaml` for the full agent descriptor.\n\
             "
        )
    }

    /// Whether `ctx.binary_path` matches this adapter's own configured
    /// binary path — a mismatch means the caller built `InstallRequestContext`
    /// inconsistently with how this adapter was constructed, which would
    /// otherwise silently plan/verify against the wrong target.
    fn check_ctx_consistency(&self, ctx: &InstallRequestContext) -> InstallResult<()> {
        if ctx.binary_path.as_path() != self.binary_path.as_path() {
            return Err(InstallError::MalformedConfig {
                path: self.config_toml_path().display().to_string(),
                reason: format!(
                    "InstallRequestContext.binary_path `{}` does not match the path `{}` this \
                     CodexAdapter was constructed with",
                    ctx.binary_path.as_path().display(),
                    self.binary_path.as_path().display()
                ),
            });
        }
        Ok(())
    }
}

impl HarnessAdapter for CodexAdapter {
    fn harness_key(&self) -> enforcer_domain::ids::HarnessId {
        enforcer_domain::ids::BuiltInHarness::Codex.id()
    }

    fn plan(&self, ctx: &InstallRequestContext) -> InstallResult<InstallReport> {
        self.check_ctx_consistency(ctx)?;
        let existing = self.read_config_toml()?;
        let desired = Self::desired_table(self.binary_path.as_path());
        let mut planned_changes = Vec::new();
        let mut warnings = Vec::new();

        let config_toml_is_update = self.config_toml_path().is_file();
        if !Self::entry_matches(&existing, &desired) {
            planned_changes.push(PlannedInstallChange {
                harness: self.harness_key(),
                kind: ArtifactKind::McpRegistration,
                path: Self::repo_root(&self.config_toml_path())?,
                description: InstallReportText::try_from(format!(
                    "upsert [mcp_servers.{SERVER_NAME}] in <codex_home>/config.toml (user/global scope)"
                ))?,
                disposition: if config_toml_is_update { ChangeDisposition::Update } else { ChangeDisposition::Create },
            });
        }

        let skill_path = self.skill_md_path();
        let desired_skill = Self::skill_md_content();
        let skill_is_update = skill_path.is_file();
        let skill_matches = skill_is_update
            && std::fs::read_to_string(&skill_path)
                .map(|s| s == desired_skill)
                .unwrap_or(false);
        if !skill_matches {
            planned_changes.push(PlannedInstallChange {
                harness: self.harness_key(),
                kind: ArtifactKind::HarnessSpecific,
                path: Self::repo_root(&skill_path)?,
                description: InstallReportText::try_from(format!(
                    "install {SERVER_NAME} skill under <codex_home>/skills"
                ))?,
                disposition: if skill_is_update {
                    ChangeDisposition::Update
                } else {
                    ChangeDisposition::Create
                },
            });
        }

        let descriptor_path = self.agent_descriptor_path();
        let desired_descriptor = Self::render_agent_descriptor();
        let descriptor_is_update = descriptor_path.is_file();
        let descriptor_matches = descriptor_is_update
            && std::fs::read_to_string(&descriptor_path)
                .map(|s| s == desired_descriptor)
                .unwrap_or(false);
        if !descriptor_matches {
            planned_changes.push(PlannedInstallChange {
                harness: self.harness_key(),
                kind: ArtifactKind::HarnessSpecific,
                path: Self::repo_root(&descriptor_path)?,
                description: InstallReportText::try_from(
                    "emit agents/openai.yaml agent descriptor".to_owned(),
                )?,
                disposition: if descriptor_is_update {
                    ChangeDisposition::Update
                } else {
                    ChangeDisposition::Create
                },
            });
        }

        let global_agents_path = self.global_agents_md_path();
        let global_agents_is_update = global_agents_path.is_file();
        let existing_global_agents = if global_agents_is_update {
            std::fs::read_to_string(&global_agents_path)
                .map_err(|e| Self::io_err(&global_agents_path, e))?
        } else {
            String::new()
        };
        let rendered_global_agents = Self::upsert_global_agents_block(
            &existing_global_agents,
            &global_agents_path.display().to_string(),
        )?;
        if rendered_global_agents != existing_global_agents {
            planned_changes.push(PlannedInstallChange {
                harness: self.harness_key(),
                kind: ArtifactKind::DoctrineReference,
                path: Self::repo_root(&global_agents_path)?,
                description: InstallReportText::try_from(
                    "upsert global AGENTS.md ocentra-enforcer managed block".to_owned(),
                )?,
                disposition: if global_agents_is_update {
                    ChangeDisposition::Update
                } else {
                    ChangeDisposition::Create
                },
            });
        }

        if !self.codex_home.as_path().is_dir() {
            warnings.push(InstallReportText::try_from(format!(
                "Codex home `{}` does not exist yet; apply will create it",
                self.codex_home.as_path().display()
            ))?);
        }

        Ok(InstallReport {
            planned_changes,
            warnings,
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

            match change.kind {
                ArtifactKind::McpRegistration => {
                    let mut doc = self.read_config_toml()?;
                    Self::merge_mcp_server(
                        &mut doc,
                        Self::desired_table(self.binary_path.as_path()),
                    );
                    self.write_config_toml(&doc)?;
                }
                ArtifactKind::HarnessSpecific if target == self.skill_md_path() => {
                    std::fs::write(&target, Self::skill_md_content())
                        .map_err(|e| Self::io_err(&target, e))?;
                }
                ArtifactKind::HarnessSpecific if target == self.agent_descriptor_path() => {
                    std::fs::write(&target, Self::render_agent_descriptor())
                        .map_err(|e| Self::io_err(&target, e))?;
                }
                ArtifactKind::DoctrineReference => {
                    let existing = if target.is_file() {
                        std::fs::read_to_string(&target).map_err(|e| Self::io_err(&target, e))?
                    } else {
                        String::new()
                    };
                    let rendered =
                        Self::upsert_global_agents_block(&existing, &target.display().to_string())?;
                    std::fs::write(&target, rendered).map_err(|e| Self::io_err(&target, e))?;
                }
                _ => {
                    return Err(InstallError::MalformedConfig {
                        path: target.display().to_string(),
                        reason: format!(
                            "CodexAdapter::apply received an unrecognized planned change kind/path pair: {:?} at `{}`",
                            change.kind,
                            target.display()
                        ),
                    });
                }
            }

            applied.push(AppliedInstallChange {
                change: change.clone(),
                status: CheckStatus::Passed,
                backup_path: backup_path.map(|p| Self::repo_root(&p)).transpose()?,
            });
        }

        Ok(ApplyResult { applied })
    }

    fn verify(&self, ctx: &InstallRequestContext) -> InstallResult<InstallVerifyReport> {
        self.check_ctx_consistency(ctx)?;
        let mut checks = Vec::new();

        let config_toml_path = self.config_toml_path();
        // A malformed `config.toml` is FAIL-CLOSED here: `verify` returns
        // the typed error directly rather than downgrading it to a
        // `passed: false` check, mirroring c03's ClaudeAdapter contract.
        let mcp_check = match self.read_config_toml() {
            Err(e) => return Err(e),
            Ok(doc) => {
                let entry = doc
                    .get("mcp_servers")
                    .and_then(Item::as_table)
                    .and_then(|servers| servers.get(SERVER_NAME))
                    .and_then(Item::as_table);
                match entry {
                    None => InstallVerifyCheck {
                        subject: CheckSubject::Harness(self.harness_key()),
                        name: InstallReportText::try_from("mcp-registration-present".to_owned())?,
                        status: CheckStatus::Failed,
                        detail: InstallReportText::try_from(format!(
                            "mcp_servers.{SERVER_NAME} missing from `{}`",
                            config_toml_path.display()
                        ))?,
                    },
                    Some(entry) => {
                        let command = entry.get("command").and_then(|v| v.as_str());
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
                                    "mcp_servers.{SERVER_NAME}.command = `{c}`, expected `{expected}`"
                                ))?,
                            },
                            None => InstallVerifyCheck {
                                subject: CheckSubject::Harness(self.harness_key()),
                                name: InstallReportText::try_from("mcp-registration-present".to_owned())?,
                                status: CheckStatus::Failed,
                                detail: InstallReportText::try_from(format!(
                                    "mcp_servers.{SERVER_NAME} has no `command` field"
                                ))?,
                            },
                        }
                    }
                }
            }
        };
        checks.push(mcp_check);

        let descriptor_path = self.agent_descriptor_path();
        let descriptor_check = if !descriptor_path.is_file() {
            InstallVerifyCheck {
                subject: CheckSubject::Harness(self.harness_key()),
                name: InstallReportText::try_from("agent-descriptor-present".to_owned())?,
                status: CheckStatus::Failed,
                detail: InstallReportText::try_from(format!(
                    "missing agent descriptor at `{}`",
                    descriptor_path.display()
                ))?,
            }
        } else {
            match std::fs::read_to_string(&descriptor_path) {
                Err(e) => return Err(Self::io_err(&descriptor_path, e)),
                Ok(raw) => match Self::validate_agent_descriptor(&raw) {
                    Ok(()) => InstallVerifyCheck {
                        subject: CheckSubject::Harness(self.harness_key()),
                        name: InstallReportText::try_from("agent-descriptor-present".to_owned())?,
                        status: CheckStatus::Passed,
                        detail: InstallReportText::try_from(String::new())?,
                    },
                    Err(reason) => InstallVerifyCheck {
                        subject: CheckSubject::Harness(self.harness_key()),
                        name: InstallReportText::try_from("agent-descriptor-present".to_owned())?,
                        status: CheckStatus::Failed,
                        detail: InstallReportText::try_from(format!(
                            "descriptor at `{}` is corrupt: {reason}",
                            descriptor_path.display()
                        ))?,
                    },
                },
            }
        };
        checks.push(descriptor_check);

        let global_agents_path = self.global_agents_md_path();
        let global_agents_check = if !global_agents_path.is_file() {
            InstallVerifyCheck {
                subject: CheckSubject::Harness(self.harness_key()),
                name: InstallReportText::try_from("global-agents-md-block-present".to_owned())?,
                status: CheckStatus::Failed,
                detail: InstallReportText::try_from(format!(
                    "missing global AGENTS.md at `{}`",
                    global_agents_path.display()
                ))?,
            }
        } else {
            match std::fs::read_to_string(&global_agents_path) {
                Err(e) => return Err(Self::io_err(&global_agents_path, e)),
                Ok(raw)
                    if raw.as_str().contains(GLOBAL_AGENTS_START)
                        && raw.as_str().contains(GLOBAL_AGENTS_END) =>
                {
                    InstallVerifyCheck {
                        subject: CheckSubject::Harness(self.harness_key()),
                        name: InstallReportText::try_from(
                            "global-agents-md-block-present".to_owned(),
                        )?,
                        status: CheckStatus::Passed,
                        detail: InstallReportText::try_from(String::new())?,
                    }
                }
                Ok(_) => InstallVerifyCheck {
                    subject: CheckSubject::Harness(self.harness_key()),
                    name: InstallReportText::try_from("global-agents-md-block-present".to_owned())?,
                    status: CheckStatus::Failed,
                    detail: InstallReportText::try_from(format!(
                        "managed block markers missing from `{}`",
                        global_agents_path.display()
                    ))?,
                },
            }
        };
        checks.push(global_agents_check);

        let skill_exists = self.skill_md_path().is_file();
        let skill_check = InstallVerifyCheck {
            subject: CheckSubject::Harness(self.harness_key()),
            name: InstallReportText::try_from("user-skill-present".to_owned())?,
            status: if skill_exists {
                CheckStatus::Passed
            } else {
                CheckStatus::Failed
            },
            detail: InstallReportText::try_from(if skill_exists {
                String::new()
            } else {
                format!("missing user skill at `{}`", self.skill_md_path().display())
            })?,
        };
        checks.push(skill_check);

        Ok(InstallVerifyReport { checks })
    }
}

/// Plan the UNINSTALL direction for this adapter: remove the
/// `mcp_servers.<SERVER_NAME>` entry, the skill dir, the agent descriptor,
/// and the global `AGENTS.md` managed block. Kept as a free function
/// (rather than a second [`HarnessAdapter`] impl), mirroring
/// [`crate::adapters::claude::ClaudeAdapter::plan_uninstall`].
impl CodexAdapter {
    /// Compute the uninstall plan: every artifact this adapter would
    /// remove, if present.
    ///
    /// # Errors
    /// Returns [`InstallError::MalformedConfig`] if `config.toml` exists
    /// but is not valid TOML.
    pub fn plan_uninstall(&self, ctx: &InstallRequestContext) -> InstallResult<InstallReport> {
        self.check_ctx_consistency(ctx)?;
        let existing = self.read_config_toml()?;
        let mut planned_changes = Vec::new();

        let has_entry = existing
            .get("mcp_servers")
            .and_then(Item::as_table)
            .and_then(|servers| servers.get(SERVER_NAME))
            .is_some();
        if has_entry {
            planned_changes.push(PlannedInstallChange {
                harness: self.harness_key(),
                kind: ArtifactKind::McpRegistration,
                path: Self::repo_root(&self.config_toml_path())?,
                description: InstallReportText::try_from(format!(
                    "remove mcp_servers.{SERVER_NAME} from <codex_home>/config.toml"
                ))?,
                disposition: ChangeDisposition::Update,
            });
        }

        if self.skill_md_path().is_file() {
            planned_changes.push(PlannedInstallChange {
                harness: self.harness_key(),
                kind: ArtifactKind::HarnessSpecific,
                path: Self::repo_root(&self.skill_md_path())?,
                description: InstallReportText::try_from(format!(
                    "remove {SERVER_NAME} skill under <codex_home>/skills"
                ))?,
                disposition: ChangeDisposition::Update,
            });
        }

        if self.agent_descriptor_path().is_file() {
            planned_changes.push(PlannedInstallChange {
                harness: self.harness_key(),
                kind: ArtifactKind::HarnessSpecific,
                path: Self::repo_root(&self.agent_descriptor_path())?,
                description: InstallReportText::try_from(
                    "remove agents/openai.yaml agent descriptor".to_owned(),
                )?,
                disposition: ChangeDisposition::Update,
            });
        }

        if self.global_agents_md_path().is_file() {
            let raw = std::fs::read_to_string(self.global_agents_md_path())
                .map_err(|e| Self::io_err(&self.global_agents_md_path(), e))?;
            if raw.as_str().contains(GLOBAL_AGENTS_START) {
                planned_changes.push(PlannedInstallChange {
                    harness: self.harness_key(),
                    kind: ArtifactKind::DoctrineReference,
                    path: Self::repo_root(&self.global_agents_md_path())?,
                    description: InstallReportText::try_from(
                        "remove global AGENTS.md ocentra-enforcer managed block".to_owned(),
                    )?,
                    disposition: ChangeDisposition::Update,
                });
            }
        }

        Ok(InstallReport {
            planned_changes,
            warnings: vec![],
        })
    }

    /// Apply a previously computed [`Self::plan_uninstall`] report: delete
    /// each targeted file, restoring `config.toml` to a merge with the
    /// enforcer entry removed (preserving every unrelated table), and
    /// stripping the managed block out of `AGENTS.md` (preserving
    /// surrounding content).
    ///
    /// # Errors
    /// Returns [`InstallError::Io`] if a removal fails.
    pub fn apply_uninstall(&self, report: &InstallReport) -> InstallResult<ApplyResult> {
        let mut applied = Vec::with_capacity(report.planned_changes.len());
        for change in &report.planned_changes {
            let target = PathBuf::from(change.path.as_str());
            let backup_path = backup_before_write(&target)?;

            match change.kind {
                ArtifactKind::McpRegistration => {
                    let mut doc = self.read_config_toml()?;
                    Self::remove_mcp_server(&mut doc);
                    self.write_config_toml(&doc)?;
                }
                ArtifactKind::DoctrineReference => {
                    let existing = if target.is_file() {
                        std::fs::read_to_string(&target).map_err(|e| Self::io_err(&target, e))?
                    } else {
                        String::new()
                    };
                    let stripped = remove_global_agents_block(&existing);
                    std::fs::write(&target, stripped).map_err(|e| Self::io_err(&target, e))?;
                }
                _ => {
                    if target.is_file() {
                        std::fs::remove_file(&target).map_err(|e| Self::io_err(&target, e))?;
                    }
                }
            }

            applied.push(AppliedInstallChange {
                change: change.clone(),
                status: CheckStatus::Passed,
                backup_path: backup_path.map(|p| Self::repo_root(&p)).transpose()?,
            });
        }
        Ok(ApplyResult { applied })
    }
}

/// Strip the transitional `<!-- ocentra-enforcer:start/end -->` managed
/// block (if present) out of `text`, preserving surrounding content —
/// the uninstall-direction mirror of
/// [`CodexAdapter::upsert_global_agents_block`].
fn remove_global_agents_block(text: &str) -> String {
    let Some(start) = text.find(GLOBAL_AGENTS_START) else {
        return text.to_owned();
    };
    let Some(from_start) = text.get(start..) else {
        return text.to_owned();
    };
    let Some(end) = from_start.find(GLOBAL_AGENTS_END) else {
        return text.to_owned();
    };
    let end_abs = start + end + GLOBAL_AGENTS_END.len();
    let Some(before) = text.get(..start) else {
        return text.to_owned();
    };
    let Some(after) = text.get(end_abs..) else {
        return text.to_owned();
    };
    let after = after.trim_start_matches('\n');
    format!("{before}{after}")
}

#[cfg(test)]
mod tests {
    use super::{
        remove_global_agents_block, CodexAdapter, InstallError, GLOBAL_AGENTS_END,
        GLOBAL_AGENTS_START, SERVER_NAME,
    };
    use crate::core::HarnessAdapter;
    use enforcer_domain::install_types::InstallRequestContext;
    use std::fs;
    use std::path::Path;
    use toml_edit::DocumentMut;

    fn ctx(
        binary: &Path,
    ) -> Result<InstallRequestContext, enforcer_domain::boundary::decode_error::DecodeError> {
        InstallRequestContext::try_with_defaults(binary.to_path_buf())
    }

    fn fixture_home() -> Result<tempfile::TempDir, std::io::Error> {
        tempfile::tempdir()
    }

    #[test]
    fn plan_on_fresh_home_proposes_every_artifact() -> Result<(), Box<dyn std::error::Error>> {
        let home = fixture_home()?;
        let binary = home.path().join("bin").join("enforcer");
        let adapter = CodexAdapter::try_new(home.path().to_path_buf(), binary.clone())?;
        let report = adapter.plan(&ctx(&binary)?)?;
        assert_eq!(report.planned_changes.len(), 4);
        assert_eq!(report.planned_changes.len(), 4);
        Ok(())
    }

    #[test]
    fn full_install_apply_then_verify_all_green() -> Result<(), Box<dyn std::error::Error>> {
        let home = fixture_home()?;
        let binary = home.path().join("bin").join("enforcer");
        fs::create_dir_all(binary.parent().ok_or("expected a parent dir")?)?;
        fs::write(&binary, b"installed-enforcer")?;
        let adapter = CodexAdapter::try_new(home.path().to_path_buf(), binary.clone())?;

        let plan = adapter.plan(&ctx(&binary)?)?;
        assert_eq!(plan.planned_changes.len(), 4);
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
            "expected all checks to pass, got {verify:?}"
        );

        // Idempotent re-install: second plan is a no-op.
        let second_plan = adapter.plan(&ctx(&binary)?)?;
        assert!(second_plan.planned_changes.is_empty());
        Ok(())
    }

    #[test]
    fn command_points_at_absolute_binary_no_node_shim() -> Result<(), Box<dyn std::error::Error>> {
        let home = fixture_home()?;
        let binary = home.path().join("bin").join("enforcer");
        fs::create_dir_all(binary.parent().ok_or("expected a parent dir")?)?;
        fs::write(&binary, b"installed-enforcer")?;
        let adapter = CodexAdapter::try_new(home.path().to_path_buf(), binary.clone())?;
        let plan = adapter.plan(&ctx(&binary)?)?;
        adapter.apply(&plan)?;

        let raw = fs::read_to_string(adapter.config_toml_path())?;
        assert!(
            raw.as_str()
                .contains(&binary.display().to_string().replace('\\', "\\\\"))
                || raw.as_str().contains(&binary.display().to_string())
        );
        assert!(!raw.as_str().contains("\"node\""));
        assert!(!raw.as_str().contains(".mjs"));
        Ok(())
    }

    #[test]
    fn install_then_uninstall_restores_pre_state_byte_for_byte(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let home = fixture_home()?;
        let binary = home.path().join("bin").join("enforcer");
        fs::create_dir_all(binary.parent().ok_or("expected a parent dir")?)?;
        fs::write(&binary, b"installed-enforcer")?;

        // Pre-existing unrelated state that must survive round-trip.
        let config_toml_path = home.path().join("config.toml");
        let pre_state = "[mcp_servers.other-tool]\ncommand = \"/usr/bin/other-tool\"\n\n[some_other_table]\nkeep = \"me\"\n";
        fs::write(&config_toml_path, pre_state)?;

        let adapter = CodexAdapter::try_new(home.path().to_path_buf(), binary.clone())?;
        let install_plan = adapter.plan(&ctx(&binary)?)?;
        adapter.apply(&install_plan)?;

        let mid = fs::read_to_string(&config_toml_path)?;
        assert!(mid.as_str().contains("other-tool"));
        assert!(mid.as_str().contains("keep = \"me\""));
        assert!(mid
            .as_str()
            .contains(&format!("[mcp_servers.{SERVER_NAME}]")));

        let uninstall_plan = adapter.plan_uninstall(&ctx(&binary)?)?;
        assert_eq!(uninstall_plan.planned_changes.len(), 4);
        adapter.apply_uninstall(&uninstall_plan)?;

        let post = fs::read_to_string(&config_toml_path)?;
        assert!(post.as_str().contains("other-tool"));
        assert!(post.as_str().contains("keep = \"me\""));
        assert!(!post
            .as_str()
            .contains(&format!("[mcp_servers.{SERVER_NAME}]")));

        assert!(!adapter.skill_md_path().is_file());
        assert!(!adapter.agent_descriptor_path().is_file());
        Ok(())
    }

    #[test]
    fn merge_preserves_unrelated_mcp_servers_and_tables() -> Result<(), Box<dyn std::error::Error>>
    {
        let mut doc: DocumentMut =
            "[mcp_servers.other-tool]\ncommand = \"/usr/bin/other-tool\"\n\n[misc]\nx = 42\n"
                .parse()?;
        let desired = CodexAdapter::desired_table(Path::new("/abs/enforcer"));
        CodexAdapter::merge_mcp_server(&mut doc, desired);

        assert_eq!(doc["misc"]["x"].as_integer(), Some(42));
        assert_eq!(
            doc["mcp_servers"]["other-tool"]["command"].as_str(),
            Some("/usr/bin/other-tool")
        );
        assert!(doc["mcp_servers"][SERVER_NAME].is_table());
        Ok(())
    }

    #[test]
    fn entry_matches_detects_idempotent_noop() {
        let desired = CodexAdapter::desired_table(Path::new("/abs/enforcer"));
        let mut doc = DocumentMut::new();
        CodexAdapter::merge_mcp_server(&mut doc, desired.clone());
        assert!(CodexAdapter::entry_matches(&doc, &desired));

        let other = CodexAdapter::desired_table(Path::new("/other/enforcer"));
        assert!(!CodexAdapter::entry_matches(&doc, &other));
    }

    #[test]
    fn malformed_config_toml_is_a_detected_plan_failure() -> Result<(), Box<dyn std::error::Error>>
    {
        let home = fixture_home()?;
        let binary = home.path().join("bin").join("enforcer");
        fs::write(home.path().join("config.toml"), "not [ valid toml")?;
        let adapter = CodexAdapter::try_new(home.path().to_path_buf(), binary.clone())?;
        let result = adapter.plan(&ctx(&binary)?);
        assert!(matches!(result, Err(InstallError::MalformedConfig { .. })));
        Ok(())
    }

    #[test]
    fn verify_fails_closed_on_malformed_config_toml() -> Result<(), Box<dyn std::error::Error>> {
        let home = fixture_home()?;
        let binary = home.path().join("bin").join("enforcer");
        fs::write(home.path().join("config.toml"), "not [ valid toml")?;
        let adapter = CodexAdapter::try_new(home.path().to_path_buf(), binary.clone())?;
        let result = adapter.verify(&ctx(&binary)?);
        assert!(matches!(result, Err(InstallError::MalformedConfig { .. })));
        Ok(())
    }

    #[test]
    fn verify_fails_when_descriptor_missing() -> Result<(), Box<dyn std::error::Error>> {
        let home = fixture_home()?;
        let binary = home.path().join("bin").join("enforcer");
        let mut doc = DocumentMut::new();
        CodexAdapter::merge_mcp_server(&mut doc, CodexAdapter::desired_table(&binary));
        fs::write(home.path().join("config.toml"), doc.to_string())?;
        let adapter = CodexAdapter::try_new(home.path().to_path_buf(), binary.clone())?;
        let report = adapter.verify(&ctx(&binary)?)?;
        assert!(!report.checks.iter().all(|check| matches!(
            check.status,
            enforcer_domain::install_types::CheckStatus::Passed
        )));
        let descriptor_check = report
            .checks
            .iter()
            .find(|check| check.name.as_str() == "agent-descriptor-present")
            .ok_or("expected an agent-descriptor-present check")?;
        assert!(!matches!(
            descriptor_check.status,
            enforcer_domain::install_types::CheckStatus::Passed
        ));
        Ok(())
    }

    #[test]
    fn verify_fails_when_descriptor_corrupt() -> Result<(), Box<dyn std::error::Error>> {
        let home = fixture_home()?;
        let binary = home.path().join("bin").join("enforcer");
        let mut doc = DocumentMut::new();
        CodexAdapter::merge_mcp_server(&mut doc, CodexAdapter::desired_table(&binary));
        fs::write(home.path().join("config.toml"), doc.to_string())?;
        let agents_dir = home.path().join("agents");
        fs::create_dir_all(&agents_dir)?;
        fs::write(
            agents_dir.join("openai.yaml"),
            "not a valid descriptor at all",
        )?;
        let adapter = CodexAdapter::try_new(home.path().to_path_buf(), binary.clone())?;
        let report = adapter.verify(&ctx(&binary)?)?;
        assert!(!report.checks.iter().all(|check| matches!(
            check.status,
            enforcer_domain::install_types::CheckStatus::Passed
        )));
        Ok(())
    }

    #[test]
    fn agent_descriptor_renders_valid_content() {
        let rendered = CodexAdapter::render_agent_descriptor();
        assert!(matches!(
            CodexAdapter::validate_agent_descriptor(&rendered),
            Ok(())
        ));
        assert!(rendered
            .as_str()
            .contains(&format!("display_name: {SERVER_NAME}")));
        assert!(rendered
            .as_str()
            .contains("allow_implicit_invocation: true"));
    }

    #[test]
    fn validate_agent_descriptor_rejects_missing_display_name() {
        let result = CodexAdapter::validate_agent_descriptor("allow_implicit_invocation: true\n");
        assert_eq!(result, Err("missing `display_name:` field".to_owned()));
    }

    #[test]
    fn validate_agent_descriptor_rejects_missing_implicit_invocation() {
        let result = CodexAdapter::validate_agent_descriptor("display_name: enforcer\n");
        assert_eq!(
            result,
            Err("missing `allow_implicit_invocation: true`".to_owned())
        );
    }

    #[test]
    fn ctx_binary_path_mismatch_is_a_detected_error() -> Result<(), Box<dyn std::error::Error>> {
        let home = fixture_home()?;
        let binary = home.path().join("bin").join("enforcer");
        let other_binary = home.path().join("bin").join("other-enforcer");
        let adapter = CodexAdapter::try_new(home.path().to_path_buf(), binary)?;
        let result = adapter.plan(&ctx(&other_binary)?);
        assert!(matches!(result, Err(InstallError::MalformedConfig { .. })));
        Ok(())
    }

    #[test]
    fn global_agents_block_markers_are_byte_for_byte_transitional() {
        assert_eq!(GLOBAL_AGENTS_START, "<!-- ocentra-enforcer:start -->");
        assert_eq!(GLOBAL_AGENTS_END, "<!-- ocentra-enforcer:end -->");
    }

    #[test]
    fn global_agents_block_upsert_preserves_surrounding_content(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let existing =
            "before\n<!-- ocentra-enforcer:start -->\nold\n<!-- ocentra-enforcer:end -->\nafter\n";
        let out = CodexAdapter::upsert_global_agents_block(existing, "AGENTS.md")?;
        assert!(out.as_str().starts_with("before\n"));
        assert!(out.as_str().contains("after"));
        assert!(!out.as_str().contains("old"));
        assert!(out.as_str().contains(SERVER_NAME));
        Ok(())
    }

    #[test]
    fn global_agents_block_upsert_appends_when_absent() -> Result<(), Box<dyn std::error::Error>> {
        let out = CodexAdapter::upsert_global_agents_block("# My AGENTS\n", "AGENTS.md")?;
        assert!(out.as_str().starts_with("# My AGENTS\n"));
        assert!(out.as_str().contains(GLOBAL_AGENTS_START));
        assert!(out.as_str().contains(GLOBAL_AGENTS_END));
        Ok(())
    }

    #[test]
    fn global_agents_block_upsert_detects_malformed_duplicate_markers(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let existing = "<!-- ocentra-enforcer:start -->\na\n<!-- ocentra-enforcer:start -->\nb\n<!-- ocentra-enforcer:end -->\n";
        let result = CodexAdapter::upsert_global_agents_block(existing, "AGENTS.md");
        let error = result
            .err()
            .ok_or("duplicate managed markers must be rejected")?;
        assert_eq!(
            error,
            InstallError::ManagedBlockInvalid {
                path: "AGENTS.md".to_owned(),
                marker: "ocentra-enforcer".to_owned(),
                reason: "expected exactly one begin/end marker pair, found 2 begin and 1 end"
                    .to_owned(),
            }
        );
        Ok(())
    }

    #[test]
    fn remove_global_agents_block_strips_only_the_marked_section() {
        let existing =
            "before\n<!-- ocentra-enforcer:start -->\nold\n<!-- ocentra-enforcer:end -->\nafter\n";
        let out = remove_global_agents_block(existing);
        assert!(out.as_str().contains("before"));
        assert!(out.as_str().contains("after"));
        assert!(!out.as_str().contains("old"));
        assert!(!out.as_str().contains(GLOBAL_AGENTS_START));
    }
}
