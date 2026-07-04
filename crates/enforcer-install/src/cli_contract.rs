//! The CLI contract seam `enforcer-cli` (arc-22) consumes when it wires
//! `Command::Install` (today a bare stub in `enforcer_cli::cli::Command`,
//! see that crate's module docs) to this crate's [`crate::core`] verbs.
//!
//! This module owns the REQUEST shape only — no clap derive lives here
//! (clap grammar is arc-22's `cli.rs`; this crate stays clap-agnostic so it
//! can be driven by a future non-CLI caller, e.g. the UI, arc-24). arc-22
//! constructs [`InstallRequest`]/[`UpdateRequest`]/[`DoctorRequest`] from
//! its own parsed args and calls into [`crate::core`].
//!
//! # Binding contract (RUST_ARCHITECTURE.md, "Global-install scope contract")
//!
//! `--scope user|project`, **default `user`/global** — never silently
//! defaults to `project`. `--dry-run` performs the full `plan(ctx)` step
//! and renders the resulting [`crate::report::InstallReport`] without ever
//! calling `apply(report)` — a dry run is a strict subset of a real run,
//! not a separate code path that can drift from it. Non-TTY output is JSON
//! (the `--json`/pipe-detection responsibility lives in `enforcer-cli`;
//! this seam only carries the flag that selects the behavior).

use std::path::PathBuf;

/// Install scope: **user (global)** is the default and the release
/// posture. `Project` is an explicit, non-default opt-in — useful only for
/// developing the enforcer itself (this repo's own root `.mcp.json`, which
/// the installer does NOT emit into consumers per RUST_ARCHITECTURE.md).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Scope {
    /// User-level (global) registry — e.g. Claude Code's top-level
    /// `mcpServers` in `~/.claude.json`, Codex's `~/.codex/config.toml`.
    /// The canonical/default install target.
    #[default]
    User,
    /// Per-repo project registry (e.g. a checked-in `.mcp.json`). Explicit
    /// opt-in only; never the implicit result of omitting `--scope`.
    Project,
}

/// Output-rendering mode for a non-interactive (non-TTY / piped) caller.
/// arc-22 decides whether stdout is a TTY; this flag is the seam that
/// tells `core`/`report` to render machine-readable JSON instead of the
/// human-formatted report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputMode {
    /// Human-formatted report to a terminal.
    #[default]
    Human,
    /// Machine-readable JSON (non-TTY default, or explicit `--json`).
    Json,
}

/// Shared fields every `enforcer install|uninstall|update|doctor`
/// invocation carries, regardless of verb.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestContext {
    /// `--scope user|project`. Defaults to [`Scope::User`].
    pub scope: Scope,
    /// `--dry-run`: plan only, apply nothing, zero filesystem writes.
    pub dry_run: bool,
    /// Rendering mode for the resulting report.
    pub output: OutputMode,
    /// Absolute path of the `enforcer` binary being installed/registered.
    /// Adapters MUST point a harness's MCP registration at this exact
    /// path — never a relative path, which cannot resolve from an
    /// arbitrary repo cwd (RUST_ARCHITECTURE.md).
    pub binary_path: PathBuf,
}

impl RequestContext {
    /// Build a context with the release-default posture: user scope,
    /// not a dry run, human output. Callers override individual fields as
    /// needed (`--scope project`, `--dry-run`, non-TTY JSON).
    #[must_use]
    pub fn with_defaults(binary_path: PathBuf) -> Self {
        Self {
            scope: Scope::default(),
            dry_run: false,
            output: OutputMode::default(),
            binary_path,
        }
    }
}

/// `enforcer install [--scope user|project] [--dry-run]` request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallRequest {
    /// Shared scope/dry-run/output/binary-path fields.
    pub context: RequestContext,
    /// Restrict the install to specific harness adapter keys (e.g.
    /// `["claude", "codex"]`). Empty means "every detected/known harness"
    /// (c02's autodetect scope, not owned here).
    pub only_harnesses: Vec<String>,
}

/// `enforcer uninstall [--scope user|project] [--dry-run]` request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UninstallRequest {
    /// Shared scope/dry-run/output/binary-path fields.
    pub context: RequestContext,
    /// Restrict the uninstall to specific harness adapter keys.
    pub only_harnesses: Vec<String>,
}

/// `enforcer update [--dry-run]` request — the binary-swap verb. Scope is
/// meaningless for a binary swap (the registration already points at the
/// binary path; only the bytes behind it change), so this request carries
/// no [`Scope`] field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateRequest {
    /// Whether to check-and-report only (`dry_run`) or actually perform
    /// the swap.
    pub dry_run: bool,
    /// Rendering mode for the resulting report.
    pub output: OutputMode,
}

/// `enforcer doctor` request — read-only health check, no scope/dry-run
/// distinction (doctor never writes).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DoctorRequest {
    /// Rendering mode for the resulting report.
    pub output: OutputMode,
}

