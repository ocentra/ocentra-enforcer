//! c08 acceptance-row proof (`adapter-stub-contract`,
//! `TEST_PROOF_EXPECTATIONS.md`): iterates all three Track C stubs
//! (gemini, cursor, zed) and asserts each conforms to the
//! [`enforcer_install::core::HarnessAdapter`] trait, returns a deferred
//! (never-fails) `Status`-equivalent shape, and performs ZERO filesystem
//! writes when `apply`-ed against a temp-dir copy of the checked-in
//! `tests/fixtures/stubs/pristine/**` fixture. Registry lookup for each
//! harness key must resolve (`Ok`, no panic) via `select_adapters`
//! (`enforcer_install::core`'s `--only <harness>` narrowing), proving a
//! stub's key is a first-class member of the adapter set rather than a
//! silent gap.
//!
//! Every fixture under `tests/fixtures/stubs/**` is COPIED into an
//! isolated `tempfile::tempdir()` before a test touches it — this file
//! never writes into the checked-in fixture tree.

use enforcer_install::adapters::cursor::CursorAdapter;
use enforcer_install::adapters::gemini::GeminiAdapter;
use enforcer_install::adapters::zed::ZedAdapter;
use enforcer_install::cli_contract::{InstallRequest, RequestContext};
use enforcer_install::core::{install, HarnessAdapter};
use std::path::{Path, PathBuf};

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/stubs/pristine")
}

/// Recursively copy `src` into `dst` (both directories), creating `dst`.
fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let target = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

/// Snapshot every regular file's (relative path, contents) pair under
/// `root`, for a before/after zero-write diff.
fn snapshot(root: &Path) -> std::io::Result<Vec<(PathBuf, Vec<u8>)>> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            let bytes = std::fs::read(&path)?;
            out.push((path, bytes));
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
}

#[test]
fn every_stub_conforms_to_the_harness_adapter_trait_and_writes_zero_files(
) -> Result<(), Box<dyn std::error::Error>> {
    let gemini = GeminiAdapter::new();
    let cursor = CursorAdapter::new();
    let zed = ZedAdapter::new();
    let stubs: Vec<&dyn HarnessAdapter> = vec![&gemini, &cursor, &zed];

    for stub in &stubs {
        let dir = tempfile::tempdir()?;
        copy_dir_all(&fixture_root(), dir.path())?;
        let before = snapshot(dir.path())?;

        let ctx = RequestContext::with_defaults(PathBuf::from("/abs/path/to/enforcer"));

        // plan/apply/verify must all succeed (Ok, never a panic) --
        // Adapter-trait conformance.
        let plan = stub.plan(&ctx)?;
        assert!(
            plan.is_noop(),
            "{} stub plan must produce zero planned changes",
            stub.harness_key()
        );
        assert!(
            !plan.warnings.is_empty(),
            "{} stub plan must record a deferred reason",
            stub.harness_key()
        );
        assert!(
            plan.warnings.iter().any(|w| w.contains("deferred")),
            "{} stub plan warning must state it is deferred",
            stub.harness_key()
        );

        let applied = stub.apply(&plan)?;
        assert!(
            applied.applied.is_empty(),
            "{} stub apply must perform zero writes",
            stub.harness_key()
        );

        let verify = stub.verify(&ctx)?;
        assert_eq!(
            verify.checks.len(),
            1,
            "{} stub verify must report exactly one advisory check",
            stub.harness_key()
        );
        assert!(
            verify.all_passed(),
            "{} stub verify check must be advisory (passed), not Error-severity",
            stub.harness_key()
        );
        assert!(
            verify.checks[0]
                .detail
                .contains("deferred: no mechanization yet"),
            "{} stub verify detail must state the T3 deferred reason",
            stub.harness_key()
        );

        let after = snapshot(dir.path())?;
        assert_eq!(
            before,
            after,
            "{} stub must leave the fixture directory byte-identical",
            stub.harness_key()
        );
    }
    Ok(())
}

#[test]
fn each_stub_harness_key_resolves_through_the_adapter_registry(
) -> Result<(), Box<dyn std::error::Error>> {
    let gemini = GeminiAdapter::new();
    let cursor = CursorAdapter::new();
    let zed = ZedAdapter::new();
    let adapters: Vec<&dyn HarnessAdapter> = vec![&gemini, &cursor, &zed];

    for key in ["gemini", "cursor", "zed"] {
        let request = InstallRequest {
            context: RequestContext::with_defaults(PathBuf::from("/abs/path/to/enforcer")),
            only_harnesses: vec![key.to_owned()],
        };
        let outcomes = install(&adapters, &request)?;
        assert_eq!(
            outcomes.len(),
            1,
            "registry lookup for `{key}` must resolve to exactly one adapter"
        );
        assert_eq!(outcomes[0].0, key);
    }
    Ok(())
}

#[test]
fn an_unregistered_harness_key_is_a_typed_error_not_a_silent_skip() {
    let gemini = GeminiAdapter::new();
    let cursor = CursorAdapter::new();
    let zed = ZedAdapter::new();
    let adapters: Vec<&dyn HarnessAdapter> = vec![&gemini, &cursor, &zed];

    let request = InstallRequest {
        context: RequestContext::with_defaults(PathBuf::from("/abs/path/to/enforcer")),
        only_harnesses: vec!["not-a-real-harness".to_owned()],
    };
    let result = install(&adapters, &request);
    assert!(result.is_err());
}
