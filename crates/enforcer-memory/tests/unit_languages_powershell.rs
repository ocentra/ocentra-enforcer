//! Hard tests for PowerShell, onboarded directly through the generic
//! spec-table engine
//! ([`enforcer_memory::languages::generic::parse_powershell`]) -- there
//! is no bespoke `languages::powershell` extractor to prove
//! zero-regression against (PowerShell has never had one in this
//! crate), so these tests assert against the grammar-shape ground truth
//! recorded in
//! [`enforcer_memory::languages::spec::LangSpec::powershell`]'s own doc
//! comment directly: a `function` statement, a class with a method and
//! `class Dog : Animal` heritage, a `command` call (paren-less, bare
//! space-separated arguments), a member-call `invokation_expression`,
//! and a `using namespace`/`using module` directive recorded as an
//! IMPORTS edge (mirroring the baseline's own `parse_powershell_imports`
//! exactly).

use enforcer_memory::languages::generic::parse_powershell;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_powershell";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_memory::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_function_statement() {
    let src = "function Helper {\n}\n";
    let parsed = parse_powershell(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Helper"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_class_with_method_and_defines_edge() {
    let src = "class Animal {\n    [void] Speak() {\n    }\n}\n";
    let parsed = parse_powershell(src);
    let kinds: Vec<(&str, SymbolKind)> = parsed
        .symbols
        .iter()
        .map(|s| (s.name.as_str(), s.kind))
        .collect();
    assert!(kinds.contains(&("Animal", SymbolKind::Class)), "{kinds:?}");
    assert!(kinds.contains(&("Speak", SymbolKind::Method)), "{kinds:?}");
    assert!(
        parsed
            .defines
            .iter()
            .any(|d| d.container_name == "Animal" && d.member_name == "Speak"),
        "{:?}",
        parsed.defines
    );
}

#[test]
fn extracts_class_heritage_as_inherits() {
    let src = "class Dog : Animal {\n}\n";
    let parsed = parse_powershell(src);
    assert!(
        parsed
            .inherits
            .iter()
            .any(|i| i.sub_name == "Dog" && i.super_name == "Animal"),
        "{:?}",
        parsed.inherits
    );
}

#[test]
fn extracts_enum_statement() {
    let src = "enum Color {\n    Red\n    Green\n}\n";
    let parsed = parse_powershell(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Color"),
        Some(&SymbolKind::Enum),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_bare_command_call() -> TestResult {
    let src = "function Helper {\n    Add-Numbers 1 2\n}\n";
    let parsed = parse_powershell(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "Add-Numbers")
        .ok_or("expected an Add-Numbers command call")?;
    assert_eq!(call.from_symbol.as_deref(), Some("Helper"), "{call:?}");
    assert_eq!(call.arg_texts, vec!["1", "2"], "{call:?}");
    Ok(())
}

#[test]
fn extracts_member_call_as_invokation_expression() -> TestResult {
    let src = "$d.Speak()\n";
    let parsed = parse_powershell(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "Speak")
        .ok_or("expected a Speak member call")?;
    assert_eq!(call.receiver_text.as_deref(), Some("$d"), "{call:?}");
    Ok(())
}

#[test]
fn extracts_using_namespace_as_import() {
    let src = "using namespace System.Collections.Generic\n";
    let parsed = parse_powershell(src);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"System.Collections.Generic"), "{paths:?}");
}

#[test]
fn extracts_using_module_as_import() {
    let src = "using module MyModule\n";
    let parsed = parse_powershell(src);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"MyModule"), "{paths:?}");
}

#[test]
fn using_directive_is_also_recorded_as_a_call() {
    // `using namespace ...`/`using module ...` are ordinary `command`
    // nodes (callee text literally `"using"`) -- the IMPORTS edge is
    // additional, not a replacement, mirroring Zig's identical
    // `@import` builtin-is-also-a-call convention elsewhere in this
    // crate.
    let src = "using namespace System.Collections.Generic\n";
    let parsed = parse_powershell(src);
    let callees: Vec<&str> = parsed.calls.iter().map(|c| c.callee.as_str()).collect();
    assert!(callees.contains(&"using"), "{callees:?}");
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_powershell("function ((( this is not valid powershell @@@");
    let _ = parsed;
}

#[test]
fn fixture_file_parses_without_panic() -> TestResult {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join(FIXTURE_DIR).join("widget.ps1");
    let src = fs::read_to_string(&fixture)?;
    let parsed = parse_powershell(&src);
    assert!(
        parsed.symbols.iter().any(|s| s.name == "Animal"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.symbols.iter().any(|s| s.name == "Dog"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.symbols.iter().any(|s| s.name == "Helper"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed
            .imports
            .iter()
            .any(|i| i.module_path == "System.Collections.Generic"),
        "{:?}",
        parsed.imports
    );
    assert!(
        parsed
            .inherits
            .iter()
            .any(|i| i.sub_name == "Dog" && i.super_name == "Animal"),
        "{:?}",
        parsed.inherits
    );
    Ok(())
}
