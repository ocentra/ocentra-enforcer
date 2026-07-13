//! External behavioral evidence for the CFML error-handling validators.

use enforcer_domain::findings::ScanScope;
use enforcer_domain::paths::RelPath;
use enforcer_validator::validator::{ValidationInput, Validator};

use enforcer_lang_cfml::rules::err::{EmptyCatchSwallowValidator, TypedThrowValidator};

fn validate(
    validator: &dyn Validator,
    source: &str,
) -> Result<Vec<enforcer_domain::findings::Finding>, Box<dyn std::error::Error>> {
    let file: RelPath = "src/example.cfc".parse()?;
    Ok(validator.validate(ValidationInput {
        file: &file,
        source,
        scope: ScanScope::Files,
    }))
}

#[test]
fn typed_throw_rejects_a_bare_message_throw() -> Result<(), Box<dyn std::error::Error>> {
    let validator = TypedThrowValidator::new()?;
    let findings = validate(&validator, "throw(message=\"bad input\");")?;
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id.as_str(), "CF-ERR-1.1");
    Ok(())
}

#[test]
fn typed_throw_accepts_a_namespaced_type() -> Result<(), Box<dyn std::error::Error>> {
    let validator = TypedThrowValidator::new()?;
    let findings = validate(
        &validator,
        "throw(type=\"app.validation.invalidOrder\", message=\"bad input\");",
    )?;
    assert!(findings.is_empty());
    Ok(())
}

#[test]
fn empty_catch_and_return_true_catch_are_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let validator = EmptyCatchSwallowValidator::new()?;
    assert_eq!(validate(&validator, "catch(any problem) {}")?.len(), 1);
    assert_eq!(
        validate(&validator, "catch(any problem) { return true; }")?.len(),
        1
    );
    Ok(())
}

#[test]
fn rethrowing_catch_is_accepted() -> Result<(), Box<dyn std::error::Error>> {
    let validator = EmptyCatchSwallowValidator::new()?;
    assert!(validate(&validator, "catch(any problem) { rethrow; }")?.is_empty());
    Ok(())
}
