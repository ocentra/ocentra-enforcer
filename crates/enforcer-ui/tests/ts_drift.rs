//! Fail-closed `ts_rs` drift test (arc-24's requirement checklist):
//! byte-compares the COMMITTED `frontend/src/bindings/*.ts` against a
//! fresh `ts_rs::TS::export_all_to` emit. A hand-edited or stale
//! committed `.ts` — or a domain-type change that was never re-exported —
//! fails this test, closing the door on the frontend ever drifting from
//! the Rust wire contract it is derived from.
//!
//! This test is intentionally proven BOTH ways per the workpack's
//! acceptance criteria: it passes today against the real committed
//! bindings ([`drift_test_passes_on_committed_bindings`]), and a second
//! test constructs an independent scratch "committed" directory with one
//! byte mutated to prove the SAME comparison logic fails closed rather
//! than silently passing ([`drift_comparison_fails_on_mutated_binding`]).

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use enforcer_ui::ts_export::export_all;

fn committed_bindings_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("frontend/src/bindings")
}

/// Byte-compare every `.ts` file in `committed` against a fresh export
/// into a scratch directory. Returns the list of mismatching/missing/
/// extra filenames (empty = no drift).
fn diff_against_fresh_export(committed: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    let fresh_dir = tempfile::tempdir()?;
    export_all(fresh_dir.path())?;

    let mut committed_files: Vec<String> = fs::read_dir(committed)?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| name.ends_with(".ts"))
        .collect();
    committed_files.sort();

    let mut fresh_files: Vec<String> = fs::read_dir(fresh_dir.path())?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| name.ends_with(".ts"))
        .collect();
    fresh_files.sort();

    let mut mismatches = Vec::new();

    if committed_files != fresh_files {
        mismatches.push(format!(
            "file set differs: committed={committed_files:?} fresh={fresh_files:?}"
        ));
        return Ok(mismatches);
    }

    for name in &committed_files {
        let committed_bytes = fs::read(committed.join(name))?;
        let fresh_bytes = fs::read(fresh_dir.path().join(name))?;
        if committed_bytes != fresh_bytes {
            mismatches.push(name.clone());
        }
    }

    Ok(mismatches)
}

/// PASS: the real committed `frontend/src/bindings/` byte-matches a
/// fresh export, i.e. nobody hand-edited the generated output and no
/// UI-facing type changed without re-running `enforcer-ui-export-ts`.
#[test]
fn drift_test_passes_on_committed_bindings() -> Result<(), Box<dyn Error>> {
    let committed = committed_bindings_dir();
    let mismatches = diff_against_fresh_export(&committed)?;
    assert!(
        mismatches.is_empty(),
        "committed TS bindings have drifted from a fresh ts_rs export; re-run \
         `cargo run -p enforcer-ui --bin enforcer-ui-export-ts` and commit the \
         diff. Drifted files: {mismatches:?}"
    );
    Ok(())
}

/// FAIL-CLOSED PROOF: copy the real committed bindings into a scratch
/// "committed" directory, mutate one byte of one file (simulating a
/// hand-edit or a domain-type change that was never re-exported), and
/// prove the SAME comparison used above reports that file as drifted
/// rather than silently accepting it.
#[test]
fn drift_comparison_fails_on_mutated_binding() -> Result<(), Box<dyn Error>> {
    let real_committed = committed_bindings_dir();
    let scratch = tempfile::tempdir()?;

    for entry in fs::read_dir(&real_committed)? {
        let entry = entry?;
        let name = entry.file_name();
        fs::copy(entry.path(), scratch.path().join(&name))?;
    }

    // Mutate Report.ts: append a byte that a fresh export will never
    // produce, simulating drift.
    let target = scratch.path().join("Report.ts");
    let mut contents = fs::read(&target)?;
    contents.push(b'\n');
    contents.extend_from_slice(b"// hand-edited drift\n");
    fs::write(&target, contents)?;

    let mismatches = diff_against_fresh_export(scratch.path())?;
    assert!(
        mismatches.contains(&"Report.ts".to_owned()),
        "drift comparison must fail closed on a mutated binding, got mismatches: {mismatches:?}"
    );
    Ok(())
}
