//! x03 — the one-time TRANSITIONAL migration that upgrades already-installed
//! `ocentra-enforcer` harness registrations to the neutral `enforcer` name
//! (x01's shipped rename target).
//!
//! # Charter
//!
//! x01 renamed the SHIPPED name-source surfaces (crate/binary/MCP
//! server-name consts) from `ocentra-enforcer` to `enforcer`. Any machine
//! that installed the enforcer BEFORE that rename still carries a legacy
//! `ocentra-enforcer` MCP server registration in its harness configs (the
//! Claude `~/.claude.json` `mcpServers` map, the Codex `config.toml`
//! `[mcp_servers]` table, the transitional Codex `AGENTS.md`
//! `<!-- ocentra-enforcer:start/end -->` managed block, and possibly other
//! adapters' own surfaces) plus legacy `rust_rules_*`/`ocentra_enforcer_*`
//! tool-name string literals inside those same files (e.g. an `AGENTS.md`
//! doctrine block naming a tool by its old alias). Doctrine forbids
//! lingering `ocentra`: this module UPGRADES those already-installed
//! entries in place rather than keeping the old name alive as a permanent
//! alias.
//!
//! This module is invoked by the installer's `doctor`/`migrate` path (arc-22
//! CLI wiring, out of this workpack's `owns:` scope) — it is a plain Rust
//! function/type on that path, never a standalone binary or a `.ts`/Node
//! script.
//!
//! # What this module does NOT do
//!
//! It does not touch [`enforcer_mcp::registry::CANONICAL_TOOLS`] or
//! [`enforcer_mcp::aliases`] — the LIVE MCP server's own tool-name family
//! (still `ocentra_enforcer_*` canonical + `rust_rules_*` alias, per x01's
//! deviation note: that rename is a distinct, not-yet-landed cutover this
//! workpack does not own). This module only rewrites ALREADY-INSTALLED
//! on-disk harness config files so they name the MCP SERVER `enforcer`
//! (`mcp__enforcer__*` at the tool-invocation surface) instead of the old
//! `ocentra-enforcer` server key, and cleans up stray legacy tool-name
//! string literals embedded in prose/managed blocks inside those same
//! files.
//!
//! # Idempotence + one-time notice
//!
//! A [`ConfigTarget`] with no legacy entry left is a no-op: [`migrate`]
//! reports zero [`MigrationFindingDto`]s and writes zero bytes on a second run. The
//! single one-time notice ([`MigrationOutcomeDto::notice`]) is emitted exactly
//! once per [`migrate`] call that performs at least one rewrite — never
//! once per file, and never repeated on a subsequent idempotent no-op run.
//!
//! BOUNDARY-INVARIANT: migration targets and serialized config documents are
//! parsed into typed harness ids, absolute install paths, and format-specific
//! JSON/TOML values before rewrite logic runs. Invalid target descriptors or
//! malformed documents are rejected without partially rewriting the source.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use toml_edit::DocumentMut;

use crate::backup::BACKUP_SUFFIX;
use crate::error::{InstallError, InstallResult};
use enforcer_domain::ids::HarnessId;
use enforcer_domain::install_types::{
    ConfigFormat, FindingKind, InstallReportText, InstallTargetPath, MigrationFinding,
    MigrationOutcome, RewrittenFile,
};
use enforcer_mcp::name::SERVER_NAME;

/// The legacy product-identity MCP server registration key this migration
/// retires. Distinct from [`SERVER_NAME`] (the x01-owned, already-neutral
/// target this migration rewrites TO).
pub const LEGACY_SERVER_NAME: &str = "ocentra-enforcer";

/// Legacy canonical tool-name-family prefix this migration scrubs from
/// config prose (managed blocks, doctrine references). Mirrors
/// `enforcer_mcp::aliases::CANONICAL_TOOL_PREFIX`'s historical value, kept
/// as a local literal here since this module has no dependency edge on
/// that internal, still-live tool-registry family.
pub const LEGACY_CANONICAL_TOOL_PREFIX: &str = "ocentra_enforcer_";

/// Legacy compatibility alias tool-name-family prefix this migration
/// scrubs from config prose. Mirrors
/// `enforcer_mcp::aliases::LEGACY_ALIAS_PREFIX`'s historical value.
pub const LEGACY_ALIAS_TOOL_PREFIX: &str = "rust_rules_";

/// The neutral tool-name-family prefix every rewritten reference becomes.
/// Derived from [`SERVER_NAME`] so a future server-name change cannot
/// silently desync this migration's rewrite target.
fn neutral_tool_prefix() -> String {
    format!("mcp__{SERVER_NAME}__")
}

// ---------------------------------------------------------------------
// Config target descriptors — the file surfaces this migration scans
// ---------------------------------------------------------------------

