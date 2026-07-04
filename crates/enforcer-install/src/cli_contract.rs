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

#[cfg(test)]
mod tests {
    use super::{OutputMode, RequestContext, Scope};
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
}
