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

use enforcer_core::error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};
use regex::Regex;

/// One legacy-protocol-token pattern: the `regex` crate has no
/// lookahead/lookbehind, so each pattern explicitly CONSUMES the trailing
/// boundary character (a non-alnum char, or end of line) instead of
/// asserting on it, which is what lets `TLSv1` avoid matching inside
/// `TLSv1.2`/`TLSv1.3` (a plain `\bTLSv1\b` would match there too, since
/// `.` is already a non-word character and satisfies `\b` on its own).
struct LegacyTlsPattern {
    regex: &'static str,
    label: &'static str,
}

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
fn is_explicit_disable(line: &str, start: usize) -> bool {
    matches!(line[..start].chars().last(), Some('-') | Some('!'))
}

/// `CYBER-TLS.1` — flags a legacy TLS/SSL version (`SSLv2`, `SSLv3`,
/// `TLSv1`/`TLSv1.0`, `TLSv1.1`) being enabled in server config or
/// application source, scanned line by line.
pub struct TlsLegacyVersionValidator {
    rule_id: RuleId,
    patterns: Vec<(Regex, &'static str)>,
}

impl TlsLegacyVersionValidator {
    pub fn new() -> Result<Self, DecodeError> {
        let mut patterns = Vec::with_capacity(LEGACY_TLS_PATTERNS_SRC.len());
        for entry in LEGACY_TLS_PATTERNS_SRC {
            let regex = Regex::new(entry.regex)
                .map_err(|err| DecodeError::new("cyberskillsTlsLegacyPattern", err.to_string()))?;
            patterns.push((regex, entry.label));
        }
        Ok(Self {
            rule_id: "CYBER-TLS.1".parse()?,
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
        for (index, line) in input.source.lines().enumerate() {
            let line_number = u32::try_from(index).unwrap_or(u32::MAX).saturating_add(1);
            let mut matched_labels: Vec<&str> = Vec::new();
            for (regex, label) in &self.patterns {
                let enabling_hit = regex
                    .find_iter(line)
                    .any(|matched| !is_explicit_disable(line, matched.start()));
                if enabling_hit && !matched_labels.contains(label) {
                    matched_labels.push(*label);
                }
            }
            if matched_labels.is_empty() {
                continue;
            }
            findings.push(Finding {
                rule_id: self.rule_id.clone(),
                severity: Severity::Error,
                title: "Legacy TLS/SSL version enabled".to_owned(),
                detail: format!(
                    "Line enables a legacy protocol version: {}. Fix: remove SSLv2/SSLv3/TLSv1/\
                     TLSv1.1 from the allowed protocol list and require TLSv1.2 or TLSv1.3 only.",
                    matched_labels.join(", ")
                ),
                file: input.file.clone(),
                line: line_number,
                snippet: Some(line.to_owned()),
            });
        }
        findings
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use enforcer_validator::harness::run_fixture_parity;

    use super::TlsLegacyVersionValidator;

    fn manifest_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn cyberskills_net_tls() -> Result<(), Box<dyn std::error::Error>> {
        let v = TlsLegacyVersionValidator::new()?;
        run_fixture_parity(
            &v,
            &manifest_dir(),
            "tests/fixtures/cyberskills/net.tls-legacy/bad/legacy.conf",
            "tests/fixtures/cyberskills/net.tls-legacy/good/modern.conf",
        )?;
        Ok(())
    }
}