/// Which serialization format a [`ConfigTarget`] file uses, so [`migrate`]
/// knows how to parse/rewrite it. Reuses the same `serde_json`/`toml_edit`
/// value-merge machinery the c03/c06 adapters use for their OWN installs —
/// this migration does not reimplement config parsing.
/// One harness config file this migration scans + (if needed) rewrites.
/// A caller (arc-22's `doctor`/`migrate` CLI wiring) builds this list from
/// the SAME per-adapter config-locate logic c02/c03/c06 already use (this
/// module does not invent a bespoke harness-discovery scanner) — each
/// `ClaudeAdapter`/`CodexAdapter`/... instance already knows its own
/// config paths (`claude_json_path()`, `config_toml_path()`,
/// `global_agents_md_path()`, ...).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigTarget {
    /// Which harness this file belongs to (matches
    /// [`crate::report::HarnessKey`] values, e.g. `"claude"`, `"codex"`).
    pub harness: HarnessId,
    /// Absolute path to the config file.
    pub path: InstallTargetPath,
    /// How to parse/rewrite this file.
    pub format: ConfigFormat,
}

impl ConfigTarget {
    /// Build a target descriptor.
    pub fn try_new(harness: String, path: PathBuf, format: ConfigFormat) -> InstallResult<Self> {
        Ok(Self {
            harness: HarnessId::try_from(harness)?,
            path: InstallTargetPath::try_from(path)?,
            format,
        })
    }
}

// ---------------------------------------------------------------------
// Structured report types — c01/arc-23 MigrationFindingDto/ApplyResultDto shape
// ---------------------------------------------------------------------

/// What kind of legacy residue a [`MigrationFindingDto`] describes.
/// One concrete piece of legacy residue [`scan`] detected, or [`migrate`]
/// rewrote. Every [`migrate`] call reports these instead of a bare
/// `println` — c01/arc-23's typed report shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationFindingDto {
    /// Which harness this finding belongs to (`""` for the repo-level
    /// legacy-skill-dir finding, which is not harness-scoped).
    pub harness: String,
    /// Absolute path of the file (or directory) this finding is about.
    pub path: String,
    /// What kind of legacy residue this is.
    #[serde(with = "crate::boundary::install_type_wire::finding_kind")]
    pub kind: FindingKind,
    /// Human-readable detail (e.g. the exact legacy key/literal found).
    pub detail: String,
}

/// One config file this migration rewrote.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RewrittenFileDto {
    /// The file that was rewritten.
    pub path: String,
    /// Path to the timestamped backup written before the rewrite.
    pub backup_path: String,
}

/// The full structured result [`migrate`] returns — the c01/arc-23
/// `Report`/`ApplyResultDto` shape this workpack's requirement checklist
/// names, not a `println`-based script output.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationOutcomeDto {
    /// Every legacy residue this run found (before rewriting).
    pub findings: Vec<MigrationFindingDto>,
    /// Every file this run actually rewrote.
    pub rewritten: Vec<RewrittenFileDto>,
    /// The legacy skill dir path, if this run retired it.
    pub retired_skill_dir: Option<String>,
    /// The single one-time migration notice, present only when this run
    /// performed at least one rewrite (file rewrite or skill-dir
    /// retirement) — never emitted on an idempotent no-op run, and never
    /// emitted more than once per [`migrate`] call regardless of how many
    /// files it touched.
    pub notice: Option<String>,
}

impl From<MigrationFinding> for MigrationFindingDto {
    fn from(value: MigrationFinding) -> Self {
        Self {
            harness: value
                .harness
                .map_or_else(String::new, |harness| harness.as_str().to_owned()),
            path: value.path.display().to_string(),
            kind: value.kind,
            detail: value.detail.as_str().to_owned(),
        }
    }
}

impl TryFrom<MigrationFindingDto> for MigrationFinding {
    type Error = InstallError;

    fn try_from(value: MigrationFindingDto) -> InstallResult<Self> {
        Ok(Self {
            harness: (!value.harness.is_empty())
                .then(|| HarnessId::try_from(value.harness))
                .transpose()?,
            path: InstallTargetPath::try_from(PathBuf::from(value.path))?,
            kind: value.kind,
            detail: InstallReportText::try_from(value.detail)?,
        })
    }
}

impl From<RewrittenFile> for RewrittenFileDto {
    fn from(value: RewrittenFile) -> Self {
        Self {
            path: value.path.display().to_string(),
            backup_path: value.backup_path.display().to_string(),
        }
    }
}

impl TryFrom<RewrittenFileDto> for RewrittenFile {
    type Error = InstallError;

    fn try_from(value: RewrittenFileDto) -> InstallResult<Self> {
        Ok(Self {
            path: InstallTargetPath::try_from(PathBuf::from(value.path))?,
            backup_path: InstallTargetPath::try_from(PathBuf::from(value.backup_path))?,
        })
    }
}

