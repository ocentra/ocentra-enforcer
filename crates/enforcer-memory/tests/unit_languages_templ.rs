//! Hard tests for Templ, onboarded directly through the generic
//! spec-table engine ([`enforcer_memory::languages::generic::parse_templ`])
//! -- there is no bespoke `languages::templ` extractor to prove
//! zero-regression against (Templ has never had one in this crate), so
//! these tests assert against the grammar-shape ground truth recorded in
//! [`enforcer_memory::languages::spec::LangSpec::templ`]'s own doc comment
//! directly: Go-grammar-shaped `function_declaration`/`call_expression`
//! fields, `import_declaration`'s grouped-import IMPORTS (the baseline's
//! own `templ_import_types` array's `"import"` entry is a phantom unnamed
//! token, not a real node -- see that doc comment for the full finding).

use enforcer_memory::languages::generic::parse_templ;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_templ";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_memory::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_function_declaration() {
    let src = r#"
package widget

func helper(x int) int {
	return x + 1
}
"#;
    let parsed = parse_templ(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "helper"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_import_declaration_as_imports_edge() {
    let src = r#"
package widget

import "fmt"
"#;
    let parsed = parse_templ(src);
    assert!(
        parsed.imports.iter().any(|i| i.module_path == "fmt"),
        "{:?}",
        parsed.imports
    );
}

#[test]
fn extracts_grouped_import_declaration() {
    let src = r#"
package widget

import (
	"fmt"
	"strings"
)
"#;
    let parsed = parse_templ(src);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"fmt"));
    assert!(paths.contains(&"strings"));
}

#[test]
fn extracts_function_call_with_real_fields() -> TestResult {
    let src = r#"
package widget

func draw(x int) int {
	return helper(x)
}
"#;
    let parsed = parse_templ(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper")
        .ok_or("expected a helper call")?;
    assert_eq!(call.from_symbol.as_deref(), Some("draw"), "{call:?}");
    Ok(())
}

#[test]
fn if_statement_is_recognized_as_a_branch_node() {
    let src = r#"
package widget

func draw(x int) int {
	if x > 0 {
		return helper(x)
	}
	return helper(x)
}
"#;
    let parsed = parse_templ(src);
    let helper_calls = parsed.calls.iter().filter(|c| c.callee == "helper").count();
    assert_eq!(helper_calls, 2, "{:?}", parsed.calls);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_templ("func ( { this is not valid templ @@@");
    let _ = parsed;
}

#[test]
fn fixture_file_parses_without_panic() -> TestResult {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join(FIXTURE_DIR).join("widget.templ");
    let src = fs::read_to_string(&fixture)?;
    let parsed = parse_templ(&src);
    assert!(
        parsed.symbols.iter().any(|s| s.name == "helper"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.imports.iter().any(|i| i.module_path == "fmt"),
        "{:?}",
        parsed.imports
    );
    Ok(())
}
