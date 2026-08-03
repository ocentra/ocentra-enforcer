//! Hard tests for Meson, onboarded directly through the generic
//! spec-table engine
//! ([`enforcer_syntax::languages::generic::parse_meson`]) -- there is no
//! bespoke `languages::meson` extractor to prove zero-regression against,
//! so these tests assert against the grammar-shape ground truth recorded
//! in [`enforcer_syntax::languages::spec::LangSpec::meson`]'s own doc
//! comment directly: `normal_command`'s own `command` field, this
//! language's total lack of a function-definition concept, and branch
//! recognition (`if_command`/`foreach_command`, NOT baseline's stale
//! `if_statement`/`foreach_statement`).

use enforcer_syntax::languages::generic::parse_meson;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_meson";

#[test]
fn extracts_plain_command_call() {
    let src = r#"project('widget', 'c')"#;
    let parsed = parse_meson(src);
    let callees: Vec<&str> = parsed.calls.iter().map(|c| c.callee.as_str()).collect();
    assert!(callees.contains(&"project"));
}

#[test]
fn no_function_symbols_are_ever_produced() {
    // Meson's build DSL has no user-defined-function concept at all --
    // see `LangSpec::meson`'s own doc comment on why `func_types` is
    // empty (a real semantic gap, not a naming correction).
    let src = r#"
project('widget', 'c')
executable('widget', 'widget.c')
"#;
    let parsed = parse_meson(src);
    assert!(parsed.symbols.is_empty(), "{:?}", parsed.symbols);
}

#[test]
fn extracts_call_inside_if_command() {
    let src = r#"
if get_option('buildtype') == 'debug'
  add_project_arguments('-DDEBUG', language: 'c')
endif
"#;
    let parsed = parse_meson(src);
    let callees: Vec<&str> = parsed.calls.iter().map(|c| c.callee.as_str()).collect();
    assert!(callees.contains(&"add_project_arguments"));
    assert!(callees.contains(&"get_option"));
}

#[test]
fn extracts_call_inside_foreach_command() {
    let src = r#"
foreach f : files
  message(f)
endforeach
"#;
    let parsed = parse_meson(src);
    let callees: Vec<&str> = parsed.calls.iter().map(|c| c.callee.as_str()).collect();
    assert!(callees.contains(&"message"));
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_meson("if ( { this is not valid meson @@@");
    let _ = parsed;
}

#[test]
fn fixture_file_parses_without_panic() -> TestResult {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join(FIXTURE_DIR).join("widget.meson");
    let src = fs::read_to_string(&fixture)?;
    let parsed = parse_meson(&src);
    assert!(
        parsed.calls.iter().any(|c| c.callee == "project"),
        "{:?}",
        parsed.calls
    );
    assert!(
        parsed.calls.iter().any(|c| c.callee == "executable"),
        "{:?}",
        parsed.calls
    );
    assert!(
        parsed
            .calls
            .iter()
            .any(|c| c.callee == "add_project_arguments"),
        "{:?}",
        parsed.calls
    );
    Ok(())
}
