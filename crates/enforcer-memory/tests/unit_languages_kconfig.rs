//! Hard tests for Kconfig, onboarded directly through the generic
//! spec-table engine
//! ([`enforcer_memory::languages::generic::parse_kconfig`]) -- there is
//! no bespoke `languages::kconfig` extractor to prove zero-regression
//! against, so these tests assert against the grammar-shape ground truth
//! recorded in [`enforcer_memory::languages::spec::LangSpec::kconfig`]'s
//! own doc comment directly: `config`/`menuconfig`/`choice` symbol
//! extraction (NOT baseline's own `type_definition`, which has no name
//! field at all), and `source`'s own quirk-claimed IMPORTS edge.

use enforcer_memory::languages::generic::parse_kconfig;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_kconfig";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_memory::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_config_symbol() {
    let src = r#"
config WIDGET
	bool "Enable widget support"
	default y
"#;
    let parsed = parse_kconfig(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "WIDGET"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_menuconfig_symbol() {
    let src = r#"
menuconfig WIDGET_ADVANCED
	bool "Advanced widget options"
"#;
    let parsed = parse_kconfig(src);
    assert!(
        parsed.symbols.iter().any(|s| s.name == "WIDGET_ADVANCED"),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_choice_and_nested_config() {
    let src = r#"
choice
	prompt "Widget backend"

config WIDGET_BACKEND_SOFTWARE
	bool "Software backend"

endchoice
"#;
    let parsed = parse_kconfig(src);
    assert!(
        parsed
            .symbols
            .iter()
            .any(|s| s.name == "WIDGET_BACKEND_SOFTWARE"),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_source_as_import() {
    let src = r#"source "drivers/widget/Kconfig""#;
    let parsed = parse_kconfig(src);
    assert!(
        parsed
            .imports
            .iter()
            .any(|i| i.module_path == "drivers/widget/Kconfig"),
        "{:?}",
        parsed.imports
    );
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_kconfig("config ( { this is not valid kconfig @@@");
    let _ = parsed;
}

#[test]
fn fixture_file_parses_without_panic() -> TestResult {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join(FIXTURE_DIR).join("Kconfig.widget");
    let src = fs::read_to_string(&fixture)?;
    let parsed = parse_kconfig(&src);
    assert!(
        parsed.symbols.iter().any(|s| s.name == "WIDGET"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed
            .symbols
            .iter()
            .any(|s| s.name == "WIDGET_BACKEND_SOFTWARE"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed
            .imports
            .iter()
            .any(|i| i.module_path == "drivers/widget/Kconfig"),
        "{:?}",
        parsed.imports
    );
    Ok(())
}
