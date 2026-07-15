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
use regex::Regex;

use super::iac_terraform::{
    allows_public_cidr, ingress_subblocks, int_attr, resource_blocks, string_attr_eq,
};

/// Sensitive service ports that must never be exposed to `0.0.0.0/0` (SSH
/// 22 is covered by `iac_terraform`'s dedicated rule and excluded here).
const SENSITIVE_PORTS: &[(i64, &str)] = &[
    (3389, "RDP"),
    (3306, "MySQL"),
    (5432, "PostgreSQL"),
    (6379, "Redis"),
    (27017, "MongoDB"),
    (1433, "MSSQL"),
    (9200, "Elasticsearch"),
    (23, "Telnet"),
];

/// Extract an unquoted HCL boolean attribute (`name = true` / `false`).
fn bool_attr(body: &str, name: &str) -> Option<bool> {
    let pattern = Regex::new(&format!(r"(?i)\b{name}\s*=\s*(true|false)\b")).ok()?;
    let value = pattern.captures(body)?.get(1)?.as_str();
    Some(value.eq_ignore_ascii_case("true"))
}

/// If an ingress rule body opens `0.0.0.0/0` on a sensitive port (or a range
/// covering one, or a wide-open range), name the exposed service.
fn exposed_sensitive_port(body: &str) -> Option<&'static str> {
    let from = int_attr(body, "from_port")?;
    let to = int_attr(body, "to_port")?;
    if from <= 0 && to >= 65535 {
        return Some("all ports (0.0.0.0/0:0-65535)");
    }
    SENSITIVE_PORTS
        .iter()
        .find(|(port, _)| from <= *port && *port <= to)
        .map(|(_, name)| *name)
}

/// `CYBER-AWS.1` — additional AWS Terraform hardening gate.
pub struct AwsResourceHardeningValidator {
    rule_id: RuleId,
}

impl AwsResourceHardeningValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "CYBER-AWS.1".parse()?,
        })
    }

    fn finding(&self, input: &ValidationInput<'_>, line: u32, detail: String) -> Finding {
        Finding {
            rule_id: self.rule_id.clone(),
            severity: Severity::Error,
            title: "AWS resource violates a hardening check".to_owned(),
            detail,
            file: input.file.clone(),
            line,
            snippet: None,
        }
    }
}

impl Validator for AwsResourceHardeningValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let mut findings = Vec::new();
        for block in resource_blocks(input.source) {
            match block.resource_type {
                "aws_s3_bucket" | "aws_s3_bucket_acl"
                    if string_attr_eq(block.body, "acl", "public-read")
                        || string_attr_eq(block.body, "acl", "public-read-write") =>
                {
                    findings.push(self.finding(
                        &input,
                        block.line,
                        format!(
                            "S3 bucket '{}' grants a public ACL (`acl = \"public-read\"` / \
                             `\"public-read-write\"`), exposing objects to the internet. Fix: \
                             use `private` and grant access via a scoped bucket policy.",
                            block.name
                        ),
                    ));
                }
                "aws_db_instance" if bool_attr(block.body, "publicly_accessible") == Some(true) => {
                    findings.push(self.finding(
                        &input,
                        block.line,
                        format!(
                            "RDS instance '{}' sets `publicly_accessible = true`, exposing the \
                             database to the internet. Fix: set it to false and reach the DB \
                             through a private subnet/VPC.",
                            block.name
                        ),
                    ));
                }
                "aws_ebs_volume" if bool_attr(block.body, "encrypted") != Some(true) => {
                    findings.push(self.finding(
                        &input,
                        block.line,
                        format!(
                            "EBS volume '{}' is not `encrypted = true`, leaving data at rest \
                             unprotected. Fix: set `encrypted = true`.",
                            block.name
                        ),
                    ));
                }
                "aws_security_group_rule"
                    if string_attr_eq(block.body, "type", "ingress")
                        && allows_public_cidr(block.body) =>
                {
                    if let Some(service) = exposed_sensitive_port(block.body) {
                        findings.push(self.finding(
                            &input,
                            block.line,
                            format!(
                                "security group rule '{}' exposes {service} to `0.0.0.0/0`. \
                                 Fix: restrict `cidr_blocks` to a known range or use a bastion \
                                 / VPN / private networking.",
                                block.name
                            ),
                        ));
                    }
                }
                "aws_security_group" => {
                    for ingress in ingress_subblocks(block.body) {
                        if allows_public_cidr(ingress) {
                            if let Some(service) = exposed_sensitive_port(ingress) {
                                findings.push(self.finding(
                                    &input,
                                    block.line,
                                    format!(
                                        "security group '{}' exposes {service} to `0.0.0.0/0`. \
                                         Fix: restrict `cidr_blocks` to a known range or use a \
                                         bastion / VPN / private networking.",
                                        block.name
                                    ),
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
    use std::path::PathBuf;

    use enforcer_validator::harness::run_fixture_parity;

    use super::AwsResourceHardeningValidator;

    fn manifest_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn cyberskills_cloud_aws() -> Result<(), Box<dyn std::error::Error>> {
        let validator = AwsResourceHardeningValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/cyberskills/cloud.aws.resource-hardening/bad/public.tf",
            "tests/fixtures/cyberskills/cloud.aws.resource-hardening/good/hardened.tf",
        )?;
        Ok(())
    }
}
