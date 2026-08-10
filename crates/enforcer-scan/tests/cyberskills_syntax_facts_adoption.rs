//! BOUNDARY-INVARIANT: this test consumes only the public UL04 function-fact
//! interface and never imports a parser tree or grammar implementation.
//!
//! NEGATIVE-TEST: malformed, unsupported, unavailable, and unsafe inputs are
//! explicit non-clean outcomes rather than empty successful fact sets.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::memory_types::{ParserRelativePath, ParserSourceText};
use enforcer_domain::syntax_types::{FactCapability, ParseOutcome, ProviderIdentity};
use enforcer_syntax::facts::function_facts::analyze;
use enforcer_syntax::parsers::Language;

fn analyze_rust(
    source: &str,
) -> Result<enforcer_domain::syntax_types::SyntaxAnalysisResult, DecodeError> {
    analyze(
        Language::Rust,
        ParserSourceText::from(source),
        ParserRelativePath::from("src/example.rs"),
    )
}

#[test]
fn clean_function_facts_cross_the_typed_consumer_seam() -> Result<(), Box<dyn std::error::Error>> {
    let source = "fn accepted() {}\n";
    let result = analyze_rust(source)?;

    assert_eq!(result.outcome(), ParseOutcome::ParsedClean);
    assert_eq!(result.error_count(), 0);
    assert_eq!(result.missing_count(), 0);
    assert!(result
        .capabilities()
        .contains(FactCapability::FunctionFacts));
    assert_eq!(result.function_facts().len(), 1);
    assert_eq!(result.function_facts()[0].name(), "accepted");
    assert!(result.function_facts()[0].span().byte_range().end() <= source.len());
    assert_eq!(result.function_facts()[0].span().line_range().start(), 1);
    Ok(())
}

#[test]
fn malformed_function_input_is_recovered_but_not_clean() -> Result<(), Box<dyn std::error::Error>> {
    let result = analyze_rust("fn broken( { let _ = 1; }\n")?;

    assert_eq!(result.outcome(), ParseOutcome::ParsedWithErrors);
    assert!(result.error_count() > 0 || result.missing_count() > 0);
    Ok(())
}

#[test]
fn unsupported_unavailable_and_unsafe_inputs_remain_explicit(
) -> Result<(), Box<dyn std::error::Error>> {
    let unsupported = analyze(
        Language::TextOnly,
        ParserSourceText::from("plain notes\n"),
        ParserRelativePath::from("notes/unsupported.txt"),
    )?;
    assert_eq!(unsupported.outcome(), ParseOutcome::Unsupported);
    assert_eq!(
        unsupported.provenance().provider(),
        ProviderIdentity::Unsupported
    );
    assert!(unsupported.function_facts().is_empty());

    let unavailable = analyze(
        Language::Dart,
        ParserSourceText::from("void unavailable() {}\n"),
        ParserRelativePath::from("src/example.dart"),
    )?;
    assert_eq!(unavailable.outcome(), ParseOutcome::ProviderUnavailable);
    assert_eq!(
        unavailable.provenance().provider(),
        ProviderIdentity::Unavailable
    );
    assert!(unavailable.function_facts().is_empty());

    let unsafe_input = analyze(
        Language::Rust,
        ParserSourceText::from("\0fn unsafe_input() {}\n"),
        ParserRelativePath::from("src/unsafe.rs"),
    )?;
    assert_eq!(unsafe_input.outcome(), ParseOutcome::UnsafeInputRefused);
    assert!(unsafe_input.function_facts().is_empty());
    Ok(())
}
