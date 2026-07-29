//! Hard tests for SSH client config files, onboarded directly through
//! the generic spec-table engine
//! ([`enforcer_memory::languages::generic::parse_sshconfig`]). Grammar
//! VENDORED (`vendor/tree-sitter-sshclientconfig-local/`) -- the
//! published `tree-sitter-ssh-client-config` crate's own binding pins an
//! incompatible `tree-sitter` version, see
//! [`enforcer_memory::languages::spec::LangSpec::sshconfig`]'s own doc
//! comment (including why the real root node kind is `client_config`,
//! not the baseline's own `source_file`).

use enforcer_memory::languages::generic::parse_sshconfig;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_sshconfig";

#[test]
fn extracts_one_module_symbol_for_the_client_config_root() -> TestResult {
    let src = fs::read_to_string(format!("{FIXTURE_DIR}/config"))?;
    let parsed = parse_sshconfig(&src);
    assert_eq!(parsed.symbols.len(), 1, "{:?}", parsed.symbols);
    assert_eq!(parsed.symbols[0].kind, SymbolKind::Module);
    Ok(())
}

#[test]
fn extracts_no_calls_imports_or_defines() -> TestResult {
    let src = fs::read_to_string(format!("{FIXTURE_DIR}/config"))?;
    let parsed = parse_sshconfig(&src);
    assert!(parsed.calls.is_empty());
    assert!(parsed.imports.is_empty());
    assert!(parsed.defines.is_empty());
    Ok(())
}
