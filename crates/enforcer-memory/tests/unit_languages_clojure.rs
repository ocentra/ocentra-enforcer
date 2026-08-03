//! Hard tests for Clojure, onboarded directly through the generic
//! spec-table engine
//! ([`enforcer_syntax::languages::generic::parse_clojure`]) -- there is
//! no bespoke `languages::clojure` extractor to prove zero-regression
//! against (Clojure has never had one in this crate), so these tests
//! assert against the grammar-shape ground truth recorded in
//! [`enforcer_syntax::languages::spec::LangSpec::clojure`]'s own doc
//! comment directly: def-form recognition and disambiguation from a
//! plain call (both are `list_lit` nodes with no syntactic distinction),
//! the baseline's own def-head keyword table (`defn`/`def`/`defrecord`/
//! `definterface`/...), the baseline's UNFILTERED call-callee recording
//! (a def-form's own head keyword, e.g. `"defn"`, IS ALSO recorded as a
//! call -- this is intentional, matching
//! `internal/cbm/extract_calls.c`'s `extract_lisp_callee` exactly, not a
//! bug), and `ns`/`require` IMPORTS.

use enforcer_syntax::languages::generic::parse_clojure;
use enforcer_syntax::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_clojure";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_syntax::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn defn_is_a_function_symbol() {
    let src = "(defn helper [x y]\n  (+ x y))\n";
    let parsed = parse_clojure(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "helper"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn def_is_also_a_function_symbol() {
    let src = "(def widget {:a 1})\n";
    let parsed = parse_clojure(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "widget"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn defrecord_is_a_struct_symbol() {
    let src = "(defrecord Point [x y])\n";
    let parsed = parse_clojure(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Point"),
        Some(&SymbolKind::Struct),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn definterface_is_an_interface_symbol() {
    let src = "(definterface Shape\n  (draw [this]))\n";
    let parsed = parse_clojure(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Shape"),
        Some(&SymbolKind::Interface),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn defprotocol_is_an_interface_symbol() {
    let src = "(defprotocol Drawable\n  (draw [this]))\n";
    let parsed = parse_clojure(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Drawable"),
        Some(&SymbolKind::Interface),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn defmethod_is_a_function_symbol() {
    let src = "(defmethod area :circle [c]\n  (:r c))\n";
    let parsed = parse_clojure(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "area"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn ordinary_call_is_not_mistaken_for_a_def() {
    // NOTE: this does NOT assert `parsed.symbols.is_empty()` -- the
    // root `source` node's own `module_types` handling (a real,
    // documented, and accepted imprecision -- see `LangSpec::clojure`'s
    // own doc comment) always mints ONE spurious Module symbol off the
    // root's first named child when the file has no leading `(ns ...)`
    // form, regardless of what that first form actually is. This test's
    // own job is narrower: a plain call must never ALSO be mistaken for
    // a def (no Function/Struct/Interface symbol minted for it).
    let src = "(println \"hello\")\n";
    let parsed = parse_clojure(src);
    assert!(
        !parsed.symbols.iter().any(|s| matches!(
            s.kind,
            SymbolKind::Function | SymbolKind::Struct | SymbolKind::Interface
        )),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn plain_call_still_records_a_callee() {
    let src = "(println \"hello\")\n";
    let parsed = parse_clojure(src);
    assert!(
        parsed.calls.iter().any(|c| c.callee == "println"),
        "{:?}",
        parsed.calls
    );
}

#[test]
fn defn_head_keyword_is_also_recorded_as_a_call() {
    // Matches the baseline's own unfiltered `extract_lisp_callee` exactly
    // -- a def-form's own head keyword is ALSO a call callee, by design,
    // not a bug (`extract_lisp_callee` has no knowledge of
    // `lisp_is_def_head` at all).
    let src = "(defn helper [x] x)\n";
    let parsed = parse_clojure(src);
    assert!(
        parsed.calls.iter().any(|c| c.callee == "defn"),
        "{:?}",
        parsed.calls
    );
}

#[test]
fn namespace_qualified_call_keeps_full_callee_text() -> TestResult {
    let src = "(defn f []\n  (str/join \",\" [1 2 3]))\n";
    let parsed = parse_clojure(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "str/join")
        .ok_or("expected a str/join call")?;
    assert_eq!(
        call.arg_texts,
        vec!["\",\"".to_string(), "[1 2 3]".to_string()],
        "{call:?}"
    );
    Ok(())
}

#[test]
fn call_inside_defn_body_records_from_symbol_scope() -> TestResult {
    let src = "(defn outer []\n  (helper 1 2))\n";
    let parsed = parse_clojure(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper")
        .ok_or("expected a helper call")?;
    assert_eq!(call.from_symbol.as_deref(), Some("outer"), "{call:?}");
    Ok(())
}

#[test]
fn module_scope_call_has_no_from_symbol() -> TestResult {
    let src = "(helper 1 2)\n";
    let parsed = parse_clojure(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper")
        .ok_or("expected a helper call")?;
    assert_eq!(call.from_symbol, None, "{call:?}");
    Ok(())
}

#[test]
fn nested_defn_forms_each_get_their_own_scope() -> TestResult {
    let src = "(defn outer []\n  (defn inner []\n    (helper))\n  (inner))\n";
    let parsed = parse_clojure(src);
    let helper_call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper")
        .ok_or("expected a helper call")?;
    assert_eq!(
        helper_call.from_symbol.as_deref(),
        Some("inner"),
        "{helper_call:?}"
    );
    Ok(())
}

#[test]
fn ns_require_vector_clause_is_an_import() {
    let src = "(ns app.core\n  (:require [clojure.string :as str]))\n";
    let parsed = parse_clojure(src);
    assert!(
        parsed
            .imports
            .iter()
            .any(|i| i.module_path == "clojure.string"),
        "{:?}",
        parsed.imports
    );
}

#[test]
fn plain_require_quoted_symbol_is_an_import() {
    let src = "(require 'clojure.set)\n";
    let parsed = parse_clojure(src);
    assert!(
        parsed
            .imports
            .iter()
            .any(|i| i.module_path == "clojure.set"),
        "{:?}",
        parsed.imports
    );
}

#[test]
fn if_form_is_recorded_as_a_call_not_a_branch() {
    // Matches the baseline's own genuinely-empty Clojure
    // `branching_node_types` -- `if`/`when`/`cond` are syntactically
    // indistinguishable `list_lit` calls the baseline makes no attempt to
    // recognize as decision points for this language.
    let src = "(defn check [x]\n  (if (> x 0)\n    (println \"pos\")\n    (println \"neg\")))\n";
    let parsed = parse_clojure(src);
    assert!(
        parsed.calls.iter().any(|c| c.callee == "if"),
        "{:?}",
        parsed.calls
    );
}

#[test]
fn empty_list_does_not_panic_and_is_not_mistaken_for_a_def() {
    let src = "(defn f [] ())\n";
    let parsed = parse_clojure(src);
    assert!(
        parsed.symbols.iter().any(|s| s.name == "f"),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_clojure("(defn ( this is not valid clojure @@@");
    let _ = parsed;
}

#[test]
fn fixture_file_parses_without_panic() -> TestResult {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join(FIXTURE_DIR).join("widget.clj");
    let src = fs::read_to_string(&fixture)?;
    let parsed = parse_clojure(&src);
    assert!(
        parsed.symbols.iter().any(|s| s.name == "Widget"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.symbols.iter().any(|s| s.name == "helper"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.symbols.iter().any(|s| s.name == "draw"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed
            .imports
            .iter()
            .any(|i| i.module_path == "clojure.string"),
        "{:?}",
        parsed.imports
    );
    Ok(())
}
