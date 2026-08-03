//! Bounded function-fact provider for the UL04 contract slice.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::memory_types::{ParserRelativePath, ParserSourceText};
use enforcer_domain::paths::RelPath;
use enforcer_domain::syntax_types::{
    ByteRange, CapabilitySet, FunctionFact, LanguageIdentity, LineRange, ParseOutcome,
    ParseQuality, ProviderIdentity, ProviderProvenance, ProviderVersion, SyntaxAnalysisInput,
    SyntaxAnalysisResult, SyntaxSpan,
};
use tree_sitter::{Node, Parser};

use crate::languages;
use crate::parsers::Language;

/// Analyze one bounded language slice without exposing a native parse tree.
pub fn analyze(
    language: Language,
    source: ParserSourceText<'_>,
    rel_path: ParserRelativePath<'_>,
) -> Result<SyntaxAnalysisResult, DecodeError> {
    let source_text = source.as_str();
    let file = RelPath::try_new(rel_path.as_str())?;
    // ALLOC-JUSTIFICATION: the result owns one short, stable language label.
    let language_identity = LanguageIdentity::try_new(format!("{language:?}"))?;

    if matches!(language, Language::TextOnly) {
        return SyntaxAnalysisResult::try_new(
            SyntaxAnalysisInput::empty()
                .with_language(language_identity)
                .with_file(file)
                .with_provenance(ProviderProvenance::new(
                    ProviderIdentity::Unsupported,
                    ProviderVersion::TreeSitter025,
                ))
                .with_outcome(ParseOutcome::Unsupported)
                .with_quality(ParseQuality::NotParsed)
                .with_capabilities(CapabilitySet::empty())
                .with_function_facts(Vec::new()),
        );
    }

    if languages::has_unsafe_tree_sitter_input(source_text) {
        return SyntaxAnalysisResult::try_new(
            SyntaxAnalysisInput::empty()
                .with_language(language_identity)
                .with_file(file)
                .with_provenance(ProviderProvenance::new(
                    ProviderIdentity::Unavailable,
                    ProviderVersion::TreeSitter025,
                ))
                .with_outcome(ParseOutcome::UnsafeInputRefused)
                .with_quality(ParseQuality::NotParsed)
                .with_capabilities(CapabilitySet::empty())
                .with_function_facts(Vec::new()),
        );
    }

    let Some((provider, version, grammar)) = provider_for(language) else {
        return SyntaxAnalysisResult::try_new(
            SyntaxAnalysisInput::empty()
                .with_language(language_identity)
                .with_file(file)
                .with_provenance(ProviderProvenance::new(
                    ProviderIdentity::Unavailable,
                    ProviderVersion::TreeSitter025,
                ))
                .with_outcome(ParseOutcome::ProviderUnavailable)
                .with_quality(ParseQuality::NotParsed)
                .with_capabilities(CapabilitySet::empty())
                .with_function_facts(Vec::new()),
        );
    };

    let mut parser = Parser::new();
    parser
        .set_language(&grammar)
        .map_err(|_| DecodeError::new("provider", "provider rejected the grammar binding"))?;
    let tree = parser
        .parse(source_text, None)
        .ok_or_else(|| DecodeError::new("provider", "parser returned no syntax tree"))?;
    let quality_counts = count_quality_nodes(tree.root_node());
    let error_count = quality_counts.errors;
    let missing_count = quality_counts.missing;
    let mut facts = Vec::new();
    collect_function_facts(
        tree.root_node(),
        ParserSourceText::from(source_text),
        language,
        &mut facts,
    );
    let (outcome, quality) = if error_count == 0 && missing_count == 0 {
        (ParseOutcome::ParsedClean, ParseQuality::Clean)
    } else {
        (
            ParseOutcome::ParsedWithErrors,
            ParseQuality::recovered(
                std::num::NonZeroUsize::new(error_count),
                std::num::NonZeroUsize::new(missing_count),
            ),
        )
    };

    SyntaxAnalysisResult::try_new(
        SyntaxAnalysisInput::empty()
            .with_language(language_identity)
            .with_file(file)
            .with_provenance(ProviderProvenance::new(provider, version))
            .with_outcome(outcome)
            .with_quality(quality)
            .with_capabilities(CapabilitySet::function_facts())
            .with_function_facts(facts),
    )
}

