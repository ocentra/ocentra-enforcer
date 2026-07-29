//! Hard tests for SCSS, onboarded directly through the generic
//! spec-table engine ([`enforcer_memory::languages::generic::parse_scss`])
//! -- there is no bespoke `languages::scss` extractor to prove
//! zero-regression against (SCSS has never had one in this crate), so
//! these tests assert against the grammar-shape ground truth recorded in
//! [`enforcer_memory::languages::spec::LangSpec::scss`]'s own doc comment
//! directly: `function_statement`/`mixin_statement`'s own real `name`
//! field (no quirk needed for the def-name case, contradicting the
//! baseline's own doc comment), their unfielded `block` body child,
//! `call_expression`'s unfielded `function_name` callee, and
//! `import_statement`/`use_statement`'s quote-stripped `string_value`
//! path.

use enforcer_memory::languages::generic::parse_scss;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_scss";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_memory::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_mixin_statement_as_function() {
    let src = "@mixin flex-center {\n  display: flex;\n}\n";
    let parsed = parse_scss(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "flex-center"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_function_statement_as_function() {
    let src = "@function double($x) {\n  @return $x * 2;\n}\n";
    let parsed = parse_scss(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "double"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_import_statement_path_unquoted() -> TestResult {
    let src = "@import \"base\";\n";
    let parsed = parse_scss(src);
    let path = parsed
        .imports
        .first()
        .ok_or("expected an import")?
        .module_path
        .as_str();
    assert_eq!(path, "base", "{:?}", parsed.imports);
    Ok(())
}

#[test]
fn extracts_use_statement_path_unquoted() -> TestResult {
    let src = "@use \"sass:math\";\n";
    let parsed = parse_scss(src);
    let path = parsed
        .imports
        .first()
        .ok_or("expected an import")?
        .module_path
        .as_str();
    assert_eq!(path, "sass:math", "{:?}", parsed.imports);
    Ok(())
}

#[test]
fn extracts_call_expression_callee_from_unfielded_function_name() -> TestResult {
    let src = ".widget {\n  color: double($primary);\n}\n";
    let parsed = parse_scss(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "double")
        .ok_or("expected a double call")?;
    assert_eq!(call.arg_texts, vec!["$primary".to_string()], "{call:?}");
    Ok(())
}

#[test]
fn extracts_include_statement_as_call_to_mixin_name() -> TestResult {
    let src = ".widget {\n  @include flex-center;\n}\n";
    let parsed = parse_scss(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "flex-center")
        .ok_or("expected an include call")?;
    let _ = call;
    Ok(())
}

#[test]
fn call_inside_mixin_body_is_found_via_unfielded_block_recursion() -> TestResult {
    // `mixin_statement`'s own body-holding child (`block`) is unfielded
    // (see `LangSpec::scss`'s own doc comment) -- this asserts the quirk's
    // kind-search recursion actually reaches a nested call, not just that
    // the mixin symbol itself is recorded.
    let src = "@mixin wrapper {\n  @include flex-center;\n}\n";
    let parsed = parse_scss(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "flex-center")
        .ok_or("expected a nested include call inside the mixin body")?;
    assert_eq!(call.from_symbol.as_deref(), Some("wrapper"), "{call:?}");
    Ok(())
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_scss("@mixin ??? this is not valid scss @@@");
    let _ = parsed;
}

#[test]
fn fixture_file_parses_without_panic() -> TestResult {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join(FIXTURE_DIR).join("widget.scss");
    let src = fs::read_to_string(&fixture)?;
    let parsed = parse_scss(&src);
    assert!(
        parsed.symbols.iter().any(|s| s.name == "flex-center"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.symbols.iter().any(|s| s.name == "double"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.imports.iter().any(|i| i.module_path == "base"),
        "{:?}",
        parsed.imports
    );
    assert!(
        parsed.calls.iter().any(|c| c.callee == "double"),
        "{:?}",
        parsed.calls
    );
    Ok(())
}
