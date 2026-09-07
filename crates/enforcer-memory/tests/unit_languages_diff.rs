//! Hard tests for Diff/patch, onboarded directly through the generic
//! spec-table engine
//! ([`enforcer_syntax::languages::generic::parse_diff`]). Tier-0 (see
//! [`enforcer_syntax::languages::spec::LangSpec::diff`]'s own doc
//! comment): `command` is fully fieldless (confirmed via a real
//! `node-types.json` dump), so
//! [`enforcer_syntax::languages::generic::diff_call_override`] reads it
//! positionally -- a nominal reuse of the call-edge shape for the
//! `diff --git ...` header line, matching baseline's own choice.

use enforcer_syntax::languages::generic::parse_diff;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_diff";

#[test]
fn extracts_module_symbol_for_source_root() {
    let src = "diff --git a/x b/x\n--- a/x\n+++ b/x\n@@ -1 +1 @@\n-a\n+b\n";
    let parsed = parse_diff(src);
    assert!(!parsed.symbols.is_empty(), "{:?}", parsed.symbols);
}

#[test]
fn extracts_git_diff_header_command_as_call() {
    let src = "diff --git a/x b/x\n--- a/x\n+++ b/x\n@@ -1 +1 @@\n-a\n+b\n";
    let parsed = parse_diff(src);
    assert!(
        parsed.calls.iter().any(|c| c.callee == "diff"),
        "{:?}",
        parsed.calls
    );
}

#[test]
fn parses_fixture_without_panicking() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join("sample.diff");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_diff(&src);
    assert!(!parsed.symbols.is_empty(), "{:?}", parsed.symbols);
    assert!(
        parsed.calls.iter().any(|c| c.callee == "diff"),
        "{:?}",
        parsed.calls
    );
    Ok(())
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = "diff --git a/x b/x\n--- a/x\n+++ b/x\n@@ -1 +1 @@\n-a\n+b\n";
    let first = parse_diff(src);
    let second = parse_diff(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_diff("not really a diff @@@ ###");
    let _ = parsed;
}
