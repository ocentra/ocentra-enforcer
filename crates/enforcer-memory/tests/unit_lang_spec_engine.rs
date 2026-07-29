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

use enforcer_memory::languages::{
    c, cpp, csharp, generic, go, java, php, python, rust, typescript,
};
use enforcer_memory::parsers::{Language, ParsedFile};
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

/// [`assert_parsed_file_equivalent`] plus the richer X06-vocabulary
/// fields (`implements`/`decorates`/`type_refs`) -- Go's bespoke
/// extractor emits TYPE_REF edges its G1 generic-engine migration never
/// wired (accepted per G1's own scope note: "Rich-tier behaviors the
/// generic walker doesn't yet do ... stay on the bespoke path for now;
/// G3 generalizes them"), so those three fields are checked only for
/// languages -- like Rust, migrated this wave -- whose quirk hooks
/// reproduce them in full, not folded into the base Go-compatible
/// helper.
fn assert_parsed_file_equivalent_full(bespoke: &ParsedFile, generic: &ParsedFile, label: &str) {
    assert_parsed_file_equivalent(bespoke, generic, label);
    assert_eq!(
        sorted_debug(&bespoke.implements),
        sorted_debug(&generic.implements),
        "{label}: implements differ"
    );
    assert_eq!(
        sorted_debug(&bespoke.decorates),
        sorted_debug(&generic.decorates),
        "{label}: decorates differ"
    );
    assert_eq!(
        sorted_debug(&bespoke.type_refs),
        sorted_debug(&generic.type_refs),
        "{label}: type_refs differ"
    );
}

fn compare(src: &str, is_test_file: bool, label: &str) {
    let bespoke = go::parse(src, is_test_file);
    let via_generic = generic::go::parse_go(src, is_test_file);
    assert_parsed_file_equivalent(&bespoke, &via_generic, label);
}

fn assert_go_equivalent(src: &str, is_test_file: bool, label: &str) {
    compare(src, is_test_file, label);
}

fn assert_malformed_parsers_complete(bespoke: ParsedFile, generic: ParsedFile) {
    // For malformed input the contract is successful, panic-free completion;
    // the parser outputs may legitimately differ because of error recovery.
    drop((bespoke, generic));
}

fn compare_rust(src: &str, label: &str) {
    let bespoke = rust::parse(src);
    let via_generic = generic::parse_rust(src);
    assert_parsed_file_equivalent_full(&bespoke, &via_generic, label);
}

fn compare_typescript(src: &str, language: Language, label: &str) {
    let bespoke = typescript::parse(src, language);
    let via_generic = generic::parse_typescript(src);
    assert_parsed_file_equivalent_full(&bespoke, &via_generic, label);
}

fn compare_python(src: &str, label: &str) {
    let bespoke = python::parse(src);
    let via_generic = generic::parse_python(src);
    assert_parsed_file_equivalent_full(&bespoke, &via_generic, label);
}

fn compare_java(src: &str, label: &str) {
    let bespoke = java::parse(src);
    let via_generic = generic::parse_java(src);
    assert_parsed_file_equivalent_full(&bespoke, &via_generic, label);
}

fn compare_c(src: &str, is_test_file: bool, label: &str) {
    let bespoke = c::parse(src, is_test_file);
    let via_generic = generic::parse_c(src, is_test_file);
    assert_parsed_file_equivalent_full(&bespoke, &via_generic, label);
}

fn compare_cpp(src: &str, is_test_file: bool, label: &str) {
    let bespoke = cpp::parse(src, is_test_file);
    let via_generic = generic::parse_cpp(src, is_test_file);
    assert_parsed_file_equivalent_full(&bespoke, &via_generic, label);
}

fn compare_csharp(src: &str, label: &str) {
    let bespoke = csharp::parse(src);
    let via_generic = generic::csharp::parse_csharp(src);
    assert_parsed_file_equivalent_full(&bespoke, &via_generic, label);
}

fn compare_php(src: &str, label: &str) {
    let bespoke = php::parse(src);
    let via_generic = generic::php::parse_php(src);
    assert_parsed_file_equivalent_full(&bespoke, &via_generic, label);
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
    let via_generic = generic::go::parse_go(src, false);
    // Malformed-source tolerance only: tree-sitter's own error recovery
    // may legitimately differ in exactly which partial nodes it
    // manages to recognize between two independent walks over its
    // error-recovery tree, so this case is a panic-safety check only,
    // not a content-equivalence one (both callers already exercise
    // that in `unit_languages_go.rs::malformed_source_does_not_panic`).
    assert_malformed_parsers_complete(bespoke, via_generic);
}

#[test]
fn matches_bespoke_on_shared_fixture_file() -> TestResult {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join(FIXTURE_DIR).join("widget.go");
    let src = fs::read_to_string(&fixture)?;
    assert_go_equivalent(&src, false, "shared_fixture_widget_go");
    Ok(())
}

#[test]
fn matches_bespoke_on_shared_fixture_test_file() -> TestResult {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join(FIXTURE_DIR).join("widget_test.go");
    let src = fs::read_to_string(&fixture)?;
    assert_go_equivalent(&src, true, "shared_fixture_widget_test_go");
    Ok(())
}

// G1b: same zero-regression proof, now for Rust
// (`languages::generic::parse_rust` vs the bespoke `languages::rust`
// extractor) -- mirrors every scenario `unit_languages_rust.rs` and
// `unit_vocab_edges.rs`'s Rust cases already cover.

