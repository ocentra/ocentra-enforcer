//! Hard tests for VHDL, onboarded directly through the generic
//! spec-table engine ([`enforcer_memory::languages::generic::parse_vhdl`])
//! -- there is no bespoke `languages::vhdl` extractor to prove
//! zero-regression against (VHDL has never had one in this crate), so
//! these tests assert against the grammar-shape ground truth recorded in
//! [`enforcer_memory::languages::spec::LangSpec::vhdl`]'s own doc comment
//! directly: `entity_declaration`'s own `[entity]` field,
//! `subprogram_declaration`/`_definition`'s own nested
//! `function_specification`/`[function]`-field name resolution,
//! `parenthesis_group`'s own preceding-sibling callee reconstruction,
//! `component_instantiation_statement`'s own real `[component]` field,
//! and `library_clause`/`use_clause`'s own field-less dotted-path
//! IMPORTS.

use enforcer_memory::languages::generic::parse_vhdl;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_vhdl";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_memory::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_entity_via_entity_field() {
    let src = "entity widget is\nend entity widget;\n";
    let parsed = parse_vhdl(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "widget"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_architecture_via_architecture_field() {
    let src = "entity widget is\nend entity widget;\narchitecture rtl of widget is\nbegin\nend architecture rtl;\n";
    let parsed = parse_vhdl(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "rtl"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_subprogram_via_nested_specification_field() {
    let src = "entity widget is\nend entity widget;\narchitecture rtl of widget is\n  function helper(x : integer) return integer is\n  begin\n    return x + 1;\n  end function helper;\nbegin\nend architecture rtl;\n";
    let parsed = parse_vhdl(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "helper"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn parenthesis_group_callee_uses_preceding_sibling() -> TestResult {
    let src = "entity widget is\nend entity widget;\narchitecture rtl of widget is\n  function helper(x : integer) return integer is\n  begin\n    return x + 1;\n  end function helper;\nbegin\n  process\n  begin\n    if helper(1) > 0 then\n      report \"ok\";\n    end if;\n  end process;\nend architecture rtl;\n";
    let parsed = parse_vhdl(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper")
        .ok_or("expected a helper call")?;
    assert!(call.line > 0, "{call:?}");
    Ok(())
}

#[test]
fn component_instantiation_uses_component_field() -> TestResult {
    let src = "entity widget is\nend entity widget;\narchitecture rtl of widget is\nbegin\n  inst1: component_name port map (a => b);\nend architecture rtl;\n";
    let parsed = parse_vhdl(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "component_name")
        .ok_or("expected a component_name call")?;
    assert!(call.line > 0, "{call:?}");
    Ok(())
}

#[test]
fn library_clause_records_library_name_as_import() -> TestResult {
    let src = "library IEEE;\nentity widget is\nend entity widget;\n";
    let parsed = parse_vhdl(src);
    let import = parsed
        .imports
        .iter()
        .find(|i| i.module_path == "IEEE")
        .ok_or("expected an IEEE import")?;
    assert!(import.line > 0, "{import:?}");
    Ok(())
}

#[test]
fn use_clause_records_dotted_path_minus_all_suffix() -> TestResult {
    let src = "library IEEE;\nuse IEEE.STD_LOGIC_1164.ALL;\nentity widget is\nend entity widget;\n";
    let parsed = parse_vhdl(src);
    let import = parsed
        .imports
        .iter()
        .find(|i| i.module_path == "IEEE.STD_LOGIC_1164")
        .ok_or("expected an IEEE.STD_LOGIC_1164 import")?;
    assert!(import.line > 0, "{import:?}");
    Ok(())
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_vhdl("entity @@@ this is not valid vhdl ###");
    let _ = parsed;
}

#[test]
fn fixture_file_parses_without_panic() -> TestResult {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join(FIXTURE_DIR).join("widget.vhd");
    let src = fs::read_to_string(&fixture)?;
    let parsed = parse_vhdl(&src);
    assert!(
        parsed.symbols.iter().any(|s| s.name == "widget"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.symbols.iter().any(|s| s.name == "helper"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed
            .imports
            .iter()
            .any(|i| i.module_path == "IEEE.STD_LOGIC_1164"),
        "{:?}",
        parsed.imports
    );
    assert!(
        parsed.calls.iter().any(|c| c.callee == "helper"),
        "{:?}",
        parsed.calls
    );
    Ok(())
}
