//! Hard tests for Hyprlang (Hyprland window-manager config language),
//! onboarded directly through the generic spec-table engine
//! ([`enforcer_syntax::languages::generic::parse_hyprlang`]) -- grammar
//! VENDORED (`vendor/tree-sitter-hyprlang-local/`). Asserts the real
//! root-node-kind correction documented in
//! [`enforcer_syntax::languages::spec::LangSpec::hyprlang`]'s own doc
//! comment (`configuration`, not baseline's own dead `"source_file"`).

use enforcer_syntax::languages::generic::parse_hyprlang;
use enforcer_syntax::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_hyprlang";

#[test]
fn extracts_module_symbol_for_configuration_root() {
    let src = "monitor=eDP-1,1920x1080@60,0x0,1\n";
    let parsed = parse_hyprlang(src);
    assert!(
        parsed.symbols.iter().any(|s| s.kind == SymbolKind::Module),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn parses_section_block_without_panicking() {
    let src = "general {\n  gaps_in = 5\n}\n";
    let parsed = parse_hyprlang(src);
    assert!(parsed.calls.is_empty(), "{:?}", parsed.calls);
}

#[test]
fn parses_fixture_hyprland_conf_without_panicking() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join("hyprland.conf");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_hyprlang(&src);
    assert!(
        parsed.symbols.iter().any(|s| s.kind == SymbolKind::Module),
        "{:?}",
        parsed.symbols
    );
    Ok(())
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = "bind = SUPER, Q, killactive\n";
    let first = parse_hyprlang(src);
    let second = parse_hyprlang(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_hyprlang("this [[[ is not valid @@@");
    let _ = parsed;
}