#[test]
fn matches_bespoke_rust_on_function_and_test_symbols() {
    let src = r#"
fn normal_fn() {}

#[test]
fn a_test() {}

#[tokio::test]
async fn an_async_test() {}
"#;
    compare_rust(src, "rust_function_and_test_symbols");
}

#[test]
fn matches_bespoke_rust_on_struct_enum_trait() {
    compare_rust(
        "struct Foo; enum Bar { A } trait Baz {}",
        "rust_struct_enum_trait",
    );
}

#[test]
fn matches_bespoke_rust_on_use_imports() {
    compare_rust(
        "use crate::graph::MemoryGraph; use std::{fs, path::Path};",
        "rust_use_imports",
    );
}

#[test]
fn matches_bespoke_rust_on_use_imports_with_alias_and_wildcard() {
    compare_rust(
        "use std::fmt::Debug as Dbg; use std::collections::*;",
        "rust_use_imports_alias_wildcard",
    );
}

#[test]
fn matches_bespoke_rust_on_call_edges() {
    compare_rust(
        "fn f() { helper(); other::thing(1, 2); }",
        "rust_call_edges",
    );
}

#[test]
fn matches_bespoke_rust_on_method_call_receiver_hints() {
    let src = r#"
struct Widget;
impl Widget {
    fn draw(&self) {
        self.helper();
        Widget::new().draw();
        "x".len();
        other.thing();
    }
}
"#;
    compare_rust(src, "rust_method_call_receiver_hints");
}

#[test]
fn matches_bespoke_rust_on_trait_inherits_impl_implements_defines_type_ref() {
    let src = r#"
trait Base {}
trait Sub: Base {}

struct Thing;

impl Sub for Thing {}

impl Thing {
    fn compute(&self, input: i32) -> i32 {
        input + 1
    }
}
"#;
    compare_rust(src, "rust_trait_inherits_impl_implements_defines_type_ref");
}

#[test]
fn matches_bespoke_rust_on_decorates_from_attribute_macro() {
    compare_rust(
        "#[some_macro]\nfn decorated() {}\n",
        "rust_decorates_from_attribute_macro",
    );
}

#[test]
fn matches_bespoke_rust_on_named_closure_binding() {
    compare_rust(
        "fn f() { let handler = |x: i32| x + 1; }",
        "rust_named_closure_binding",
    );
}

#[test]
fn matches_bespoke_rust_on_const_and_static_items() {
    compare_rust(
        "const MAX: i32 = 10;\nstatic REGISTRY: i32 = 0;",
        "rust_const_and_static_items",
    );
}

#[test]
fn matches_bespoke_rust_on_module_and_type_alias() {
    compare_rust(
        "mod widget { pub struct Inner; }\ntype ID = u64;",
        "rust_module_and_type_alias",
    );
}

#[test]
fn matches_bespoke_rust_on_malformed_source_does_not_panic() {
    let src = "fn ( { this is not valid rust @@@";
    let bespoke = rust::parse(src);
    let via_generic = generic::parse_rust(src);
    // Malformed-source tolerance only, same rationale as
    // `matches_bespoke_on_malformed_source_does_not_panic` above.
    assert_malformed_parsers_complete(bespoke, via_generic);
}

// G1b: same zero-regression proof, now for TypeScript/JavaScript
// (`languages::generic::parse_typescript` vs the bespoke
// `languages::typescript` extractor) -- mirrors every scenario
// `unit_languages_typescript.rs` and `unit_vocab_edges.rs`'s TS cases
// already cover, for both `Language::TypeScript` and
// `Language::JavaScript` (same grammar/quirks row, per the bespoke
// extractor's own module doc).

#[test]
fn matches_bespoke_ts_on_function_class_interface_symbols() {
    compare_typescript(
        "function foo() {} class Bar {} interface Baz {}",
        Language::TypeScript,
        "ts_function_class_interface_symbols",
    );
}

#[test]
fn matches_bespoke_ts_on_import_statements() {
    compare_typescript(
        "import { foo } from \"./foo\";\nimport bar from 'bar-pkg';",
        Language::TypeScript,
        "ts_import_statements",
    );
}

#[test]
fn matches_bespoke_js_on_call_edges() {
    compare_typescript(
        "function f() { helper(); ns.thing(1); }",
        Language::JavaScript,
        "js_call_edges",
    );
}

#[test]
fn matches_bespoke_js_on_express_style_route() {
    compare_typescript(
        "app.get(\"/users/:id\", (req, res) => { res.send(1); });",
        Language::JavaScript,
        "js_express_style_route",
    );
}

#[test]
fn matches_bespoke_ts_on_nestjs_decorator_route() {
    compare_typescript(
        "class C { @Post(\"/items\") create() {} }",
        Language::TypeScript,
        "ts_nestjs_decorator_route",
    );
}

#[test]
fn matches_bespoke_ts_on_full_label_set() {
    let src = r#"
enum Color { Red, Blue }
type Alias = string;
namespace NS {}
const handler = (x: number) => x + 1;
class Base {}
interface Greetable {}
class Sub extends Base implements Greetable {
    greet(): void {}
}
"#;
    compare_typescript(src, Language::TypeScript, "ts_full_label_set");
}

