use enforcer_domain::findings::ScanScope;
use enforcer_domain::paths::RelPath;
use enforcer_lang_ts::rules::frontend_react::validators;
use enforcer_validator::validator::ValidationInput;

#[test]
fn frontend_react_validators_handle_truncated_syntax_without_panicking(
) -> Result<(), Box<dyn std::error::Error>> {
    let file: RelPath = "features/".parse()?;
    for validator in validators()? {
        let _ = validator.validate(ValidationInput {
            file: &file,
            source: "import {\nuseEffect(\n: any\n",
            scope: ScanScope::Files,
        });
    }
    Ok(())
}
