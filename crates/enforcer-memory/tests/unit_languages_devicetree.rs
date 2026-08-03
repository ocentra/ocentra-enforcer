//! Hard tests for DeviceTree, onboarded directly through the generic
//! spec-table engine
//! ([`enforcer_syntax::languages::generic::parse_devicetree`]). Asserts
//! against the grammar-shape ground truth recorded in
//! [`enforcer_syntax::languages::spec::LangSpec::devicetree`]'s own doc
//! comment: real `call_expression` `function`/`arguments` fields, and
//! `dtsi_include`/`preproc_include` IMPORTS via
//! [`enforcer_syntax::languages::generic::devicetree_quirk`] (the
//! generic walker's own `import_types` branch has no field-driven
//! default of its own at all).

use enforcer_syntax::languages::generic::parse_devicetree;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_devicetree";

#[test]
fn extracts_preproc_include_as_import() -> TestResult {
    let src = "/dts-v1/;\n#include \"board-common.dtsi\"\n/ {\n};\n";
    let parsed = parse_devicetree(src);
    let import = parsed
        .imports
        .iter()
        .find(|i| i.module_path.contains("board-common.dtsi"))
        .ok_or("expected a board-common.dtsi import")?;
    let _ = import;
    Ok(())
}

#[test]
fn extracts_call_expression_via_real_fields() -> TestResult {
    // `call_expression` is only reachable inside an `integer_cells`
    // (`< ... >`) property value in this grammar's own `grammar.js`
    // (`_integer_cell_items` is the only production listing
    // `$.call_expression`) -- a bare, unbracketed `foo = FOO(1, 2);` is
    // a genuine parse ERROR (confirmed directly via a real parse-tree
    // dump: `_property_value` only ever accepts `integer_cells`/
    // `string_literal`/`byte_string_literal`/`reference`/`incbin`,
    // never a bare expression), so this fixture wraps the macro-call in
    // angle brackets to reach the real node.
    let src = "/dts-v1/;\n/ {\n    foo = <FOO(1, 2)>;\n};\n";
    let parsed = parse_devicetree(src);
    parsed
        .calls
        .iter()
        .find(|c| c.callee == "FOO")
        .ok_or("expected a FOO call")?;
    Ok(())
}

#[test]
fn extracts_module_symbol_for_document_root() {
    let src = "/dts-v1/;\n/ {\n};\n";
    let parsed = parse_devicetree(src);
    assert!(!parsed.symbols.is_empty(), "{:?}", parsed.symbols);
}

#[test]
fn parses_fixture_board_without_panicking() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join("board.dts");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_devicetree(&src);
    assert!(
        parsed
            .imports
            .iter()
            .any(|i| i.module_path.contains("board-common.dtsi")),
        "{:?}",
        parsed.imports
    );
    Ok(())
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = "/dts-v1/;\n/ {\n};\n";
    let first = parse_devicetree(src);
    let second = parse_devicetree(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_devicetree("this is not valid devicetree @@@ /// ###");
    let _ = parsed;
}