impl From<MigrationOutcome> for MigrationOutcomeDto {
    fn from(value: MigrationOutcome) -> Self {
        Self {
            findings: value
                .findings
                .into_iter()
                .map(MigrationFindingDto::from)
                .collect(),
            rewritten: value
                .rewritten
                .into_iter()
                .map(RewrittenFileDto::from)
                .collect(),
            retired_skill_dir: value
                .retired_skill_dir
                .map(|path| path.display().to_string()),
            notice: value.notice.map(|notice| notice.as_str().to_owned()),
        }
    }
}

impl TryFrom<MigrationOutcomeDto> for MigrationOutcome {
    type Error = InstallError;

    fn try_from(value: MigrationOutcomeDto) -> InstallResult<Self> {
        Ok(Self {
            findings: value
                .findings
                .into_iter()
                .map(MigrationFinding::try_from)
                .collect::<Result<_, _>>()?,
            rewritten: value
                .rewritten
                .into_iter()
                .map(RewrittenFile::try_from)
                .collect::<Result<_, _>>()?,
            retired_skill_dir: value
                .retired_skill_dir
                .map(|path| InstallTargetPath::try_from(PathBuf::from(path)))
                .transpose()?,
            notice: value.notice.map(InstallReportText::try_from).transpose()?,
        })
    }
}

impl MigrationOutcomeDto {
    /// True when this run found (and thus rewrote) zero legacy residue —
    /// the idempotent-second-run signal a fixture asserts on.
    #[must_use]
    pub fn is_noop(&self) -> bool {
        self.findings.is_empty() && self.rewritten.is_empty() && self.retired_skill_dir.is_none()
    }
}

/// The fixed one-time notice text every migrating run with at least one
/// rewrite emits. A caller (arc-22 CLI) surfaces this once to the human;
/// it is never re-emitted on a subsequent idempotent no-op run.
const MIGRATION_NOTICE: &str = "one-time migration: legacy `ocentra-enforcer` MCP registration(s) \
and/or `rust_rules_*`/`ocentra_enforcer_*` tool-name references were rewritten to `enforcer` \
(mcp__enforcer__*). This is a transitional upgrade, not a permanent alias -- no `ocentra-enforcer` \
entry is retained.";

// ---------------------------------------------------------------------
// Scan (read-only detection, no filesystem writes)
// ---------------------------------------------------------------------

fn io_err(path: &Path, e: impl std::fmt::Display) -> InstallError {
    InstallError::Io {
        path: path.display().to_string(),
        reason: e.to_string(),
    }
}

/// Read-only detection: report every [`MigrationFindingDto`] across `targets` without
/// writing anything to disk. Used both as [`migrate`]'s own first phase and
/// standalone as the post-migration re-scan a fixture/doctor check drives
/// to assert zero residue.
///
/// # Errors
/// Returns [`InstallError::MalformedConfig`] if a target file exists but
/// fails to parse for its declared [`ConfigFormat`] — a malformed config is
/// a detected, typed error, never a silent skip.
pub fn scan(targets: &[ConfigTarget]) -> InstallResult<Vec<MigrationFindingDto>> {
    let mut findings = Vec::new();
    for target in targets {
        findings.extend(scan_one(target)?);
    }
    Ok(findings
        .into_iter()
        .map(MigrationFindingDto::from)
        .collect())
}

fn scan_one(target: &ConfigTarget) -> InstallResult<Vec<MigrationFinding>> {
    if !target.path.as_path().is_file() {
        return Ok(Vec::new());
    }
    let raw = std::fs::read_to_string(target.path.as_path())
        .map_err(|e| io_err(target.path.as_path(), e))?;

    match target.format {
        ConfigFormat::JsonMcpServers => scan_json(target, &raw),
        ConfigFormat::TomlMcpServers => scan_toml(target, &raw),
        ConfigFormat::ManagedText => scan_text_literals(target, &raw),
    }
}

fn migration_finding(
    target: &ConfigTarget,
    kind: FindingKind,
    detail: String,
) -> InstallResult<MigrationFinding> {
    Ok(MigrationFinding {
        harness: Some(target.harness.clone()),
        path: target.path.clone(),
        kind,
        detail: InstallReportText::try_from(detail)?,
    })
}

fn migration_outcome_is_noop(outcome: &MigrationOutcome) -> bool {
    outcome.findings.is_empty()
        && outcome.rewritten.is_empty()
        && outcome.retired_skill_dir.is_none()
}

