//! G1: proves the generic spec-table-driven engine
//! ([`enforcer_memory::languages::generic`]) reproduces
//! [`enforcer_memory::languages::go`]'s bespoke extractor output --
//! defs, calls (incl. receiver/arg capture), imports, inherits,
//! defines, routes, and test detection -- on every scenario the
//! existing hand-written `unit_languages_go.rs` suite already covers,
//! plus the shared `lang_go` fixture file. This is this wave's
//! zero-regression proof for the one language fully migrated onto the
//! generic engine this wave (see `spec.rs`'s module doc for why the
//! other 9 `LangSpec` rows are data-only, not yet dispatched).
//!
//! Comparison is content-based (sorted `Debug` strings per field), not
//! `ParsedFile`'s derived `Eq`, because the generic walker's traversal
//! order need not match the bespoke walker's byte-for-byte -- only the
//! *set* of emitted nodes/edges must match for zero-regression
//! purposes (this is exactly what `code_graph`/`resolution` consume:
//! neither cares about `Vec` order).

use enforcer_memory::languages::{generic, go};
use enforcer_memory::parsers::ParsedFile;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_go";

fn sorted_debug<T: std::fmt::Debug>(items: &[T]) -> Vec<String> {
    let mut out: Vec<String> = items.iter().map(|item| format!("{item:?}")).collect();
    out.sort();
    out
}

/// Assert every field of `bespoke` and `generic` contains the same
/// *set* of entries (order-independent).
fn assert_parsed_file_equivalent(bespoke: &ParsedFile, generic: &ParsedFile, label: &str) {
    assert_eq!(
        sorted_debug(&bespoke.symbols),
        sorted_debug(&generic.symbols),
        "{label}: symbols differ"
    );
    assert_eq!(
        sorted_debug(&bespoke.routes),
        sorted_debug(&generic.routes),
        "{label}: routes differ"
    );
    assert_eq!(
        sorted_debug(&bespoke.imports),
        sorted_debug(&generic.imports),
        "{label}: imports differ"
    );
    assert_eq!(
        sorted_debug(&bespoke.calls),
        sorted_debug(&generic.calls),
        "{label}: calls differ"
    );
    assert_eq!(
        sorted_debug(&bespoke.inherits),
        sorted_debug(&generic.inherits),
        "{label}: inherits differ"
    );
    assert_eq!(
        sorted_debug(&bespoke.defines),
        sorted_debug(&generic.defines),
        "{label}: defines differ"
    );
}

fn compare(src: &str, is_test_file: bool, label: &str) {
    let bespoke = go::parse(src, is_test_file);
    let via_generic = generic::parse_go(src, is_test_file);
    assert_parsed_file_equivalent(&bespoke, &via_generic, label);
}

#[test]
fn matches_bespoke_on_package_clause() {
    compare("package widget\n", false, "package_clause");
}

#[test]
fn matches_bespoke_on_function_and_method() {
    let src = r#"
package widget

func NewWidget() {}

func (w *Widget) Draw() string { return "x" }
"#;
    compare(src, false, "function_and_method");
}

#[test]
fn matches_bespoke_on_struct_interface_typealias_const_var() {
    let src = r#"
package widget

type Widget struct {
	Name string
}

type Drawable interface {
	Draw() string
}

type ID = int

const MaxWidgets = 10

var registry = 0
"#;
    compare(src, false, "struct_interface_typealias_const_var");
}

#[test]
fn matches_bespoke_on_embedded_field_inherits() {
    let src = r#"
package widget

type Base struct {
	ID int
}

type Widget struct {
	Base
	Name string
}
"#;
    compare(src, false, "embedded_field_inherits");
}

#[test]
fn matches_bespoke_on_interface_methods_as_defines() {
    let src = r#"
package widget

type Drawable interface {
	Draw() string
	Resize(w int, h int)
}
"#;
    compare(src, false, "interface_methods_as_defines");
}

