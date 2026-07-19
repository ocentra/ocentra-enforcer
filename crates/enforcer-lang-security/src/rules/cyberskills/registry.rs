//! The cyberskills-cluster `Validator`-registration seam: every rule id
//! this module owns, paired with its constructed [`Validator`]. Kept
//! DELIBERATELY SEPARATE from the top-level `SEC-*` registry builder (whose
//! count-parity seam is pinned to the workpack's
//! authoritative count of 22 by `tests/completeness.rs`) — the cyberskills
//! cluster is a distinct rule family (`CYBER-*` prefix) with its own count,
//! not a `SEC-*` row, so it must not perturb that completeness assertion.

use enforcer_domain::boundary::decode_error::DecodeError;

type RegistryRow = crate::rules::registry::RegistryRow;

use super::auth_jwt::JwtSecurityValidator;
use super::cloud_aws::AwsResourceHardeningValidator;
use super::cloud_azure::{
    AzureStorageMinTls12Validator, AzureStoragePublicBlobValidator,
    AzureStorageRequireHttpsValidator,
};
use super::cloud_gcp::GcpResourceHardeningValidator;
use super::cmd_injection::CommandInjectionValidator;
use super::dependency_confusion::DependencyConfusionClaimableValidator;
use super::docker_daemon::DockerDaemonHardeningValidator;
use super::dockerfile_hardening::DockerfileHardeningValidator;
use super::fileless_malware::{
    FilelessAnalysisReportValidator, FilelessMalwareValidator, FilelessTelemetryBaselineValidator,
};
use super::github_actions::GithubActionsSecurityValidator;
use super::iac_terraform::{
    IamNoWildcardActionValidator, S3EncryptionRequiredValidator, SgNoPublicSshIngressValidator,
};
use super::insecure_deser::InsecureDeserializationValidator;
use super::k8s_pod_security::K8sPodSecurityValidator;
use super::k8s_rbac::K8sRbacValidator;
use super::mass_assignment::MassAssignmentValidator;
use super::mcp_tool_poisoning::McpToolPoisoningValidator;
use super::net_tls::TlsLegacyVersionValidator;
use super::nosql_injection::NoSqlInjectionValidator;
use super::oauth_misconfig::OauthMisconfigValidator;
use super::path_traversal::PathTraversalValidator;
use super::proto_pollution::PrototypePollutionValidator;
use super::provider_credentials::ProviderCredentialValidator;
use super::sqli_source::SqlInjectionSourceValidator;
use super::ssti::TemplateInjectionValidator;
use super::tls_verify::TlsVerificationDisabledValidator;
use super::type_juggle::TypeJugglingValidator;
use super::waf_sqli::WafSqliSignatureValidator;
use super::weak_crypto::WeakCryptoValidator;
use super::web_cors::CorsMisconfigValidator;
use super::web_headers::{
    CookieSecureHttponlySamesiteValidator, CspMissingValidator, HstsMissingOrWeakValidator,
};
use super::web_ssrf::SsrfMetadataValidator;
use super::websocket_security::WebSocketSecurityValidator;

