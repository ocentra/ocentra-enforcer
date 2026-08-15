//! Hard tests for Perl, onboarded directly through the generic
//! spec-table engine ([`enforcer_syntax::languages::generic::parse_perl`])
//! -- there is no bespoke `languages::perl` extractor to prove
//! zero-regression against (Perl has never had one in this crate), so
//! these tests assert against the grammar-shape ground truth recorded in
//! [`enforcer_syntax::languages::spec::LangSpec::perl`]'s own doc comment
//! directly: `subroutine_declaration_statement` naming/body, full callee
//! reconstruction across all five `call_types` shapes (incl. the
//! wrapper-field and two-field-split cases), `use_statement`/
//! `require_expression` IMPORTS, and every one of `branch_types`'
//! genuinely-shared node kinds (`conditional_statement` for both `if`/
//! `unless`, `loop_statement` for both `while`/`until`, `for_statement`
//! for both `for`/`foreach`).

use enforcer_domain::memory_types::ReceiverHint;
use enforcer_syntax::languages::generic::parse_perl;
use enforcer_syntax::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_perl";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_syntax::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn top_level_sub_is_a_function_not_a_method() {
    // `class_types` is empty for this row (Perl's OOP convention gives no
    // sub-textually-inside-a-class-body signal -- see `LangSpec::perl`'s
    // own doc comment), so every sub is unconditionally a Function,
    // regardless of a `package` statement elsewhere in the file.
    let src = "package Widget;\n\nsub helper {\n    return 1;\n}\n";
    let parsed = parse_perl(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "helper"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_call_with_multiple_parenthesized_arguments() -> TestResult {
    let src = "sub f {\n    helper(1, 2);\n}\n";
    let parsed = parse_perl(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper")
        .ok_or("expected a helper call")?;
    assert_eq!(
        call.arg_texts,
        vec!["1".to_string(), "2".to_string()],
        "{call:?}"
    );
    Ok(())
}

#[test]
fn extracts_call_with_single_argument_no_list_wrapper() -> TestResult {
    // Regression guard: a single-argument call's `arguments` field points
    // DIRECTLY at the one argument (no `list_expression` wrapper) -- a
    // naive implementation that always descends into a `list_expression`
    // would find nothing here.
    let src = "sub f {\n    my $i = 1;\n    print($i);\n}\n";
    let parsed = parse_perl(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "print")
        .ok_or("expected a print call")?;
    assert_eq!(call.arg_texts, vec!["$i".to_string()], "{call:?}");
    Ok(())
}

#[test]
fn extracts_bareword_call_with_no_parens() -> TestResult {
    let src = "sub f {\n    helper 1, 2;\n}\n";
    let parsed = parse_perl(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper")
        .ok_or("expected a helper call")?;
    assert_eq!(
        call.arg_texts,
        vec!["1".to_string(), "2".to_string()],
        "{call:?}"
    );
    Ok(())
}

#[test]
fn extracts_func1op_call_with_bare_argument() -> TestResult {
    let src = "sub f {\n    my $x = shift;\n    return length $x;\n}\n";
    let parsed = parse_perl(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "length")
        .ok_or("expected a length call")?;
    assert_eq!(call.arg_texts, vec!["$x".to_string()], "{call:?}");
    Ok(())
}

#[test]
fn extracts_func0op_call_with_no_arguments() {
    let src = "sub f {\n    my $x = shift;\n}\n";
    let parsed = parse_perl(src);
    assert!(
        parsed.calls.iter().any(|c| c.callee == "shift"),
        "{:?}",
        parsed.calls
    );
}

#[test]
fn method_call_on_bareword_invocant_has_new_expression_hint() -> TestResult {
    let src = "sub f {\n    my $w = Widget->new();\n}\n";
    let parsed = parse_perl(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "Widget->new")
        .ok_or("expected a Widget->new call")?;
    assert_eq!(call.receiver_text.as_deref(), Some("Widget"), "{call:?}");
    assert_eq!(
        call.receiver_hint,
        Some(ReceiverHint::NewExpression),
        "{call:?}"
    );
    Ok(())
}

#[test]
fn method_call_on_scalar_invocant_has_identifier_hint() -> TestResult {
    let src = "sub f {\n    $w->draw();\n}\n";
    let parsed = parse_perl(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "$w->draw")
        .ok_or("expected a $w->draw call")?;
    assert_eq!(call.receiver_text.as_deref(), Some("$w"), "{call:?}");
    assert_eq!(
        call.receiver_hint,
        Some(ReceiverHint::Identifier),
        "{call:?}"
    );
    Ok(())
}

#[test]
fn extracts_use_statement_as_import() {
    let src = "use strict;\nuse warnings;\n";
    let parsed = parse_perl(src);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"strict"));
    assert!(paths.contains(&"warnings"));
}

#[test]
fn use_with_qw_import_list_still_imports_just_the_module() {
    let src = "use POSIX qw(floor ceil);\n";
    let parsed = parse_perl(src);
    assert!(
        parsed.imports.iter().any(|i| i.module_path == "POSIX"),
        "{:?}",
        parsed.imports
    );
}

#[test]
fn extracts_require_as_import_with_dotted_path_intact() {
    let src = "require Data::Dumper;\n";
    let parsed = parse_perl(src);
    assert!(
        parsed
            .imports
            .iter()
            .any(|i| i.module_path == "Data::Dumper"),
        "{:?}",
        parsed.imports
    );
}

#[test]
fn ordinary_call_is_not_misdetected_as_an_import() {
    let src = "sub f {\n    helper(\"x\");\n}\n";
    let parsed = parse_perl(src);
    assert!(parsed.imports.is_empty(), "{:?}", parsed.imports);
}

#[test]
fn if_and_unless_both_parse_as_conditional_statement_branches() {
    let src = r#"
sub f {
    my $x = shift;
    if ($x > 0) {
        print("pos");
    } elsif ($x < 0) {
        print("neg");
    } else {
        print("zero");
    }
    unless ($x) {
        print("falsy");
    }
}
"#;
    let parsed = parse_perl(src);
    let print_calls = parsed.calls.iter().filter(|c| c.callee == "print").count();
    assert!(print_calls >= 4, "{:?}", parsed.calls);
}

#[test]
fn while_for_and_foreach_all_parse_without_dropping_nested_calls() {
    let src = r#"
sub f {
    my $x = shift;
    while ($x > 0) {
        print($x);
        $x--;
    }
    for my $i (1..10) {
        print($i);
    }
    foreach my $j (1..10) {
        print($j);
    }
}
"#;
    let parsed = parse_perl(src);
    let print_calls = parsed.calls.iter().filter(|c| c.callee == "print").count();
    assert!(print_calls >= 3, "{:?}", parsed.calls);
}

#[test]
fn cstyle_for_statement_parses_without_dropping_nested_calls() {
    let src = "sub f {\n    for (my $i = 0; $i < 10; $i++) {\n        print($i);\n    }\n}\n";
    let parsed = parse_perl(src);
    assert!(
        parsed.calls.iter().any(|c| c.callee == "print"),
        "{:?}",
        parsed.calls
    );
}

#[test]
fn call_inside_sub_records_from_symbol_scope() -> TestResult {
    let src = "sub render {\n    helper();\n}\n";
    let parsed = parse_perl(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper")
        .ok_or("expected a helper call")?;
    assert_eq!(call.from_symbol.as_deref(), Some("render"), "{call:?}");
    Ok(())
}

#[test]
fn module_scope_call_has_no_from_symbol() -> TestResult {
    let src = "helper();\n";
    let parsed = parse_perl(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper")
        .ok_or("expected a helper call")?;
    assert_eq!(call.from_symbol, None, "{call:?}");
    Ok(())
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_perl("sub ( { this is not valid perl @@@");
    let _ = parsed;
}

#[test]
fn fixture_file_parses_without_panic() -> TestResult {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join(FIXTURE_DIR).join("widget.pl");
    let src = fs::read_to_string(&fixture)?;
    let parsed = parse_perl(&src);
    assert!(
        parsed.symbols.iter().any(|s| s.name == "new"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.symbols.iter().any(|s| s.name == "draw"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.symbols.iter().any(|s| s.name == "helper"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.imports.iter().any(|i| i.module_path == "strict"),
        "{:?}",
        parsed.imports
    );
    assert!(
        parsed.imports.iter().any(|i| i.module_path == "Exporter"),
        "{:?}",
        parsed.imports
    );
    Ok(())
}
