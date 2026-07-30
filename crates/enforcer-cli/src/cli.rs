//! The clap grammar: `Cli`/`Command` plus the shared tri-modal
//! [`ScopeArgs`] group. Parsing only -- no I/O, no engine calls. See
//! [`crate::commands`] for dispatch.
//!
//! # Tri-modal scope grammar
//! Every scope-taking subcommand (`check`, `scan`, `verify`) embeds
//! [`ScopeArgs`]: `<paths...>` XOR `--base <sha> --head <sha>` XOR
//! `--all`. clap's `ArgGroup` with `multiple = false` enforces "exactly
//! one" at parse time -- a `--base`/`--head` + `<paths...>` collision is a
//! clap error (usage-error exit class), not a runtime finding.
//!
//! # No override flag
//! There is no flag anywhere in this grammar that suppresses a finding.
//! Grep this file: the only exemption path is a declarative, committed,
//! gated waiver read from `enforcer-config`, never a CLI switch.
//!
//! # `verify` is orthogonal to the d06 lifecycle family
//! `verify --mode {fast,local,ci,parent}` is a scope/aggregation PROFILE
//! over checks (see [`crate::verify::VerifyMode`]), not a lifecycle phase.
//! d06's `plan|implement|check|fix|review` family (`src/lifecycle.rs`,
//! owned by d06, sequenced after this skeleton) is a SEPARATE axis; `check`
//! (this crate's tri-modal scan) and `verify` are not aliases of each
//! other, and d06's `review` verb is likewise distinct from `verify
//! --mode parent`.

use std::path::PathBuf;

use clap::{ArgGroup, Args, Parser, Subcommand};

use crate::advise::AdviseTarget;
use crate::architecture::ArchitectureLanguage;
use crate::onboard::OnboardArgs;
use crate::verify::VerifyMode;

/// The `enforcer` binary's top-level grammar.
#[derive(Debug, Parser)]
#[command(
    name = "enforcer",
    version,
    about = "Ocentra Enforcer -- mechanical enforcement, one binary."
)]
#[doc = "Top-level parsed command-line grammar."]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

/// Every first-class subcommand. `check`/`scan` are the primary
/// mechanical-enforcement entry points; `serve` is the MCP stdio surface
/// (`full` feature only); `coordination`/`ledger` (alias) are the
/// multi-agent hub surface (`full` feature only); `verify`/`advise`/
/// `architecture` are the four orphaned Node subcommands ported per the
/// WAVE 4 gap.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Run every wired language-family validator over a scope and render
    /// a `Report`. Exit 0 (clean) or `Violations` (1, findings present).
    Check(ScopeArgs),
    /// Alias-shaped synonym for `check` at the engine level -- kept as a
    /// distinct subcommand name because `enforcer scan` reads naturally
    /// as "just look", matching the legacy CLI's separate verb.
    Scan(ScopeArgs),
    /// Start the `enforcer-mcp` stdio server (default), or -- with
    /// `--ui` -- the g01 human-invoked UI serve surface (`enforcer-ui`).
    /// One binary = CLI + MCP + UI. Compiled out entirely under
    /// `--features lite` (CI never needs an MCP round trip or a UI
    /// surface; a headless run's exit code is the whole verdict).
    #[cfg(feature = "full")]
    Serve(ServeArgs),
    /// Alias-shaped synonym for `serve --ui` -- the g01 UI serve surface,
    /// reached directly without the `--ui` flag. Both spellings resolve
    /// to the identical surface (`enforcer_ui::serve::ServeAlias`).
    #[cfg(feature = "full")]
    Ui(ServeArgs),
    /// Register the installed binary with every supported user-level
    /// harness, then verify each native registration before exiting.
    Install,
    /// Verify every supported user-level harness registration without
    /// changing any files. Exit 0 only when every native doctor check passes.
    Doctor,
    /// Plan/workpack scaffolding and validation (arc-20).
    Plan,
    /// Proof-artifact recording/inspection (arc-17).
    Proof,
    /// Multi-agent coordination hub (arc-16). `full` feature only;
    /// compiled out under `lite` together with its `ledger` alias.
    #[cfg(feature = "full")]
    #[command(visible_alias = "ledger")]
    Coordination,
    /// Codebase-memory graph tools. The `cli` adapter forwards every
    /// trailing token to enforcer-memory's MCP-compatible CLI transport.
    #[cfg(feature = "full")]
    Memory {
        #[command(subcommand)]
        action: MemoryAction,
    },
    /// The four verify modes over the tri-modal scope grammar: `fast`
    /// (quick local subset), `local` (default dev), `ci` (headless
    /// mechanical), `parent` (OcentraParent-parity superset). Orthogonal
    /// to the d06 lifecycle `check`/`review` verbs -- see module docs.
    Verify(VerifyArgs),
    /// The literal-risk CLI seam (arc-13). Today accepts exactly one
    /// target, `literals`; any other target is a usage error.
    Advise {
        /// The advise target. Only `literals` is supported today.
        target: AdviseTarget,
    },
    /// The architecture-policy / import-boundaries named-check family
    /// (`src/cli-checks.mjs` port). A bare `architecture` with no `check`
    /// token is a usage error -- the subcommand only, deliberately.
    Architecture {
        #[command(subcommand)]
        action: ArchitectureAction,
    },
    /// f02 ratchet-first onboarding: create `.enforce/`, write (or
    /// preserve) the project profile, capture a baseline over every
    /// current violation, and register the project. Explicit and
    /// re-runnable (idempotent) -- see `enforcer_scan::onboard`.
    Onboard(OnboardArgs),
    /// Harness hook entry points. These commands consume a harness payload
    /// from stdin and return the harness-native decision before a write lands.
    Hook {
        #[command(subcommand)]
        action: HookAction,
    },
}

