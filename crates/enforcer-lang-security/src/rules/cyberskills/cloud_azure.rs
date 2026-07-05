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

use enforcer_core::error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

#[derive(Debug, serde::Deserialize)]
struct StorageAccountSnapshot {
    #[serde(default, rename = "name")]
    name: Option<String>,
    #[serde(default, rename = "allow_blob_public_access")]
    allow_blob_public_access: Option<bool>,
    #[serde(default, rename = "enable_https_traffic_only")]
    enable_https_traffic_only: Option<bool>,
    #[serde(default, rename = "minimum_tls_version")]
    minimum_tls_version: Option<String>,
}

fn parse(source: &str) -> Option<StorageAccountSnapshot> {
    serde_json::from_str(source).ok()
}

fn account_label(account: &StorageAccountSnapshot) -> &str {
    account.name.as_deref().unwrap_or("<unnamed>")
}

/// `CYBER-AZURE-BLOB-PUBLIC.1` — `allow_blob_public_access == true` is
/// flagged (agent.py: `check: public_blob_access`, severity Critical).
pub struct AzureStoragePublicBlobValidator {
    rule_id: RuleId,
}

impl AzureStoragePublicBlobValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "CYBER-AZURE-BLOB-PUBLIC.1".parse()?,
        })
    }
}

impl Validator for AzureStoragePublicBlobValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let Some(account) = parse(input.source) else {
            return Vec::new();
        };
        if account.allow_blob_public_access != Some(true) {
            return Vec::new();
        }
        vec![Finding {
            rule_id: self.rule_id.clone(),
            severity: Severity::Error,
            title: "Storage account allows public blob access".to_owned(),
            detail: format!(
                "Storage account '{}' has `allow_blob_public_access: true`. Fix: set \
                 `allow_blob_public_access` to `false` on the storage account.",
                account_label(&account)
            ),
            file: input.file.clone(),
            line: 1,
            snippet: None,
        }]
    }
}

/// `CYBER-AZURE-HTTPS.1` — `enable_https_traffic_only == false` is flagged
/// (agent.py: `check: https_enforcement`, severity High).
pub struct AzureStorageRequireHttpsValidator {
    rule_id: RuleId,
}

impl AzureStorageRequireHttpsValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "CYBER-AZURE-HTTPS.1".parse()?,
        })
    }
}

impl Validator for AzureStorageRequireHttpsValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let Some(account) = parse(input.source) else {
            return Vec::new();
        };
        if account.enable_https_traffic_only != Some(false) {
            return Vec::new();
        }
        vec![Finding {
            rule_id: self.rule_id.clone(),
            severity: Severity::Error,
            title: "Storage account allows HTTP traffic".to_owned(),
            detail: format!(
                "Storage account '{}' has `enable_https_traffic_only: false`. Fix: enable \
                 'Secure transfer required' (`enable_https_traffic_only: true`).",
                account_label(&account)
            ),
            file: input.file.clone(),
            line: 1,
            snippet: None,
        }]
    }
}

/// `CYBER-AZURE-TLS12.1` — `minimum_tls_version` of `TLS1_0`/`TLS1_1` is
/// flagged (agent.py: `check: minimum_tls_version`, severity High).
pub struct AzureStorageMinTls12Validator {
    rule_id: RuleId,
}

impl AzureStorageMinTls12Validator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "CYBER-AZURE-TLS12.1".parse()?,
        })
    }
}

impl Validator for AzureStorageMinTls12Validator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let Some(account) = parse(input.source) else {
            return Vec::new();
        };
        let Some(min_tls) = account.minimum_tls_version.as_deref() else {
            return Vec::new();
        };
        if min_tls != "TLS1_0" && min_tls != "TLS1_1" {
            return Vec::new();
        }
        vec![Finding {
            rule_id: self.rule_id.clone(),
            severity: Severity::Error,
            title: "Storage account allows a weak minimum TLS version".to_owned(),
            detail: format!(
                "Storage account '{}' has `minimum_tls_version: {min_tls}` (should be \
                 `TLS1_2`). Fix: set the minimum TLS version to TLS1_2.",
                account_label(&account)
            ),
            file: input.file.clone(),
            line: 1,
            snippet: None,
        }]
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use enforcer_validator::harness::run_fixture_parity;

    use super::{
        AzureStorageMinTls12Validator, AzureStoragePublicBlobValidator,
        AzureStorageRequireHttpsValidator,
    };

    fn manifest_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn cyberskills_cloud_azure_public_blob() -> Result<(), Box<dyn std::error::Error>> {
        let validator = AzureStoragePublicBlobValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/cyberskills/cloud.azure.storage-public-blob/bad/public.json",
            "tests/fixtures/cyberskills/cloud.azure.storage-public-blob/good/private.json",
        )?;
        Ok(())
    }

    #[test]
    fn cyberskills_cloud_azure_https() -> Result<(), Box<dyn std::error::Error>> {
        let validator = AzureStorageRequireHttpsValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/cyberskills/cloud.azure.storage-require-https/bad/http.json",
            "tests/fixtures/cyberskills/cloud.azure.storage-require-https/good/https.json",
        )?;
        Ok(())
    }

    #[test]
    fn cyberskills_cloud_azure_tls12() -> Result<(), Box<dyn std::error::Error>> {
        let validator = AzureStorageMinTls12Validator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/cyberskills/cloud.azure.storage-min-tls12/bad/tls10.json",
            "tests/fixtures/cyberskills/cloud.azure.storage-min-tls12/good/tls12.json",
        )?;
        Ok(())
    }
}
