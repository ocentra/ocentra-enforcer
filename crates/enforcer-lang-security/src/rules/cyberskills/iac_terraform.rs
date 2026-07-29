//! `CYBER-IAC-S3-SSE.1` + `CYBER-IAC-IAM-WILDCARD.1` +
//! `CYBER-IAC-SG-SSH.1` (all T1) — the Terraform IaC cluster, harvest
//! target 1 (h11 workpack): ported from the inline Rego deny-rules in
//! `vendor/anthropic-cybersecurity-skills/skills/auditing-terraform-infrastructure-for-security/SKILL.md`
//! (L146-187):
//!
//! - `policy/aws_s3_encryption.rego`: an `aws_s3_bucket` resource block
//!   missing `server_side_encryption_configuration` is denied.
//! - `policy/aws_iam_no_wildcards.rego`: an `aws_iam_policy` statement with
//!   `Action = "*"` (or `Action` list containing `"*"`) and
//!   `Effect = "Allow"` is denied.
//! - `policy/aws_no_public_ingress.rego`: an `aws_security_group_rule`
//!   (or inline `ingress` block) allowing `0.0.0.0/0` across a port range
//!   covering 22 is denied.
//!
//! The Rego engine itself is not reimplemented (out of scope, no OPA
//! runtime) — these are the same THREE boolean predicates the Rego
//! `deny[msg]` rules express, ported to a native Rust regex/text scan over
//! raw HCL source (no `tree-sitter`/HCL AST dependency is added; the
//! predicates are resource-block-shaped substring/regex checks, matching
//! the source-pattern style every other validator in this crate already
//! uses).

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};
use regex::Regex;

use crate::boundary::terraform::{
    allows_public_cidr, compile_regex, covers_port_22, ingress_subblocks, resource_blocks,
    statement_blocks, string_attr_eq,
};

/// Split `source` into `resource "<type>" "<name>" { ... }` blocks (brace
/// depth-aware, so a nested block inside a resource does not truncate it
/// early). Only top-level `resource` blocks are collected.
/// Find the index of the `}` that closes the `{` at `open_brace`, tracking
/// nested brace depth. Returns `None` if the source is malformed (no
/// matching close) — callers skip such a block rather than panicking.
/// `CYBER-IAC-S3-SSE.1` — an `aws_s3_bucket` resource missing
/// `server_side_encryption_configuration` is flagged (Rego:
/// `aws_s3_encryption.rego`).
#[derive(Debug)]
pub struct S3EncryptionRequiredValidator {
    rule_id: RuleId,
}

impl S3EncryptionRequiredValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: enforcer_domain::ids::BuiltInSecurityRule::CyberIacS3Sse.id(),
        })
    }
}

impl Validator for S3EncryptionRequiredValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let mut findings = Vec::new();
        for block in resource_blocks(input.source.as_str()) {
            if block.resource_type != "aws_s3_bucket" {
                continue;
            }
            if block.body.contains("server_side_encryption_configuration") {
                continue;
            }
            findings.extend(crate::boundary::finding::from_owned_source(
                (&self.rule_id, Severity::Error),
                "S3 bucket must have server-side encryption enabled",
                format!(
                    "S3 bucket '{}' has no `server_side_encryption_configuration` block. \
                     Fix: add a `server_side_encryption_configuration` block enabling SSE \
                     (e.g. AES256 or aws:kms).",
                    block.name
                ),
                input.file,
                (block.line, None),
            ));
        }
        findings
    }
}

/// `CYBER-IAC-IAM-WILDCARD.1` — an `aws_iam_policy` statement with
/// `Action == "*"` (or an `Action` list containing `"*"`) and
/// `Effect == "Allow"` is flagged (Rego: `aws_iam_no_wildcards.rego`).
#[derive(Debug)]
pub struct IamNoWildcardActionValidator {
    rule_id: RuleId,
    /// Vendor deny-rule A (`aws_iam_no_wildcards.rego` L159-165):
    /// `Action == "*"` (exact, or a list containing the exact `"*"`).
    wildcard_action: Regex,
    /// Vendor deny-rule B (L167-174): `contains(statement.Action[_], "*")`
    /// — an Action string CONTAINING `*` as a substring (e.g. `s3:*`,
    /// `iam:Put*`), scalar or in a list.
    action_contains_wildcard: Regex,
    /// Vendor deny-rule B: `statement.Resource == "*"` (exact, or a list
    /// containing `"*"` — a superset of the Rego `==`, since a resource
    /// list carrying `"*"` is equally a wildcard resource).
    resource_wildcard: Regex,
    allow_effect: Regex,
}

