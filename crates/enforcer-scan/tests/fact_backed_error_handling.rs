//! BOUNDARY-INVARIANT: this integration test proves one Rust validator consumes
//! the existing normalized function-fact capability without changing the
//! shared parser, fact, or dispatch contracts.
//!
//! NEGATIVE-TEST: legacy/unavailable analysis must be an explicit skipped
//! dispatch, never a falsely clean fact-backed result.

use std::fs;

use enforcer_domain::boundary::validation::ValidationSource;
use enforcer_domain::findings::{Finding, ScanScope};
use enforcer_domain::memory_types::{ParserRelativePath, ParserSourceText};
use enforcer_domain::paths::{RelPath, RepoRoot};
use enforcer_domain::scan_types::ResolvedScope;
use enforcer_domain::syntax_types::{ParseOutcome, ProviderVersion};
use enforcer_lang_rust::rules::error_handling::ErrorHandlingValidator;
use enforcer_scan::engine::{build_family_validators, run_with_analysis_provider};
use enforcer_validator::analysis::{
    content_hash, AnalysisOutcome, AnalysisProvider, PreparedAnalysis,
};
use enforcer_validator::validator::{AnalysisSkip, ValidationDispatch, ValidationInput, Validator};
use tempfile::tempdir;

struct RustFunctionFactsProvider;

impl AnalysisProvider for RustFunctionFactsProvider {
    fn provider_version(&self) -> ProviderVersion {
        ProviderVersion::Rust023
    }

    fn analyze(
        &self,
        file: &RelPath,
        source: ValidationSource<'_>,
        _scope: ScanScope,
    ) -> AnalysisOutcome {
        let result = enforcer_syntax::facts::function_facts::analyze(
            enforcer_syntax::parsers::Language::Rust,
            ParserSourceText::from(source.as_str()),
            ParserRelativePath::from(file.as_str()),
        );
        match result {
            Ok(result) => AnalysisOutcome::FactBacked(result),
            Err(_) => AnalysisOutcome::ParserFailure,
        }
    }
}

fn prepared_facts(
    file: &RelPath,
    source: &str,
) -> Result<PreparedAnalysis, Box<dyn std::error::Error>> {
    let outcome = RustFunctionFactsProvider.analyze(
        file,
        ValidationSource::from_text(source),
        ScanScope::Files,
    );
    let content_hash = content_hash(ValidationSource::from_text(source));
    Ok(PreparedAnalysis::new(
        content_hash,
        ProviderVersion::Rust023,
        outcome,
    ))
}

fn direct_input<'a>(file: &'a RelPath, source: &'a str) -> ValidationInput<'a> {
    ValidationInput {
        file,
        source: ValidationSource::from_text(source),
        scope: ScanScope::Files,
    }
}

fn error_findings(findings: &[Finding]) -> Vec<&Finding> {
    findings
        .iter()
        .filter(|finding| finding.rule_id.as_str() == "T1-RUSTERR.1")
        .collect()
}

fn scan_one(source: &str) -> Result<enforcer_domain::findings::Report, Box<dyn std::error::Error>> {
    let directory = tempdir()?;
    fs::create_dir_all(directory.path().join("src"))?;
    fs::write(directory.path().join("src/lib.rs"), source)?;
    let root: RepoRoot = directory.path().to_string_lossy().parse()?;
    let file: RelPath = "src/lib.rs".parse()?;
    let scope = ResolvedScope {
        kind: ScanScope::Files,
        repo_root: root,
        explicit_paths: vec![file.clone()],
        diff_range: None,
    };
    let validators = build_family_validators()?;
    Ok(run_with_analysis_provider(
        &scope,
        std::slice::from_ref(&file),
        &validators,
        &RustFunctionFactsProvider,
        enforcer_domain::config_types::InlineTestPolicy::Allow,
    ))
}

#[test]
fn fact_backed_scan_preserves_rule_identity_and_order() -> Result<(), Box<dyn std::error::Error>> {
    let report = scan_one("fn first() { panic!(\"stop\"); }\nfn second() { Some(1).unwrap(); }\n")?;
    let findings = error_findings(&report.findings);
    assert_eq!(findings.len(), 2);
    assert!(findings
        .iter()
        .all(|finding| finding.severity == enforcer_domain::severity::Severity::Error));
    assert_eq!(
        findings[0]
            .line
            .source_line()
            .map(|line| line.value().get()),
        Some(1)
    );
    assert_eq!(
        findings[1]
            .line
            .source_line()
            .map(|line| line.value().get()),
        Some(2)
    );
    let lines = findings
        .iter()
        .map(|finding| finding.line.source_line().map(|line| line.value().get()))
        .collect::<Vec<_>>();
    assert_eq!(lines, vec![Some(1), Some(2)]);
    Ok(())
}

#[test]
fn fact_backed_and_legacy_paths_match_edge_fixture_behavior(
) -> Result<(), Box<dyn std::error::Error>> {
    let validator = ErrorHandlingValidator::new()?;
    let file: RelPath = "src/lib.rs".parse()?;
    let fixtures = [
        ("fn bad() { Some(1).unwrap(); }\n", true, "banned call"),
        (
            "fn clean() { let _ = \"value.unwrap()\"; }\n",
            false,
            "string literal",
        ),
        ("fn clean() { // value.unwrap()\n }\n", false, "comment"),
        (
            "fn alias() { let value = Some(1); let alias = value; alias.unwrap(); }\n",
            true,
            "alias call",
        ),
        (
            "fn broken( { Some(1).unwrap(); }\n",
            false,
            "malformed source",
        ),
    ];
    for (source, should_find, label) in fixtures {
        let legacy = validator.validate(direct_input(&file, source));
        let prepared = prepared_facts(&file, source)?;
        let fact_backed =
            match validator.validate_with_analysis(direct_input(&file, source), Some(&prepared)) {
                ValidationDispatch::Ran(findings) => findings,
                ValidationDispatch::Skipped(reason) => {
                    return Err(format!("{label} unexpectedly skipped: {reason:?}").into())
                }
            };
        assert_eq!(legacy, fact_backed, "old/new mismatch for {label}");
        assert_eq!(
            should_find,
            !error_findings(&fact_backed).is_empty(),
            "{label}"
        );
    }
    Ok(())
}

#[test]
fn unavailable_function_facts_are_explicitly_skipped() -> Result<(), Box<dyn std::error::Error>> {
    let validator = ErrorHandlingValidator::new()?;
    let file: RelPath = "src/lib.rs".parse()?;
    let source = "fn bad() { Some(1).unwrap(); }\n";
    let legacy_analysis = PreparedAnalysis::new(
        content_hash(ValidationSource::from_text(source)),
        ProviderVersion::TreeSitter025,
        AnalysisOutcome::LegacyText,
    );
    assert_eq!(
        validator.validate_with_analysis(direct_input(&file, source), None),
        ValidationDispatch::Skipped(AnalysisSkip::NotPrepared)
    );
    assert_eq!(
        validator.validate_with_analysis(direct_input(&file, source), Some(&legacy_analysis)),
        ValidationDispatch::Skipped(AnalysisSkip::RequirementUnavailable)
    );
    assert!(matches!(
        prepared_facts(&file, source)?.outcome(),
        AnalysisOutcome::FactBacked(result)
            if matches!(result.outcome(), ParseOutcome::ParsedClean | ParseOutcome::ParsedWithErrors)
    ));
    Ok(())
}