#[test]
fn matches_bespoke_ts_on_extends_implements_and_decorator() {
    let src = r#"
class Base {}
interface Greetable {}
class Sub extends Base implements Greetable {}

@Injectable()
class Service {}
"#;
    compare_typescript(
        src,
        Language::TypeScript,
        "ts_extends_implements_and_decorator",
    );
}

#[test]
fn matches_bespoke_ts_on_method_call_receiver_hints() {
    let src = r#"
class Widget {
    draw() {
        this.helper();
        new Widget().draw();
        "x".length;
        other.thing();
    }
}
"#;
    compare_typescript(src, Language::TypeScript, "ts_method_call_receiver_hints");
}

#[test]
fn matches_bespoke_ts_on_interface_members_get_no_defines() {
    // Regression guard: `languages/typescript.rs`'s bespoke
    // `interface_declaration` arm never propagates `enclosing` into
    // the interface body, so member signatures never get a DEFINES
    // edge to the interface (unlike a class body, which does).
    let src = "interface Greetable { greet(): void; farewell(): void; }";
    compare_typescript(
        src,
        Language::TypeScript,
        "ts_interface_members_get_no_defines",
    );
}

#[test]
fn matches_bespoke_ts_on_test_method_inside_class() {
    // Regression guard: TS/JS's `is_test_name` convention is never
    // filename-gated (applies at every scope, same as Python's), so a
    // `testXxx`/`it`/`it_xxx`-named method inside a class must still
    // classify as Test, not Method.
    let src = "class Suite { testSomething() {} helper() {} }";
    compare_typescript(src, Language::TypeScript, "ts_test_method_inside_class");
}

#[test]
fn matches_bespoke_ts_on_malformed_source_does_not_panic() {
    let src = "function ( { this is not valid typescript @@@";
    let bespoke = typescript::parse(src, Language::TypeScript);
    let via_generic = generic::parse_typescript(src);
    // Malformed-source tolerance only, same rationale as
    // `matches_bespoke_on_malformed_source_does_not_panic` above.
    assert_malformed_parsers_complete(bespoke, via_generic);
}

// G1b: same zero-regression proof, now for Python
// (`languages::generic::parse_python` vs the bespoke
// `languages::python` extractor) -- mirrors every scenario
// `unit_languages_python.rs` and `unit_vocab_edges.rs`'s Python cases
// already cover.

#[test]
fn matches_bespoke_py_on_function_and_test_symbols() {
    compare_python(
        "def normal():\n    pass\n\ndef test_something():\n    pass\n",
        "py_function_and_test_symbols",
    );
}

#[test]
fn matches_bespoke_py_on_class_as_class() {
    compare_python("class Foo:\n    pass\n", "py_class_as_class");
}

#[test]
fn matches_bespoke_py_on_imports() {
    compare_python("import os\nfrom typing import List\n", "py_imports");
}

#[test]
fn matches_bespoke_py_on_grouped_and_aliased_imports() {
    compare_python(
        "import os, sys\nimport numpy as np\n",
        "py_grouped_and_aliased_imports",
    );
}

#[test]
fn matches_bespoke_py_on_call_edges() {
    compare_python("def f():\n    helper()\n    ns.thing()\n", "py_call_edges");
}

#[test]
fn matches_bespoke_py_on_method_call_receiver_hints() {
    let src = "class Widget:\n    def draw(self):\n        self.helper()\n        Widget().draw()\n        \"x\".upper()\n        other.thing()\n";
    compare_python(src, "py_method_call_receiver_hints");
}

#[test]
fn matches_bespoke_py_on_flask_route_decorator() {
    compare_python(
        "@app.route(\"/hello\")\ndef hello():\n    pass\n",
        "py_flask_route_decorator",
    );
}

#[test]
fn matches_bespoke_py_on_fastapi_post_decorator() {
    compare_python(
        "@router.post(\"/items\")\ndef create():\n    pass\n",
        "py_fastapi_post_decorator",
    );
}

#[test]
fn matches_bespoke_py_on_full_label_set_and_inheritance() {
    let src = r#"
class Base:
    pass

class Sub(Base):
    def method(self):
        pass

f = lambda x: x + 1

@some_decorator
def decorated():
    pass
"#;
    compare_python(src, "py_full_label_set_and_inheritance");
}

#[test]
fn matches_bespoke_py_on_test_method_inside_class() {
    // Regression guard: inside a class, a `test_*`-named method must
    // still classify as Test (not Method) -- the test-name check wins
    // over the enclosing-class-implies-Method rule at every scope,
    // same priority order `languages/python.rs`'s own
    // `function_definition` arm applies.
    let src =
        "class Suite:\n    def test_it(self):\n        pass\n    def helper(self):\n        pass\n";
    compare_python(src, "py_test_method_inside_class");
}

#[test]
fn matches_bespoke_py_on_malformed_source_does_not_panic() {
    let src = "def ( : this is not valid python @@@";
    let bespoke = python::parse(src);
    let via_generic = generic::parse_python(src);
    // Malformed-source tolerance only, same rationale as
    // `matches_bespoke_on_malformed_source_does_not_panic` above.
    assert_malformed_parsers_complete(bespoke, via_generic);
}

// G1b: same zero-regression proof, now for Java
// (`languages::generic::parse_java` vs the bespoke `languages::java`
// extractor) -- mirrors every pure-extractor scenario
// `unit_languages_java.rs` already covers.

