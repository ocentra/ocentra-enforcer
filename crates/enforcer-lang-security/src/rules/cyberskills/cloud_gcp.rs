//! `CYBER-GCP.1` (T1) — Wave-1 cyberskills: GCP Terraform resource
//! hardening, harvested from the vendored `auditing-gcp-iam-permissions`
//! and `implementing-gcp-vpc-firewall-rules` skills.
//!
//! Both vendor skills are pure `gcloud`-CLI runbooks (`SKILL.md` Step 1-6 /
//! `scripts/agent.py`): they enumerate live IAM bindings, Cloud SQL
//! instances, and firewall rules with `gcloud asset search-all-iam-policies`
//! / `gcloud compute firewall-rules list` and eyeball the JSON output — there
//! is no inline Rego/JSON deny predicate to port (unlike the Terraform IaC
//! skill `iac_terraform.rs` ports from). What both skills flag by name,
//! though, is unambiguous and well-known (CIS GCP Foundations Benchmark
//! shape), so this validator implements the same three checks natively over
//! Terraform HCL, reusing `iac_terraform`'s brace-depth-aware block parser:
//!
//! - `auditing-gcp-iam-permissions` Step 4 ("Find all principals with
//!   allUsers or allAuthenticatedUsers access") -> an IAM member/binding
//!   resource (`google_storage_bucket_iam_member`/`_binding`,
//!   `google_project_iam_member`/`_binding`) granting `allUsers` or
//!   `allAuthenticatedUsers` — Critical;
//! - `implementing-gcp-vpc-firewall-rules` Step 1 ("Find rules allowing all
//!   traffic from 0.0.0.0/0") -> a `google_compute_firewall` whose
//!   `source_ranges` contains `0.0.0.0/0` — High;
//! - the CIS-benchmark-standard Cloud SQL companion check (same "public
//!   network exposure" theme both skills audit for): a
//!   `google_sql_database_instance` with `ipv4_enabled = true` or an
//!   `authorized_networks` entry of `0.0.0.0/0` — High.
//!
//! Cross-resource / absence checks (e.g. a bucket lacking a companion
//! `google_storage_bucket_iam_policy` deny, unused service-account-key
//! rotation) are deliberately NOT emitted here — they over-flag relative to
//! what a single Terraform resource block can tell you — and are tracked as
//! follow-ups.

use crate::boundary::terraform::{
    allows_public_cidr, bool_attr, grants_public_access, list_attr_contains, resource_blocks,
};
use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

/// `CYBER-GCP.1` — GCP Terraform resource hardening gate.
/// Validator for the `CYBER-GCP.1` Terraform resource hardening rule.
#[derive(Debug)]
pub struct GcpResourceHardeningValidator {
    rule_id: RuleId,
}

impl GcpResourceHardeningValidator {
    /// Builds the validator with its canonical, validated rule identity.
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: enforcer_domain::ids::BuiltInSecurityRule::CyberGcp.id(),
        })
    }
}

impl Validator for GcpResourceHardeningValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let mut findings = Vec::new();
        for block in resource_blocks(input.source.as_str()) {
            match block.resource_type {
                "google_storage_bucket_iam_member"
                | "google_storage_bucket_iam_binding"
                | "google_storage_bucket_iam_policy"
                | "google_project_iam_member"
                | "google_project_iam_binding"
                | "google_project_iam_policy"
                    if grants_public_access(block.body) =>
                {
                    findings.extend(crate::boundary::finding::from_validation(
                        (&self.rule_id, Severity::Error),
                        "GCP resource violates a hardening check",
                        format!(
                            "IAM binding '{}' grants access to `allUsers` or \
                             `allAuthenticatedUsers`, making the resource publicly accessible \
                             to anyone on the internet (or any Google account holder). Fix: \
                             remove the public member and grant access to a scoped user, \
                             group, or service account instead.",
                            block.name
                        ),
                        &input,
                        block.line,
                    ));
                }
                "google_sql_database_instance"
                    if bool_attr(block.body, "ipv4_enabled") == Some(true)
                        || allows_public_cidr(block.body) =>
                {
                    findings.extend(crate::boundary::finding::from_validation(
                        (&self.rule_id, Severity::Error),
                        "GCP resource violates a hardening check",
                        format!(
                            "Cloud SQL instance '{}' is reachable from the public internet \
                             (`ipv4_enabled = true` and/or an `authorized_networks` entry of \
                             `0.0.0.0/0`). Fix: disable the public IP \
                             (`ipv4_enabled = false`) and connect via a private IP / Cloud SQL \
                             Auth Proxy, or scope `authorized_networks` to known ranges.",
                            block.name
                        ),
                        &input,
                        block.line,
                    ));
                }
                "google_compute_firewall"
                    if list_attr_contains(block.body, "source_ranges", "0.0.0.0/0") =>
                {
                    findings.extend(crate::boundary::finding::from_validation(
                        (&self.rule_id, Severity::Error),
                        "GCP resource violates a hardening check",
                        format!(
                            "Firewall rule '{}' allows traffic from `0.0.0.0/0` in \
                             `source_ranges`, exposing the targeted instances to the entire \
                             internet. Fix: restrict `source_ranges` to known CIDR ranges or \
                             target specific service accounts/tags instead of a public source.",
                            block.name
                        ),
                        &input,
                        block.line,
                    ));
                }
                _ => {}
            }
        }
        findings
    }
}
