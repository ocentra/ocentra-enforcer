//! `CYBER-AZURE-BLOB-PUBLIC.1` + `CYBER-AZURE-HTTPS.1` +
//! `CYBER-AZURE-TLS12.1` (all T1) — harvest target 5 (h11 workpack):
//! boolean field predicates ported from
//! `vendor/anthropic-cybersecurity-skills/skills/detecting-azure-storage-account-misconfigurations/scripts/agent.py`
//! (L36-79). The original agent.py calls the `azure-mgmt-storage` SDK to
//! fetch a live `StorageAccount` object then inspects three boolean/enum
//! fields; these validators run the SAME three predicates over a generic
//! JSON snapshot of that object (the shape a `terraform show -json` /
//! `az storage account show` dump or an IaC-plan export would carry),
//! dropping the SDK/network call entirely.
//!
//! Scope note (h11 thin-slice): the vendor `audit_storage_account`
//! function checks NINE conditions; this pack ports the THREE named in the
//! h11 workpack checklist (public blob, HTTPS-only, min TLS). The other six
//! are the same shape (boolean/enum predicates over the same snapshot) and
//! are tracked as follow-up rules, NOT silently dropped:
//! `blob_encryption` (services.blob.enabled false), `file_encryption`
//! (services.file.enabled false), `encryption_missing` (no encryption
//! object), `network_default_allow` (network_rule_set.default_action ==
//! "Allow"), `infrastructure_encryption` (require_infrastructure_encryption
//! false, Low), and `customer_managed_keys` (key_source ==
//! "Microsoft.Storage", Low). Adding them is a bounded next step.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

/// `CYBER-AZURE-BLOB-PUBLIC.1` — `allow_blob_public_access == true` is
/// flagged (agent.py: `check: public_blob_access`, severity Critical).
#[derive(Debug)]
pub struct AzureStoragePublicBlobValidator {
    rule_id: RuleId,
}

impl AzureStoragePublicBlobValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: enforcer_domain::ids::BuiltInSecurityRule::CyberAzureBlobPublic.id(),
        })
    }
}

impl Validator for AzureStoragePublicBlobValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let Some(account) = crate::boundary::cloud_azure::decode(input.source.as_str()) else {
            return Vec::new();
        };
        if account.allow_blob_public_access != Some(true) {
            return Vec::new();
        }
        crate::boundary::finding::from_owned_source(
            (&self.rule_id, Severity::Error),
            "Storage account allows public blob access",
            format!(
                "Storage account '{}' has `allow_blob_public_access: true`. Fix: set \
                 `allow_blob_public_access` to `false` on the storage account.",
                account.label()
            ),
            input.file,
            (1, None),
        )
        .into_iter()
        .collect()
    }
}

/// `CYBER-AZURE-HTTPS.1` — `enable_https_traffic_only == false` is flagged
/// (agent.py: `check: https_enforcement`, severity High).
#[derive(Debug)]
pub struct AzureStorageRequireHttpsValidator {
    rule_id: RuleId,
}

impl AzureStorageRequireHttpsValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: enforcer_domain::ids::BuiltInSecurityRule::CyberAzureHttps.id(),
        })
    }
}

impl Validator for AzureStorageRequireHttpsValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let Some(account) = crate::boundary::cloud_azure::decode(input.source.as_str()) else {
            return Vec::new();
        };
        if account.enable_https_traffic_only != Some(false) {
            return Vec::new();
        }
        crate::boundary::finding::from_owned_source(
            (&self.rule_id, Severity::Error),
            "Storage account allows HTTP traffic",
            format!(
                "Storage account '{}' has `enable_https_traffic_only: false`. Fix: enable \
                 'Secure transfer required' (`enable_https_traffic_only: true`).",
                account.label()
            ),
            input.file,
            (1, None),
        )
        .into_iter()
        .collect()
    }
}

/// `CYBER-AZURE-TLS12.1` — `minimum_tls_version` of `TLS1_0`/`TLS1_1` is
/// flagged (agent.py: `check: minimum_tls_version`, severity High).
#[derive(Debug)]
pub struct AzureStorageMinTls12Validator {
    rule_id: RuleId,
}

impl AzureStorageMinTls12Validator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: enforcer_domain::ids::BuiltInSecurityRule::CyberAzureTls12.id(),
        })
    }
}

impl Validator for AzureStorageMinTls12Validator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let Some(account) = crate::boundary::cloud_azure::decode(input.source.as_str()) else {
            return Vec::new();
        };
        let Some(min_tls) = account.minimum_tls_version.as_deref() else {
            return Vec::new();
        };
        if min_tls != "TLS1_0" && min_tls != "TLS1_1" {
            return Vec::new();
        }
        crate::boundary::finding::from_owned_source(
            (&self.rule_id, Severity::Error),
            "Storage account allows a weak minimum TLS version",
            format!(
                "Storage account '{}' has `minimum_tls_version: {min_tls}` (should be \
                 `TLS1_2`). Fix: set the minimum TLS version to TLS1_2.",
                account.label()
            ),
            input.file,
            (1, None),
        )
        .into_iter()
        .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::boundary::fixture::run_manifest_fixture_parity;

    use super::{
        AzureStorageMinTls12Validator, AzureStoragePublicBlobValidator,
        AzureStorageRequireHttpsValidator,
    };

    #[test]
    fn cyberskills_cloud_azure_public_blob() -> Result<(), Box<dyn std::error::Error>> {
        let validator = AzureStoragePublicBlobValidator::new()?;
        run_manifest_fixture_parity(
            &validator,
            "tests/fixtures/cyberskills/cloud.azure.storage-public-blob/bad/public.json",
            "tests/fixtures/cyberskills/cloud.azure.storage-public-blob/good/private.json",
        )?;
        Ok(())
    }

    #[test]
    fn cyberskills_cloud_azure_https() -> Result<(), Box<dyn std::error::Error>> {
        let validator = AzureStorageRequireHttpsValidator::new()?;
        run_manifest_fixture_parity(
            &validator,
            "tests/fixtures/cyberskills/cloud.azure.storage-require-https/bad/http.json",
            "tests/fixtures/cyberskills/cloud.azure.storage-require-https/good/https.json",
        )?;
        Ok(())
    }

    #[test]
    fn cyberskills_cloud_azure_tls12() -> Result<(), Box<dyn std::error::Error>> {
        let validator = AzureStorageMinTls12Validator::new()?;
        run_manifest_fixture_parity(
            &validator,
            "tests/fixtures/cyberskills/cloud.azure.storage-min-tls12/bad/tls10.json",
            "tests/fixtures/cyberskills/cloud.azure.storage-min-tls12/good/tls12.json",
        )?;
        Ok(())
    }
}