/// Supported harness hook entry points.
#[derive(Debug, Subcommand)]
pub enum HookAction {
    /// Validate Claude Code's `PreToolUse` Edit/Write/MultiEdit payload.
    #[command(name = "pretooluse")]
    PreToolUse,
}

/// `enforcer memory` actions. The memory crate owns the tool grammar; this
/// wrapper deliberately captures its tokens unchanged rather than duplicating
/// its flags in the top-level clap grammar.
#[cfg(feature = "full")]
#[derive(Debug, Subcommand)]
pub enum MemoryAction {
    /// Invoke a codebase-memory tool, e.g. `memory cli --json index_repository
    /// --repo-path . --stores-dir .enforce/ci-memory --mode fast`.
    #[command(trailing_var_arg = true, allow_hyphen_values = true)]
    Cli(MemoryCliArgs),
}

/// Opaque tokens forwarded to [`enforcer_memory::cli::run_cli`].
#[cfg(feature = "full")]
#[derive(Debug, Args)]
pub struct MemoryCliArgs {
    /// Tool name followed by raw JSON or hyphenated tool flags.
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub args: Vec<String>,
}

/// `architecture check --language <lang> --scope <files|diff|all>`.
#[derive(Debug, Subcommand)]
pub enum ArchitectureAction {
    /// Run the architecture-policy/import-boundaries checks.
    Check(ArchitectureCheckArgs),
}

/// Arguments for the architecture-policy check.
#[derive(Debug, Args)]
pub struct ArchitectureCheckArgs {
    /// Language family to check. Only `rust`/`typescript` route to a real
    /// validator today; other values are accepted at parse time and
    /// reported as an internal "not yet wired" outcome, never silently
    /// no-op.
    #[arg(long)]
    pub language: ArchitectureLanguage,
    #[command(flatten)]
    pub scope: ScopeArgs,
}

/// Args shared by `serve` and `ui`: `--ui` (only meaningful on `serve`;
/// `ui` is always the UI surface) plus the g01 host-bind-fail-closed
/// knobs (`--host`/`--port`/`--token`). Loopback (`127.0.0.1`) is the
/// default; a non-loopback `--host` REQUIRES `--token` or the surface
/// refuses to start (see `enforcer_ui::serve::resolve_bind`).
#[derive(Debug, Args)]
pub struct ServeArgs {
    /// Route a bare `serve` to the UI surface instead of the MCP stdio
    /// server. Ignored (always true) on `enforcer ui`.
    #[arg(long)]
    pub ui: bool,
    /// Bind host. Defaults to loopback (`127.0.0.1`).
    #[arg(long, default_value = "127.0.0.1")]
    pub host: String,
    /// Bind port. `0` (default) picks an ephemeral port.
    #[arg(long, default_value_t = 0)]
    pub port: u16,
    /// Auth token, REQUIRED for any non-loopback `--host`.
    #[arg(long)]
    pub token: Option<String>,
}