impl IamNoWildcardActionValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: enforcer_domain::ids::BuiltInSecurityRule::CyberIacIamWildcard.id(),
            wildcard_action: compile_regex(
                r#"(?i)"?Action"?\s*[:=]\s*(?:\[[^]]*"\*"[^]]*]|"\*")"#,
            )?,
            action_contains_wildcard: compile_regex(
                r#"(?i)"?Action"?\s*[:=]\s*(?:\[[^]]*"[^"]*\*[^"]*"[^]]*]|"[^"]*\*[^"]*")"#,
            )?,
            resource_wildcard: compile_regex(
                r#"(?i)"?Resource"?\s*[:=]\s*(?:\[[^]]*"\*"[^]]*]|"\*")"#,
            )?,
            allow_effect: compile_regex(r#"(?i)"?Effect"?\s*[:=]\s*"Allow""#)?,
        })
    }
}

impl Validator for IamNoWildcardActionValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let mut findings = Vec::new();
        for block in resource_blocks(input.source.as_str()) {
            if block.resource_type != "aws_iam_policy" {
                continue;
            }
            // Each `Statement` entry is scanned independently: a wildcard
            // action paired with an Allow effect anywhere in the block is
            // the same shape the Rego rule matches over
            // `resource.policy.Statement[_]`.
            let statements = statement_blocks(block.body);
            for statement in statements {
                if !self.allow_effect.is_match(statement) {
                    continue;
                }
                // Vendor deny-rule A: Action is the exact wildcard "*".
                let rule_a = self.wildcard_action.is_match(statement);
                // Vendor deny-rule B: Resource is "*" AND some Action string
                // contains "*" as a substring (e.g. `s3:*`).
                let rule_b = self.resource_wildcard.is_match(statement)
                    && self.action_contains_wildcard.is_match(statement);
                if !rule_a && !rule_b {
                    continue;
                }
                let detail = if rule_a {
                    format!(
                        "IAM policy '{}' has a statement with `Action: \"*\"` and \
                         `Effect: \"Allow\"`. Fix: enumerate the specific actions the \
                         policy actually needs instead of a wildcard.",
                        block.name
                    )
                } else {
                    format!(
                        "IAM policy '{}' has a statement with `Resource: \"*\"`, \
                         `Effect: \"Allow\"`, and a wildcard action (e.g. `service:*`). Fix: \
                         scope the resource and enumerate specific actions instead of a wildcard \
                         on all resources.",
                        block.name
                    )
                };
                findings.extend(crate::boundary::finding::from_owned_source(
                    (&self.rule_id, Severity::Error),
                    "IAM policy must not use wildcard (*) actions",
                    detail,
                    input.file,
                    (block.line, None),
                ));
                break;
            }
        }
        findings
    }
}

/// Split a policy document body into per-`Statement`-entry chunks by
/// scanning for `{`..`}` object boundaries after a `"Statement"` key. Falls
/// back to treating the whole body as one chunk if no `Statement` array
/// marker is found (still lets the wildcard+allow predicate fire on an
/// inline single-statement shape).
/// `CYBER-IAC-SG-SSH.1` — a security-group ingress rule allowing
/// `0.0.0.0/0` across a port range covering 22 is flagged (Rego:
/// `aws_no_public_ingress.rego`).
#[derive(Debug)]
pub struct SgNoPublicSshIngressValidator {
    rule_id: RuleId,
}

impl SgNoPublicSshIngressValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: enforcer_domain::ids::BuiltInSecurityRule::CyberIacSgSsh.id(),
        })
    }
}

impl Validator for SgNoPublicSshIngressValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let mut findings = Vec::new();
        for block in resource_blocks(input.source.as_str()) {
            // The two shapes the vendor Rego + inline-block convention
            // cover, kept faithful to `aws_no_public_ingress.rego` (which
            // fires only when `resource.type == "ingress"`):
            // - standalone `aws_security_group_rule`: the block body itself
            //   is the rule, and it must declare `type = "ingress"` (an
            //   `egress` rule to 0.0.0.0/0 on port 22 is NOT flagged);
            // - `aws_security_group`: only its inline `ingress { ... }`
            //   sub-blocks are ingress (egress sub-blocks are out of scope),
            //   so the whole body is never scanned as one candidate.
            let is_public_ssh_ingress = match block.resource_type {
                "aws_security_group_rule" => {
                    string_attr_eq(block.body, "type", "ingress")
                        && allows_public_cidr(block.body)
                        && covers_port_22(block.body)
                }
                "aws_security_group" => ingress_subblocks(block.body)
                    .into_iter()
                    .any(|ingress| allows_public_cidr(ingress) && covers_port_22(ingress)),
                _ => false,
            };
            if is_public_ssh_ingress {
                findings.extend(crate::boundary::finding::from_owned_source(
                    (&self.rule_id, Severity::Error),
                    "Security group rule allows SSH from 0.0.0.0/0",
                    format!(
                        "Security group rule '{}' allows ingress from `0.0.0.0/0` across a \
                         port range covering 22 (SSH). Fix: restrict `cidr_blocks` to a \
                         known range or remove port 22 from the public rule.",
                        block.name
                    ),
                    input.file,
                    (block.line, None),
                ));
            }
        }
        findings
    }
}

#[cfg(test)]
mod tests {
    use crate::boundary::fixture::{finding_count, run_manifest_fixture_parity};

    use super::SgNoPublicSshIngressValidator;
    use super::{IamNoWildcardActionValidator, S3EncryptionRequiredValidator};

    /// Regression for the egress false-positive: the vendor Rego fires only
    /// when `resource.type == "ingress"`, so a standalone
    /// `aws_security_group_rule` with `type = "egress"` opening
    /// 0.0.0.0/0 on port 22 must NOT be flagged, while the same rule with
    /// `type = "ingress"` MUST be flagged.
    #[test]
    fn sg_rule_egress_is_not_flagged_but_ingress_is() -> Result<(), Box<dyn std::error::Error>> {
        let egress = r#"
resource "aws_security_group_rule" "out" {
  type        = "egress"
  from_port   = 22
  to_port     = 22
  protocol    = "tcp"
  cidr_blocks = ["0.0.0.0/0"]
  security_group_id = "sg-123"
}
"#;
        assert_eq!(
            finding_count(
                &SgNoPublicSshIngressValidator::new()?,
                "main.tf",
                egress,
            )?,
            0,
            "an egress rule to 0.0.0.0/0 on port 22 must not be flagged (vendor requires type==ingress)"
        );

        let ingress = r#"
resource "aws_security_group_rule" "in" {
  type        = "ingress"
  from_port   = 22
  to_port     = 22
  protocol    = "tcp"
  cidr_blocks = ["0.0.0.0/0"]
  security_group_id = "sg-123"
}
"#;
        assert_eq!(
            finding_count(&SgNoPublicSshIngressValidator::new()?, "main.tf", ingress,)?,
            1,
            "a standalone ingress rule to 0.0.0.0/0 on port 22 must be flagged"
        );
        Ok(())
    }

    #[test]
    fn cyberskills_iac_tf_s3_encryption() -> Result<(), Box<dyn std::error::Error>> {
        let validator = S3EncryptionRequiredValidator::new()?;
        run_manifest_fixture_parity(
            &validator,
            "tests/fixtures/cyberskills/iac.tf.s3-encryption-required/bad/no_sse.tf",
            "tests/fixtures/cyberskills/iac.tf.s3-encryption-required/good/sse.tf",
        )?;
        Ok(())
    }

    #[test]
    fn cyberskills_iac_tf_iam_wildcard() -> Result<(), Box<dyn std::error::Error>> {
        let validator = IamNoWildcardActionValidator::new()?;
        run_manifest_fixture_parity(
            &validator,
            "tests/fixtures/cyberskills/iac.tf.iam-no-wildcard-action/bad/wildcard.tf",
            "tests/fixtures/cyberskills/iac.tf.iam-no-wildcard-action/good/scoped.tf",
        )?;
        Ok(())
    }

    #[test]
    fn cyberskills_iac_tf_sg_ssh() -> Result<(), Box<dyn std::error::Error>> {
        let validator = SgNoPublicSshIngressValidator::new()?;
        run_manifest_fixture_parity(
            &validator,
            "tests/fixtures/cyberskills/iac.tf.sg-no-public-ssh-ingress/bad/public_ssh.tf",
            "tests/fixtures/cyberskills/iac.tf.sg-no-public-ssh-ingress/good/restricted.tf",
        )?;
        Ok(())
    }
}