#[test]
fn matches_bespoke_java_on_package_as_module_symbol() {
    compare_java(
        "package com.example.widget;\n",
        "java_package_as_module_symbol",
    );
}

#[test]
fn matches_bespoke_java_on_class_interface_enum_with_distinct_kinds() {
    let src = r#"
package widget;

public interface Drawable {}

public class Widget {}

public enum Color { RED, GREEN }
"#;
    compare_java(src, "java_class_interface_enum_with_distinct_kinds");
}

#[test]
fn matches_bespoke_java_on_extends_as_inherits() {
    let src = r#"
package widget;

public class Shape {}

public class Widget extends Shape {}
"#;
    compare_java(src, "java_extends_as_inherits");
}

#[test]
fn matches_bespoke_java_on_implements_as_implements_edge() {
    let src = r#"
package widget;

public interface Drawable {}
public interface Resizable {}

public class Widget implements Drawable, Resizable {}
"#;
    compare_java(src, "java_implements_as_implements_edge");
}

#[test]
fn matches_bespoke_java_on_interface_extends_as_inherits() {
    let src = r#"
package widget;

public interface Base {}
public interface Drawable extends Base {}
"#;
    compare_java(src, "java_interface_extends_as_inherits");
}

#[test]
fn matches_bespoke_java_on_static_final_field_as_constant() {
    let src = r#"
package widget;

public class Widget {
    public static final int MAX_WIDGETS = 10;
    private String name;
}
"#;
    compare_java(src, "java_static_final_field_as_constant");
}

#[test]
fn matches_bespoke_java_on_method_as_defines_and_decorates() {
    let src = r#"
package widget;

public class Widget {
    @Override
    public String draw() { return "x"; }
}
"#;
    compare_java(src, "java_method_as_defines_and_decorates");
}

#[test]
fn matches_bespoke_java_on_imports() {
    let src = r#"
package widget;

import java.util.List;
import java.util.ArrayList;
"#;
    compare_java(src, "java_imports");
}

#[test]
fn matches_bespoke_java_on_call_edges() {
    let src = r#"
package widget;

public class Widget {
    public void f() {
        helper();
        this.name.trim();
    }
}
"#;
    compare_java(src, "java_call_edges");
}

#[test]
fn matches_bespoke_java_on_signature_type_refs() {
    let src = r#"
package widget;

public class Widget {
    public boolean combine(int a, String b) { return true; }
}
"#;
    compare_java(src, "java_signature_type_refs");
}

#[test]
fn matches_bespoke_java_on_test_annotation_detects_test_method() {
    let src = r#"
package widget;

import org.junit.Test;

public class WidgetTest {
    @Test
    public void testDraw() {}

    public void helperNotATest() {}
}
"#;
    compare_java(src, "java_test_annotation_detects_test_method");
}

#[test]
fn matches_bespoke_java_on_spring_get_mapping_route() {
    let src = r#"
package widget;

import org.springframework.web.bind.annotation.GetMapping;

public class WidgetController {
    @GetMapping("/widgets")
    public String listWidgets() { return "[]"; }
}
"#;
    compare_java(src, "java_spring_get_mapping_route");
}

#[test]
fn matches_bespoke_java_on_spring_post_mapping_named_argument_route() {
    let src = r#"
package widget;

import org.springframework.web.bind.annotation.PostMapping;

public class WidgetController {
    @PostMapping(path = "/widgets")
    public String createWidget() { return "{}"; }
}
"#;
    compare_java(src, "java_spring_post_mapping_named_argument_route");
}

#[test]
fn matches_bespoke_java_on_call_inside_method_records_from_symbol_scope() {
    let src = r#"
package widget;

public class Widget {
    public String render(Widget w) {
        return w.draw();
    }
}
"#;
    compare_java(src, "java_call_inside_method_records_from_symbol_scope");
}

#[test]
fn matches_bespoke_java_on_method_call_records_identifier_receiver_and_args() {
    let src = r#"
package widget;

public class Widget {
    public String render(Widget w, String label) {
        return w.draw(label, 42);
    }
}
"#;
    compare_java(src, "java_method_call_records_identifier_receiver_and_args");
}

#[test]
fn matches_bespoke_java_on_this_call_records_self_or_this_hint() {
    let src = r#"
package widget;

public class Widget {
    public void bump() {
        this.report();
    }
    public void report() {}
}
"#;
    compare_java(src, "java_this_call_records_self_or_this_hint");
}

#[test]
fn matches_bespoke_java_on_new_expression_receiver_records_new_hint() {
    let src = r#"
package widget;

public class Factory {
    public String make() {
        return new Widget().draw();
    }
}
"#;
    compare_java(src, "java_new_expression_receiver_records_new_hint");
}

#[test]
fn matches_bespoke_java_on_unqualified_call_has_no_receiver() {
    let src = r#"
package widget;

public class Widget {
    public void f() {
        helper();
    }
}
"#;
    compare_java(src, "java_unqualified_call_has_no_receiver");
}

#[test]
fn matches_bespoke_java_on_literal_receiver_records_literal_hint() {
    let src = r#"
package widget;

public class Widget {
    public String f() {
        return "x".trim();
    }
}
"#;
    compare_java(src, "java_literal_receiver_records_literal_hint");
}