/// Detect the STRUCTURED `mcpServers.<LEGACY_SERVER_NAME>` key only — a
/// field VALUE that happens to contain the substring `ocentra-enforcer`
/// (e.g. a `command` path pointing at the old binary name) is data, not a
/// name-surface finding; [`rewrite_json`] mirrors this exact scope so scan
/// and rewrite never disagree about what counts as legacy residue.
fn scan_json(target: &ConfigTarget, raw: &str) -> InstallResult<Vec<MigrationFinding>> {
    let value: JsonValue =
        serde_json::from_str(raw).map_err(|e| InstallError::MalformedConfig {
            path: target.path.display().to_string(),
            reason: format!("not valid JSON: {e}"),
        })?;
    let mut findings = Vec::new();
    let servers = value.get("mcpServers").and_then(JsonValue::as_object);
    if servers.is_some_and(|servers| servers.contains_key(LEGACY_SERVER_NAME)) {
        let conflicts_with_neutral =
            servers.is_some_and(|servers| servers.contains_key(SERVER_NAME));
        findings.push(migration_finding(
            target,
            if conflicts_with_neutral {
                FindingKind::ConflictingServerRegistration
            } else {
                FindingKind::LegacyServerRegistration
            },
            if conflicts_with_neutral {
                format!("mcpServers contains both {LEGACY_SERVER_NAME} and {SERVER_NAME}")
            } else {
                format!("mcpServers.{LEGACY_SERVER_NAME}")
            },
        )?);
    }
    Ok(findings)
}

/// Detect the STRUCTURED `mcp_servers.<LEGACY_SERVER_NAME>` table key
/// only — same key-only scope as [`scan_json`], mirroring [`rewrite_toml`].
fn scan_toml(target: &ConfigTarget, raw: &str) -> InstallResult<Vec<MigrationFinding>> {
    let doc: DocumentMut = raw.parse().map_err(|e| InstallError::MalformedConfig {
        path: target.path.display().to_string(),
        reason: format!("not valid TOML: {e}"),
    })?;
    let mut findings = Vec::new();
    let servers = doc.get("mcp_servers").and_then(toml_edit::Item::as_table);
    if servers.is_some_and(|servers| servers.contains_key(LEGACY_SERVER_NAME)) {
        let conflicts_with_neutral =
            servers.is_some_and(|servers| servers.contains_key(SERVER_NAME));
        findings.push(migration_finding(
            target,
            if conflicts_with_neutral {
                FindingKind::ConflictingServerRegistration
            } else {
                FindingKind::LegacyServerRegistration
            },
            if conflicts_with_neutral {
                format!("mcp_servers contains both {LEGACY_SERVER_NAME} and {SERVER_NAME}")
            } else {
                format!("mcp_servers.{LEGACY_SERVER_NAME}")
            },
        )?);
    }
    Ok(findings)
}

/// Detect legacy `rust_rules_*`/`ocentra_enforcer_*` tool-name literals and
/// bare `ocentra-enforcer` server-name mentions embedded anywhere in
/// `raw` prose (managed blocks, doctrine references) — a plain substring
/// scan, safe ONLY for [`ConfigFormat::ManagedText`] (a free-text file has
/// no field-value/name-surface distinction to preserve, unlike structured
/// JSON/TOML).
fn scan_text_literals(target: &ConfigTarget, raw: &str) -> InstallResult<Vec<MigrationFinding>> {
    let mut findings = Vec::new();
    if raw.contains(LEGACY_CANONICAL_TOOL_PREFIX) {
        findings.push(migration_finding(
            target,
            FindingKind::LegacyToolNameLiteral,
            format!("contains `{LEGACY_CANONICAL_TOOL_PREFIX}*` tool-name literal"),
        )?);
    }
    if raw.contains(LEGACY_ALIAS_TOOL_PREFIX) {
        findings.push(migration_finding(
            target,
            FindingKind::LegacyToolNameLiteral,
            format!("contains `{LEGACY_ALIAS_TOOL_PREFIX}*` tool-name literal"),
        )?);
    }
    if target.format == ConfigFormat::ManagedText && raw.contains(LEGACY_SERVER_NAME) {
        findings.push(migration_finding(
            target,
            FindingKind::LegacyServerRegistration,
            format!("prose reference to `{LEGACY_SERVER_NAME}`"),
        )?);
    }
    Ok(findings)
}

// ---------------------------------------------------------------------
// Migrate (detect + rewrite + report)
// ---------------------------------------------------------------------

