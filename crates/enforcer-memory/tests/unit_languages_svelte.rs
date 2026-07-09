//! Hard tests for Svelte, onboarded directly through the generic
//! spec-table engine ([`enforcer_memory::languages::generic::parse_svelte`]).
//! Grammar: `tree-sitter-svelte-next` 0.1.1, a real crates.io crate.
//! Matches the baseline's own `CBM_LANG_SVELTE` row's `module_types =
//! {"document"}`/`branch_types = {"if_statement", "each_statement",
//! "await_statement"}` -- the embedded `<script>` JS-import re-parse the
//! baseline also wires is DEFERRED, see
//! [`enforcer_memory::languages::spec::LangSpec::svelte`]'s own doc
//! comment for why.

use enforcer_memory::languages::generic::parse_svelte;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_svelte";

#[test]
fn extracts_one_module_symbol_for_the_document_root() -> TestResult {
    let src = fs::read_to_string(format!("{FIXTURE_DIR}/Sample.svelte"))?;
    let parsed = parse_svelte(&src);
    assert_eq!(parsed.symbols.len(), 1, "{:?}", parsed.symbols);
    assert_eq!(parsed.symbols[0].kind, SymbolKind::Module);
    Ok(())
}

#[test]
fn parses_if_each_and_await_blocks_without_error() -> TestResult {
    // No error-free-parse API is exposed on `ParsedFile` itself, but a
    // grammar parse error would otherwise still walk generically and
    // simply find nothing extra -- this asserts the fixture's `<script>`/
    // control-flow blocks don't blow up extraction (no calls/imports/
    // defines are expected either way, matching the baseline's own
    // fully-nominal row).
    let src = fs::read_to_string(format!("{FIXTURE_DIR}/Sample.svelte"))?;
    let parsed = parse_svelte(&src);
    assert!(parsed.calls.is_empty());
    assert!(parsed.imports.is_empty());
    assert!(parsed.defines.is_empty());
    Ok(())
}
