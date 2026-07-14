//! Public-API coverage for Docker daemon TLS authentication invariants.

use enforcer_domain::findings::ScanScope;
use enforcer_domain::paths::RelPath;
use enforcer_lang_security::rules::cyberskills::docker_daemon::DockerDaemonHardeningValidator;
use enforcer_validator::validator::{ValidationInput, Validator};

#[test]
fn docker_daemon_flags_tls_without_client_certificate_verification(
) -> Result<(), Box<dyn std::error::Error>> {
    let validator = DockerDaemonHardeningValidator::new()?;
    let file: RelPath = "daemon.json".parse()?;
    let findings = validator.validate(ValidationInput {
        file: &file,
        source: r#"{"tls": true}"#,
        scope: ScanScope::Files,
    });

    assert_eq!(findings.len(), 1);
    assert_eq!(
        findings[0].title,
        "Docker daemon enables TLS without client certificate verification"
    );
    Ok(())
}

#[test]
fn docker_daemon_flags_remote_tcp_host_without_tls_settings(
) -> Result<(), Box<dyn std::error::Error>> {
    let validator = DockerDaemonHardeningValidator::new()?;
    let file: RelPath = "daemon.json".parse()?;
    let findings = validator.validate(ValidationInput {
        file: &file,
        source: r#"{"hosts":["TCP://0.0.0.0:2375","unix:///var/run/docker.sock"]}"#,
        scope: ScanScope::Files,
    });

    assert_eq!(findings.len(), 1);
    assert_eq!(
        findings[0].title,
        "Docker daemon exposes a remote TCP API without TLS"
    );
    assert_eq!(
        findings[0].severity,
        enforcer_domain::severity::Severity::Error
    );
    Ok(())
}