/// Run the one-time migration over `targets` plus the legacy skill dir at
/// `legacy_skill_dir` (typically `skills/rust-rules-hard-gate` under the
/// repo/install root a caller resolves): detect every [`MigrationFindingDto`], rewrite
/// each affected config file in place (JSON value-merge / TOML upsert,
/// preserving every unrelated key, exactly like the c03/c06 adapters'
/// own install-direction merges), retire the legacy skill dir, and return
/// a [`MigrationOutcomeDto`] carrying the full typed diff. A second call with
/// the same `targets`/`legacy_skill_dir` after a successful migration is a
/// no-op ([`MigrationOutcomeDto::is_noop`] `true`) that writes zero bytes.
///
/// # Errors
/// Returns [`InstallError::MalformedConfig`] if any target fails to parse
/// for its declared format — fail-closed, never a silent skip.
/// Returns [`InstallError::BackupFailed`]/[`InstallError::Io`] if a backup
/// or rewrite fails.
pub fn migrate(
    targets: &[ConfigTarget],
    legacy_skill_dir: Option<&Path>,
) -> InstallResult<MigrationOutcomeDto> {
    let mut outcome = MigrationOutcome::default();

    let mut planned_rewrites = Vec::new();
    for target in targets {
        let before = scan_one(target)?;
        if before.is_empty() {
            continue;
        }
        if before
            .iter()
            .any(|finding| finding.kind == FindingKind::ConflictingServerRegistration)
        {
            // ALLOC-JUSTIFICATION: the typed preflight failure must retain
            // both the target path and collision explanation after scanning
            // has released its borrowed config data.
            return Err(InstallError::MalformedConfig {
                path: target.path.display().to_string(),
                reason: format!(
                    "contains both `{LEGACY_SERVER_NAME}` and `{SERVER_NAME}` MCP registrations; \
                     migration refuses to overwrite the existing neutral entry"
                ),
            });
        }
        planned_rewrites.push((target, before));
    }

    for (target, before) in planned_rewrites {
        outcome.findings.extend(before);
        let backup_path = timestamped_backup(target.path.as_path())?;
        rewrite_target(target)?;
        outcome.rewritten.push(RewrittenFile {
            path: target.path.clone(),
            backup_path: InstallTargetPath::try_from(backup_path)?,
        });
    }

    if let Some(dir) = legacy_skill_dir {
        if dir.exists() {
            let retired_skill_dir = InstallTargetPath::try_from(dir.to_path_buf())?;
            outcome.findings.push(MigrationFinding {
                harness: None,
                path: retired_skill_dir.clone(),
                kind: FindingKind::LegacySkillDirPresent,
                detail: InstallReportText::try_from(
                    "legacy skills/rust-rules-hard-gate dir present".to_owned(),
                )?,
            });
            std::fs::remove_dir_all(dir).map_err(|e| io_err(dir, e))?;
            outcome.retired_skill_dir = Some(retired_skill_dir);
        }
    }

    if !migration_outcome_is_noop(&outcome) {
        outcome.notice = Some(InstallReportText::try_from(MIGRATION_NOTICE.to_owned())?);
    }

    Ok(outcome.into())
}

/// Copy `path` to a TIMESTAMPED backup (`{path}.enforcer-bak.<unix-nanos>`)
/// before this migration rewrites it in place — distinct from
/// [`crate::backup::backup_before_write`]'s fixed-suffix, self-overwriting
/// convention (correct for a routine re-install) because a one-time
/// migration's backup is a permanent historical record of the
/// pre-migration state, not a "last attempt" scratch copy.
///
/// # Errors
/// Returns [`InstallError::BackupFailed`] if the copy fails.
fn timestamped_backup(path: &Path) -> InstallResult<PathBuf> {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| InstallError::BackupFailed {
            path: path.display().to_string(),
            reason: e.to_string(),
        })?
        .as_nanos();
    let mut backup_name = path.as_os_str().to_owned();
    backup_name.push(format!("{BACKUP_SUFFIX}.{stamp}"));
    let backup = PathBuf::from(backup_name);
    std::fs::copy(path, &backup).map_err(|e| InstallError::BackupFailed {
        path: path.display().to_string(),
        reason: e.to_string(),
    })?;
    Ok(backup)
}

fn rewrite_target(target: &ConfigTarget) -> InstallResult<()> {
    let raw = std::fs::read_to_string(target.path.as_path())
        .map_err(|e| io_err(target.path.as_path(), e))?;
    let rewritten = match target.format {
        ConfigFormat::JsonMcpServers => rewrite_json(target, &raw)?,
        ConfigFormat::TomlMcpServers => rewrite_toml(target, &raw)?,
        ConfigFormat::ManagedText => rewrite_text(&raw),
    };
    std::fs::write(target.path.as_path(), rewritten).map_err(|e| io_err(target.path.as_path(), e))
}

/// Rewrite the STRUCTURED `mcpServers.<LEGACY_SERVER_NAME>` key to
/// [`SERVER_NAME`], preserving every unrelated key/entry AND the entry's
/// own field values byte-for-byte (a `command` value that happens to
/// contain the substring `ocentra-enforcer` — e.g. an old binary path — is
/// data, not a name-surface to rewrite; only the REGISTRATION KEY is a
/// name surface here).
fn rewrite_json(target: &ConfigTarget, raw: &str) -> InstallResult<String> {
    let mut value: JsonValue =
        serde_json::from_str(raw).map_err(|e| InstallError::MalformedConfig {
            path: target.path.display().to_string(),
            reason: format!("not valid JSON: {e}"),
        })?;
    if let Some(servers) = value
        .get_mut("mcpServers")
        .and_then(JsonValue::as_object_mut)
    {
        if let Some(entry) = servers.remove(LEGACY_SERVER_NAME) {
            servers.insert(SERVER_NAME.to_owned(), entry);
        }
    }
    serde_json::to_string_pretty(&value).map_err(|e| InstallError::Io {
        path: target.path.display().to_string(),
        reason: e.to_string(),
    })
}

