// source owner: crates/enforcer-syntax/src/facts/function_facts.rs
// generator: cargo test -p enforcer-syntax --test fact_contract
// contractHash: 9e8d32f54f3b8685eebad27fe8f67b2abfb10b0feef1a17ba8767e99b15551da

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::memory_types::{ParserRelativePath, ParserSourceText};
use enforcer_domain::syntax_types::{
    ByteRange, FactCapability, LineRange, ParseOutcome, ProviderIdentity, SyntaxAnalysisResult,
};
use enforcer_syntax::facts::function_facts::analyze;
use enforcer_syntax::parsers::Language;

const RUST: &str = include_str!("fixtures/facts/positive/rust_function.rs");
const PYTHON: &str = include_str!("fixtures/facts/positive/python_function.py");
const TYPESCRIPT: &str = include_str!("fixtures/facts/positive/typescript_function.ts");
const MALFORMED_RUST: &str = include_str!("fixtures/facts/negative/malformed_rust.rs");

fn accepted(result: Result<SyntaxAnalysisResult, DecodeError>) -> Option<SyntaxAnalysisResult> {
    if let Err(error) = &result {
        assert_eq!(error.path.as_str(), "valid UL04 fixture must be accepted");
    }
    result.ok()
}

fn analyze_or_panic(
    language: Language,
    source: ParserSourceText<'_>,
    path: ParserRelativePath<'_>,
) -> Option<SyntaxAnalysisResult> {
    accepted(analyze(language, source, path))
}

#[test]
fn selected_providers_emit_closed_function_facts() {
    let cases = [
        (Language::Rust, RUST, "src/example.rs", "rust_function"),
        (
            Language::Python,
            PYTHON,
            "src/example.py",
            "python_function",
        ),
        (
            Language::TypeScript,
            TYPESCRIPT,
            "src/example.ts",
            "typescript_function",
        ),
    ];

    for (language, source, path, expected_name) in cases {
        let Some(result) = analyze_or_panic(
            language,
            ParserSourceText::from(source),
            ParserRelativePath::from(path),
        ) else {
            return;
        };
        assert_eq!(result.outcome(), ParseOutcome::ParsedClean);
        assert_eq!(result.error_count(), 0);
        assert_eq!(result.missing_count(), 0);
        assert!(result
            .capabilities()
            .contains(FactCapability::FunctionFacts));
        assert_eq!(result.function_facts().len(), 1);
        assert_eq!(result.function_facts()[0].name(), expected_name);
        assert!(result.function_facts()[0].span().byte_range().start() < source.len());
    }
}

#[test]
fn malformed_input_is_recovered_but_not_called_clean() {
    let Some(result) = analyze_or_panic(
        Language::Rust,
        ParserSourceText::from(MALFORMED_RUST),
        ParserRelativePath::from("src/broken.rs"),
    ) else {
        return;
    };
    assert_eq!(result.outcome(), ParseOutcome::ParsedWithErrors);
    assert!(result.error_count() > 0 || result.missing_count() > 0);
}

#[test]
fn unsupported_and_unavailable_are_distinct() {
    let Some(unsupported) = analyze_or_panic(
        Language::TextOnly,
        ParserSourceText::from(include_str!("fixtures/facts/negative/unsupported.txt")),
        ParserRelativePath::from("notes/unsupported.txt"),
    ) else {
        return;
    };
    assert_eq!(unsupported.outcome(), ParseOutcome::Unsupported);
    assert_eq!(
        unsupported.provenance().provider(),
        ProviderIdentity::Unsupported
    );
    assert!(unsupported.function_facts().is_empty());

    let Some(unavailable) = analyze_or_panic(
        Language::Dart,
        ParserSourceText::from("void unavailable() {}"),
        ParserRelativePath::from("src/example.dart"),
    ) else {
        return;
    };
    assert_eq!(unavailable.outcome(), ParseOutcome::ProviderUnavailable);
    assert_eq!(
        unavailable.provenance().provider(),
        ProviderIdentity::Unavailable
    );
    assert!(unavailable.function_facts().is_empty());
}

#[test]
fn unsafe_input_is_refused_before_provider() {
    let Some(result) = analyze_or_panic(
        Language::Rust,
        ParserSourceText::from("\0fn unsafe_input() {}"),
        ParserRelativePath::from("src/unsafe.rs"),
    ) else {
        return;
    };
    assert_eq!(result.outcome(), ParseOutcome::UnsafeInputRefused);
    assert_eq!(result.error_count(), 0);
    assert!(result.function_facts().is_empty());
}

#[test]
fn spans_reject_invalid_order_and_zero_lines() {
    let start = 10;
    let end = 2;
    let reversed = ByteRange::try_from_range(start..end);
    assert_eq!(
        reversed.as_ref().err().map(|error| error.path.as_str()),
        Some("span.bytes")
    );
    let zero_line = LineRange::try_from_range(0..=1);
    assert_eq!(
        zero_line.as_ref().err().map(|error| error.path.as_str()),
        Some("span.lines")
    );
}