#[test]
fn matches_bespoke_java_on_malformed_source_does_not_panic() {
    let src = "class ( { this is not valid java @@@";
    let bespoke = java::parse(src);
    let via_generic = generic::parse_java(src);
    // Malformed-source tolerance only, same rationale as
    // `matches_bespoke_on_malformed_source_does_not_panic` above.
    assert_malformed_parsers_complete(bespoke, via_generic);
}

// G1b: same zero-regression proof, now for C (`languages::generic::
// parse_c` vs the bespoke `languages::c` extractor) -- mirrors every
// pure-extractor scenario `unit_languages_c.rs` already covers. C
// needed a full quirk claim (see `LangSpec::c()`'s and
// `generic::c_quirk`'s doc comments) rather than the generic engine's
// own name-field fallback, so these tests are this language's real
// zero-regression proof, not a formality.

#[test]
fn matches_bespoke_c_on_function_and_struct_and_enum_and_typedef() {
    let src = r#"
struct Point { int x; int y; };
enum Color { RED, GREEN, BLUE };
typedef struct Point PointAlias;
typedef int MyInt;

int add(int a, int b) {
    return a + b;
}
"#;
    compare_c(src, false, "c_function_and_struct_and_enum_and_typedef");
}

#[test]
fn matches_bespoke_c_on_define_value_macro_and_const_and_variable() {
    let src = r#"
#define MAX_SIZE 128
#define EMPTY_GUARD
const int kLimit = 10;
int counter = 0;
"#;
    compare_c(src, false, "c_define_value_macro_and_const_and_variable");
}

#[test]
fn matches_bespoke_c_on_include_imports() {
    let src = r#"
#include <stdio.h>
#include "local_header.h"
"#;
    compare_c(src, false, "c_include_imports");
}

#[test]
fn matches_bespoke_c_on_call_edges() {
    compare_c(
        "void f() { helper(); other_fn(1, 2); }",
        false,
        "c_call_edges",
    );
}

#[test]
fn matches_bespoke_c_on_detects_test_by_name_convention() {
    compare_c(
        "void test_addition() {} void teardown_test() {} void normal_fn() {}",
        false,
        "c_detects_test_by_name_convention",
    );
}

#[test]
fn matches_bespoke_c_on_is_test_file_promotes_every_function_to_test() {
    compare_c(
        "void anything() {} void something_else() {}",
        true,
        "c_is_test_file_promotes_every_function_to_test",
    );
}

#[test]
fn matches_bespoke_c_on_struct_defines_edges_to_field_members() {
    compare_c(
        "struct Vec3 { float x; float y; float z; };",
        false,
        "c_struct_defines_edges_to_field_members",
    );
}

#[test]
fn matches_bespoke_c_on_pointer_and_function_pointer_declarators() {
    // Declarator-unwrapping regression guard: a pointer-returning
    // function and a function-pointer typedef both need
    // `innermost_declarator_identifier`'s recursive unwrap, not a bare
    // field-text read.
    let src = r#"
int *make_buffer(int size) {
    return 0;
}

typedef int (*FnPtr)(int);
"#;
    compare_c(src, false, "c_pointer_and_function_pointer_declarators");
}

#[test]
fn matches_bespoke_c_on_nested_struct_and_enum_inside_struct() {
    // Regression guard for the double-invocation risk this quirk's
    // `struct_specifier`/`enum_specifier`/`type_definition` arms must
    // avoid (see `generic::c_quirk`'s doc comment): a struct containing
    // a nested enum must extract both exactly once, not twice.
    let src = r#"
struct Outer {
    enum Status { OK, FAIL } status;
    int value;
};
"#;
    compare_c(src, false, "c_nested_struct_and_enum_inside_struct");
}

#[test]
fn matches_bespoke_c_on_malformed_source_does_not_panic() {
    let src = "int f( { this is not valid C @@@";
    let bespoke = c::parse(src, false);
    let via_generic = generic::parse_c(src, false);
    // Malformed-source tolerance only, same rationale as
    // `matches_bespoke_on_malformed_source_does_not_panic` above.
    assert_malformed_parsers_complete(bespoke, via_generic);
}

// G1b: same zero-regression proof, now for C++
// (`languages::generic::parse_cpp` vs the bespoke `languages::cpp`
// extractor) -- mirrors every pure-extractor scenario
// `unit_languages_cpp.rs` already covers. Like C, C++ needed a full
// quirk claim (see `LangSpec::cpp()`'s and `generic::cpp_quirk`'s doc
// comments).

#[test]
fn matches_bespoke_cpp_on_class_method_and_free_function() {
    let src = r#"
class Shape {
public:
    Shape();
    virtual double area() const;
private:
    double width_;
};

double Shape::area() const {
    return width_ * width_;
}

void helper_fn() {}
"#;
    compare_cpp(src, false, "cpp_class_method_and_free_function");
}

#[test]
fn matches_bespoke_cpp_on_inherits_edge_from_base_class_clause() {
    compare_cpp(
        "class Base {}; class Derived : public Base {};",
        false,
        "cpp_inherits_edge_from_base_class_clause",
    );
}

#[test]
fn matches_bespoke_cpp_on_detects_abstract_class_as_interface() {
    let src = r#"
class Drawable {
public:
    virtual void draw() = 0;
    virtual ~Drawable() = default;
};
"#;
    compare_cpp(src, false, "cpp_detects_abstract_class_as_interface");
}

#[test]
fn matches_bespoke_cpp_on_namespace_as_module() {
    compare_cpp(
        "namespace geometry { class Point {}; }",
        false,
        "cpp_namespace_as_module",
    );
}

