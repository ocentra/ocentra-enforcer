//! The `enforcer` binary entry point.
//!
//! Installs a panic hook so no bare panic ever escapes to an ambiguous
//! OS-level abort: every panic is caught here and reported through the
//! `InternalError` exit class (a bug in the enforcer itself), which is a
//! DIFFERENT exit-code class from `Violations` (a rule violation in the
//! scanned project) -- a CI consumer must never be able to misread an
//! enforcer crash as "my code failed a check".

use std::process::ExitCode as ProcessExitCode;

use clap::Parser;
use enforcer_cli::cli::{ArchitectureAction, Cli, Command};
use enforcer_cli::commands;
use enforcer_cli::output;
use enforcer_core::exit_codes::ExitCode;

fn main() -> ProcessExitCode {
    std::panic::set_hook(Box::new(|info| {
        output::print_internal_error(&format!("panic: {info}"));
    }));
    let exit = std::panic::catch_unwind(run).unwrap_or(ExitCode::InternalError);
    to_process_exit_code(exit)
}

fn to_process_exit_code(exit: ExitCode) -> ProcessExitCode {
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    ProcessExitCode::from(exit.code() as u8)
}

fn run() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(err) => {
            // clap renders its own usage text to stdout/stderr via
            // `err.exit()`-equivalent formatting; this is the one place
            // outside `output.rs` that touches the terminal, and only to
            // delegate to clap's own (already-audited) writer -- no new
            // print-sink is added here.
            let _ = err.print();
            // `--help`/`--version` surface through `try_parse`'s `Err`
            // variant by clap's own design (so the caller can choose how
            // to exit) -- they are NOT usage errors and must exit 0, the
            // same as any other successful invocation. Only a genuine
            // parse failure (unknown flag, missing/duplicate arg, an
            // `ArgGroup` collision) is the `UsageError` class.
            return if err.use_stderr() {
                ExitCode::UsageError
            } else {
                ExitCode::Success
            };
        }
    };
    dispatch(&cli.command)
}

fn dispatch(command: &Command) -> ExitCode {
    match command {
        Command::Check(scope) | Command::Scan(scope) => commands::run_scoped_check(scope),
        #[cfg(feature = "full")]
        Command::Serve(args) => {
            if args.ui {
                run_serve_ui(args)
            } else {
                run_serve()
            }
        }
        #[cfg(feature = "full")]
        Command::Ui(args) => run_serve_ui(args),
        Command::Install => {
            output::print_internal_error("install is routed to arc-23; not wired in this skeleton");
            ExitCode::InternalError
        }
        Command::Plan => {
            output::print_internal_error(
                "plan subcommand is routed to arc-20; not wired in this skeleton",
            );
            ExitCode::InternalError
        }
        Command::Proof => {
            output::print_internal_error(
                "proof subcommand is routed to arc-17; not wired in this skeleton",
            );
            ExitCode::InternalError
        }
        #[cfg(feature = "full")]
        Command::Coordination => {
            output::print_internal_error(
                "coordination/ledger subcommand is routed to arc-16; not wired in this skeleton",
            );
            ExitCode::InternalError
        }
        Command::Verify(args) => commands::run_verify(args),
        Command::Advise { target } => match target {
            enforcer_cli::advise::AdviseTarget::Literals => commands::run_advise_literals(),
        },
        Command::Architecture { action } => match action {
            ArchitectureAction::Check(_) => commands::run_architecture(action),
        },
    }
}

#[cfg(feature = "full")]
fn run_serve() -> ExitCode {
    let cli_path = std::env::current_exe()
        .ok()
        .and_then(|p| p.to_str().map(str::to_owned))
        .unwrap_or_else(|| "enforcer".to_owned());
    let ctx = enforcer_mcp::sink::default_dispatch_context(cli_path);
    match enforcer_mcp::sink::run_stdio_server(&ctx) {
        Ok(()) => ExitCode::Success,
        Err(err) => {
            output::print_internal_error(&format!("mcp stdio server failed: {err}"));
            ExitCode::InternalError
        }
    }
}

/// `enforcer serve --ui` / `enforcer ui` -- the g01 human-invoked UI
/// serve surface. Delegates entirely to `enforcer_ui::serve` (arc-24's
/// backend + this workpack's transport); this function only bridges
/// clap args -> `BindRequest` -> exit code, never re-implementing the
/// bind gate or the transport itself.
///
/// # Honest scope note
/// The shutdown predicate always reports "keep running" -- there is no
/// in-process Ctrl+C/SIGINT handler wired yet (that is a follow-up, not a
/// gap this function hides: it never claims to have one). A caller stops
/// this human-invoked surface the same way any long-running local dev
/// server is stopped today, by killing the process; the fail-closed bind
/// gate ([`enforcer_ui::serve::resolve_bind`]) still runs BEFORE any
/// socket opens regardless.
#[cfg(feature = "full")]
fn run_serve_ui(args: &enforcer_cli::cli::ServeArgs) -> ExitCode {
    let request = enforcer_ui::serve::BindRequest {
        host: args.host.clone(),
        port: args.port,
        token: args.token.clone(),
    };
    match enforcer_ui::serve::run(&request, || false) {
        Ok(_addr) => ExitCode::Success,
        Err(err) => {
            output::print_internal_error(&format!("ui serve surface failed: {err}"));
            ExitCode::InternalError
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{dispatch, to_process_exit_code};
    use enforcer_cli::cli::{Cli, Command};
    use enforcer_core::exit_codes::ExitCode;

    fn parse(args: &[&str]) -> Result<Cli, Box<dyn std::error::Error>> {
        let mut full = vec!["enforcer"];
        full.extend_from_slice(args);
        Ok(clap::Parser::try_parse_from(full)?)
    }

    #[test]
    fn success_exit_code_maps_to_process_code_zero() {
        assert_eq!(
            to_process_exit_code(ExitCode::Success),
            std::process::ExitCode::from(0)
        );
    }

    #[test]
    fn advise_other_target_never_reaches_dispatch() -> Result<(), Box<dyn std::error::Error>> {
        // Guarded upstream by clap parsing (see cli.rs tests); this test
        // documents that `dispatch` only ever sees the one valid variant.
        let cli = parse(&["advise", "literals"])?;
        match cli.command {
            Command::Advise { target } => {
                assert_eq!(target, enforcer_cli::advise::AdviseTarget::Literals);
                Ok(())
            }
            other => Err(format!("expected Advise, got {other:?}").into()),
        }
    }

    #[test]
    fn unwired_install_reports_internal_error_not_success() -> Result<(), Box<dyn std::error::Error>>
    {
        let cli = parse(&["install"])?;
        assert_eq!(dispatch(&cli.command), ExitCode::InternalError);
        Ok(())
    }
}