/// Rewrite the STRUCTURED `mcp_servers.<LEGACY_SERVER_NAME>` table key to
/// [`SERVER_NAME`], preserving every unrelated table/entry AND the entry's
/// own field values byte-for-byte — same key-only rewrite discipline as
/// [`rewrite_json`].
fn rewrite_toml(target: &ConfigTarget, raw: &str) -> InstallResult<String> {
    let mut doc: DocumentMut = raw.parse().map_err(|e| InstallError::MalformedConfig {
        path: target.path.display().to_string(),
        reason: format!("not valid TOML: {e}"),
    })?;
    if let Some(servers) = doc
        .get_mut("mcp_servers")
        .and_then(toml_edit::Item::as_table_mut)
    {
        if let Some(entry) = servers.remove(LEGACY_SERVER_NAME) {
            servers.insert(SERVER_NAME, entry);
        }
    }
    Ok(doc.to_string())
}

/// Rewrite every legacy tool-name/server-name literal in a
/// [`ConfigFormat::ManagedText`] file (prose managed blocks/doctrine
/// references, e.g. `AGENTS.md`) to the neutral `enforcer` form: bare
/// `ocentra-enforcer` server-name mentions become [`SERVER_NAME`], and
/// both legacy tool-name-family prefixes become `mcp__<SERVER_NAME>__`
/// (the canonical MCP tool-invocation surface, per the workpack's binding
/// rewrite target). A global substring rewrite is safe ONLY for this
/// free-text format — [`rewrite_json`]/[`rewrite_toml`] deliberately do
/// NOT call this, since a structured config's field VALUES (e.g. a binary
/// `command` path) are data, not a name surface.
fn rewrite_text(raw: &str) -> String {
    raw.replace(LEGACY_CANONICAL_TOOL_PREFIX, &neutral_tool_prefix())
        .replace(LEGACY_ALIAS_TOOL_PREFIX, &neutral_tool_prefix())
        .replace(LEGACY_SERVER_NAME, SERVER_NAME)
}

#[cfg(test)]
mod tests {
    use super::{
        migrate, scan, ConfigFormat, ConfigTarget, FindingKind, MigrationFindingDto,
        MigrationOutcomeDto, RewrittenFileDto, LEGACY_ALIAS_TOOL_PREFIX,
        LEGACY_CANONICAL_TOOL_PREFIX, LEGACY_SERVER_NAME,
    };
    use crate::error::{InstallError, InstallResult};
    use enforcer_mcp::name::SERVER_NAME;
    use std::fs;