/// Arguments selecting verification mode and scan scope.
#[derive(Debug, Args)]
pub struct VerifyArgs {
    /// Verify mode. Defaults to `local`; an empty string also coerces to
    /// `local`; any other value is a clap parse error (`UsageError` exit
    /// class), not a finding exit code.
    #[arg(long, default_value = "local")]
    pub mode: VerifyMode,
    #[command(flatten)]
    pub scope: ScopeArgs,
}

/// The tri-modal scope: `<paths...>` | `--base <sha> --head <sha>` |
/// `--all`. Exactly one is active per invocation -- enforced by the
/// `scope` `ArgGroup` (`multiple = false`, `required = false` so `check`
/// with zero args is its own usage error handled by
/// [`crate::scope::resolve_request`], not a clap-level requirement, since
/// an empty invocation should read as "you forgot a scope", not a generic
/// clap usage dump).
#[derive(Debug, Args)]
#[command(group(
    ArgGroup::new("scope")
        .args(["paths", "all"])
        .multiple(false)
        .conflicts_with("base_head")
))]
#[command(group(
    ArgGroup::new("base_head")
        .args(["base", "head"])
        .multiple(true)
))]
#[doc = "Parsed tri-modal workspace scope."]
pub struct ScopeArgs {
    /// Explicit file or directory paths (Windows or POSIX separators,
    /// either works -- normalized before comparison).
    #[arg(group = "scope")]
    pub paths: Vec<PathBuf>,
    /// Scan the whole workspace.
    #[arg(long, group = "scope")]
    pub all: bool,
    /// Older endpoint of a git diff range. Must be paired with `--head`.
    #[arg(long, requires = "head")]
    pub base: Option<String>,
    /// Newer endpoint of a git diff range. Must be paired with `--base`.
    #[arg(long, requires = "base")]
    pub head: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::{Cli, Command};
    use clap::{error::ErrorKind, Parser};

    fn parse(args: &[&str]) -> clap::error::Result<Cli> {
        let mut full = vec!["enforcer"];
        full.extend_from_slice(args);
        Cli::try_parse_from(full)
    }

    fn assert_parse_error(
        args: &[&str],
        expected: ErrorKind,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let error = parse(args)
            .err()
            .ok_or_else(|| format!("expected clap error for arguments {args:?}"))?;
        assert_eq!(error.kind(), expected, "arguments were {args:?}");
        Ok(())
    }

    #[test]
    fn check_with_explicit_paths_parses() -> Result<(), Box<dyn std::error::Error>> {
        let cli = parse(&["check", "src/lib.rs"])?;
        match cli.command {
            Command::Check(scope) => assert_eq!(scope.paths.len(), 1),
            other => return Err(format!("expected Check, got {other:?}").into()),
        }
        Ok(())
    }

    #[test]
    fn doctor_is_a_first_class_command() -> Result<(), Box<dyn std::error::Error>> {
        let cli = parse(&["doctor"])?;
        assert!(matches!(cli.command, Command::Doctor));
        Ok(())
    }

    #[test]
    fn check_with_all_parses() -> Result<(), Box<dyn std::error::Error>> {
        let cli = parse(&["check", "--all"])?;
        match cli.command {
            Command::Check(scope) => assert!(scope.all),
            other => return Err(format!("expected Check, got {other:?}").into()),
        }
        Ok(())
    }

    #[test]
    fn check_with_base_head_parses() -> Result<(), Box<dyn std::error::Error>> {
        let cli = parse(&["check", "--base", "main", "--head", "HEAD"])?;
        match cli.command {
            Command::Check(scope) => {
                assert_eq!(scope.base.as_deref(), Some("main"));
                assert_eq!(scope.head.as_deref(), Some("HEAD"));
            }
            other => return Err(format!("expected Check, got {other:?}").into()),
        }
        Ok(())
    }

    #[test]
    fn base_without_head_is_a_clap_error() -> Result<(), Box<dyn std::error::Error>> {
        assert_parse_error(
            &["check", "--base", "main"],
            ErrorKind::MissingRequiredArgument,
        )
    }

    #[test]
    fn base_head_and_paths_collision_is_a_clap_error() -> Result<(), Box<dyn std::error::Error>> {
        assert_parse_error(
            &["check", "--base", "main", "--head", "HEAD", "src/lib.rs"],
            ErrorKind::ArgumentConflict,
        )
    }