#[test]
fn matches_bespoke_cpp_on_named_lambda_binding() {
    compare_cpp(
        "auto adder = [](int a, int b) { return a + b; };",
        false,
        "cpp_named_lambda_binding",
    );
}

#[test]
fn matches_bespoke_cpp_on_named_lambda_binding_from_assignment() {
    compare_cpp(
        "int f() { adder = [](int a, int b) { return a + b; }; }",
        false,
        "cpp_named_lambda_binding_from_assignment",
    );
}

#[test]
fn matches_bespoke_cpp_on_include_imports_and_calls() {
    let src = r#"
#include <vector>
#include "myheader.h"
void f() { helper(); other::thing(1, 2); }
"#;
    compare_cpp(src, false, "cpp_include_imports_and_calls");
}

#[test]
fn matches_bespoke_cpp_on_detects_gtest_test_macro() {
    let src = r#"
TEST(MathSuite, AddsNumbers) {
    int result = 1 + 1;
}
TEST_F(FixtureSuite, DoesWork) {
}
"#;
    compare_cpp(src, false, "cpp_detects_gtest_test_macro");
}

#[test]
fn matches_bespoke_cpp_on_is_test_file_promotes_free_functions_and_methods_to_test() {
    let src = "void case_one() {} class Fixture { public: void case_two() {} };";
    compare_cpp(
        src,
        true,
        "cpp_is_test_file_promotes_free_functions_and_methods_to_test",
    );
}

#[test]
fn matches_bespoke_cpp_on_typedef_and_using_alias() {
    compare_cpp(
        "typedef int MyInt; using StringAlias = std::string;",
        false,
        "cpp_typedef_and_using_alias",
    );
}

#[test]
fn matches_bespoke_cpp_on_method_call_receiver_hints() {
    let src = r#"
class Widget {
public:
    void draw() {
        this->helper();
        Widget().draw();
        other.thing();
    }
    void helper() {}
};
"#;
    compare_cpp(src, false, "cpp_method_call_receiver_hints");
}

#[test]
fn matches_bespoke_cpp_on_nested_struct_and_enum_inside_class() {
    // Same regression guard as C's -- see
    // `matches_bespoke_c_on_nested_struct_and_enum_inside_struct`.
    let src = r#"
class Outer {
public:
    enum Status { OK, FAIL };
    int value;
};
"#;
    compare_cpp(src, false, "cpp_nested_struct_and_enum_inside_class");
}

#[test]
fn matches_bespoke_cpp_on_malformed_source_does_not_panic() {
    let src = "class ( { this is not valid C++ @@@";
    let bespoke = cpp::parse(src, false);
    let via_generic = generic::parse_cpp(src, false);
    // Malformed-source tolerance only, same rationale as
    // `matches_bespoke_on_malformed_source_does_not_panic` above.
    assert_malformed_parsers_complete(bespoke, via_generic);
}

// G1b: same zero-regression proof, now for C#
// (`languages::generic::csharp::parse_csharp` vs the bespoke
// `languages::csharp` extractor) -- mirrors every pure-extractor
// scenario `unit_languages_csharp.rs` already covers.

#[test]
fn matches_bespoke_csharp_on_class_interface_struct_enum_symbols() {
    let src = r#"
public interface IRepo {}
public class UserController {}
public struct Point {}
public enum Status { Active, Inactive }
"#;
    compare_csharp(src, "csharp_class_interface_struct_enum_symbols");
}

#[test]
fn matches_bespoke_csharp_on_method_tagged_as_method_inside_class() {
    compare_csharp(
        "class C { public void F() {} }",
        "csharp_method_tagged_as_method_inside_class",
    );
}

#[test]
fn matches_bespoke_csharp_on_namespace_as_module() {
    compare_csharp(
        "namespace MyApp.Services { class C {} }",
        "csharp_namespace_as_module",
    );
}

#[test]
fn matches_bespoke_csharp_on_using_imports() {
    compare_csharp(
        "using System;\nusing System.Collections.Generic;\n",
        "csharp_using_imports",
    );
}

#[test]
fn matches_bespoke_csharp_on_call_edges() {
    compare_csharp(
        "class C { void F() { helper(1); this.Bar(); } }",
        "csharp_call_edges",
    );
}

#[test]
fn matches_bespoke_csharp_on_inherits_and_implements_from_base_list() {
    compare_csharp(
        "class UserController : ControllerBase, IDisposable {}",
        "csharp_inherits_and_implements_from_base_list",
    );
}

#[test]
fn matches_bespoke_csharp_on_implements_only_when_base_is_interface_shaped() {
    compare_csharp(
        "class Repo : IRepo, IDisposable {}",
        "csharp_implements_only_when_base_is_interface_shaped",
    );
}

#[test]
fn matches_bespoke_csharp_on_interface_extends_as_inherits() {
    compare_csharp(
        "interface IRepo : IDisposable, IBase {}",
        "csharp_interface_extends_as_inherits",
    );
}

#[test]
fn matches_bespoke_csharp_on_attribute_decorations() {
    compare_csharp(
        "[ApiController]\npublic class UserController {}",
        "csharp_attribute_decorations",
    );
}

