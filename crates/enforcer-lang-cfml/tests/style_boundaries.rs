use enforcer_domain::boundary::validation::ValidationSource;
use enforcer_domain::findings::ScanScope;
use enforcer_domain::paths::RelPath;
use enforcer_lang_cfml::rules::style::{MissingVarScopeValidator, TypedSignatureValidator};
use enforcer_validator::validator::{ValidationInput, Validator};

fn validate(
    validator: &dyn Validator,
    source: ValidationSource<'_>,
) -> Result<Vec<enforcer_domain::findings::Finding>, Box<dyn std::error::Error>> {
    let file: RelPath = "src/example.cfc".parse()?;
    Ok(validator.validate(ValidationInput {
        file: &file,
        source,
        scope: ScanScope::Files,
    }))
}

#[test]
fn style_parsers_reject_malformed_boundaries_without_panicking(
) -> Result<(), Box<dyn std::error::Error>> {
    let signature = TypedSignatureValidator::new()?;
    let missing_scope = MissingVarScopeValidator::new()?;

    assert!(validate(
        &signature,
        ValidationSource::from_text("public function save(")
    )?
    .is_empty());
    assert!(validate(
        &missing_scope,
        ValidationSource::from_text("function demo() {\nname =")
    )?
    .is_empty());
    assert_eq!(
        validate(
            &signature,
            ValidationSource::from_text("public function save(id)")
        )?
        .len(),
        1
    );
    assert_eq!(
        validate(
            &missing_scope,
            ValidationSource::from_text("function demo() {\nname = value;"),
        )?
        .len(),
        1
    );
    Ok(())
}
