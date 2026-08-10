//! CP02 consumer-adoption contract for the accepted UL03 syntax interface.
//!
//! This test crosses the consumer seam through `enforcer-syntax` and the
//! existing scan validator assembly. It does not construct a parser, access a
//! Tree-sitter node, or introduce a second grammar/runtime owner.

use enforcer_scan::engine::build_family_validators;
use enforcer_syntax::parsers::{classify, parse_file, Language};

#[test]
fn security_consumer_uses_shared_syntax_for_typed_source_facts() -> Result<(), String> {
    let path = "tests/fixtures/cyberskills/web.command-injection/bad/inject.py";
    assert_eq!(classify(path), Language::Python);

    let parsed = parse_file(
        Language::Python,
        "def run(value):\n    return value\n",
        path,
    )
    .ok_or_else(|| "Python must have the accepted structural parser route".to_owned())?;
    assert!(parsed
        .symbols
        .iter()
        .any(|symbol| symbol.name.as_str() == "run"));
    Ok(())
}

#[test]
fn scan_consumer_assembles_cyberskills_without_a_second_syntax_owner() -> Result<(), String> {
    let validators =
        build_family_validators().map_err(|error| format!("validator build: {error:?}"))?;
    let debug = format!("{validators:?}");
    let cyberskills_count = debug
        .split("cyberskills: ")
        .nth(1)
        .and_then(|tail| tail.split(',').next())
        .and_then(|value| value.trim().parse::<usize>().ok())
        .ok_or_else(|| "family validator debug output must expose cyberskills count".to_owned())?;
    assert_eq!(cyberskills_count, 40);
    Ok(())
}
