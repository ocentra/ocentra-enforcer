//! Hard tests for Solidity onboarded directly through the generic
//! spec-table engine ([`enforcer_memory::languages::generic::parse_solidity`]
//! -- language-parity wave G2.1d). Solidity has no pre-existing bespoke
//! `languages::solidity` extractor (unlike Go/Rust/TypeScript/Python/
//! Java/C/C++/C#/PHP, each migrated from one during earlier waves), so
//! these tests assert directly against the grammar's own real shape --
//! both the `tree-sitter-solidity` crate's own `node-types.json` and a
//! real parse tree dump (a scratch `cargo run` against a minimal crate
//! depending on `tree-sitter-solidity` directly, which caught three
//! wrong assumptions `node-types.json` alone did not surface -- see
//! `LangSpec::solidity`'s own doc comment for the specifics) -- not
//! byte-for-byte parity with prior behavior.

use enforcer_domain::memory_types::ReceiverHint;
use enforcer_memory::languages::generic::parse_solidity;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;

type TestResult = Result<(), Box<dyn Error>>;

fn symbol_kind<'a>(
    symbols: &'a [enforcer_memory::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_contract_interface_and_free_function_symbols() {
    let src = r#"
interface IWidget {
    function draw() external view returns (string memory);
}

contract Widget is IWidget {
    function draw() external view returns (string memory) {
        return "x";
    }
}
"#;
    let parsed = parse_solidity(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "IWidget"),
        Some(&SymbolKind::Interface)
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "Widget"),
        Some(&SymbolKind::Class)
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "draw"),
        Some(&SymbolKind::Method)
    );
}

#[test]
fn extracts_library_struct_and_enum_symbols() {
    let src = r#"
library MathLib {
    function add(uint256 a, uint256 b) internal pure returns (uint256) {
        return a + b;
    }
}

contract C {
    struct Point {
        uint256 x;
        uint256 y;
    }

    enum Status {
        Idle,
        Running
    }
}
"#;
    let parsed = parse_solidity(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "MathLib"),
        Some(&SymbolKind::Class)
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "Point"),
        Some(&SymbolKind::Class)
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "Status"),
        Some(&SymbolKind::Enum)
    );
}

#[test]
fn extracts_type_alias_symbol() {
    // `type WidgetId is uint256;` parses as `user_defined_type_definition`
    // (confirmed via a real parse tree dump, NOT the node kind literally
    // named `type_alias` -- that kind is actually the bare library-name
    // clause inside a `using X for Y;` directive's children; see
    // `LangSpec::solidity`'s own doc comment for the full correction).
    // `user_defined_type_definition` has a real `name` field, so this
    // uses the ordinary flat `name_field` path with no quirk needed.
    let src = "type WidgetId is uint256;";
    let parsed = parse_solidity(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "WidgetId"),
        Some(&SymbolKind::TypeAlias)
    );
}

#[test]
fn extracts_state_variable_and_struct_member_fields_as_defines() {
    let src = r#"
contract C {
    uint256 public count;

    struct Point {
        uint256 x;
    }
}
"#;
    let parsed = parse_solidity(src);
    assert!(
        parsed
            .defines
            .iter()
            .any(|d| d.container_name == "C" && d.member_name == "count"),
        "{:?}",
        parsed.defines
    );
    assert!(
        parsed
            .defines
            .iter()
            .any(|d| d.container_name == "Point" && d.member_name == "x"),
        "{:?}",
        parsed.defines
    );
}

#[test]
fn extracts_import_directive_and_using_directive_as_imports() {
    let src = r#"
import "./IWidget.sol";
using SafeMath for uint256;
"#;
    let parsed = parse_solidity(src);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"./IWidget.sol"));
    assert!(paths.contains(&"SafeMath"));
}

