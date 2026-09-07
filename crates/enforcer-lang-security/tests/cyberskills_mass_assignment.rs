//! BOUNDARY-INVARIANT: CP04 exercises one existing rule through the public
//! function-facts seam; the test never imports parser nodes or grammar APIs.
//!
//! NEGATIVE-TEST: unavailable, malformed, alias, and multiline cases remain
//! explicit non-proofs rather than silently becoming native coverage.

use enforcer_domain::boundary::validation::ValidationSource;
use enforcer_domain::findings::ScanScope;
use enforcer_domain::memory_types::{ParserRelativePath, ParserSourceText};
use enforcer_domain::paths::RelPath;
use enforcer_domain::syntax_types::{ParseOutcome, ProviderVersion};
use enforcer_lang_security::rules::cyberskills::mass_assignment::MassAssignmentValidator;
use enforcer_syntax::facts::function_facts::analyze;
use enforcer_syntax::parsers::Language;
use enforcer_validator::analysis::{content_hash, AnalysisOutcome, PreparedAnalysis};
use enforcer_validator::validator::{AnalysisSkip, ValidationDispatch, ValidationInput, Validator};

fn input<'a>(file: &'a RelPath, source: &'a str) -> ValidationInput<'a> {
    ValidationInput {
        file,
        source: ValidationSource::from_text(source),
        scope: ScanScope::Files,
    }
}

fn prepared(
    source: &str,
    language: Language,
    path: &str,
) -> Result<PreparedAnalysis, Box<dyn std::error::Error>> {
    let result = analyze(
        language,
        ParserSourceText::from(source),
        ParserRelativePath::from(path),
    )?;
    assert_eq!(result.outcome(), ParseOutcome::ParsedClean);
    Ok(PreparedAnalysis::new(
        content_hash(ValidationSource::from_text(source)),
        ProviderVersion::TreeSitter025,
        AnalysisOutcome::FactBacked(result),
    ))
}

fn fact_findings(
    validator: &MassAssignmentValidator,
    source: &str,
    language: Language,
    path: &str,
) -> Result<usize, Box<dyn std::error::Error>> {
    let file: RelPath = path.parse()?;
    let analysis = prepared(source, language, path)?;
    let dispatch = validator.validate_with_analysis(input(&file, source), Some(&analysis));
    match dispatch {
        ValidationDispatch::Ran(findings) => Ok(findings.len()),
        ValidationDispatch::Skipped(reason) => {
            Err(format!("fact-backed case unexpectedly skipped: {reason:?}").into())
        }
    }
}

#[test]
fn function_facts_remove_top_level_prose_and_preserve_real_python_sink(
) -> Result<(), Box<dyn std::error::Error>> {
    let validator = MassAssignmentValidator::new()?;
    let prose = "# user.update(**request.json)\n\nvalue = 1\n";
    let prose_file: RelPath = "app.py".parse()?;
    assert_eq!(validator.validate(input(&prose_file, prose)).len(), 1);
    assert_eq!(
        fact_findings(&validator, prose, Language::Python, "app.py")?,
        0
    );

    let vulnerable = "def update():\n    user.update(**request.json)\n";
    let vulnerable_file: RelPath = "app.py".parse()?;
    assert_eq!(
        validator
            .validate(input(&vulnerable_file, vulnerable))
            .len(),
        1
    );
    assert_eq!(
        fact_findings(&validator, vulnerable, Language::Python, "app.py")?,
        1
    );
    Ok(())
}

#[test]
fn function_facts_preserve_javascript_sink_and_record_unproved_shapes(
) -> Result<(), Box<dyn std::error::Error>> {
    let validator = MassAssignmentValidator::new()?;
    let javascript = "function update(req) { return User.create(req.body); }\n";
    let file: RelPath = "app.js".parse()?;
    assert_eq!(validator.validate(input(&file, javascript)).len(), 1);
    assert_eq!(
        fact_findings(&validator, javascript, Language::JavaScript, "app.js")?,
        1
    );

    let multiline = "def update():\n    user.update(\n        **request.json\n    )\n";
    let multiline_file: RelPath = "app.py".parse()?;
    assert_eq!(
        validator.validate(input(&multiline_file, multiline)).len(),
        0
    );
    assert_eq!(
        fact_findings(&validator, multiline, Language::Python, "app.py")?,
        0
    );

    let alias = "from flask import request as req\ndef update():\n    user.update(**req.json)\n";
    let alias_file: RelPath = "app.py".parse()?;
    assert_eq!(validator.validate(input(&alias_file, alias)).len(), 0);
    assert_eq!(
        fact_findings(&validator, alias, Language::Python, "app.py")?,
        0
    );
    Ok(())
}

#[test]
fn malformed_provider_result_is_explicitly_unavailable_to_the_rule(
) -> Result<(), Box<dyn std::error::Error>> {
    let validator = MassAssignmentValidator::new()?;
    let source = "def broken(:\n    user.update(**request.json)\n";
    let file: RelPath = "app.py".parse()?;
    let result = analyze(
        Language::Python,
        ParserSourceText::from(source),
        ParserRelativePath::from("app.py"),
    )?;
    assert_eq!(result.outcome(), ParseOutcome::ParsedWithErrors);
    let prepared = PreparedAnalysis::new(
        content_hash(ValidationSource::from_text(source)),
        ProviderVersion::TreeSitter025,
        AnalysisOutcome::FactBacked(result),
    );
    assert_eq!(
        validator.validate_with_analysis(input(&file, source), Some(&prepared)),
        ValidationDispatch::Skipped(AnalysisSkip::RequirementUnavailable)
    );
    assert_eq!(validator.validate(input(&file, source)).len(), 1);
    Ok(())
}
