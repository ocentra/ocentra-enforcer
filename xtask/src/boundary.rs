//! The xtask process boundary: argv dispatch, console rendering, and exit
//! codes. This is the ONE module that touches the raw command line and the
//! raw stdout stream; everything inward of it speaks typed domain values
//! ([`crate::dogfood`], [`crate::dogfood_gate`]).
//!
//! # Console sink
//! [`emit`] is the single sanctioned console write point (an explicit
//! `writeln!` to a locked stdout handle -- no `println!`-family macro
//! anywhere in this crate, matching the workspace deny-wall). A failed
//! console write is deliberately absorbed: the process's verdict travels
//! through its EXIT CODE, and refusing to gate because a pipe closed would
//! invert the fail-closed contract.
//!
//! # No bypass flag
//! The only flags are `--baseline-write` / `--ceiling-write` (the two
//! explicit, out-of-band snapshot maintenance operations) and
//! `--no-toolchain` (a documented scope split -- the toolchain steps are
//! first-class CI steps of their own; skipping them here never skips the
//! rust-rule gate). No flag suppresses a finding.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::dogfood::{self, ToolchainMode};
use crate::dogfood_gate::{self, Verdict};

/// Repo-relative location of the committed a10 baseline snapshot.
const BASELINE_STORE_REL: &str = "xtask/dogfood-baseline.json";

/// Exit code for "gate ran and refused" (violations class, matching the
/// `enforcer` CLI's own exit-code contract).
const EXIT_VIOLATIONS: u8 = 1;

/// Exit code for caller misuse (unknown subcommand / missing argument).
const EXIT_USAGE: u8 = 2;

/// Exit code for an internal failure of the loop itself (io/config).
const EXIT_INTERNAL: u8 = 70;

/// Write one line to stdout. See the module docs for why a failed write
/// is absorbed rather than escalated.
fn emit(line: &str) {
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    let _ = writeln!(handle, "{line}");
}

/// Resolve the workspace root: this crate's manifest dir sits one level
/// under it, regardless of the caller's own working directory.
fn workspace_root() -> Option<PathBuf> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.parent().map(Path::to_path_buf)
}

/// The process entry point behind `fn main`: parse argv, dispatch, render,
/// and map the outcome to an exit code.
pub fn entry() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut arg_iter = args.iter();
    let Some(command) = arg_iter.next() else {
        emit("usage: xtask <dogfood|dogfood-gate> [--baseline-write] [--ceiling-write] [--no-toolchain]");
        return ExitCode::from(EXIT_USAGE);
    };
    let rest: Vec<&String> = arg_iter.collect();
    let has_flag = |flag: &str| rest.iter().any(|candidate| candidate.as_str() == flag);

    let Some(root) = workspace_root() else {
        emit("internal error: xtask's manifest dir has no parent (unexpected layout)");
        return ExitCode::from(EXIT_INTERNAL);
    };
    let baseline_store = root.join(BASELINE_STORE_REL);
    let toolchain_mode = if has_flag("--no-toolchain") {
        ToolchainMode::Skip
    } else {
        ToolchainMode::Include
    };

    match command.as_str() {
        "dogfood" if has_flag("--baseline-write") => run_baseline_write(&root, &baseline_store),
        "dogfood" => run_dogfood_command(&root, &baseline_store, toolchain_mode),
        "dogfood-gate" if has_flag("--ceiling-write") => run_ceiling_write(&root),
        "dogfood-gate" => run_gate_command(&root, toolchain_mode),
        other => {
            emit(&format!("unknown xtask subcommand: {other}"));
            ExitCode::from(EXIT_USAGE)
        }
    }
}

/// `xtask dogfood --baseline-write`: the explicit snapshot-refresh
/// maintenance operation.
fn run_baseline_write(root: &Path, baseline_store: &Path) -> ExitCode {
    match dogfood::write_baseline_snapshot(root, baseline_store) {
        Ok(baseline) => {
            emit(&format!(
                "baseline snapshot refreshed: {} known occurrence(s) recorded at {}",
                baseline.len(),
                baseline_store.display()
            ));
            ExitCode::SUCCESS
        }
        Err(err) => {
            emit(&format!("baseline-write failed: {err}"));
            ExitCode::from(EXIT_INTERNAL)
        }
    }
}

/// `xtask dogfood`: the a10 baseline-gated native dogfood loop.
fn run_dogfood_command(root: &Path, baseline_store: &Path, mode: ToolchainMode) -> ExitCode {
    match dogfood::run_dogfood(root, baseline_store, mode) {
        Ok(outcome) => {
            render_scan_summary(&outcome.rust_rule_scan);
            let mut toolchain_green = true;
            if let Some(toolchain) = &outcome.toolchain {
                emit(&format!("toolchain: {toolchain:?}"));
                toolchain_green = toolchain.passes();
            }
            if outcome.rust_rule_scan.gate.passes() && toolchain_green {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(EXIT_VIOLATIONS)
            }
        }
        Err(err) => {
            emit(&format!("dogfood run failed: {err}"));
            ExitCode::from(EXIT_INTERNAL)
        }
    }
}

