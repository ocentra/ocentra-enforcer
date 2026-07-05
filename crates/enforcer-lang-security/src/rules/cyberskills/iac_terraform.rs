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

use enforcer_core::error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};
use regex::Regex;

/// Split `source` into `resource "<type>" "<name>" { ... }` blocks (brace
/// depth-aware, so a nested block inside a resource does not truncate it
/// early). Only top-level `resource` blocks are collected.
struct ResourceBlock<'a> {
    resource_type: &'a str,
    name: &'a str,
    body: &'a str,
    line: u32,
}

fn resource_blocks(source: &str) -> Vec<ResourceBlock<'_>> {
    let Ok(header) = Regex::new(r#"resource\s+"([A-Za-z0-9_]+)"\s+"([A-Za-z0-9_-]+)"\s*\{"#) else {
        return Vec::new();
    };
    let mut blocks = Vec::new();
    for capture in header.captures_iter(source) {
        let Some(whole) = capture.get(0) else {
            continue;
        };
        let open_brace = whole.end() - 1;
        let Some(close_brace) = matching_brace(source, open_brace) else {
            continue;
        };
        let line = 1 + source[..whole.start()].matches('\n').count() as u32;
        blocks.push(ResourceBlock {
            resource_type: capture.get(1).map_or("", |m| m.as_str()),
            name: capture.get(2).map_or("", |m| m.as_str()),
            body: &source[open_brace + 1..close_brace],
            line,
        });
    }
    blocks
}

/// Find the index of the `}` that closes the `{` at `open_brace`, tracking
/// nested brace depth. Returns `None` if the source is malformed (no
/// matching close) — callers skip such a block rather than panicking.
fn matching_brace(source: &str, open_brace: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut depth = 0i32;
    for (offset, byte) in bytes.iter().enumerate().skip(open_brace) {
        match byte {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(offset);
                }
            }
            _ => {}
        }
    }
    None
}

/// `CYBER-IAC-S3-SSE.1` — an `aws_s3_bucket` resource missing
/// `server_side_encryption_configuration` is flagged (Rego:
/// `aws_s3_encryption.rego`).
pub struct S3EncryptionRequiredValidator {
    rule_id: RuleId,
}

impl S3EncryptionRequiredValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "CYBER-IAC-S3-SSE.1".parse()?,
        })
    }
}

impl Validator for S3EncryptionRequiredValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let mut findings = Vec::new();
        for block in resource_blocks(input.source) {
            if block.resource_type != "aws_s3_bucket" {
                continue;
            }
            if block.body.contains("server_side_encryption_configuration") {
                continue;
            }
            findings.push(Finding {
                rule_id: self.rule_id.clone(),
                severity: Severity::Error,
                title: "S3 bucket must have server-side encryption enabled".to_owned(),
                detail: format!(
                    "S3 bucket '{}' has no `server_side_encryption_configuration` block. \
                     Fix: add a `server_side_encryption_configuration` block enabling SSE \
                     (e.g. AES256 or aws:kms).",
                    block.name
                ),
                file: input.file.clone(),
                line: block.line,
                snippet: None,
            });
        }
        findings
    }
}

/// `CYBER-IAC-IAM-WILDCARD.1` — an `aws_iam_policy` statement with
/// `Action == "*"` (or an `Action` list containing `"*"`) and
/// `Effect == "Allow"` is flagged (Rego: `aws_iam_no_wildcards.rego`).
pub struct IamNoWildcardActionValidator {
    rule_id: RuleId,
    wildcard_action: Regex,
    allow_effect: Regex,
}

fn compile_regex(pattern: &str) -> Result<Regex, DecodeError> {
    Regex::new(pattern).map_err(|err| DecodeError::new("cyberskillsIacRegex", err.to_string()))
}

impl IamNoWildcardActionValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "CYBER-IAC-IAM-WILDCARD.1".parse()?,
            wildcard_action: compile_regex(
                r#"(?i)"?Action"?\s*[:=]\s*(?:\[[^]]*"\*"[^]]*]|"\*")"#,
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
        for block in resource_blocks(input.source) {
            if block.resource_type != "aws_iam_policy" {
                continue;
            }
            // Each `Statement` entry is scanned independently: a wildcard
            // action paired with an Allow effect anywhere in the block is
            // the same shape the Rego rule matches over
            // `resource.policy.Statement[_]`.
            let statements = split_statements(block.body);
            for statement in statements {
                if self.wildcard_action.is_match(statement) && self.allow_effect.is_match(statement)
                {
                    findings.push(Finding {
                        rule_id: self.rule_id.clone(),
                        severity: Severity::Error,
                        title: "IAM policy must not use wildcard (*) actions".to_owned(),
                        detail: format!(
                            "IAM policy '{}' has a statement with `Action: \"*\"` and \
                             `Effect: \"Allow\"`. Fix: enumerate the specific actions the \
                             policy actually needs instead of a wildcard.",
                            block.name
                        ),
                        file: input.file.clone(),
                        line: block.line,
                        snippet: None,
                    });
                    break;
                }
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
fn split_statements(body: &str) -> Vec<&str> {
    let Some(marker) = body.find("Statement") else {
        return vec![body];
    };
    let mut chunks = Vec::new();
    let mut cursor = marker;
    let bytes = body.as_bytes();
    while let Some(rel_open) = body[cursor..].find('{') {
        let open = cursor + rel_open;
        let Some(close) = matching_brace(body, open) else {
            break;
        };
        chunks.push(&body[open..=close]);
        cursor = close + 1;
        if cursor >= bytes.len() {
            break;
        }
    }
    if chunks.is_empty() {
        vec![body]
    } else {
        chunks
    }
}

/// `CYBER-IAC-SG-SSH.1` — a security-group ingress rule allowing
/// `0.0.0.0/0` across a port range covering 22 is flagged (Rego:
/// `aws_no_public_ingress.rego`).
pub struct SgNoPublicSshIngressValidator {
    rule_id: RuleId,
}

impl SgNoPublicSshIngressValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "CYBER-IAC-SG-SSH.1".parse()?,
        })
    }
}