fn provider_for(
    language: Language,
) -> Option<(ProviderIdentity, ProviderVersion, tree_sitter::Language)> {
    match language {
        Language::Rust => Some((
            ProviderIdentity::TreeSitterRust,
            ProviderVersion::Rust023,
            tree_sitter_rust::LANGUAGE.into(),
        )),
        Language::Python => Some((
            ProviderIdentity::TreeSitterPython,
            ProviderVersion::Python023,
            tree_sitter_python::LANGUAGE.into(),
        )),
        Language::TypeScript | Language::JavaScript => Some((
            ProviderIdentity::TreeSitterTypeScript,
            ProviderVersion::TypeScript023,
            tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        )),
        Language::Tsx => Some((
            ProviderIdentity::TreeSitterTypeScript,
            ProviderVersion::TypeScript023,
            tree_sitter_typescript::LANGUAGE_TSX.into(),
        )),
        Language::Go => Some((
            ProviderIdentity::TreeSitterGo,
            ProviderVersion::Go023,
            tree_sitter_go::LANGUAGE.into(),
        )),
        _ => None,
    }
}

fn count_quality_nodes(node: Node<'_>) -> QualityCounts {
    let mut counts = QualityCounts {
        errors: usize::from(node.is_error()),
        missing: usize::from(node.is_missing()),
    };
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        let child_counts = count_quality_nodes(child);
        counts.errors = counts.errors.saturating_add(child_counts.errors);
        counts.missing = counts.missing.saturating_add(child_counts.missing);
    }
    counts
}

/// BRAND-INVARIANT: counts are observations from one native parse tree and
/// never cross the public fact boundary as an unbranded aggregate.
#[derive(Debug, Clone, Copy)]
struct QualityCounts {
    errors: usize,
    missing: usize,
}

fn collect_function_facts(
    node: Node<'_>,
    source: ParserSourceText<'_>,
    language: Language,
    facts: &mut Vec<FunctionFact>,
) {
    if function_node_kind(node, language).is_some() {
        if let (Some(name_node), Ok(span)) =
            (node.child_by_field_name("name"), span_for(node, source))
        {
            if let Ok(name) = name_node.utf8_text(source.as_str().as_bytes()) {
                // ALLOC-JUSTIFICATION: one owned function name is the fact value.
                if let Ok(fact) = FunctionFact::try_new(name.to_owned(), span) {
                    facts.push(fact);
                }
            }
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_function_facts(child, source, language, facts);
    }
}

#[derive(Debug, Clone, Copy)]
enum FunctionNodeKind {
    Declaration,
}

fn function_node_kind(node: Node<'_>, language: Language) -> Option<FunctionNodeKind> {
    let kind = node.kind();
    match language {
        Language::Rust if kind == "function_item" => Some(FunctionNodeKind::Declaration),
        Language::Python if matches!(kind, "function_definition" | "async_function_definition") => {
            Some(FunctionNodeKind::Declaration)
        }
        Language::TypeScript | Language::JavaScript | Language::Tsx => {
            if matches!(kind, "function_declaration" | "method_definition") {
                Some(FunctionNodeKind::Declaration)
            } else {
                None
            }
        }
        Language::Go if matches!(kind, "function_declaration" | "method_declaration") => {
            Some(FunctionNodeKind::Declaration)
        }
        _ => None,
    }
}

fn span_for(node: Node<'_>, source: ParserSourceText<'_>) -> Result<SyntaxSpan, DecodeError> {
    let range = node.byte_range();
    if range.end > source.as_str().len() {
        return Err(DecodeError::new(
            "span.bytes",
            "provider span exceeds source bytes",
        ));
    }
    let byte_range = ByteRange::try_from_range(range)?;
    let line_range = LineRange::try_from_range(
        node.start_position().row.saturating_add(1)..=node.end_position().row.saturating_add(1),
    )?;
    Ok(SyntaxSpan::from_ranges(byte_range, line_range))
}
