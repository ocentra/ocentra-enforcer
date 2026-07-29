//! `CYBER-AWS.1` (T1) — Wave-1 cyberskills: additional AWS Terraform
//! misconfiguration checks, harvested from the vendored
//! `auditing-aws-s3-bucket-permissions`,
//! `remediating-s3-bucket-misconfiguration`, and
//! `securing-aws-iam-permissions` skills. It complements the three
//! `iac_terraform` rules (S3 encryption, IAM wildcard, public SSH ingress)
//! with the next-highest-yield, low-false-positive, per-resource checks:
//!
//! - `aws_s3_bucket` / `aws_s3_bucket_acl` with a public ACL
//!   (`acl = "public-read"` / `"public-read-write"`) — Critical;
//! - `aws_db_instance` with `publicly_accessible = true` — Critical;
//! - `aws_ebs_volume` not explicitly `encrypted = true` — High;
//! - `aws_security_group` / `aws_security_group_rule` ingress from
//!   `0.0.0.0/0` reaching a SENSITIVE non-SSH port (RDP 3389, MySQL 3306,
//!   PostgreSQL 5432, Redis 6379, MongoDB 27017, MSSQL 1433, Elasticsearch
//!   9200, Telnet 23) or a wide-open range — Critical.
//!
//! It reuses `iac_terraform`'s HCL block parser (no new dependency, no AWS
//! API). Cross-resource / absence checks (a bucket lacking a companion
//! `aws_s3_bucket_public_access_block`, missing versioning) are deliberately
//! NOT emitted here — they over-flag — and are tracked as follow-ups.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

use crate::boundary::terraform::{
    allows_public_cidr, bool_attr, exposed_sensitive_port, ingress_subblocks, resource_blocks,
    string_attr_eq,
};

/// `CYBER-AWS.1` — additional AWS Terraform hardening gate.
#[derive(Debug)]
pub struct AwsResourceHardeningValidator {
    rule_id: RuleId,
}

impl AwsResourceHardeningValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: enforcer_domain::ids::BuiltInSecurityRule::CyberAws.id(),
        })
    }
}

impl Validator for AwsResourceHardeningValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let mut findings = Vec::new();
        for block in resource_blocks(input.source.as_str()) {
            match block.resource_type {
                "aws_s3_bucket" | "aws_s3_bucket_acl"
                    if string_attr_eq(block.body, "acl", "public-read")
                        || string_attr_eq(block.body, "acl", "public-read-write") =>
                {
                    findings.extend(crate::boundary::finding::from_validation(
                        (&self.rule_id, Severity::Error),
                        "AWS resource violates a hardening check",
                        format!(
                            "S3 bucket '{}' grants a public ACL (`acl = \"public-read\"` / \
                             `\"public-read-write\"`), exposing objects to the internet. Fix: \
                             use `private` and grant access via a scoped bucket policy.",
                            block.name
                        ),
                        &input,
                        block.line,
                    ));
                }
                "aws_db_instance" if bool_attr(block.body, "publicly_accessible") == Some(true) => {
                    findings.extend(crate::boundary::finding::from_validation(
                        (&self.rule_id, Severity::Error),
                        "AWS resource violates a hardening check",
                        format!(
                            "RDS instance '{}' sets `publicly_accessible = true`, exposing the \
                             database to the internet. Fix: set it to false and reach the DB \
                             through a private subnet/VPC.",
                            block.name
                        ),
                        &input,
                        block.line,
                    ));
                }
                "aws_ebs_volume" if bool_attr(block.body, "encrypted") != Some(true) => {
                    findings.extend(crate::boundary::finding::from_validation(
                        (&self.rule_id, Severity::Error),
                        "AWS resource violates a hardening check",
                        format!(
                            "EBS volume '{}' is not `encrypted = true`, leaving data at rest \
                             unprotected. Fix: set `encrypted = true`.",
                            block.name
                        ),
                        &input,
                        block.line,
                    ));
                }
                "aws_security_group_rule"
                    if string_attr_eq(block.body, "type", "ingress")
                        && allows_public_cidr(block.body) =>
                {
                    if let Some(service) = exposed_sensitive_port(block.body) {
                        findings.extend(crate::boundary::finding::from_validation(
                            (&self.rule_id, Severity::Error),
                            "AWS resource violates a hardening check",
                            format!(
                                "security group rule '{}' exposes {service} to `0.0.0.0/0`. \
                                 Fix: restrict `cidr_blocks` to a known range or use a bastion \
                                 / VPN / private networking.",
                                block.name
                            ),
                            &input,
                            block.line,
                        ));
                    }
                }
                "aws_security_group" => {
                    for ingress in ingress_subblocks(block.body) {
                        if allows_public_cidr(ingress) {
                            if let Some(service) = exposed_sensitive_port(ingress) {
                                findings.extend(crate::boundary::finding::from_validation(
                                    (&self.rule_id, Severity::Error),
                                    "AWS resource violates a hardening check",
                                    format!(
                                        "security group '{}' exposes {service} to `0.0.0.0/0`. \
                                         Fix: restrict `cidr_blocks` to a known range or use a \
                                         bastion / VPN / private networking.",
                                        block.name
                                    ),
                                    &input,
                                    block.line,
                                ));
                                break;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        findings
    }
}

#[cfg(test)]
mod tests {
    use crate::boundary::fixture::run_manifest_fixture_parity;

    use super::AwsResourceHardeningValidator;

    #[test]
    fn cyberskills_cloud_aws() -> Result<(), Box<dyn std::error::Error>> {
        let validator = AwsResourceHardeningValidator::new()?;
        run_manifest_fixture_parity(
            &validator,
            "tests/fixtures/cyberskills/cloud.aws.resource-hardening/bad/public.tf",
            "tests/fixtures/cyberskills/cloud.aws.resource-hardening/good/hardened.tf",
        )?;
        Ok(())
    }
}