/// Extract an integer HCL attribute (`name = 22` or `name = "22"`) from
/// `body`, if present.
fn int_attr(body: &str, name: &str) -> Option<i64> {
    let pattern = Regex::new(&format!(r#"(?i){name}\s*=\s*"?(-?\d+)"?"#)).ok()?;
    pattern
        .captures(body)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse().ok())
}

fn allows_public_cidr(body: &str) -> bool {
    body.contains("0.0.0.0/0")
}

fn covers_port_22(body: &str) -> bool {
    match (int_attr(body, "from_port"), int_attr(body, "to_port")) {
        (Some(from), Some(to)) => from <= 22 && to >= 22,
        _ => false,
    }
}

impl Validator for SgNoPublicSshIngressValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let mut findings = Vec::new();
        for block in resource_blocks(input.source) {
            let is_sg_rule = block.resource_type == "aws_security_group_rule"
                || block.resource_type == "aws_security_group";
            if !is_sg_rule {
                continue;
            }
            // Both a standalone `aws_security_group_rule` (type = "ingress"
            // implicit via `ingress`/`type` attrs) and an inline `ingress {
            // ... }` block inside `aws_security_group` are checked: scan
            // every `ingress { ... }` sub-block plus the block body itself
            // (the standalone-rule shape has no nested `ingress` block).
            let mut candidates = vec![block.body];
            candidates.extend(ingress_subblocks(block.body));
            for candidate in candidates {
                let is_ingress = candidate.contains("ingress")
                    || block.resource_type == "aws_security_group_rule";
                if is_ingress && allows_public_cidr(candidate) && covers_port_22(candidate) {
                    findings.push(Finding {
                        rule_id: self.rule_id.clone(),
                        severity: Severity::Error,
                        title: "Security group rule allows SSH from 0.0.0.0/0".to_owned(),
                        detail: format!(
                            "Security group rule '{}' allows ingress from `0.0.0.0/0` across a \
                             port range covering 22 (SSH). Fix: restrict `cidr_blocks` to a \
                             known range or remove port 22 from the public rule.",
                            block.name
                        ),
                        file: input.file.clone(),
                        line: block.line,
                        snippet: None,
                    });
                    break;
                }
            }
        }
        findings
    }
}

fn ingress_subblocks(body: &str) -> Vec<&str> {
    let Ok(header) = Regex::new(r"ingress\s*\{") else {
        return Vec::new();
    };
    let mut blocks = Vec::new();
    for capture in header.captures_iter(body) {
        let Some(whole) = capture.get(0) else {
            continue;
        };
        let open_brace = whole.end() - 1;
        if let Some(close_brace) = matching_brace(body, open_brace) {
            blocks.push(&body[open_brace + 1..close_brace]);
        }
    }
    blocks
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use enforcer_validator::harness::run_fixture_parity;

    use super::SgNoPublicSshIngressValidator;
    use super::{IamNoWildcardActionValidator, S3EncryptionRequiredValidator};

    fn manifest_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn cyberskills_iac_tf_s3_encryption() -> Result<(), Box<dyn std::error::Error>> {
        let validator = S3EncryptionRequiredValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/cyberskills/iac.tf.s3-encryption-required/bad/no_sse.tf",
            "tests/fixtures/cyberskills/iac.tf.s3-encryption-required/good/sse.tf",
        )?;
        Ok(())
    }

    #[test]
    fn cyberskills_iac_tf_iam_wildcard() -> Result<(), Box<dyn std::error::Error>> {
        let validator = IamNoWildcardActionValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/cyberskills/iac.tf.iam-no-wildcard-action/bad/wildcard.tf",
            "tests/fixtures/cyberskills/iac.tf.iam-no-wildcard-action/good/scoped.tf",
        )?;
        Ok(())
    }

    #[test]
    fn cyberskills_iac_tf_sg_ssh() -> Result<(), Box<dyn std::error::Error>> {
        let validator = SgNoPublicSshIngressValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/cyberskills/iac.tf.sg-no-public-ssh-ingress/bad/public_ssh.tf",
            "tests/fixtures/cyberskills/iac.tf.sg-no-public-ssh-ingress/good/restricted.tf",
        )?;
        Ok(())
    }
}
