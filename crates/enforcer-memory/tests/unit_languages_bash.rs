//! Hard tests for Bash, onboarded directly through the generic
//! spec-table engine ([`enforcer_syntax::languages::generic::parse_bash`])
//! -- there is no bespoke `languages::bash` extractor to prove
//! zero-regression against (Bash has never had one in this crate), so
//! these tests assert against the grammar-shape ground truth recorded in
//! [`enforcer_syntax::languages::spec::LangSpec::bash`]'s own doc comment
//! directly: `function`-keyword and bare-POSIX `function_definition`
//! naming, `command`-node CALLS with `argument`-field `arg_texts`,
//! `source`/`.` IMPORTS detection, and `if`/`while`/`for`/`case` branch
//! recognition (exercised indirectly via fixture parsing -- this crate's
//! own `ParsedFile` shape has no complexity field, see
//! `crates/enforcer-memory/src/parsers/mod.rs`'s module doc).

use enforcer_syntax::languages::generic::parse_bash;
use enforcer_syntax::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_bash";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_syntax::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_function_keyword_definition() {
    let src = r#"
function greet() {
  echo "hi"
}
"#;
    let parsed = parse_bash(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "greet"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_bare_posix_definition() {
    // No `function` keyword -- still the same `function_definition` node
    // kind, confirmed via a real parse tree dump.
    let src = r#"
greet() {
  echo "hi"
}
"#;
    let parsed = parse_bash(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "greet"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_command_call() -> TestResult {
    let src = "greet world\n";
    let parsed = parse_bash(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "greet")
        .ok_or("expected a greet call")?;
    assert_eq!(call.arg_texts, vec!["world".to_string()], "{call:?}");
    Ok(())
}

#[test]
fn call_with_multiple_arguments_records_arg_texts() -> TestResult {
    let src = "greet world again\n";
    let parsed = parse_bash(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "greet")
        .ok_or("expected a greet call")?;
    assert_eq!(
        call.arg_texts,
        vec!["world".to_string(), "again".to_string()],
        "{call:?}"
    );
    Ok(())
}

#[test]
fn call_inside_function_records_from_symbol_scope() -> TestResult {
    let src = r#"
render() {
  greet world
}
"#;
    let parsed = parse_bash(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "greet")
        .ok_or("expected a greet call")?;
    assert_eq!(call.from_symbol.as_deref(), Some("render"), "{call:?}");
    Ok(())
}

#[test]
fn module_scope_call_has_no_from_symbol() -> TestResult {
    let src = "greet world\n";
    let parsed = parse_bash(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "greet")
        .ok_or("expected a greet call")?;
    assert_eq!(call.from_symbol, None, "{call:?}");
    Ok(())
}

#[test]
fn extracts_source_command_as_import() {
    let src = "source ./lib.sh\n";
    let parsed = parse_bash(src);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"./lib.sh"));
}

#[test]
fn extracts_dot_command_as_import() {
    let src = ". ./other.sh\n";
    let parsed = parse_bash(src);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"./other.sh"));
}

#[test]
fn source_command_is_also_recorded_as_a_call() {
    let src = "source ./lib.sh\n";
    let parsed = parse_bash(src);
    let callees: Vec<&str> = parsed.calls.iter().map(|c| c.callee.as_str()).collect();
    assert!(callees.contains(&"source"));
}

#[test]
fn ordinary_command_is_not_misdetected_as_an_import() {
    let src = "echo hi\n";
    let parsed = parse_bash(src);
    assert!(parsed.imports.is_empty(), "{:?}", parsed.imports);
}

#[test]
fn extracts_calls_inside_if_branches() {
    let src = r#"
if [ -f x ]; then
  greet yes
else
  greet no
fi
"#;
    let parsed = parse_bash(src);
    let callees: Vec<&str> = parsed.calls.iter().map(|c| c.callee.as_str()).collect();
    assert!(callees.contains(&"greet"));
    assert_eq!(callees.iter().filter(|c| **c == "greet").count(), 2);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_bash("function ( { this is not valid bash @@@");
    let _ = parsed;
}

#[test]
fn supplementary_plane_input_is_rejected_before_native_scanner() {
    assert_eq!(parse_bash("\u{c44ab}\\"), Default::default());
}

#[test]
fn fixture_file_parses_without_panic() -> TestResult {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join(FIXTURE_DIR).join("widget.sh");
    let src = fs::read_to_string(&fixture)?;
    let parsed = parse_bash(&src);
    assert!(
        parsed.symbols.iter().any(|s| s.name == "greet"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.symbols.iter().any(|s| s.name == "draw"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.imports.iter().any(|i| i.module_path == "./lib.sh"),
        "{:?}",
        parsed.imports
    );
    assert!(
        parsed.calls.iter().any(|c| c.callee == "greet"),
        "{:?}",
        parsed.calls
    );
    Ok(())
}
