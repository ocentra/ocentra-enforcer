//! `CYBER-TLS-VERIFY.1` (T1) — harvest target: disabled TLS
//! certificate/hostname verification, for
//! `vendor/anthropic-cybersecurity-skills/skills/performing-ssl-tls-security-assessment`.
//!
//! Harvest note: the vendor `scripts/agent.py` is a LIVE assessment tool —
//! it builds a `sslyze` `ServerScanRequest`/`Scanner`, opens a real
//! connection to a target `hostname:port`, and inspects the returned
//! `ServerScanResult` (accepted cipher suites, certificate deployment,
//! Heartbleed/CCS-injection/ROBOT scan-command results). There is no
//! inline static-text predicate over source code to port verbatim — the
//! skill assumes TLS verification already runs correctly on the *scanning*
//! side and audits the *server's* configuration instead. Per the h11
//! workpack fallback, this validator instead implements the well-known,
//! Semgrep-style deterministic static-source check for the inverse and far
//! more common vulnerability class the same assessment domain guards
//! against: application/client code that has TURNED OFF certificate or
//! hostname verification, which silently defeats everything sslyze would
//! otherwise be validating and is a textbook MITM enabler. Each sink below
//! is a well-known, high-confidence pattern from that standard SAST
//! lexicon (Python `requests`/`ssl`, Node's `https`/`tls` options and the
//! `NODE_TLS_REJECT_UNAUTHORIZED` env escape hatch, Go's `crypto/tls`,
//! PHP's cURL binding, and the shell `curl` CLI itself), scanned line by
//! line exactly like the sibling `net_tls.rs` legacy-protocol validator.

use crate::boundary::pattern::{LabelledPattern, LabelledPatternSource as TlsVerifySink};
use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

/// The disabled-TLS-verification sinks this validator flags. Each is a
/// well-known, high-confidence "verification turned off" API/flag from the
/// standard SAST lexicon for this vulnerability class — not an exhaustive
/// enumeration of every library that can disable TLS verification.
const TLS_VERIFY_SINKS_SRC: &[TlsVerifySink] = &[
    TlsVerifySink {
        regex: r"\bverify\s*=\s*False\b",
        label: "Python requests verify=False",
    },
    TlsVerifySink {
        regex: r"ssl\._create_unverified_context\s*\(",
        label: "ssl._create_unverified_context()",
    },
    TlsVerifySink {
        regex: r"\bcheck_hostname\s*=\s*False\b",
        label: "check_hostname = False",
    },
    TlsVerifySink {
        regex: r"\bssl\.CERT_NONE\b",
        label: "ssl.CERT_NONE",
    },
    TlsVerifySink {
        regex: r"\brejectUnauthorized\s*:\s*false\b",
        label: "rejectUnauthorized: false",
    },
    TlsVerifySink {
        regex: r#"NODE_TLS_REJECT_UNAUTHORIZED\s*=\s*['"]?0['"]?"#,
        label: "NODE_TLS_REJECT_UNAUTHORIZED=0",
    },
    TlsVerifySink {
        regex: r"\bInsecureSkipVerify\s*:\s*true\b",
        label: "Go InsecureSkipVerify: true",
    },
    TlsVerifySink {
        regex: r"CURLOPT_SSL_VERIFYPEER\s*(?:=>|,)\s*(?:0|false|FALSE)\b",
        label: "PHP CURLOPT_SSL_VERIFYPEER disabled",
    },
    TlsVerifySink {
        // Case-sensitive: curl is a lowercase Unix binary name and its
        // long options are case-sensitive, so this cannot collide with
        // anything else.
        regex: r"\bcurl\b.*--insecure\b",
        label: "curl --insecure",
    },
    TlsVerifySink {
        // Case-sensitive and deliberately NOT `(?i)`: curl's short flags
        // are case-sensitive and `-K`/`--config` (a DIFFERENT, unrelated
        // flag) must never be confused with `-k`/`--insecure`.
        regex: r"\bcurl\b(?:.*\s)?-k(?:\s|$)",
        label: "curl -k",
    },
];

/// `CYBER-TLS-VERIFY.1` — flags disabled TLS certificate/hostname
/// verification (a MITM enabler) in application source, config, or shell
/// commands, scanned line by line.
#[derive(Debug)]
pub struct TlsVerificationDisabledValidator {
    rule_id: RuleId,
    sinks: Vec<LabelledPattern>,
}

impl TlsVerificationDisabledValidator {
    pub fn new() -> Result<Self, DecodeError> {
        let mut sinks = Vec::with_capacity(TLS_VERIFY_SINKS_SRC.len());
        for entry in TLS_VERIFY_SINKS_SRC {
            sinks.push(LabelledPattern::compile_source(
                "cyberskillsTlsVerifySink",
                entry,
            )?);
        }
        Ok(Self {
            rule_id: enforcer_domain::ids::BuiltInSecurityRule::CyberTlsVerify.id(),
            sinks,
        })
    }
}

impl Validator for TlsVerificationDisabledValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let mut findings = Vec::new();
        for (index, line) in input.source.as_str().lines().enumerate() {
            let line_number = u32::try_from(index).unwrap_or(u32::MAX).saturating_add(1);
            let mut matched_labels: Vec<&str> = Vec::new();
            for pattern in &self.sinks {
                if pattern.regex().is_match(line)
                    && !matched_labels.contains(&pattern.label().as_str())
                {
                    matched_labels.push(pattern.label().as_str());
                }
            }
            if matched_labels.is_empty() {
                continue;
            }
            findings.extend(crate::boundary::finding::from_source(
                (&self.rule_id, Severity::Error),
                "TLS certificate/hostname verification disabled",
                format!(
                    "Line disables TLS certificate or hostname verification: {}. This is a \
                     man-in-the-middle enabler: an attacker on the network path can present any \
                     certificate and go undetected. Fix: remove the disabling flag/option and let \
                     the TLS stack verify the peer certificate and hostname (e.g. drop \
                     `verify=False`, use `ssl.create_default_context()`, set \
                     `rejectUnauthorized: true`, `InsecureSkipVerify: false`, \
                     `CURLOPT_SSL_VERIFYPEER` to `1`, and drop `curl -k`/`--insecure`).",
                    matched_labels.join(", ")
                ),
                input.file,
                (line_number, Some(line)),
            ));
        }
        findings
    }
}

#[cfg(test)]
mod tests {
    use crate::boundary::fixture::run_manifest_fixture_parity;

    use super::TlsVerificationDisabledValidator;

    #[test]
    fn cyberskills_tls_verify() -> Result<(), Box<dyn std::error::Error>> {
        let v = TlsVerificationDisabledValidator::new()?;
        run_manifest_fixture_parity(
            &v,
            "tests/fixtures/cyberskills/net.tls-verify-disabled/bad/insecure.py",
            "tests/fixtures/cyberskills/net.tls-verify-disabled/good/secure.py",
        )?;
        Ok(())
    }
}
