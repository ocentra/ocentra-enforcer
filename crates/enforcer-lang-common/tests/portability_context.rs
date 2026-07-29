use enforcer_domain::findings::ScanScope;
use enforcer_domain::paths::RelPath;
use enforcer_lang_common::port_platform::{DeclaredScope, PortabilityValidator};
use enforcer_validator::validator::{ValidationInput, Validator};

fn validate(
    path: &str,
    source: &str,
) -> Result<Vec<enforcer_domain::findings::Finding>, Box<dyn std::error::Error>> {
    let validator = PortabilityValidator::new("PORT-1.1".parse()?, DeclaredScope::Undeclared);
    let file: RelPath = path.parse()?;
    Ok(validator.validate(ValidationInput {
        file: &file,
        source: enforcer_domain::boundary::validation::ValidationSource::from_text(source),
        scope: ScanScope::Files,
    }))
}

#[test]
fn documentation_and_generated_template_content_are_not_commands(
) -> Result<(), Box<dyn std::error::Error>> {
    assert!(validate("scripts/README.md", "# run build.ps1\n")?.is_empty());
    assert!(validate(
        "scripts/generated/build.template.ps1",
        "run build.ps1 --release\n"
    )?
    .is_empty());
    Ok(())
}

#[test]
fn executable_script_commands_remain_governed() -> Result<(), Box<dyn std::error::Error>> {
    let findings = validate("scripts/build.mjs", "run build.ps1 --release\n")?;
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id.as_str(), "PORT-1.1");
    Ok(())
}

#[test]
fn non_script_source_mentions_are_not_commands() -> Result<(), Box<dyn std::error::Error>> {
    assert!(validate("src/lib.rs", "let example = \"build.ps1\";\n")?.is_empty());
    Ok(())
}
