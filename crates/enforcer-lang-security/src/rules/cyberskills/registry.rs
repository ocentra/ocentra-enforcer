//! The cyberskills-cluster `Validator`-registration seam: every rule id
//! this module owns, paired with its constructed [`Validator`]. Kept
//! DELIBERATELY SEPARATE from [`super::super::registry::build_all`] (the
//! `SEC-*` family's count-parity seam, pinned to the workpack's
//! authoritative count of 22 by `tests/completeness.rs`) — the cyberskills
//! cluster is a distinct rule family (`CYBER-*` prefix) with its own count,
//! not a `SEC-*` row, so it must not perturb that completeness assertion.

use enforcer_core::error::DecodeError;
use enforcer_validator::validator::Validator;

use super::auth_jwt::JwtSecurityValidator;
use super::cloud_aws::AwsResourceHardeningValidator;
use super::cloud_azure::{
    AzureStorageMinTls12Validator, AzureStoragePublicBlobValidator,
    AzureStorageRequireHttpsValidator,
};
use super::cloud_gcp::GcpResourceHardeningValidator;
use super::dependency_confusion::DependencyConfusionClaimableValidator;
use super::dockerfile_hardening::DockerfileHardeningValidator;
use super::iac_terraform::{
    IamNoWildcardActionValidator, S3EncryptionRequiredValidator, SgNoPublicSshIngressValidator,
};
use super::k8s_pod_security::K8sPodSecurityValidator;
use super::k8s_rbac::K8sRbacValidator;
use super::net_tls::TlsLegacyVersionValidator;
use super::provider_credentials::ProviderCredentialValidator;
use super::waf_sqli::WafSqliSignatureValidator;
use super::web_cors::CorsMisconfigValidator;
use super::web_headers::{
    CookieSecureHttponlySamesiteValidator, CspMissingValidator, HstsMissingOrWeakValidator,
};

/// One registry row: the rule id this row proves, paired with the
/// constructed [`Validator`] trait object.
pub struct RegistryRow {
    /// The rule id this row proves, e.g. `CYBER-IAC-S3-SSE.1`.
    pub rule_id: &'static str,
    /// The constructed validator for this rule.
    pub validator: Box<dyn Validator>,
}

/// Build every cyberskills-cluster row this module owns. Fails closed
/// (propagates the first construction error) rather than silently
/// dropping a malformed entry.
pub fn build_all() -> Result<Vec<RegistryRow>, DecodeError> {
    Ok(vec![
        RegistryRow {
            rule_id: "CYBER-IAC-S3-SSE.1",
            validator: Box::new(S3EncryptionRequiredValidator::new()?),
        },
        RegistryRow {
            rule_id: "CYBER-IAC-IAM-WILDCARD.1",
            validator: Box::new(IamNoWildcardActionValidator::new()?),
        },
        RegistryRow {
            rule_id: "CYBER-IAC-SG-SSH.1",
            validator: Box::new(SgNoPublicSshIngressValidator::new()?),
        },
        RegistryRow {
            rule_id: "CYBER-AZURE-BLOB-PUBLIC.1",
            validator: Box::new(AzureStoragePublicBlobValidator::new()?),
        },
        RegistryRow {
            rule_id: "CYBER-AZURE-HTTPS.1",
            validator: Box::new(AzureStorageRequireHttpsValidator::new()?),
        },
        RegistryRow {
            rule_id: "CYBER-AZURE-TLS12.1",
            validator: Box::new(AzureStorageMinTls12Validator::new()?),
        },
        RegistryRow {
            rule_id: "CYBER-HEADERS-HSTS.1",
            validator: Box::new(HstsMissingOrWeakValidator::new()?),
        },
        RegistryRow {
            rule_id: "CYBER-HEADERS-CSP.1",
            validator: Box::new(CspMissingValidator::new()?),
        },
        RegistryRow {
            rule_id: "CYBER-COOKIE-SECURE.1",
            validator: Box::new(CookieSecureHttponlySamesiteValidator::new()?),
        },
        RegistryRow {
            rule_id: "CYBER-DEPCONFUSION.1",
            validator: Box::new(DependencyConfusionClaimableValidator::new()?),
        },
        RegistryRow {
            rule_id: "CYBER-WAF-SQLI.1",
            validator: Box::new(WafSqliSignatureValidator::new()?),
        },
        RegistryRow {
            rule_id: "CYBER-K8S-POD.1",
            validator: Box::new(K8sPodSecurityValidator::new()?),
        },
        RegistryRow {
            rule_id: "CYBER-DOCKER.1",
            validator: Box::new(DockerfileHardeningValidator::new()?),
        },
        RegistryRow {
            rule_id: "CYBER-SECRET.1",
            validator: Box::new(ProviderCredentialValidator::new()?),
        },
        RegistryRow {
            rule_id: "CYBER-AWS.1",
            validator: Box::new(AwsResourceHardeningValidator::new()?),
        },
        RegistryRow {
            rule_id: "CYBER-K8S-RBAC.1",
            validator: Box::new(K8sRbacValidator::new()?),
        },
        RegistryRow {
            rule_id: "CYBER-GCP.1",
            validator: Box::new(GcpResourceHardeningValidator::new()?),
        },
        RegistryRow {
            rule_id: "CYBER-AUTH-JWT.1",
            validator: Box::new(JwtSecurityValidator::new()?),
        },
        RegistryRow {
            rule_id: "CYBER-CORS.1",
            validator: Box::new(CorsMisconfigValidator::new()?),
        },
        RegistryRow {
            rule_id: "CYBER-TLS.1",
            validator: Box::new(TlsLegacyVersionValidator::new()?),
        },
    ])
}

#[cfg(test)]
mod tests {
    use super::build_all;

    #[test]
    fn registry_builds_cleanly() -> Result<(), Box<dyn std::error::Error>> {
        let rows = build_all()?;
        assert_eq!(rows.len(), 20);
        let ids: Vec<&str> = rows.iter().map(|row| row.rule_id).collect();
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
        ] {
            assert!(
                ids.contains(&expected),
                "missing registry row for {expected}"
            );
        }
        Ok(())
    }
}
