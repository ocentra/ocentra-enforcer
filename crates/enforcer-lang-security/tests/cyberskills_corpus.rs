//! Pen-test-grade labeled corpus for the h11 cyberskills source-pattern
//! families. Each `_corpus/<family>.json` holds many minimal inputs labeled
//! by VENDOR behavior: `flag` (a real misconfig/attack the family must
//! detect — no false negatives) or `clean` (a benign near-miss the family
//! must NOT flag — no false positives). Every case runs against the whole
//! family (union of its validators); a `flag` case must yield >=1 finding,
//! a `clean` case must yield 0. This is the detection/prevention proof: it
//! exercises hundreds of variants, not one happy-path fixture pair.
//!
//! The corpus was generated from the vendored skills and then RECONCILED:
//! every case was run against the validators and each mismatch adjudicated
//! against the vendor source (a mislabeled case was corrected to the vendor
//! verdict; a real detection gap was fixed in the rule). Cases for vendor
//! checks outside h11's named thin-slice (e.g. the additional Azure
//! encryption/network checks) were removed and are tracked as follow-ups.

use std::path::PathBuf;

use enforcer_domain::findings::ScanScope;
use enforcer_domain::paths::RelPath;
use enforcer_lang_security::rules::cyberskills::auth_jwt::JwtSecurityValidator;
use enforcer_lang_security::rules::cyberskills::cloud_aws::AwsResourceHardeningValidator;
use enforcer_lang_security::rules::cyberskills::cloud_azure::{
    AzureStorageMinTls12Validator, AzureStoragePublicBlobValidator,
    AzureStorageRequireHttpsValidator,
};
use enforcer_lang_security::rules::cyberskills::cloud_gcp::GcpResourceHardeningValidator;
use enforcer_lang_security::rules::cyberskills::cmd_injection::CommandInjectionValidator;
use enforcer_lang_security::rules::cyberskills::dependency_confusion::DependencyConfusionClaimableValidator;
use enforcer_lang_security::rules::cyberskills::docker_daemon::DockerDaemonHardeningValidator;
use enforcer_lang_security::rules::cyberskills::dockerfile_hardening::DockerfileHardeningValidator;
use enforcer_lang_security::rules::cyberskills::fileless_malware::FilelessMalwareValidator;
use enforcer_lang_security::rules::cyberskills::github_actions::GithubActionsSecurityValidator;
use enforcer_lang_security::rules::cyberskills::iac_terraform::{
    IamNoWildcardActionValidator, S3EncryptionRequiredValidator, SgNoPublicSshIngressValidator,
};
use enforcer_lang_security::rules::cyberskills::insecure_deser::InsecureDeserializationValidator;
use enforcer_lang_security::rules::cyberskills::k8s_pod_security::K8sPodSecurityValidator;
use enforcer_lang_security::rules::cyberskills::k8s_rbac::K8sRbacValidator;
use enforcer_lang_security::rules::cyberskills::mass_assignment::MassAssignmentValidator;
use enforcer_lang_security::rules::cyberskills::mcp_tool_poisoning::McpToolPoisoningValidator;
use enforcer_lang_security::rules::cyberskills::net_tls::TlsLegacyVersionValidator;
use enforcer_lang_security::rules::cyberskills::nosql_injection::NoSqlInjectionValidator;
use enforcer_lang_security::rules::cyberskills::oauth_misconfig::OauthMisconfigValidator;
use enforcer_lang_security::rules::cyberskills::path_traversal::PathTraversalValidator;
use enforcer_lang_security::rules::cyberskills::proto_pollution::PrototypePollutionValidator;
use enforcer_lang_security::rules::cyberskills::sqli_source::SqlInjectionSourceValidator;
use enforcer_lang_security::rules::cyberskills::ssti::TemplateInjectionValidator;
use enforcer_lang_security::rules::cyberskills::tls_verify::TlsVerificationDisabledValidator;
use enforcer_lang_security::rules::cyberskills::type_juggle::TypeJugglingValidator;
use enforcer_lang_security::rules::cyberskills::waf_sqli::WafSqliSignatureValidator;
use enforcer_lang_security::rules::cyberskills::weak_crypto::WeakCryptoValidator;
use enforcer_lang_security::rules::cyberskills::web_cors::CorsMisconfigValidator;
use enforcer_lang_security::rules::cyberskills::web_headers::{
    CookieSecureHttponlySamesiteValidator, CspMissingValidator, HstsMissingOrWeakValidator,
};
use enforcer_lang_security::rules::cyberskills::web_ssrf::SsrfMetadataValidator;
use enforcer_lang_security::rules::cyberskills::websocket_security::WebSocketSecurityValidator;
use enforcer_validator::validator::{ValidationInput, Validator};