#[test]
fn extracts_call_edges_including_new_expression() {
    let src = r#"
contract C {
    function f(address a) public {
        helper();
        Helper h = new Helper(a);
        h.register();
    }
}

contract Helper {
    constructor(address a) {}
    function register() public {}
}
"#;
    let parsed = parse_solidity(src);
    let callees: Vec<&str> = parsed.calls.iter().map(|c| c.callee.as_str()).collect();
    assert!(callees.contains(&"helper"));
    // Qualified (`h.register`, not bare `register`): the receiver is
    // captured as part of the fully-written callee text, same
    // convention every other qualified call in this crate uses (e.g.
    // Go's `w.Draw`).
    assert!(callees.contains(&"h.register"));
    // `new Helper(a)` -- captured as `call_expression` whose unwrapped
    // `function` field is the nested `new_expression`'s own text
    // (includes the `new` keyword, matching how Go's `NewXxx(...)`
    // constructor-idiom convention is captured as literal callee text
    // too rather than stripped).
    assert!(callees.contains(&"new Helper"));
}

#[test]
fn call_inside_function_records_from_symbol_scope() -> TestResult {
    let src = r#"
contract C {
    function f() public {
        helper();
    }
}
"#;
    let parsed = parse_solidity(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper")
        .ok_or("expected a helper call")?;
    assert_eq!(call.from_symbol.as_deref(), Some("f"));
    Ok(())
}

#[test]
fn method_call_records_identifier_receiver() -> TestResult {
    let src = r#"
contract C {
    function f(Helper h) public {
        h.register();
    }
}
"#;
    let parsed = parse_solidity(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "h.register")
        .ok_or("expected an h.register call")?;
    assert_eq!(call.receiver_text.as_deref(), Some("h"));
    assert_eq!(call.receiver_hint, Some(ReceiverHint::Identifier));
    Ok(())
}

#[test]
fn extracts_branch_heavy_contract_method_without_panicking() {
    let src = r#"
contract C {
    function f(uint256 amount) public returns (uint256) {
        uint256 total = 0;
        if (amount > 0) {
            total += amount;
        } else {
            total += 1;
        }
        for (uint256 i = 0; i < amount; i++) {
            total += i;
        }
        while (total > 1000) {
            total -= 1;
        }
        return total;
    }
}
"#;
    let parsed = parse_solidity(src);
    assert_eq!(symbol_kind(&parsed.symbols, "f"), Some(&SymbolKind::Method));
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = r#"
import "./IWidget.sol";

contract Widget is IWidget {
    uint256 public count;

    function draw() external view returns (string memory) {
        return "x";
    }
}
"#;
    let first = parse_solidity(src);
    let second = parse_solidity(src);
    assert_eq!(first, second);
}

#[test]
fn contract_inheritance_clause_records_an_inherits_edge() -> TestResult {
    // language-parity wave G3 stage 1: `inheritance_specifier` is an
    // unfielded repeated child of `contract_declaration` (confirmed via
    // `tree-sitter-solidity`'s own `node-types.json`), so this was
    // previously falling through to zero INHERITS edges.
    let src = r#"
contract Base {}

contract Widget is Base {
    function draw() external {}
}
"#;
    let parsed = parse_solidity(src);
    let edge = parsed
        .inherits
        .iter()
        .find(|i| i.sub_name == "Widget")
        .ok_or("expected an INHERITS edge for Widget")?;
    assert_eq!(edge.super_name, "Base");
    Ok(())
}

#[test]
fn contract_with_multiple_bases_and_constructor_args_records_every_ancestor() {
    let src = r#"
contract Widget is Base1, Base2(42) {
    function draw() external {}
}
"#;
    let parsed = parse_solidity(src);
    let supers: Vec<&str> = parsed
        .inherits
        .iter()
        .filter(|i| i.sub_name == "Widget")
        .map(|i| i.super_name.as_str())
        .collect();
    assert_eq!(supers, vec!["Base1", "Base2"], "{:?}", parsed.inherits);
}

#[test]
fn interface_extending_another_interface_records_an_inherits_edge() -> TestResult {
    let src = r#"
interface IBase {}

interface IWidget is IBase {
    function draw() external;
}
"#;
    let parsed = parse_solidity(src);
    let edge = parsed
        .inherits
        .iter()
        .find(|i| i.sub_name == "IWidget")
        .ok_or("expected an INHERITS edge for IWidget")?;
    assert_eq!(edge.super_name, "IBase");
    Ok(())
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_solidity("contract ( { this is not valid solidity @@@");
    let _ = parsed;
}