/// Build every cyberskills-cluster row this module owns. Fails closed
/// (propagates the first construction error) rather than silently
/// dropping a malformed entry.
pub fn build_all() -> Result<Vec<RegistryRow>, DecodeError> {
    Ok(vec![
        RegistryRow::from_validator(Box::new(S3EncryptionRequiredValidator::new()?)),
        RegistryRow::from_validator(Box::new(IamNoWildcardActionValidator::new()?)),
        RegistryRow::from_validator(Box::new(SgNoPublicSshIngressValidator::new()?)),
        RegistryRow::from_validator(Box::new(AzureStoragePublicBlobValidator::new()?)),
        RegistryRow::from_validator(Box::new(AzureStorageRequireHttpsValidator::new()?)),
        RegistryRow::from_validator(Box::new(AzureStorageMinTls12Validator::new()?)),
        RegistryRow::from_validator(Box::new(HstsMissingOrWeakValidator::new()?)),
        RegistryRow::from_validator(Box::new(CspMissingValidator::new()?)),
        RegistryRow::from_validator(Box::new(CookieSecureHttponlySamesiteValidator::new()?)),
        RegistryRow::from_validator(Box::new(DependencyConfusionClaimableValidator::new()?)),
        RegistryRow::from_validator(Box::new(WafSqliSignatureValidator::new()?)),
        RegistryRow::from_validator(Box::new(K8sPodSecurityValidator::new()?)),
        RegistryRow::from_validator(Box::new(DockerfileHardeningValidator::new()?)),
        RegistryRow::from_validator(Box::new(ProviderCredentialValidator::new()?)),
        RegistryRow::from_validator(Box::new(AwsResourceHardeningValidator::new()?)),
        RegistryRow::from_validator(Box::new(K8sRbacValidator::new()?)),
        RegistryRow::from_validator(Box::new(GcpResourceHardeningValidator::new()?)),
        RegistryRow::from_validator(Box::new(JwtSecurityValidator::new()?)),
        RegistryRow::from_validator(Box::new(CorsMisconfigValidator::new()?)),
        RegistryRow::from_validator(Box::new(TlsLegacyVersionValidator::new()?)),
        RegistryRow::from_validator(Box::new(SsrfMetadataValidator::new()?)),
        RegistryRow::from_validator(Box::new(CommandInjectionValidator::new()?)),
        RegistryRow::from_validator(Box::new(PathTraversalValidator::new()?)),
        RegistryRow::from_validator(Box::new(InsecureDeserializationValidator::new()?)),
        RegistryRow::from_validator(Box::new(WeakCryptoValidator::new()?)),
        RegistryRow::from_validator(Box::new(TlsVerificationDisabledValidator::new()?)),
        RegistryRow::from_validator(Box::new(SqlInjectionSourceValidator::new()?)),
        RegistryRow::from_validator(Box::new(TemplateInjectionValidator::new()?)),
        RegistryRow::from_validator(Box::new(NoSqlInjectionValidator::new()?)),
        RegistryRow::from_validator(Box::new(PrototypePollutionValidator::new()?)),
        RegistryRow::from_validator(Box::new(GithubActionsSecurityValidator::new()?)),
        RegistryRow::from_validator(Box::new(MassAssignmentValidator::new()?)),
        RegistryRow::from_validator(Box::new(FilelessMalwareValidator::new()?)),
        RegistryRow::from_validator(Box::new(FilelessTelemetryBaselineValidator::new())),
        RegistryRow::from_validator(Box::new(FilelessAnalysisReportValidator::new())),
        RegistryRow::from_validator(Box::new(TypeJugglingValidator::new()?)),
        RegistryRow::from_validator(Box::new(OauthMisconfigValidator::new()?)),
        RegistryRow::from_validator(Box::new(DockerDaemonHardeningValidator::new()?)),
        RegistryRow::from_validator(Box::new(McpToolPoisoningValidator::new()?)),
        RegistryRow::from_validator(Box::new(WebSocketSecurityValidator::new()?)),
    ])
}

#[cfg(test)]
mod tests {
    use super::build_all;

    #[test]
    fn registry_builds_cleanly() -> Result<(), Box<dyn std::error::Error>> {
        let rows = build_all()?;
        assert_eq!(rows.len(), 40);
        let ids: Vec<&str> = rows.iter().map(|row| row.rule_id().as_str()).collect();
        for expected in [
            "CYBER-IAC-S3-SSE.1",
            "CYBER-IAC-IAM-WILDCARD.1",
            "CYBER-IAC-SG-SSH.1",
            "CYBER-AZURE-BLOB-PUBLIC.1",
            "CYBER-AZURE-HTTPS.1",
            "CYBER-AZURE-TLS12.1",
            "CYBER-HEADERS-HSTS.1",
            "CYBER-HEADERS-CSP.1",
            "CYBER-COOKIE-SECURE.1",
            "CYBER-DEPCONFUSION.1",
            "CYBER-WAF-SQLI.1",
            "CYBER-K8S-POD.1",
            "CYBER-DOCKER.1",
            "CYBER-SECRET.1",
            "CYBER-AWS.1",
            "CYBER-K8S-RBAC.1",
            "CYBER-GCP.1",
            "CYBER-AUTH-JWT.1",
            "CYBER-CORS.1",
            "CYBER-TLS.1",
            "CYBER-SSRF.1",
            "CYBER-CMD-INJECT.1",
            "CYBER-PATH-TRAVERSAL.1",
            "CYBER-DESERIALIZE.1",
            "CYBER-WEAK-CRYPTO.1",
            "CYBER-TLS-VERIFY.1",
            "CYBER-SQLI-SOURCE.1",
            "CYBER-SSTI.1",
            "CYBER-NOSQL-INJECT.1",
            "CYBER-PROTO-POLLUTION.1",
            "CYBER-GHA.1",
            "CYBER-MASS-ASSIGN.1",
            "CYBER-FILELESS-MALWARE.1",
            "CYBER-FILELESS-TELEMETRY.1",
            "CYBER-FILELESS-REPORT.1",
            "CYBER-TYPE-JUGGLE.1",
            "CYBER-OAUTH.1",
            "CYBER-DOCKER-DAEMON.1",
            "CYBER-MCP-POISON.1",
            "CYBER-WEBSOCKET.1",
        ] {
            assert!(
                ids.contains(&expected),
                "missing registry row for {expected}"
            );
        }
        Ok(())
    }
}
