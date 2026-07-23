//! Hard tests for Elixir, onboarded directly through the generic
//! spec-table engine
//! ([`enforcer_memory::languages::generic::parse_elixir`]) -- there is
//! no bespoke `languages::elixir` extractor to prove zero-regression
//! against (Elixir has never had one in this crate), so these tests
//! assert against the grammar-shape ground truth recorded in
//! [`enforcer_memory::languages::spec::LangSpec::elixir`]'s own doc
//! comment directly: `defmodule` as a Class symbol, `def`/`defp` as
//! Function symbols (incl. the guard-clause `when` unwrap -- a
//! deliberate improvement over the baseline's own
//! `extract_elixir_func_def`, which drops a guarded `def`'s name
//! entirely), `alias`/`import`/`use`/`require` IMPORTS, and ordinary
//! `identifier`/`dot`-target calls.

use enforcer_memory::languages::generic::parse_elixir;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_elixir";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_memory::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_defmodule_as_class_symbol() {
    let src = r#"
defmodule Widget do
end
"#;
    let parsed = parse_elixir(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Widget"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_dotted_module_name() {
    let src = r#"
defmodule Widget.Sub do
end
"#;
    let parsed = parse_elixir(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Widget.Sub"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_def_as_function_symbol() {
    let src = r#"
defmodule Widget do
  def draw(name) do
    name
  end
end
"#;
    let parsed = parse_elixir(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "draw"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_defp_as_function_symbol() {
    let src = r#"
defmodule Widget do
  defp helper(label) do
    label
  end
end
"#;
    let parsed = parse_elixir(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "helper"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_zero_arg_def_with_no_parens() {
    let src = r#"
defmodule Widget do
  def go do
    1
  end
end
"#;
    let parsed = parse_elixir(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "go"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_guarded_def_name_a_deliberate_improvement_over_baseline() {
    // The C baseline's own `extract_elixir_func_def` only ever checks
    // for a bare `call`/`identifier` first-argument shape -- a guarded
    // `def bar(x) when x > 0 do` first-argument is neither (it is a
    // `binary_operator` with operator `"when"`), so the baseline itself
    // silently drops this def's name entirely and never extracts it.
    // This crate's own `elixir_def_name` unwraps exactly one such layer
    // before applying the same check, recovering the name.
    let src = r#"
defmodule Widget do
  def bar(x) when x > 0 do
    x + 1
  end
end
"#;
    let parsed = parse_elixir(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "bar"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn multi_clause_function_registers_both_clauses_under_the_same_name() {
    let src = r#"
defmodule Widget do
  def draw(name) when name != "" do
    name
  end

  def draw(_name) do
    "unnamed"
  end
end
"#;
    let parsed = parse_elixir(src);
    let draw_count = parsed.symbols.iter().filter(|s| s.name == "draw").count();
    assert_eq!(draw_count, 2, "{:?}", parsed.symbols);
}

#[test]
fn extracts_alias_as_import() {
    let src = r#"
defmodule Widget do
  alias Bar.Baz
end
"#;
    let parsed = parse_elixir(src);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"Bar.Baz"));
}

#[test]
fn extracts_import_use_require_as_imports() {
    let src = r#"
defmodule Widget do
  import Enum
  use GenServer
  require Logger
end
"#;
    let parsed = parse_elixir(src);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"Enum"));
    assert!(paths.contains(&"GenServer"));
    assert!(paths.contains(&"Logger"));
}

#[test]
fn extracts_ordinary_bare_identifier_call() -> TestResult {
    let src = r#"
defmodule Widget do
  def go do
    helper()
  end
end
"#;
    let parsed = parse_elixir(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper")
        .ok_or("expected a helper call")?;
    assert_eq!(call.from_symbol.as_deref(), Some("go"), "{call:?}");
    Ok(())
}

#[test]
fn extracts_dotted_module_call() -> TestResult {
    let src = r#"
defmodule Widget do
  def go do
    Logger.info("hi")
  end
end
"#;
    let parsed = parse_elixir(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "Logger.info")
        .ok_or("expected a Logger.info call")?;
    let _ = call;
    Ok(())
}

#[test]
fn call_inside_guarded_def_records_from_symbol_scope() -> TestResult {
    let src = r#"
defmodule Widget do
  def bar(x) when x > 0 do
    helper(x)
  end
end
"#;
    let parsed = parse_elixir(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper")
        .ok_or("expected a helper call")?;
    assert_eq!(call.from_symbol.as_deref(), Some("bar"), "{call:?}");
    Ok(())
}

#[test]
fn a_bare_binary_operator_argument_resolves_no_callee() {
    // Mirrors the baseline's own `extract_scripting_callee`: a `call`
    // node whose own first child is neither `identifier` nor `dot`
    // (e.g. the guard expression `x > 0` itself, a `binary_operator`)
    // resolves no callee and is not recorded as a CALLS edge -- this is
    // never actually reached as a standalone top-level `call` node for
    // `x > 0` (it has no `arguments`/`do_block` at all, so it never
    // matches `LangSpec::elixir`'s own `call_types = ["call"]`), so this
    // test instead documents that a def's own guard clause text itself
    // never spuriously becomes a callee named e.g. `"x"` -- it is fully
    // consumed by `elixir_def_name`'s own unwrap, not left behind for
    // the ordinary call-override path to misinterpret.
    let src = r#"
defmodule Widget do
  def bar(x) when x > 0 do
    1
  end
end
"#;
    let parsed = parse_elixir(src);
    assert!(
        !parsed.calls.iter().any(|c| c.callee == "x"),
        "{:?}",
        parsed.calls
    );
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_elixir("defmodule ( { this is not valid elixir @@@");
    let _ = parsed;
}

#[test]
fn fixture_file_parses_without_panic() -> TestResult {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join(FIXTURE_DIR).join("widget.ex");
    let src = fs::read_to_string(&fixture)?;
    let parsed = parse_elixir(&src);
    assert!(
        parsed.symbols.iter().any(|s| s.name == "Widget"),
        "{:?}",
        parsed.symbols
    );
    let draw_count = parsed.symbols.iter().filter(|s| s.name == "draw").count();
    assert_eq!(draw_count, 2, "{:?}", parsed.symbols);
    assert!(
        parsed.symbols.iter().any(|s| s.name == "helper"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed
            .imports
            .iter()
            .any(|i| i.module_path == "Helper.Text"),
        "{:?}",
        parsed.imports
    );
    assert!(
        parsed.imports.iter().any(|i| i.module_path == "Logger"),
        "{:?}",
        parsed.imports
    );
    Ok(())
}
