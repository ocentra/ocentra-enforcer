//! `CYBER-SSRF.1` (T1) — harvest target: cloud-instance-metadata SSRF
//! predicate for `vendor/anthropic-cybersecurity-skills/skills/performing-ssrf-vulnerability-exploitation`
//! and `vendor/anthropic-cybersecurity-skills/skills/performing-blind-ssrf-exploitation`.
//!
//! Harvest note: both vendor `scripts/agent.py` files are live exploitation
//! agents (they fire real HTTP requests at a `--target-url`/`--target`
//! through `requests`) — there is no inline static-analysis predicate to
//! port verbatim. What both scripts agree on, verbatim, is the payload
//! *target list* they probe: `METADATA_PAYLOADS` / `SSRF_PAYLOADS["aws_metadata"
//! | "gcp_metadata" | "azure_metadata"]` in
//! `performing-ssrf-vulnerability-exploitation/scripts/agent.py` L15-23 and
//! `performing-blind-ssrf-exploitation/scripts/agent.py` L27-40 both hard-code
//! `169.254.169.254` (AWS IMDS + Azure IMDS, which also lives on the same
//! link-local IP) and `metadata.google.internal` (GCP) as the metadata hosts
//! worth attacking. Per the h11 workpack fallback (no inline detection
//! predicate in the vendor script => implement the well-known deterministic
//! Semgrep-style source check for the vulnerability class), this validator
//! is a literal-match line-scanner over the full well-known cloud-metadata
//! address set: the two vendor-verified hosts above, plus the two other
//! platform link-local metadata IPs the same SSRF class always targets
//! (`169.254.170.2` ECS task metadata, `100.100.100.200` Alibaba Cloud) and
//! the EC2 IPv6 IMDS host (`fd00:ec2::254`). No user-input taint tracking is
//! attempted (per spec) — a bare literal reference to one of these
//! addresses anywhere in scanned source is an extremely high-confidence
//! signal on its own (these addresses have no legitimate purpose outside
//! deliberate instance-metadata access), so this is intentionally a pure
//! textual match, not a call-graph/data-flow analysis.

use enforcer_core::error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};
use regex::Regex;

/// One well-known cloud-instance-metadata endpoint literal: the exact
/// address string and a human label naming the platform/service it belongs
/// to (used in finding details).
struct MetadataEndpoint {
    literal: &'static str,
    label: &'static str,
}

/// The well-known cloud-metadata address set. `169.254.169.254` and
/// `metadata.google.internal` are ported verbatim from both vendor
/// `agent.py` payload tables; the remaining three are the same SSRF
/// class's other platform-specific link-local metadata addresses.
const METADATA_ENDPOINTS: &[MetadataEndpoint] = &[
    MetadataEndpoint {
        literal: "169.254.169.254",
        label: "AWS/Azure/GCP instance metadata service (link-local IMDS IP)",
    },
    MetadataEndpoint {
        literal: "metadata.google.internal",
        label: "GCP metadata hostname",
    },
    MetadataEndpoint {
        literal: "169.254.170.2",
        label: "AWS ECS task metadata endpoint",
    },
    MetadataEndpoint {
        literal: "100.100.100.200",
        label: "Alibaba Cloud instance metadata endpoint",
    },
    MetadataEndpoint {
        literal: "fd00:ec2::254",
        label: "AWS EC2 IPv6 instance metadata endpoint",
    },
];

/// `CYBER-SSRF.1` — literal cloud-instance-metadata endpoint access.
pub struct SsrfMetadataValidator {
    rule_id: RuleId,
    endpoints: Vec<(Regex, &'static str, &'static str)>,
}

impl SsrfMetadataValidator {
    pub fn new() -> Result<Self, DecodeError> {
        let mut endpoints = Vec::with_capacity(METADATA_ENDPOINTS.len());
        for entry in METADATA_ENDPOINTS {
            let pattern = format!("(?i){}", regex::escape(entry.literal));
            let regex = Regex::new(&pattern).map_err(|err| {
                DecodeError::new("cyberskillsSsrfMetadataPattern", err.to_string())
            })?;
            endpoints.push((regex, entry.literal, entry.label));
        }
        Ok(Self {
            rule_id: "CYBER-SSRF.1".parse()?,
            endpoints,
        })
    }
}

impl Validator for SsrfMetadataValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let mut findings = Vec::new();
        for (index, line) in input.source.lines().enumerate() {
            let line_number = u32::try_from(index).unwrap_or(u32::MAX).saturating_add(1);
            for (regex, literal, label) in &self.endpoints {
                if !regex.is_match(line) {
                    continue;
                }
                findings.push(Finding {
                    rule_id: self.rule_id.clone(),
                    severity: Severity::Error,
                    title: "Cloud instance metadata endpoint referenced".to_owned(),
                    detail: format!(
                        "Line references `{literal}` — {label}. This is a classic SSRF /\
                         credential-theft target: any code path that lets a request reach this \
                         address can be used to steal instance credentials or configuration. \
                         Fix: never let user-influenced input reach outbound requests without an \
                         allowlist, and block link-local/metadata addresses (169.254.0.0/16, \
                         100.100.100.200, fd00:ec2::254, metadata.google.internal) at the network \
                         egress layer."
                    ),
                    file: input.file.clone(),
                    line: line_number,
                    snippet: Some(line.to_owned()),
                });
            }
        }
        findings
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use enforcer_validator::harness::run_fixture_parity;

    use super::SsrfMetadataValidator;

    fn manifest_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn cyberskills_web_ssrf() -> Result<(), Box<dyn std::error::Error>> {
        let validator = SsrfMetadataValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/cyberskills/web.ssrf-metadata/bad/metadata.py",
            "tests/fixtures/cyberskills/web.ssrf-metadata/good/safe.py",
        )?;
        Ok(())
    }
}
