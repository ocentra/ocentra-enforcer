//! `enforcer-cli` (arc-22) — the crate that compiles to the `enforcer`
//! binary: the clap CLI AND the MCP stdio server behind `serve`, one
//! distributed artifact, neither surface secondary.
//!
//! # Charter
//!
//! Per `RUST_ARCHITECTURE.md` ("One binary IS the engine"), this crate is
//! the top integration point of the workspace: it wires
//! [`enforcer_config`] (the single declarative control plane both the CLI
//! and MCP read), [`enforcer_scan`] (the tri-modal scope resolver + engine),
//! [`enforcer_domain`]'s `Report` rendering, and [`enforcer_mcp`]'s stdio
//! server, never reimplementing any of their logic.
//!
//! # Modules
//! - [`cli`] — the clap grammar (`Cli`/`Command`), including the tri-modal
//!   scope group (`<paths...>` | `--base/--head` | `--all`) shared by
//!   every scope-taking subcommand. Parsing only; no I/O.
//! - [`scope`] — bridges the clap-parsed scope flags to
//!   `enforcer_scan::scope::ScopeRequest`.
//! - [`commands`] — one dispatch function per subcommand; the only module
//!   (besides [`output`]) allowed to touch the filesystem/process exit
//!   path. Delegates all engine work to the owning crate.
//! - [`output`] — the ONE sanctioned print-sink module. Every stdout/stderr
//!   write in this crate funnels through here; every other module obeys
//!   the workspace `[lints]` deny wall (`print_stdout`/`print_stderr`
//!   denied).
//! - [`fix_hints`] — the terse `Fix:` hint lookup rendered under each
//!   finding. There is deliberately no accompanying override/bypass
//!   flag anywhere in this crate — the only sanctioned exemption path is a
//!   declarative, committed, gated waiver read through `enforcer-config`
//!   (`enforcer_domain::findings::Report::waived`), never a CLI flag.
//! - [`name`] — the TRANSITIONAL `BINARY_NAME` const (x01 owns the final
//!   value), the CLI-side counterpart to `enforcer_mcp::name::SERVER_NAME`.
//!
//! # `lite`/`full` feature split (ONE source tree, DRY)
//! `full` (default) pulls in `enforcer-coordination` and enables the
//! `coordination`/`ledger` subcommands. `lite` (`--no-default-features
//! --features lite`) excludes that dependency from the graph entirely and
//! the affected subcommands do not exist in the clap grammar at all under
//! that feature — a headless CI binary never links, let alone runs,
//! multi-agent coordination code. `serve` (the MCP/UI-facing surface) is
//! likewise a `full`-only subcommand: CI is fully mechanical, never an
//! MCP round trip.
//!
//! # Exit-code taxonomy
//! [`enforcer_domain::core_types::ExitCode`] is the entire contract: a rule
//! violation is [`enforcer_domain::core_types::ExitCode::Violations`] (1); a
//! clap usage error is [`enforcer_domain::core_types::ExitCode::UsageError`]
//! (2); a bad/missing config is
//! [`enforcer_domain::core_types::ExitCode::ConfigError`] (78); an internal
//! bug (panic, I/O failure, decode failure not attributable to the
//! scanned project) is
//! [`enforcer_domain::core_types::ExitCode::InternalError`] (70). [`main`]
//! (`src/main.rs`) installs a panic hook so no bare panic ever escapes to
//! an ambiguous OS-level abort; every panic is reported through the
//! internal-error class, pointing at the enforcer itself, never coerced
//! into a generic non-zero a CI consumer could misread as "my code is
//! bad".
//!
//! No `pub use` barrels (workspace doctrine): consumers path through the
//! modules directly.

pub mod advise;
pub mod architecture;
pub mod cli;
pub mod commands;
pub mod fix_hints;
pub mod hook;
pub mod lifecycle;
pub mod name;
pub mod onboard;
pub mod output;
pub mod scope;
pub mod verify;
