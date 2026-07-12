//! `xtask` -- the workspace's task runner.
//!
//! Two subcommands, both self-enforcement gates:
//! - `xtask dogfood` (a10): the native dogfood loop -- the baseline-gated
//!   rust-rule self-scan over `crates/**` plus the `cargo fmt`/`clippy`/
//!   `deny`/`audit` toolchain steps. `--baseline-write` refreshes the
//!   committed baseline snapshot instead of gating (the one sanctioned,
//!   explicit, out-of-band maintenance operation).
//! - `xtask dogfood-gate` (z01): the terminal composing proof gate --
//!   composes `dogfood`, the e01 literal-scan floor (against its
//!   committed T2 ceiling; `--ceiling-write` refreshes it), and the b02
//!   PLAN-* structure report into one persisted manifest plus a
//!   hash-chained proof-journal record.
//!
//! No bypass flag exists on either subcommand's gating behavior --
//! consistent with the `enforcer` CLI's own no-override doctrine (see
//! `crates/enforcer-cli/src/cli.rs` module docs). This file is
//! deliberately thin: all argv/console/exit-code handling lives in
//! [`boundary`], the domain logic in [`dogfood`]/[`dogfood_gate`].

mod boundary;
mod dogfood;
mod dogfood_gate;

fn main() -> std::process::ExitCode {
    boundary::entry()
}
