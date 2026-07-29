//! `CYBER-TLS.1` (T1) — harvest target: legacy TLS/SSL protocol-version
//! detection, ported from
//! `vendor/anthropic-cybersecurity-skills/skills/configuring-tls-1-3-for-secure-communications`
//! and
//! `vendor/anthropic-cybersecurity-skills/skills/performing-ssl-tls-security-assessment`.
//!
//! Neither vendor `agent.py` carries an inline static-text predicate to
//! port verbatim: `configuring-tls-1-3-for-secure-communications/scripts/agent.py`
//! opens a live TCP socket per protocol (`ssl.SSLContext` +
//! `socket.create_connection`) and reports `severity: "CRITICAL"` whenever
//! `"1.0"` or `"1.1"` appears in the negotiated version name (L18-46);
//! `performing-ssl-tls-security-assessment/scripts/agent.py` drives the
//! `sslyze` library's `ServerScanRequest` and flags `SSLv2`/`SSLv3` as
//! `"critical"` "Deprecated Protocol" and `TLS 1.0`/`TLS 1.1` as `"high"`
//! "Legacy Protocol" whenever the corresponding cipher-suite scan comes back
//! non-empty (L57-86). Both skills are live-network probes with no
//! offline/static-text matcher to harvest verbatim. This validator
//! implements the well-known deterministic static-source equivalent of that
//! SAME classification (SSLv2/SSLv3/TLSv1(.0)/TLSv1.1 => legacy; TLSv1.2 /
//! TLSv1.3 => modern) applied to server-config and application-source text
//! (nginx `ssl_protocols`, Apache `SSLProtocol`, Python `ssl.PROTOCOL_*`,
//! Node `secureProtocol`/`minVersion`, Java `SSLContext.getInstance(...)`)
//! instead of a live handshake.

use crate::boundary::pattern::{LabelledPattern, LabelledPatternSource as LegacyTlsPattern};
use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

/// Legacy TLS/SSL version tokens this validator flags, covering both the
/// dotted (`TLSv1.1`) and underscore (`TLSv1_1`, as in Python's
/// `ssl.PROTOCOL_TLSv1_1` or Node's `'TLSv1_1_method'`) spellings seen in
/// real server configs and application source.
const LEGACY_TLS_PATTERNS_SRC: &[LegacyTlsPattern] = &[
    LegacyTlsPattern {
        regex: r"(?i)SSLv2(?:[^0-9A-Za-z]|$)",
        label: "SSLv2",
    },
    LegacyTlsPattern {
        regex: r"(?i)SSLv3(?:[^0-9A-Za-z]|$)",
        label: "SSLv3",
    },
    LegacyTlsPattern {
        regex: r"(?i)TLSv1\.0(?:[^0-9A-Za-z]|$)",
        label: "TLSv1.0",
    },
    LegacyTlsPattern {
        regex: r"(?i)TLSv1(?:_method\b|[^._0-9A-Za-z]|$)",
        label: "TLSv1 (1.0)",
    },
    LegacyTlsPattern {
        regex: r"(?i)TLSv1\.1(?:[^0-9A-Za-z]|$)",
        label: "TLSv1.1",
    },
    LegacyTlsPattern {
        regex: r"(?i)TLSv1_1(?:[^0-9A-Za-z]|$)",
        label: "TLSv1.1",
    },
];

/// True when the character immediately before the match start is a `-` or
/// `!`, i.e. this is an explicit "disable this protocol" directive (Apache
/// `SSLProtocol all -SSLv3 -TLSv1 -TLSv1.1`, mod_ssl `!SSLv3` hardening
/// syntax) rather than something that enables the legacy version.
/// `CYBER-TLS.1` — flags a legacy TLS/SSL version (`SSLv2`, `SSLv3`,
/// `TLSv1`/`TLSv1.0`, `TLSv1.1`) being enabled in server config or
/// application source, scanned line by line.
#[derive(Debug)]
pub struct TlsLegacyVersionValidator {
    rule_id: RuleId,
    patterns: Vec<LabelledPattern>,
}

impl TlsLegacyVersionValidator {
    pub fn new() -> Result<Self, DecodeError> {
        let mut patterns = Vec::with_capacity(LEGACY_TLS_PATTERNS_SRC.len());
        for entry in LEGACY_TLS_PATTERNS_SRC {
            patterns.push(LabelledPattern::compile_source(
                "cyberskillsTlsLegacyPattern",
                entry,
            )?);
        }
        Ok(Self {
            rule_id: enforcer_domain::ids::BuiltInSecurityRule::CyberNetTls.id(),
            patterns,
        })
    }
}

impl Validator for TlsLegacyVersionValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let mut findings = Vec::new();
        for (index, line) in input.source.as_str().lines().enumerate() {
            let line_number = u32::try_from(index).unwrap_or(u32::MAX).saturating_add(1);
            let mut matched_labels: Vec<&str> = Vec::new();
            for pattern in &self.patterns {
                let enabling_hit = pattern.regex().find_iter(line).any(|matched| {
                    !crate::boundary::source_predicates::is_explicit_disable(line, matched.start())
                });
                if enabling_hit && !matched_labels.contains(&pattern.label().as_str()) {
                    matched_labels.push(pattern.label().as_str());
                }
            }
            if matched_labels.is_empty() {
                continue;
            }
            findings.extend(crate::boundary::finding::from_source(
                (&self.rule_id, Severity::Error),
                "Legacy TLS/SSL version enabled",
                format!(
                    "Line enables a legacy protocol version: {}. Fix: remove SSLv2/SSLv3/TLSv1/\
                     TLSv1.1 from the allowed protocol list and require TLSv1.2 or TLSv1.3 only.",
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

    use super::TlsLegacyVersionValidator;

    #[test]
    fn cyberskills_net_tls() -> Result<(), Box<dyn std::error::Error>> {
        let v = TlsLegacyVersionValidator::new()?;
        run_manifest_fixture_parity(
            &v,
            "tests/fixtures/cyberskills/net.tls-legacy/bad/legacy.conf",
            "tests/fixtures/cyberskills/net.tls-legacy/good/modern.conf",
        )?;
        Ok(())
    }
}
