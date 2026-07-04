//! `enforcer-ui-export-ts` — regenerates the committed
//! `crates/enforcer-ui/frontend/src/bindings/*.ts` from the Rust
//! `ts_rs::TS` derives. Run this after changing any UI-facing
//! `enforcer-domain` type or [`enforcer_ui::payload`] type, then commit
//! the resulting diff; `tests/ts_drift.rs` fails the build if the
//! committed output and a fresh export ever disagree.
//!
//! Failure-path diagnostics go through `tracing::error!` (structured
//! JSON on stderr via `enforcer_core::tracing_setup`), not a bare
//! `eprintln!` -- this crate carries no print-sink exemption, matching
//! `RR-4.3`/`SRC-1.2`'s deny-by-default posture on debug/console macros.

use std::path::PathBuf;
use std::process::ExitCode;

use enforcer_ui::ts_export::export_all;

fn main() -> ExitCode {
    let _ = enforcer_core::tracing_setup::init("info");
    let out_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("frontend/src/bindings");
    match export_all(&out_dir) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            tracing::error!(error = %err, "enforcer-ui-export-ts: export failed");
            ExitCode::FAILURE
        }
    }
}
