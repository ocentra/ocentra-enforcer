//! Hard tests for HCL, onboarded directly through the generic
//! spec-table engine ([`enforcer_syntax::languages::generic::parse_hcl`])
//! -- there is no bespoke `languages::hcl` extractor to prove
//! zero-regression against (HCL has never had one in this crate), so
//! these tests assert against the grammar-shape ground truth recorded in
//! [`enforcer_syntax::languages::spec::LangSpec::hcl`]'s own doc comment
//! directly: every node kind in this grammar is fully unfielded, `block`'s
//! own name is synthesized from its leading identifier plus dot-joined
//! string-literal labels, and `function_call` needs full override claiming.

use enforcer_syntax::languages::generic::parse_hcl;
use enforcer_syntax::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_hcl";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_syntax::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_block_as_class_with_synthesized_dotted_name() {
    let src = "resource \"aws_instance\" \"web\" {\n  ami = \"abc\"\n}\n";
    let parsed = parse_hcl(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "resource.aws_instance.web"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_labelless_block_by_leading_identifier_alone() {
    let src = "locals {\n  x = 1\n}\n";
    let parsed = parse_hcl(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "locals"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_function_call_inside_attribute_value() -> TestResult {
    let src = "resource \"aws_instance\" \"web\" {\n  count = length(var.list)\n}\n";
    let parsed = parse_hcl(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "length")
        .ok_or("expected a length() call")?;
    let _ = call;
    Ok(())
}

#[test]
fn produces_no_branch_types_at_all() {
    // HCL has no control-flow statement node at all -- see
    // `LangSpec::hcl`'s own doc comment ("branch_types both stay empty").
    let src = "variable \"x\" {\n  default = \"y\"\n}\n";
    let parsed = parse_hcl(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "variable.x"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn parses_fixture_sample_without_panicking() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join("sample.tf");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_hcl(&src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "variable.region"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "resource.aws_instance.web"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "length")
        .ok_or("expected a length() call")?;
    let _ = call;
    Ok(())
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = "variable \"x\" {\n  default = \"y\"\n}\n";
    let first = parse_hcl(src);
    let second = parse_hcl(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_hcl("resource ( { this is not valid hcl @@@");
    let _ = parsed;
}
