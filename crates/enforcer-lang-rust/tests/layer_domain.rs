use enforcer_domain::findings::ScanScope;
use enforcer_domain::paths::RelPath;
use enforcer_lang_rust::rules::error_handling::layer_domain::LayerDomainValidator;
use enforcer_validator::validator::{ValidationInput, Validator};

fn validate_domain_source(source: &str) -> Result<Vec<enforcer_domain::findings::Finding>, String> {
    let validator = LayerDomainValidator::new().map_err(|error| error.to_string())?;
    let file: RelPath = "crates/example/src/domain/service.rs"
        .parse()
        .map_err(|error: enforcer_domain::boundary::decode_error::DecodeError| error.to_string())?;
    Ok(validator.validate(ValidationInput {
        file: &file,
        source: enforcer_domain::boundary::validation::ValidationSource::from_text(source),
        scope: ScanScope::Files,
    }))
}

#[test]
fn detects_forbidden_root_and_two_segment_imports_without_flagging_near_matches(
) -> Result<(), String> {
    let findings = validate_domain_source(
        "use reqwest::Client;\nuse tokio::net::TcpStream;\nuse tokio::time::sleep;",
    )?;
    let titles: Vec<&str> = findings
        .iter()
        .map(|finding| finding.title.as_str())
        .collect();
    assert_eq!(titles.len(), 2);
    assert!(titles.iter().any(|title| title.contains("reqwest")));
    assert!(titles.iter().any(|title| title.contains("tokio::net")));
    assert!(!titles.iter().any(|title| title.contains("tokio::time")));
    Ok(())
}

#[test]
fn nested_and_empty_import_groups_remain_total_without_changing_group_semantics(
) -> Result<(), String> {
    let findings = validate_domain_source("use tokio::{fs, time};\nuse std::{};")?;
    assert!(findings.is_empty());
    Ok(())
}