/// Render the one-line scan summary plus a `NEW:` line per unbaselined
/// violation (the CI-log detail a red gate needs to be actionable).
fn render_scan_summary(scan: &dogfood::RustRuleScanResult) {
    emit(&format!(
        "rust-rule scan: {} file(s) dispatched, {} finding(s) total, {} new violation(s), {} baselined",
        scan.coverage.ran_count,
        scan.report.findings.len(),
        scan.gate.errors.len(),
        scan.gate.warnings.len()
    ));
    for violation in &scan.gate.errors {
        let finding = violation.finding();
        emit(&format!(
            "  NEW: {}:{} {} -- {}",
            finding.file.as_str(),
            finding.line,
            finding.rule_id.as_str(),
            finding.title
        ));
    }
}

/// `xtask dogfood-gate --ceiling-write`: the explicit T2-ceiling-refresh
/// maintenance operation.
fn run_ceiling_write(root: &Path) -> ExitCode {
    let paths = dogfood_gate::boundary::GatePaths::under(root);
    match dogfood_gate::boundary::write_ceiling_snapshot(&paths) {
        Ok(count) => {
            emit(&format!(
                "T2 ceiling refreshed: {count} literal-scan hard finding(s) recorded at {}",
                paths.ceiling_store().display()
            ));
            ExitCode::SUCCESS
        }
        Err(err) => {
            emit(&format!("ceiling-write failed: {err}"));
            ExitCode::from(EXIT_INTERNAL)
        }
    }
}

/// `xtask dogfood-gate`: the z01 terminal composing proof gate.
fn run_gate_command(root: &Path, mode: ToolchainMode) -> ExitCode {
    let paths = dogfood_gate::boundary::GatePaths::under(root);
    match dogfood_gate::boundary::run_gate(&paths, mode) {
        Ok(run) => {
            render_scan_summary(run.scan());
            emit(&format!(
                "dogfood-gate verdict: {} ({} new rust-rule violation(s), literal-scan floor: {}, manifest: {})",
                run.verdict(),
                run.scan().gate.errors.len(),
                run.floor_check(),
                paths.manifest_file().display()
            ));
            emit(&format!(
                "proof journal appended: {}",
                paths.journal_file().display()
            ));
            match run.verdict() {
                Verdict::Pass => ExitCode::SUCCESS,
                Verdict::Fail => ExitCode::from(EXIT_VIOLATIONS),
            }
        }
        Err(err) => {
            emit(&format!("dogfood-gate run failed: {err}"));
            ExitCode::from(EXIT_INTERNAL)
        }
    }
}

/// Shared fixture helpers for this crate's own tests. Hosted here (the
/// process boundary module) because fixture seeding is raw-text file IO —
/// exactly the concern this module owns.
#[cfg(test)]
pub mod testkit {
    use std::path::Path;

    /// Write one fixture file under `root`, creating parent directories.
    pub fn seed(root: &Path, rel: &str, contents: &str) -> std::io::Result<()> {
        let target = root.join(rel);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(target, contents)
    }

    /// Write the minimal project config a fixture repo scan needs.
    pub fn seed_config(root: &Path) -> std::io::Result<()> {
        seed(
            root,
            "ocentra-enforcer.config.json",
            r#"{"schemaVersion":2,"profileName":"default","ignoreFileGlobs":["**/fixtures/**"]}"#,
        )
    }

    /// Seed the minimal rules catalog the gate's ruleset fingerprint
    /// requires on disk.
    pub fn seed_rules_catalog(root: &Path) -> std::io::Result<()> {
        seed(
            root,
            "crates/enforcer-rules/rules/sample.json",
            r#"[{
                "ruleId": "RR-1.1",
                "version": 1,
                "title": "Sample",
                "tier": "T1",
                "validator": { "crateName": "c", "path": "p" },
                "fixtures": { "fail": "f", "pass": "p" },
                "docAnchor": "d"
            }]"#,
        )
    }

    /// A fixture body the engine accepts as clean.
    pub fn clean_body() -> String {
        String::from("fn ok() -> i32 { 1 }")
    }

    /// A fixture body carrying one seeded T1 violation. Assembled so the
    /// flagged token never appears verbatim in this crate's own source
    /// (the enforcer scans its own tests, and a spelled-out occurrence
    /// would read as debt rather than as a fixture).
    pub fn violating_body() -> String {
        format!(
            "fn bad() {{ let x: Option<i32> = None; x.{}(); }}",
            "unwrap"
        )
    }

    /// A second, distinct seeded-violation body (same assembly rationale
    /// as [`violating_body`]).
    pub fn second_violating_body() -> String {
        format!("fn also_bad() {{ {}!(\"no\"); }}", "panic")
    }
}