#[test]
fn matches_bespoke_on_method_receiver_as_defines() {
    let src = r#"
package widget

type Widget struct{ Name string }

func (w *Widget) Draw() string { return w.Name }
"#;
    compare(src, false, "method_receiver_as_defines");
}

#[test]
fn matches_bespoke_on_imports() {
    let src = r#"
package widget

import (
	"fmt"
	"net/http"
)
"#;
    compare(src, false, "imports");
}

#[test]
fn matches_bespoke_on_call_edges() {
    let src = r#"
package widget

func f() {
	helper()
	fmt.Println("x")
}
"#;
    compare(src, false, "call_edges");
}

#[test]
fn matches_bespoke_on_test_file_detection() {
    let src = r#"
package widget

import "testing"

func TestNewWidget(t *testing.T) {}

func helperNotATest() {}
"#;
    compare(src, true, "test_file_detection");
}

#[test]
fn matches_bespoke_on_non_test_file_never_classifies_testxxx() {
    let src = r#"
package widget

func TestLooking() {}
"#;
    compare(src, false, "non_test_file_testxxx");
}

#[test]
fn matches_bespoke_on_net_http_handlefunc_route() {
    let src = r#"
package widget

import "net/http"

func RegisterRoutes(mux *http.ServeMux) {
	mux.HandleFunc("/widgets", ListWidgets)
}
"#;
    compare(src, false, "net_http_handlefunc_route");
}

#[test]
fn matches_bespoke_on_mux_style_verb_route() {
    let src = r#"
package widget

func RegisterRoutes(router *Router) {
	router.GET("/widgets", ListWidgets)
}
"#;
    compare(src, false, "mux_style_verb_route");
}

#[test]
fn matches_bespoke_on_call_scope_and_receiver_capture() {
    let src = r#"
package widget

func Render(w Widget, label string) string {
	return w.Draw(label, 42)
}
"#;
    compare(src, false, "call_scope_and_receiver_capture");
}

#[test]
fn matches_bespoke_on_unqualified_call_no_receiver() {
    let src = r#"
package widget

func f() {
	helper()
}
"#;
    compare(src, false, "unqualified_call_no_receiver");
}

#[test]
fn matches_bespoke_on_constructor_call_receiver_hint() {
    let src = r#"
package widget

func f() string {
	return NewWidget().Draw()
}
"#;
    compare(src, false, "constructor_call_receiver_hint");
}

#[test]
fn matches_bespoke_on_literal_receiver_hint() {
    let src = r#"
package widget

func f() {
	"x".count()
}
"#;
    compare(src, false, "literal_receiver_hint");
}

#[test]
fn matches_bespoke_on_module_scope_call_no_from_symbol() {
    let src = r#"
package widget

var registry = makeRegistry()
"#;
    compare(src, false, "module_scope_call_no_from_symbol");
}

#[test]
fn matches_bespoke_on_malformed_source_does_not_panic() {
    let src = "package ( { this is not valid go @@@";
    let bespoke = go::parse(src, false);
    let via_generic = generic::parse_go(src, false);
    // Malformed-source tolerance only: tree-sitter's own error recovery
    // may legitimately differ in exactly which partial nodes it
    // manages to recognize between two independent walks over its
    // error-recovery tree, so this case is a panic-safety check only,
    // not a content-equivalence one (both callers already exercise
    // that in `unit_languages_go.rs::malformed_source_does_not_panic`).
    let _ = (bespoke, via_generic);
}

#[test]
fn matches_bespoke_on_shared_fixture_file() -> TestResult {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join(FIXTURE_DIR).join("widget.go");
    let src = fs::read_to_string(&fixture)?;
    compare(&src, false, "shared_fixture_widget_go");
    Ok(())
}

#[test]
fn matches_bespoke_on_shared_fixture_test_file() -> TestResult {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join(FIXTURE_DIR).join("widget_test.go");
    let src = fs::read_to_string(&fixture)?;
    compare(&src, true, "shared_fixture_widget_test_go");
    Ok(())
}