#[path = "support/corpus.rs"]
mod corpus_support;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Run every case in `corpus_file` against the whole `family` (union of
/// validators) and assert the flag/clean label holds. All mismatches are
/// collected and reported together so reconciliation sees the full set.
fn assert_family(
    corpus_file: &str,
    file_name: &str,
    family: &[Box<dyn Validator>],
) -> Result<(), Box<dyn std::error::Error>> {
    let cases = corpus_support::load(&manifest_dir(), corpus_file)?;
    assert!(!cases.is_empty(), "empty corpus {corpus_file}");
    let file: RelPath = file_name.parse()?;

    let mut mismatches = Vec::new();
    for case in &cases {
        let findings: usize = family
            .iter()
            .map(|validator| {
                validator
                    .validate(ValidationInput {
                        file: &file,
                        source: enforcer_domain::boundary::validation::ValidationSource::from_text(
                            &case.input,
                        ),
                        scope: ScanScope::Files,
                    })
                    .len()
            })
            .sum();
        let flagged = findings > 0;
        let want_flag = match case.expect.as_str() {
            "flag" => true,
            "clean" => false,
            other => return Err(format!("case `{}`: bad expect `{other}`", case.name).into()),
        };
        if flagged != want_flag {
            mismatches.push(format!(
                "  [{}] expected {} but got {} findings ({}). reason: {}",
                case.name,
                case.expect,
                findings,
                if flagged { "flagged" } else { "clean" },
                case.reason
            ));
        }
    }

    assert!(
        mismatches.is_empty(),
        "{corpus_file}: {} of {} cases mismatched:\n{}",
        mismatches.len(),
        cases.len(),
        mismatches.join("\n")
    );
    Ok(())
}

#[test]
fn corpus_iac_terraform() -> Result<(), Box<dyn std::error::Error>> {
    let family: Vec<Box<dyn Validator>> = vec![
        Box::new(S3EncryptionRequiredValidator::new()?),
        Box::new(IamNoWildcardActionValidator::new()?),
        Box::new(SgNoPublicSshIngressValidator::new()?),
    ];
    assert_family("iac_terraform.json", "main.tf", &family)
}

#[test]
fn corpus_cloud_aws() -> Result<(), Box<dyn std::error::Error>> {
    let family: Vec<Box<dyn Validator>> = vec![Box::new(AwsResourceHardeningValidator::new()?)];
    assert_family("cloud_aws.json", "main.tf", &family)
}

#[test]
fn corpus_cloud_gcp() -> Result<(), Box<dyn std::error::Error>> {
    let family: Vec<Box<dyn Validator>> = vec![Box::new(GcpResourceHardeningValidator::new()?)];
    assert_family("cloud_gcp.json", "main.tf", &family)
}

#[test]
fn corpus_k8s_rbac() -> Result<(), Box<dyn std::error::Error>> {
    let family: Vec<Box<dyn Validator>> = vec![Box::new(K8sRbacValidator::new()?)];
    assert_family("k8s_rbac.json", "rbac.yaml", &family)
}

#[test]
fn corpus_auth_jwt() -> Result<(), Box<dyn std::error::Error>> {
    let family: Vec<Box<dyn Validator>> = vec![Box::new(JwtSecurityValidator::new()?)];
    assert_family("auth_jwt.json", "auth.js", &family)
}

#[test]
fn corpus_web_cors() -> Result<(), Box<dyn std::error::Error>> {
    let family: Vec<Box<dyn Validator>> = vec![Box::new(CorsMisconfigValidator::new()?)];
    assert_family("web_cors.json", "cors.txt", &family)
}

#[test]
fn corpus_net_tls() -> Result<(), Box<dyn std::error::Error>> {
    let family: Vec<Box<dyn Validator>> = vec![Box::new(TlsLegacyVersionValidator::new()?)];
    assert_family("net_tls.json", "tls.conf", &family)
}

#[test]
fn corpus_web_ssrf() -> Result<(), Box<dyn std::error::Error>> {
    let family: Vec<Box<dyn Validator>> = vec![Box::new(SsrfMetadataValidator::new()?)];
    assert_family("web_ssrf.json", "app.py", &family)
}

#[test]
fn corpus_cmd_injection() -> Result<(), Box<dyn std::error::Error>> {
    let family: Vec<Box<dyn Validator>> = vec![Box::new(CommandInjectionValidator::new()?)];
    assert_family("cmd_injection.json", "app.py", &family)
}

#[test]
fn corpus_path_traversal() -> Result<(), Box<dyn std::error::Error>> {
    let family: Vec<Box<dyn Validator>> = vec![Box::new(PathTraversalValidator::new()?)];
    assert_family("path_traversal.json", "app.py", &family)
}

#[test]
fn path_traversal_ignores_prose_words_that_only_contain_req(
) -> Result<(), Box<dyn std::error::Error>> {
    let validator = PathTraversalValidator::new()?;
    let file: RelPath = "spec.rs".parse()?;
    let prose = "/// the first file (frequently a whole namespace subtree)";
    let prose_findings = validator.validate(ValidationInput {
        file: &file,
        source: enforcer_domain::boundary::validation::ValidationSource::from_text(prose),
        scope: ScanScope::Files,
    });
    assert!(prose_findings.is_empty());

    let dangerous = "open(request_path)";
    let dangerous_findings = validator.validate(ValidationInput {
        file: &file,
        source: enforcer_domain::boundary::validation::ValidationSource::from_text(dangerous),
        scope: ScanScope::Files,
    });
    assert_eq!(dangerous_findings.len(), 1);
    Ok(())
}

#[test]
fn corpus_insecure_deser() -> Result<(), Box<dyn std::error::Error>> {
    let family: Vec<Box<dyn Validator>> = vec![Box::new(InsecureDeserializationValidator::new()?)];
    assert_family("insecure_deser.json", "app.py", &family)
}

#[test]
fn corpus_weak_crypto() -> Result<(), Box<dyn std::error::Error>> {
    let family: Vec<Box<dyn Validator>> = vec![Box::new(WeakCryptoValidator::new()?)];
    assert_family("weak_crypto.json", "app.py", &family)
}

#[test]
fn corpus_tls_verify() -> Result<(), Box<dyn std::error::Error>> {
    let family: Vec<Box<dyn Validator>> = vec![Box::new(TlsVerificationDisabledValidator::new()?)];
    assert_family("tls_verify.json", "app.py", &family)
}

#[test]
fn corpus_cloud_azure() -> Result<(), Box<dyn std::error::Error>> {
    let family: Vec<Box<dyn Validator>> = vec![
        Box::new(AzureStoragePublicBlobValidator::new()?),
        Box::new(AzureStorageRequireHttpsValidator::new()?),
        Box::new(AzureStorageMinTls12Validator::new()?),
    ];
    assert_family("cloud_azure.json", "account.json", &family)
}

/// Headers is the one family whose three checks each evaluate the SAME
/// response independently, so a fixture crafted to isolate one header
/// legitimately omits the others (which would otherwise trip their
/// "missing" branch). Each case is therefore routed to the ONE validator
/// its name/branch names, not the union.
#[test]
fn corpus_web_headers() -> Result<(), Box<dyn std::error::Error>> {
    let hsts = HstsMissingOrWeakValidator::new()?;
    let csp = CspMissingValidator::new()?;
    let cookie = CookieSecureHttponlySamesiteValidator::new()?;

    let cases = corpus_support::load(&manifest_dir(), "web_headers.json")?;
    assert!(!cases.is_empty(), "empty web_headers corpus");
    let file: RelPath = "response.json".parse()?;

    let mut mismatches = Vec::new();
    for case in &cases {
        let tag = format!("{} {}", case.name, case.branch).to_ascii_lowercase();
        // A case naming one header is routed to that validator; a combined
        // response case (all headers present) runs the whole family.
        // Route by which header CATEGORY the case names (samesite belongs
        // to the cookie category, not a separate one). A case naming one
        // category runs only that validator — its fixture legitimately omits
        // the other headers; a "combined" case (whole response) runs the
        // full family.
        let is_hsts = tag.contains("hsts");
        let is_csp = tag.contains("csp");
        let is_cookie = tag.contains("cookie") || tag.contains("samesite");
        let categories = [is_hsts, is_csp, is_cookie].iter().filter(|b| **b).count();
        let selected: Vec<&dyn Validator> = if tag.contains("combined") || categories != 1 {
            vec![&hsts, &csp, &cookie]
        } else if is_hsts {
            vec![&hsts]
        } else if is_csp {
            vec![&csp]
        } else {
            vec![&cookie]
        };
        let findings: usize = selected
            .iter()
            .map(|validator| {
                validator
                    .validate(ValidationInput {
                        file: &file,
                        source: enforcer_domain::boundary::validation::ValidationSource::from_text(
                            &case.input,
                        ),
                        scope: ScanScope::Files,
                    })
                    .len()
            })
            .sum();
        let flagged = findings > 0;
        let want_flag = match case.expect.as_str() {
            "flag" => true,
            "clean" => false,
            other => return Err(format!("case `{}`: bad expect `{other}`", case.name).into()),
        };
        if flagged != want_flag {
            mismatches.push(format!(
                "  [{}] expected {} but got {} findings ({}). reason: {}",
                case.name,
                case.expect,
                findings,
                if flagged { "flagged" } else { "clean" },
                case.reason
            ));
        }
    }
    assert!(
        mismatches.is_empty(),
        "web_headers.json: {} of {} cases mismatched:\n{}",
        mismatches.len(),
        cases.len(),
        mismatches.join("\n")
    );
    Ok(())
}

#[test]
fn corpus_dependency_confusion() -> Result<(), Box<dyn std::error::Error>> {
    let family: Vec<Box<dyn Validator>> =
        vec![Box::new(DependencyConfusionClaimableValidator::new()?)];
    assert_family("dependency_confusion.json", "package.json", &family)
}

#[test]
fn corpus_waf_sqli() -> Result<(), Box<dyn std::error::Error>> {
    let family: Vec<Box<dyn Validator>> = vec![Box::new(WafSqliSignatureValidator::new()?)];
    assert_family("waf_sqli.json", "access.log", &family)
}

#[test]
fn waf_sqli_treats_log_evidence_but_not_source_code_as_a_waf_input(
) -> Result<(), Box<dyn std::error::Error>> {
    let validator = WafSqliSignatureValidator::new()?;
    let source_file: RelPath = "crates/product/src/query.rs".parse()?;
    let source_findings = validator.validate(ValidationInput {
        file: &source_file,
        source: enforcer_domain::boundary::validation::ValidationSource::from_text(
            "// UNION SELECT is a WAF signature",
        ),
        scope: ScanScope::Files,
    });
    assert!(source_findings.is_empty());

    let log_file: RelPath = "evidence/waf/access.log".parse()?;
    let log_findings = validator.validate(ValidationInput {
        file: &log_file,
        source: enforcer_domain::boundary::validation::ValidationSource::from_text(
            "GET /?q=UNION SELECT password FROM users",
        ),
        scope: ScanScope::Files,
    });
    assert_eq!(log_findings.len(), 1);
    Ok(())
}

#[test]
fn corpus_k8s_pod_security() -> Result<(), Box<dyn std::error::Error>> {
    let family: Vec<Box<dyn Validator>> = vec![Box::new(K8sPodSecurityValidator::new()?)];
    assert_family("k8s_pod.json", "workload.yaml", &family)
}

#[test]
fn corpus_dockerfile_hardening() -> Result<(), Box<dyn std::error::Error>> {
    let family: Vec<Box<dyn Validator>> = vec![Box::new(DockerfileHardeningValidator::new()?)];
    assert_family("dockerfile.json", "Dockerfile", &family)
}

#[test]
fn corpus_sqli_source() -> Result<(), Box<dyn std::error::Error>> {
    let family: Vec<Box<dyn Validator>> = vec![Box::new(SqlInjectionSourceValidator::new()?)];
    assert_family("sqli_source.json", "app.py", &family)
}

#[test]
fn corpus_ssti() -> Result<(), Box<dyn std::error::Error>> {
    let family: Vec<Box<dyn Validator>> = vec![Box::new(TemplateInjectionValidator::new()?)];
    assert_family("ssti.json", "app.py", &family)
}

#[test]
fn corpus_nosql_injection() -> Result<(), Box<dyn std::error::Error>> {
    let family: Vec<Box<dyn Validator>> = vec![Box::new(NoSqlInjectionValidator::new()?)];
    assert_family("nosql_injection.json", "app.js", &family)
}

#[test]
fn corpus_proto_pollution() -> Result<(), Box<dyn std::error::Error>> {
    let family: Vec<Box<dyn Validator>> = vec![Box::new(PrototypePollutionValidator::new()?)];
    assert_family("proto_pollution.json", "app.js", &family)
}

#[test]
fn corpus_github_actions() -> Result<(), Box<dyn std::error::Error>> {
    let family: Vec<Box<dyn Validator>> = vec![Box::new(GithubActionsSecurityValidator::new()?)];
    assert_family("github_actions.json", "workflow.yml", &family)
}

#[test]
fn corpus_mass_assignment() -> Result<(), Box<dyn std::error::Error>> {
    let family: Vec<Box<dyn Validator>> = vec![Box::new(MassAssignmentValidator::new()?)];
    assert_family("mass_assignment.json", "app.py", &family)
}

#[test]
fn corpus_type_juggle() -> Result<(), Box<dyn std::error::Error>> {
    let family: Vec<Box<dyn Validator>> = vec![Box::new(TypeJugglingValidator::new()?)];
    assert_family("type_juggle.json", "app.php", &family)
}

#[test]
fn corpus_oauth_misconfig() -> Result<(), Box<dyn std::error::Error>> {
    let family: Vec<Box<dyn Validator>> = vec![Box::new(OauthMisconfigValidator::new()?)];
    assert_family("oauth_misconfig.json", "app.js", &family)
}

#[test]
fn corpus_docker_daemon() -> Result<(), Box<dyn std::error::Error>> {
    let family: Vec<Box<dyn Validator>> = vec![Box::new(DockerDaemonHardeningValidator::new()?)];
    assert_family("docker_daemon.json", "daemon.json", &family)
}

#[test]
fn corpus_mcp_tool_poisoning() -> Result<(), Box<dyn std::error::Error>> {
    let family: Vec<Box<dyn Validator>> = vec![Box::new(McpToolPoisoningValidator::new()?)];
    assert_family("mcp_tool_poisoning.json", "tools.json", &family)
}

#[test]
fn corpus_fileless_malware() -> Result<(), Box<dyn std::error::Error>> {
    let family: Vec<Box<dyn Validator>> = vec![Box::new(FilelessMalwareValidator::new()?)];
    assert_family("fileless_malware.json", "evidence.txt", &family)
}

#[test]
fn corpus_websocket_security() -> Result<(), Box<dyn std::error::Error>> {
    let family: Vec<Box<dyn Validator>> = vec![Box::new(WebSocketSecurityValidator::new()?)];
    assert_family("websocket_security.json", "server.js", &family)
}
// NOTE: the exhaustive provider-credential corpus lives in
// provider_credentials.rs as a CODE-BUILT test (secret strings are assembled
// from parts at runtime) so no real-secret-shaped literal is ever committed —
// otherwise GitHub push protection (correctly) blocks the fixture.