/// Which `enforcer` verb produced a [`CommandEnvelope`] — the stable
/// `command` discriminant a non-TTY/JSON caller matches on. Renders as its
/// lowercase name on the wire (`"install"`, `"uninstall"`, `"update"`,
/// `"doctor"`) so a scripted caller never has to guess from shape alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CommandName {
    /// `enforcer install`.
    Install,
    /// `enforcer uninstall`.
    Uninstall,
    /// `enforcer update`.
    Update,
    /// `enforcer doctor`.
    Doctor,
}

/// The stable non-TTY JSON envelope every `enforcer install|uninstall|
/// update|doctor` invocation renders (workpack c01 acceptance row: "non-TTY
/// output deserializes as JSON with a stable `command`/`checks` schema").
/// `enforcer-cli` (arc-22) is the only caller that decides TTY-vs-not and
/// serializes this to stdout; this type is the shape it serializes, kept
/// here so the schema lives next to the request/response types it wraps
/// instead of being re-invented per call site.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandEnvelope {
    /// Which verb produced this envelope.
    pub command: CommandName,
    /// True when every check inside `checks` passed (an empty `checks` is
    /// vacuously `true` — `install`/`uninstall` with nothing to verify
    /// yet, e.g. before any adapter has run `verify`).
    pub ok: bool,
    /// Every [`crate::report::VerifyCheck`]-shaped health check this
    /// invocation ran or aggregated. `install`/`uninstall` populate this
    /// from each adapter's post-apply expectations; `doctor` populates it
    /// directly from [`crate::core::doctor`].
    pub checks: Vec<crate::report::VerifyCheck>,
}

impl CommandEnvelope {
    /// Build an envelope from a command name and a flattened check list,
    /// deriving `ok` from whether every check passed.
    #[must_use]
    pub fn new(command: CommandName, checks: Vec<crate::report::VerifyCheck>) -> Self {
        let ok = checks.iter().all(|c| c.passed);
        Self { command, ok, checks }
    }
}

#[cfg(test)]
mod tests {
    use super::{CommandEnvelope, CommandName, OutputMode, RequestContext, Scope};
    use crate::report::VerifyCheck;
    use std::path::PathBuf;

    #[test]
    fn scope_default_is_user_never_project() {
        assert_eq!(Scope::default(), Scope::User);
    }

    #[test]
    fn output_mode_default_is_human() {
        assert_eq!(OutputMode::default(), OutputMode::Human);
    }

    #[test]
    fn request_context_with_defaults_is_user_scope_not_dry_run() {
        let ctx = RequestContext::with_defaults(PathBuf::from("/abs/enforcer"));
        assert_eq!(ctx.scope, Scope::User);
        assert!(!ctx.dry_run);
        assert_eq!(ctx.output, OutputMode::Human);
    }

    #[test]
    fn scope_serializes_camel_case() -> Result<(), Box<dyn std::error::Error>> {
        let wire = serde_json::to_string(&Scope::User)?;
        assert_eq!(wire, "\"user\"");
        let wire = serde_json::to_string(&Scope::Project)?;
        assert_eq!(wire, "\"project\"");
        Ok(())
    }

    #[test]
    fn command_envelope_has_a_stable_command_checks_schema(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let envelope = CommandEnvelope::new(
            CommandName::Doctor,
            vec![VerifyCheck {
                harness: "claude".to_owned(),
                name: "mcp-registration-present".to_owned(),
                passed: true,
                detail: String::new(),
            }],
        );
        let wire = serde_json::to_string(&envelope)?;
        let value: serde_json::Value = serde_json::from_str(&wire)?;
        assert_eq!(value["command"], "doctor");
        assert_eq!(value["ok"], true);
        assert!(value["checks"].is_array());
        assert_eq!(value["checks"][0]["harness"], "claude");
        let back: CommandEnvelope = serde_json::from_str(&wire)?;
        assert_eq!(back, envelope);
        Ok(())
    }

    #[test]
    fn command_envelope_ok_is_false_when_any_check_fails() {
        let envelope = CommandEnvelope::new(
            CommandName::Install,
            vec![VerifyCheck {
                harness: "codex".to_owned(),
                name: "mcp-registration-present".to_owned(),
                passed: false,
                detail: "missing registration".to_owned(),
            }],
        );
        assert!(!envelope.ok);
    }

    #[test]
    fn command_envelope_ok_is_vacuously_true_for_no_checks() {
        let envelope = CommandEnvelope::new(CommandName::Update, vec![]);
        assert!(envelope.ok);
    }

    #[test]
    fn command_name_renders_lowercase_on_the_wire() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(serde_json::to_string(&CommandName::Install)?, "\"install\"");
        assert_eq!(
            serde_json::to_string(&CommandName::Uninstall)?,
            "\"uninstall\""
        );
        assert_eq!(serde_json::to_string(&CommandName::Update)?, "\"update\"");
        assert_eq!(serde_json::to_string(&CommandName::Doctor)?, "\"doctor\"");
        Ok(())
    }
}