#[test]
fn matches_bespoke_csharp_on_detects_nunit_xunit_mstest_test_attributes() {
    let src = r#"
class T {
    [Test] public void A() {}
    [Fact] public void B() {}
    [TestMethod] public void C() {}
    public void NotATest() {}
}
"#;
    compare_csharp(src, "csharp_detects_nunit_xunit_mstest_test_attributes");
}

#[test]
fn matches_bespoke_csharp_on_http_attribute_route() {
    let src = r#"
class C {
    [HttpGet("/users/{id}")]
    public void GetUser() {}
}
"#;
    compare_csharp(src, "csharp_http_attribute_route");
}

#[test]
fn matches_bespoke_csharp_on_minimal_api_map_route() {
    compare_csharp(
        r#"app.MapGet("/users/{id}", () => {});"#,
        "csharp_minimal_api_map_route",
    );
}

#[test]
fn matches_bespoke_csharp_on_const_and_static_readonly_fields_as_constants() {
    let src = r#"
class C {
    const int MaxCount = 10;
    static readonly string Prefix = "x";
    public int NotConst = 0;
}
"#;
    compare_csharp(src, "csharp_const_and_static_readonly_fields_as_constants");
}

#[test]
fn matches_bespoke_csharp_on_defines_edge_from_class_to_method() {
    compare_csharp(
        "class C { void F() {} }",
        "csharp_defines_edge_from_class_to_method",
    );
}

#[test]
fn matches_bespoke_csharp_on_type_refs_from_method_signature() {
    compare_csharp(
        "class C { public User GetUser(int id, string name) { return null; } }",
        "csharp_type_refs_from_method_signature",
    );
}

#[test]
fn matches_bespoke_csharp_on_named_local_function_as_lambda() {
    compare_csharp(
        "class C { void Outer() { int Add(int a, int b) => a + b; } }",
        "csharp_named_local_function_as_lambda",
    );
}

#[test]
fn matches_bespoke_csharp_on_expression_bodied_method() {
    // Regression guard: an expression-bodied member (`=>`, no `body`
    // field) must still classify as Method + DEFINES + get its call
    // walked with the right `fn_scope`, matching
    // `languages/csharp.rs`'s own `method_declaration` arm's `else`
    // fallback path.
    let src = "class C { int F() => helper(); }";
    compare_csharp(src, "csharp_expression_bodied_method");
}

#[test]
fn matches_bespoke_csharp_on_nested_class_inside_namespace() {
    // Regression guard for the double-invocation risk this quirk's
    // `class_declaration`/`interface_declaration`/`enum_declaration`
    // arms must avoid, same rationale as C's/C++'s identical guards.
    let src = "namespace App { class Outer { class Inner {} } }";
    compare_csharp(src, "csharp_nested_class_inside_namespace");
}

#[test]
fn matches_bespoke_csharp_on_attribute_route_and_member_traversal() {
    let src = r#"
[Route("api/widgets")]
class WidgetsController : ControllerBase {
    [HttpGet("health")]
    string Health() => "ok";
}
"#;
    compare_csharp(src, "csharp_attribute_route_and_member_traversal");
}

#[test]
fn matches_bespoke_csharp_on_malformed_source_does_not_panic() {
    let src = "class ( { this is not valid C# @@@";
    let bespoke = csharp::parse(src);
    let via_generic = generic::csharp::parse_csharp(src);
    // Malformed-source tolerance only, same rationale as
    // `matches_bespoke_on_malformed_source_does_not_panic` above.
    assert_malformed_parsers_complete(bespoke, via_generic);
}

// G1b: same zero-regression proof, now for PHP
// (`languages::generic::php::parse_php` vs the bespoke `languages::php`
// extractor) -- mirrors every pure-extractor scenario
// `unit_languages_php.rs` already covers. PHP needed `call_override`
// (not just `on_unmatched_node`) for its four call-shaped node kinds
// specifically so `from_symbol`/`from_symbol_line` still thread
// through correctly (see `LangSpec::php()`'s and
// `generic::php_call_override`'s doc comments).

#[test]
fn matches_bespoke_php_on_class_interface_function_symbols() {
    let src = "<?php\nclass UserController {}\ninterface Repo {}\nfunction top_level() {}\n";
    compare_php(src, "php_class_interface_function_symbols");
}

#[test]
fn matches_bespoke_php_on_method_tagged_as_method_inside_class() {
    compare_php(
        "<?php class C { public function f() {} }",
        "php_method_tagged_as_method_inside_class",
    );
}

#[test]
fn matches_bespoke_php_on_namespace_as_module() {
    compare_php(
        "<?php namespace App\\Services; class C {}",
        "php_namespace_as_module",
    );
}

#[test]
fn matches_bespoke_php_on_use_imports() {
    let src = "<?php\nuse App\\Models\\User;\nuse App\\Bar as Baz;\nuse function App\\Helpers\\format_name;\n";
    compare_php(src, "php_use_imports");
}

#[test]
fn matches_bespoke_php_on_require_include_as_imports() {
    let src = r#"<?php
require 'config.php';
require_once 'bootstrap.php';
include 'helpers.php';
"#;
    compare_php(src, "php_require_include_as_imports");
}

#[test]
fn matches_bespoke_php_on_call_edges() {
    compare_php(
        "<?php function f() { helper($x); $this->bar(); Route::get('/x', 'y'); }",
        "php_call_edges",
    );
}

