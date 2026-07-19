//! `enforcer onboard [repo]` (f02): the CLI subcommand wiring for
//! ratchet-first onboarding. Delegates the entire flow to
//! `enforcer_scan::onboard::onboard` -- this module only bridges clap args
//! -> `RepoRoot` -> exit code, the same shape `crate::commands` uses for
//! every other subcommand. Kept in its own file (rather than folded into
//! `commands.rs`) because this workpack's file grant is scoped to
//! `src/onboard.rs`, not `src/commands.rs`.
//!
//! # Deviations from the f02.md checklist (recorded per this workpack's
//! own proof-rule requirement)
//! - **No MCP `onboard` tool wired here.** Registering a new MCP tool
//!   lives in `enforcer-mcp`'s own registry, a crate outside this
//!   workpack's file grant (additive one-line registrations only, in
//!   mod/lib/command tables). `enforcer_scan::onboard::onboard` is a
//!   plain, CLI-agnostic function specifically so that MCP wiring is a
//!   thin follow-up adapter over the same core, not a redesign.
//! - **Silent on success** (exit 0, no stdout write). `crate::output` is
//!   this crate's own designated ONE sanctioned print-sink module (see
//!   `src/lib.rs`'s charter); adding a success-summary printer belongs to
//!   that file, which is outside this workpack's file grant. Failures
//!   still route through the existing `output::print_internal_error`, so
//!   this subcommand never silently swallows an error (an invalid or
//!   malformed repo path is a reported, non-zero exit) -- it only omits a
//!   success-path summary line.

use clap::Args;
use enforcer_core::error::Result as CoreResult;
use enforcer_domain::core_types::ExitCode;
use enforcer_domain::paths::RepoRoot;

use crate::output;

/// `enforcer onboard [repo]`: create `.enforce/`, write (or preserve) the
/// project profile, capture a ratchet-first baseline, and register the
/// project. See `enforcer_scan::onboard` module docs for the full
/// contract.
#[derive(Debug, Args)]
#[doc = "Clap args for the onboard subcommand; Debug is derived above intentionally."]
pub struct OnboardArgs {
    /// Repository path to onboard. Defaults to the current working
    /// directory.
    // BRAND-INVARIANT: the raw caller-supplied path exactly as clap parsed
    // it from argv; it is validated into a branded `RepoRoot` (absolute,
    // separator-normalized) by `resolve_repo_root` before ANY use, and
    // private so nothing can read it un-validated.
    repo: Option<std::path::PathBuf>,
}

/// Resolve the effective repo root: the explicit `[repo]` argument if
/// given, otherwise the current working directory -- validated into a
/// branded [`RepoRoot`] (typed decode error on an invalid or relative
/// path, never a silent default).
fn resolve_repo_root(args: &OnboardArgs) -> CoreResult<RepoRoot> {
    let path = match &args.repo {
        // CLONE-JUSTIFICATION: the explicit arg is borrowed from clap's
        // parsed struct; the resolver needs an owned PathBuf either way
        // (the cwd branch below allocates one from the OS).
        Some(explicit) => explicit.clone(),
        None => std::env::current_dir()?,
    };
    Ok(path.to_string_lossy().parse::<RepoRoot>()?)
}

/// Run `enforcer onboard`.
pub fn run_onboard(args: &OnboardArgs) -> ExitCode {
    let root = match resolve_repo_root(args) {
        Ok(root) => root,
        Err(err) => {
            output::print_internal_error(&format!("onboard: repo root did not resolve: {err}"));
            return ExitCode::InternalError;
        }
    };
    match enforcer_scan::onboard::onboard(&root) {
        Ok(_outcome) => ExitCode::Success,
        Err(err) => {
            output::print_internal_error(&format!("onboard failed: {err}"));
            ExitCode::InternalError
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{resolve_repo_root, run_onboard, OnboardArgs};
    use enforcer_core::error::{Error as CoreError, Result as CoreResult};
    use enforcer_domain::core_types::ExitCode;
    use enforcer_domain::paths::RepoRoot;

    #[test]
    fn resolve_repo_root_defaults_to_cwd_when_repo_arg_absent() -> CoreResult<()> {
        let args = OnboardArgs { repo: None };
        let resolved = resolve_repo_root(&args)?;
        let expected = std::env::current_dir()?
            .to_string_lossy()
            .parse::<RepoRoot>()?;
        assert_eq!(resolved, expected);
        Ok(())
    }

    #[test]
    fn resolve_repo_root_uses_explicit_repo_arg() -> CoreResult<()> {
        let temp = tempfile::tempdir()?;
        let expected = temp.path().to_string_lossy().parse::<RepoRoot>()?;
        let explicit = temp.path().to_path_buf();
        let args = OnboardArgs {
            repo: Some(explicit),
        };
        let resolved = resolve_repo_root(&args)?;
        assert_eq!(resolved, expected);
        Ok(())
    }

    #[test]
    fn resolve_repo_root_rejects_a_relative_repo_arg_with_a_typed_decode_error() {
        let relative = std::path::PathBuf::from("relative/subdir");
        let args = OnboardArgs {
            repo: Some(relative),
        };
        let outcome = resolve_repo_root(&args);
        assert!(
            matches!(outcome, Err(CoreError::Decode(_))),
            "a relative repo path must fail as a typed decode error, never a silent default"
        );
    }

    #[test]
    fn onboard_a_fresh_temp_repo_exits_success_and_scaffolds_enforce() -> CoreResult<()> {
        let temp = tempfile::tempdir()?;
        std::fs::create_dir_all(temp.path().join("src"))?;
        std::fs::write(temp.path().join("src/lib.rs"), "fn good() -> i32 { 42 }\n")?;
        let explicit = temp.path().to_path_buf();
        let args = OnboardArgs {
            repo: Some(explicit),
        };
        assert_eq!(run_onboard(&args), ExitCode::Success);
        assert!(temp.path().join(".enforce").join("baseline.json").exists());
        Ok(())
    }

    #[test]
    fn onboard_reports_internal_error_for_a_repo_root_that_cannot_resolve() {
        let empty = std::path::PathBuf::new();
        let args = OnboardArgs { repo: Some(empty) };
        assert_eq!(run_onboard(&args), ExitCode::InternalError);
    }
}