    fn copy_fixture(name: &str) -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let fixture_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/migrate_legacy_name")
            .join(name);
        copy_dir_recursive(&fixture_root, dir.path())?;
        Ok(dir)
    }

    fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
        fs::create_dir_all(dst)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let dst_path = dst.join(entry.file_name());
            if entry.file_type()?.is_dir() {
                copy_dir_recursive(&entry.path(), &dst_path)?;
            } else {
                fs::copy(entry.path(), &dst_path)?;
            }
        }
        Ok(())
    }

    fn claude_target(dir: &std::path::Path) -> InstallResult<ConfigTarget> {
        ConfigTarget::try_new(
            "claude".to_owned(),
            dir.join(".claude.json"),
            ConfigFormat::JsonMcpServers,
        )
    }

    fn codex_targets(dir: &std::path::Path) -> InstallResult<Vec<ConfigTarget>> {
        Ok(vec![
            ConfigTarget::try_new(
                "codex".to_owned(),
                dir.join("config.toml"),
                ConfigFormat::TomlMcpServers,
            )?,
            ConfigTarget::try_new(
                "codex".to_owned(),
                dir.join("AGENTS.md"),
                ConfigFormat::ManagedText,
            )?,
        ])
    }

    // -- fail fixture: migrate-legacy-config-present -----------------

    #[test]
    fn fail_fixture_legacy_config_present_is_detected_by_scan(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let dir = copy_fixture("claude_legacy_present")?;
        let target = claude_target(dir.path())?;
        let findings = scan(&[target])?;
        assert!(
            findings
                .iter()
                .any(|f| f.kind == FindingKind::LegacyServerRegistration),
            "expected the legacy ocentra-enforcer registration to be detected, got {findings:?}"
        );
        Ok(())
    }

    #[test]
    fn fail_fixture_unmigrated_config_still_contains_legacy_key_verbatim(
    ) -> Result<(), Box<dyn std::error::Error>> {
        // A re-scan of an UNMIGRATED fixture must still find the old entry
        // -- the workpack's named fail case (`migrate-legacy-config-present`).
        let dir = copy_fixture("claude_legacy_present")?;
        let raw = fs::read_to_string(dir.path().join(".claude.json"))?;
        assert!(raw.as_str().contains(LEGACY_SERVER_NAME));
        Ok(())
    }

    // -- pass fixture: migrate-legacy-config-rewritten ----------------

    #[test]
    fn pass_fixture_claude_migrate_rewrites_legacy_entry_to_enforcer(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let dir = copy_fixture("claude_legacy_present")?;
        let target = claude_target(dir.path())?;
        let outcome = migrate(std::slice::from_ref(&target), None)?;

        assert!(!outcome.is_noop());
        assert_eq!(outcome.rewritten.len(), 1);
        assert_eq!(outcome.notice.as_deref(), Some(super::MIGRATION_NOTICE));

        let raw = fs::read_to_string(target.path.as_path())?;
        assert!(
            !raw.as_str().contains(LEGACY_SERVER_NAME),
            "post-migration file must contain zero ocentra-enforcer entries, got: {raw}"
        );
        let value: serde_json::Value = serde_json::from_str(&raw)?;
        assert!(value["mcpServers"][SERVER_NAME].is_object());
        assert_eq!(
            value["mcpServers"][SERVER_NAME]["command"],
            "/usr/local/bin/enforcer"
        );
        // Unrelated entries/keys survive the rewrite untouched.
        assert_eq!(
            value["mcpServers"]["codebase-memory-mcp"]["command"],
            "/usr/local/bin/codebase-memory-mcp"
        );
        assert_eq!(value["someUnrelatedTopLevelKey"], "keep-me");

        // Post-migration re-scan finds zero ocentra-enforcer entries.
        let rescan = scan(&[target])?;
        assert!(
            rescan.is_empty(),
            "expected zero findings after migration, got {rescan:?}"
        );
        Ok(())
    }

    #[test]
    fn pass_fixture_codex_migrate_rewrites_toml_and_agents_md(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let dir = copy_fixture("codex_legacy_present")?;
        let targets = codex_targets(dir.path())?;
        let outcome = migrate(&targets, None)?;

        assert!(!outcome.is_noop());
        assert_eq!(outcome.rewritten.len(), 2);

        let toml_raw = fs::read_to_string(dir.path().join("config.toml"))?;
        assert!(!toml_raw.as_str().contains(LEGACY_SERVER_NAME));
        assert!(toml_raw
            .as_str()
            .contains(&format!("[mcp_servers.{SERVER_NAME}]")));
        assert!(toml_raw.as_str().contains("[mcp_servers.other-tool]"));
        assert!(toml_raw.as_str().contains("[misc]"));

        let agents_raw = fs::read_to_string(dir.path().join("AGENTS.md"))?;
        assert!(!agents_raw.as_str().contains(LEGACY_SERVER_NAME));
        assert!(!agents_raw.as_str().contains(LEGACY_CANONICAL_TOOL_PREFIX));
        assert!(!agents_raw.as_str().contains(LEGACY_ALIAS_TOOL_PREFIX));
        assert!(agents_raw
            .as_str()
            .contains(&format!("mcp__{SERVER_NAME}__check")));
        assert!(agents_raw
            .as_str()
            .contains(&format!("mcp__{SERVER_NAME}__run")));
        assert!(agents_raw.as_str().contains("# Some User Content"));
        assert!(agents_raw
            .as_str()
            .contains("# Other user content stays untouched"));

        let rescan = scan(&targets)?;
        assert!(rescan.is_empty(), "expected zero findings, got {rescan:?}");
        Ok(())
    }

    // -- detection test: rename-migration-contract --------------------

    #[test]
    fn rename_migration_contract() -> Result<(), Box<dyn std::error::Error>> {
        let dir = copy_fixture("claude_legacy_present")?;
        let target = claude_target(dir.path())?;
        let pre_bytes = fs::read(target.path.as_path())?;

        // Detection: legacy entry + legacy tool name present pre-migration.
        let pre_findings = scan(std::slice::from_ref(&target))?;
        assert_eq!(pre_findings.len(), 1);

        // First run: rewrite + single one-time notice.
        let first = migrate(std::slice::from_ref(&target), None)?;
        assert!(!first.is_noop());
        assert_eq!(first.notice.as_deref(), Some(super::MIGRATION_NOTICE));
        assert_eq!(first.rewritten.len(), 1);

        // Byte-for-byte-preserved timestamped backup of the PRE-migration
        // state.
        let backup_path = &first.rewritten[0].backup_path;
        let backup_bytes = fs::read(backup_path)?;
        assert_eq!(backup_bytes, pre_bytes);
        assert!(backup_path.as_str().contains(".enforcer-bak."));

        // Idempotent re-run: zero file changes, zero notice.
        let post_bytes = fs::read(target.path.as_path())?;
        let second = migrate(std::slice::from_ref(&target), None)?;
        assert!(second.is_noop());
        assert!(second.notice.is_none());
        assert!(second.rewritten.is_empty());
        let post_bytes_after_second_run = fs::read(target.path.as_path())?;
        assert_eq!(post_bytes, post_bytes_after_second_run);

        // Zero-lingering-ocentra post-scan.
        let final_scan = scan(&[target])?;
        assert!(final_scan.is_empty());
        Ok(())
    }

    #[test]
    fn malformed_config_yields_a_typed_error_not_a_silent_skip(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let dir = copy_fixture("malformed")?;
        let target = claude_target(dir.path())?;

        let scan_result = scan(std::slice::from_ref(&target));
        assert!(matches!(
            scan_result,
            Err(InstallError::MalformedConfig { .. })
        ));

        let migrate_result = migrate(&[target], None);
        assert!(matches!(
            migrate_result,
            Err(InstallError::MalformedConfig { .. })
        ));
        Ok(())
    }

    #[test]
    fn a_clean_config_with_no_legacy_entry_is_already_a_noop(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join(".claude.json");
        fs::write(
            &path,
            serde_json::to_string(&serde_json::json!({
                "mcpServers": { SERVER_NAME: { "command": "/abs/enforcer" } }
            }))?,
        )?;
        let target =
            ConfigTarget::try_new("claude".to_owned(), path, ConfigFormat::JsonMcpServers)?;
        let outcome = migrate(&[target], None)?;
        assert!(outcome.is_noop());
        assert!(outcome.notice.is_none());
        Ok(())
    }

    #[test]
    fn missing_config_file_is_a_noop_not_an_error() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let target = ConfigTarget::try_new(
            "claude".to_owned(),
            dir.path().join("does-not-exist.json"),
            ConfigFormat::JsonMcpServers,
        )?;
        let outcome = migrate(&[target], None)?;
        assert!(outcome.is_noop());
        Ok(())
    }

    // -- legacy-skill-retirement test: legacy-skill-retired -----------

    #[test]
    fn legacy_skill_retired() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let skill_dir = dir.path().join("skills").join("rust-rules-hard-gate");
        fs::create_dir_all(skill_dir.join("agents"))?;
        fs::write(skill_dir.join("SKILL.md"), "legacy skill body")?;
        fs::write(skill_dir.join("agents").join("openai.yaml"), "legacy: true")?;

        let outcome = migrate(&[], Some(&skill_dir))?;
        assert!(!outcome.is_noop());
        assert_eq!(
            outcome.retired_skill_dir.as_deref(),
            Some(skill_dir.display().to_string().as_str())
        );
        assert!(!skill_dir.exists());
        assert_eq!(outcome.notice.as_deref(), Some(super::MIGRATION_NOTICE));

        // Zero rust-rules-hard-gate residue post-retirement -- the
        // workpack's named acceptance bar.
        assert!(!skill_dir.exists());
        Ok(())
    }

    #[test]
    fn legacy_skill_dir_retirement_is_idempotent() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let skill_dir = dir.path().join("skills").join("rust-rules-hard-gate");
        fs::create_dir_all(&skill_dir)?;
        fs::write(skill_dir.join("SKILL.md"), "legacy skill body")?;

        migrate(&[], Some(&skill_dir))?;
        assert!(!skill_dir.exists());

        // Second run: dir already gone -- no-op, not an error.
        let second = migrate(&[], Some(&skill_dir))?;
        assert!(second.is_noop());
        Ok(())
    }

    #[test]
    /// Round-trip proof for the complete migration outcome transport shape.
    fn migration_outcome_round_trip_through_json() -> Result<(), Box<dyn std::error::Error>> {
        let finding = MigrationFindingDto {
            harness: "codex".to_owned(),
            path: "/home/user/.codex/config.toml".to_owned(),
            kind: FindingKind::LegacyServerRegistration,
            detail: "legacy MCP server key".to_owned(),
        };
        let rewritten_file = RewrittenFileDto {
            path: "/home/user/.codex/config.toml".to_owned(),
            backup_path: "/home/user/.codex/config.toml.bak".to_owned(),
        };
        let outcome = MigrationOutcomeDto {
            findings: vec![finding],
            rewritten: vec![rewritten_file],
            retired_skill_dir: None,
            notice: None,
        };
        let wire = serde_json::to_string(&outcome)?;
        let back: MigrationOutcomeDto = serde_json::from_str(&wire)?;
        assert_eq!(back, outcome);
        let round_trip_finding: &MigrationFindingDto = &back.findings[0];
        let round_trip_rewrite: &RewrittenFileDto = &back.rewritten[0];
        assert_eq!(round_trip_finding.harness, "codex");
        assert!(round_trip_rewrite.backup_path.ends_with(".bak"));
        Ok(())
    }
}