#[test]
fn matches_bespoke_php_on_call_inside_method_records_from_symbol_scope() {
    // Regression guard: the whole reason PHP's calls need
    // `call_override` rather than `on_unmatched_node` -- a call inside
    // a method body must still record `from_symbol`/`from_symbol_line`
    // set to that method, matching `languages/php.rs`'s own
    // `fn_scope` threading exactly.
    let src = "<?php class C { public function render() { return $this->draw(); } } ";
    compare_php(src, "php_call_inside_method_records_from_symbol_scope");
}

#[test]
fn matches_bespoke_php_on_extends_as_inherits() {
    compare_php("<?php class C extends Base {}", "php_extends_as_inherits");
}

#[test]
fn matches_bespoke_php_on_implements_edges() {
    compare_php(
        "<?php class C implements Countable, IteratorAggregate {}",
        "php_implements_edges",
    );
}

#[test]
fn matches_bespoke_php_on_interface_extends_as_inherits() {
    compare_php(
        "<?php interface Repo extends Countable, Base {}",
        "php_interface_extends_as_inherits",
    );
}

#[test]
fn matches_bespoke_php_on_php8_attribute_decorations() {
    compare_php(
        "<?php #[Something]\nclass C {}",
        "php_php8_attribute_decorations",
    );
}

#[test]
fn matches_bespoke_php_on_detects_phpunit_test_via_attribute_and_test_case_extends() {
    let src = r#"<?php
class MyTest extends TestCase {
    public function testFoo() {}
    public function helper() {}
}
class Other {
    #[Test]
    public function checkThing() {}
    public function notATest() {}
}
"#;
    compare_php(
        src,
        "php_detects_phpunit_test_via_attribute_and_test_case_extends",
    );
}

#[test]
fn matches_bespoke_php_on_laravel_route_call() {
    compare_php(
        "<?php Route::get('/api/x', 'Controller@method');",
        "php_laravel_route_call",
    );
}

#[test]
fn matches_bespoke_php_on_symfony_route_attribute() {
    let src = r#"<?php
class C {
    #[Route("/users/{id}")]
    public function getUser() {}
}
"#;
    compare_php(src, "php_symfony_route_attribute");
}

/// Direct generic-engine regression: iterator traversal retains source order
/// through nested attributes and a static call's argument list.
#[test]
fn generic_php_child_iteration_preserves_route_and_call_argument_order() -> TestResult {
    let src = r#"<?php
class C {
    #[Route("/first"), Route("/second")]
    public function save(int $first, string $second): Result {
        return Route::post("/first", [$first, $second]);
    }
}
"#;
    let parsed = generic::php::parse_php(src);
    let routes: Vec<(&str, &str)> = parsed
        .routes
        .iter()
        .map(|route| (route.method.as_str(), route.path.as_str()))
        .collect();
    assert_eq!(
        routes,
        vec![("", "/first"), ("", "/second"), ("POST", "/first")],
        "{routes:?}"
    );

    let call = parsed
        .calls
        .iter()
        .find(|call| call.callee == "Route::post")
        .ok_or("expected a Route::post call")?;
    assert_eq!(
        call.arg_texts,
        vec!["\"/first\"".to_string(), "[$first, $second]".to_string()],
        "{call:?}"
    );
    Ok(())
}

#[test]
fn matches_bespoke_php_on_const_declaration_and_define_call_as_constants() {
    let src = r#"<?php
class C { const MAX = 10; }
define("APP_VERSION", "1.0");
"#;
    compare_php(src, "php_const_declaration_and_define_call_as_constants");
}

#[test]
fn matches_bespoke_php_on_defines_edge_from_class_to_method() {
    compare_php(
        "<?php class C { public function f() {} }",
        "php_defines_edge_from_class_to_method",
    );
}

#[test]
fn matches_bespoke_php_on_type_refs_from_method_signature() {
    let src =
        "<?php class C { public function getUser(int $id, string $name): User { return null; } }";
    compare_php(src, "php_type_refs_from_method_signature");
}

#[test]
fn matches_bespoke_php_on_named_closure_and_arrow_function_as_lambda() {
    let src = r#"<?php
$f = function($x) { return $x + 1; };
$g = fn($x) => $x * 2;
"#;
    compare_php(src, "php_named_closure_and_arrow_function_as_lambda");
}

#[test]
fn matches_bespoke_php_on_trait_as_class() {
    // Regression guard: `LangSpec::php()`'s `class_types` was missing
    // `"trait_declaration"` before this wave's audit -- see that
    // const's own doc comment.
    compare_php(
        "<?php trait Greetable { public function greet() {} }",
        "php_trait_as_class",
    );
}

#[test]
fn matches_bespoke_php_on_nested_class_inside_namespace() {
    // Regression guard for the double-invocation risk this quirk's
    // `class_declaration`/`interface_declaration` arms must avoid,
    // same rationale as C's/C++'s/C#'s identical guards.
    let src = "<?php namespace App { class Outer { class Inner {} } }";
    compare_php(src, "php_nested_class_inside_namespace");
}

#[test]
fn matches_bespoke_php_on_malformed_source_does_not_panic() {
    let src = "<?php class ( { this is not valid PHP @@@";
    let bespoke = php::parse(src);
    let via_generic = generic::php::parse_php(src);
    // Malformed-source tolerance only, same rationale as
    // `matches_bespoke_on_malformed_source_does_not_panic` above.
    assert_malformed_parsers_complete(bespoke, via_generic);
}