    #[test]
    fn all_and_paths_collision_is_a_clap_error() -> Result<(), Box<dyn std::error::Error>> {
        assert_parse_error(
            &["check", "--all", "src/lib.rs"],
            ErrorKind::ArgumentConflict,
        )
    }

    #[test]
    fn all_and_base_head_collision_is_a_clap_error() -> Result<(), Box<dyn std::error::Error>> {
        assert_parse_error(
            &["check", "--all", "--base", "main", "--head", "HEAD"],
            ErrorKind::ArgumentConflict,
        )
    }

    #[test]
    fn no_such_thing_as_an_override_flag() -> Result<(), Box<dyn std::error::Error>> {
        // Documented, checked assertion: none of these ever parse.
        for bogus in [
            "--force",
            "--no-verify",
            "--skip",
            "--ignore-findings",
            "--bypass",
        ] {
            assert_parse_error(&["check", "--all", bogus], ErrorKind::UnknownArgument)?;
        }
        Ok(())
    }

    #[test]
    fn verify_defaults_to_local_mode() -> Result<(), Box<dyn std::error::Error>> {
        let cli = parse(&["verify", "--all"])?;
        match cli.command {
            Command::Verify(args) => {
                assert_eq!(args.mode, crate::verify::VerifyMode::Local);
            }
            other => return Err(format!("expected Verify, got {other:?}").into()),
        }
        Ok(())
    }

    #[test]
    fn verify_mode_bogus_is_a_clap_error() -> Result<(), Box<dyn std::error::Error>> {
        assert_parse_error(
            &["verify", "--all", "--mode", "bogus"],
            ErrorKind::InvalidValue,
        )
    }

    #[test]
    fn advise_literals_parses() -> Result<(), Box<dyn std::error::Error>> {
        let cli = parse(&["advise", "literals"])?;
        match cli.command {
            Command::Advise { target } => {
                assert_eq!(target, crate::advise::AdviseTarget::Literals);
            }
            other => return Err(format!("expected Advise, got {other:?}").into()),
        }
        Ok(())
    }

    #[test]
    fn advise_other_target_is_a_usage_error() -> Result<(), Box<dyn std::error::Error>> {
        assert_parse_error(&["advise", "somethingElse"], ErrorKind::InvalidValue)
    }

    #[test]
    fn bare_architecture_without_check_is_a_usage_error() -> Result<(), Box<dyn std::error::Error>>
    {
        assert_parse_error(
            &["architecture"],
            ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand,
        )
    }

    #[test]
    fn architecture_check_parses() -> Result<(), Box<dyn std::error::Error>> {
        let cli = parse(&["architecture", "check", "--language", "rust", "--all"])?;
        match cli.command {
            Command::Architecture { action } => match action {
                super::ArchitectureAction::Check(args) => {
                    assert!(args.scope.all);
                }
            },
            other => return Err(format!("expected Architecture, got {other:?}").into()),
        }
        Ok(())
    }

    #[cfg(feature = "full")]
    #[test]
    fn ledger_is_a_visible_alias_of_coordination() -> Result<(), Box<dyn std::error::Error>> {
        let via_ledger = parse(&["ledger"])?;
        let via_coordination = parse(&["coordination"])?;
        assert!(matches!(via_ledger.command, Command::Coordination));
        assert!(matches!(via_coordination.command, Command::Coordination));
        Ok(())
    }

    #[cfg(feature = "full")]
    #[test]
    fn memory_cli_forwards_hyphenated_tool_flags() -> Result<(), Box<dyn std::error::Error>> {
        let cli = parse(&[
            "memory",
            "cli",
            "--json",
            "index_repository",
            "--repo-path",
            ".",
            "--stores-dir",
            ".enforce/ci-memory",
            "--mode",
            "fast",
        ])?;
        match cli.command {
            Command::Memory {
                action: super::MemoryAction::Cli(args),
            } => assert_eq!(
                args.args,
                [
                    "--json",
                    "index_repository",
                    "--repo-path",
                    ".",
                    "--stores-dir",
                    ".enforce/ci-memory",
                    "--mode",
                    "fast",
                ]
            ),
            other => return Err(format!("expected Memory CLI, got {other:?}").into()),
        }
        Ok(())
    }
}
