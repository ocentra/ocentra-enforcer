//! `CYBER-WAF-SQLI.1` (T2, scored) — harvest targets 2 and 3 (h11
//! workpack): the 17-entry `SQLI_PATTERNS` regex+severity table and the
//! `MODSEC_RULE_MAP` 942xxx lexicon, ported verbatim from
//! `vendor/anthropic-cybersecurity-skills/skills/detecting-sql-injection-via-waf-logs/scripts/agent.py`
//! (L14-55). The original agent.py parses ModSecurity/WAF audit log files
//! from disk; this validator applies the SAME pattern table + rule-id
//! lexicon to each line of the given source text (a WAF/access-log excerpt
//! passed as the file under scan) — no log-file-format parsing beyond
//! per-line regex matching is added, and no CLI/engine dependency is
//! introduced.
//!
//! T2 shape: unlike the T1 boolean validators elsewhere in this module,
//! this is a SCORED matcher — each hit carries a confidence/severity
//! label (carried in the finding `detail`, mirroring the
//! `MONEY-CRIT-CLASSIFY.1` scored-finding convention in `enforcer-security`
//! since `Finding` has no dedicated scored-model field), proven against a
//! LABELED corpus (benign vs malicious log lines), not a single pass/fail
//! pair.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

use crate::boundary::pattern::{
    PatternConfidence, ScoredPattern, ScoredPatternSource as SqliPattern,
};

/// The 17-entry `SQLI_PATTERNS` table (agent.py L14-32), ported verbatim.
const SQLI_PATTERNS_SRC: &[SqliPattern] = &[
    SqliPattern {
        regex: r"(?i)\bUNION\s+(?:ALL\s+)?SELECT\b",
        label: "UNION-based",
        confidence: PatternConfidence::Critical,
    },
    SqliPattern {
        regex: r#"(?i)\bOR\s+['"]?\d+['"]?\s*=\s*['"]?\d+"#,
        label: "Tautology (OR 1=1)",
        confidence: PatternConfidence::High,
    },
    SqliPattern {
        regex: r#"(?i)\bAND\s+['"]?\d+['"]?\s*=\s*['"]?\d+"#,
        label: "Tautology (AND 1=1)",
        confidence: PatternConfidence::High,
    },
    SqliPattern {
        regex: r"(?i)\bSLEEP\s*\(\s*\d+\s*\)",
        label: "Time-based blind (SLEEP)",
        confidence: PatternConfidence::Critical,
    },
    SqliPattern {
        regex: r"(?i)\bBENCHMARK\s*\(",
        label: "Time-based blind (BENCHMARK)",
        confidence: PatternConfidence::Critical,
    },
    SqliPattern {
        regex: r"(?i)\bWAITFOR\s+DELAY\b",
        label: "Time-based blind (WAITFOR)",
        confidence: PatternConfidence::Critical,
    },
    SqliPattern {
        regex: r#"(?i)['"]\s*;\s*(?:DROP|DELETE|UPDATE|INSERT)\b"#,
        label: "Stacked query",
        confidence: PatternConfidence::Critical,
    },
    SqliPattern {
        regex: r"(?i)\bINFORMATION_SCHEMA\b",
        label: "Schema enumeration",
        confidence: PatternConfidence::High,
    },
    SqliPattern {
        regex: r"(?i)\bLOAD_FILE\s*\(",
        label: "File read (LOAD_FILE)",
        confidence: PatternConfidence::Critical,
    },
    SqliPattern {
        regex: r"(?i)\bINTO\s+(?:OUT|DUMP)FILE\b",
        label: "File write (INTO OUTFILE)",
        confidence: PatternConfidence::Critical,
    },
    SqliPattern {
        regex: r"(?i)\bCONCAT\s*\(.*\bSELECT\b",
        label: "Nested SELECT in CONCAT",
        confidence: PatternConfidence::High,
    },
    SqliPattern {
        regex: r"(?i)\bGROUP_CONCAT\s*\(",
        label: "Data extraction (GROUP_CONCAT)",
        confidence: PatternConfidence::High,
    },
    SqliPattern {
        regex: r"(?i)\bEXTRACTVALUE\s*\(",
        label: "Error-based (EXTRACTVALUE)",
        confidence: PatternConfidence::High,
    },
    SqliPattern {
        regex: r"(?i)\bUPDATEXML\s*\(",
        label: "Error-based (UPDATEXML)",
        confidence: PatternConfidence::High,
    },
    SqliPattern {
        regex: r"(?i)(?:--|#|/\*)\s*$",
        label: "Comment termination",
        confidence: PatternConfidence::Medium,
    },
    SqliPattern {
        regex: r"(?i)\bCHAR\s*\(\s*\d+(?:\s*,\s*\d+)*\s*\)",
        label: "CHAR() encoding bypass",
        confidence: PatternConfidence::Medium,
    },
    SqliPattern {
        regex: r"(?i)0x[0-9a-f]{6,}",
        label: "Hex encoding bypass",
        confidence: PatternConfidence::Medium,
    },
];

/// The `MODSEC_RULE_MAP` 942xxx lexicon (agent.py L34-55), ported verbatim.
/// `942xxx` is ModSecurity's SQL-injection CRS rule family, so a log line
/// carrying one of these ids is a WAF-confirmed SQLi hit: the id both
/// TRIGGERS a finding (even when no raw `SQLI_PATTERNS` signature survives
/// in the logged text) and enriches its detail with the rule description.
/// Severity weight used to pick the WORST hit on a line when multiple
/// patterns fire (mirrors the intuitive critical > high > medium ranking
/// the corpus script's severity labels imply).
fn to_severity(confidence: PatternConfidence) -> Severity {
    match confidence {
        PatternConfidence::Critical | PatternConfidence::High => Severity::Error,
        PatternConfidence::Medium => Severity::Warning,
    }
}

/// `CYBER-WAF-SQLI.1` — T2 scored matcher over WAF/access-log lines.
#[derive(Debug)]
pub struct WafSqliSignatureValidator {
    rule_id: RuleId,
    patterns: Vec<ScoredPattern>,
}

impl WafSqliSignatureValidator {
    pub fn new() -> Result<Self, DecodeError> {
        let mut patterns = Vec::with_capacity(SQLI_PATTERNS_SRC.len());
        for entry in SQLI_PATTERNS_SRC {
            patterns.push(ScoredPattern::compile_source(
                "cyberskillsWafSqliPattern",
                entry,
            )?);
        }
        Ok(Self {
            rule_id: enforcer_domain::ids::BuiltInSecurityRule::CyberWafSqli.id(),
            patterns,
        })
    }
}

impl Validator for WafSqliSignatureValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let mut findings = Vec::new();
        for (index, line) in input.source.as_str().lines().enumerate() {
            let line_number = u32::try_from(index).unwrap_or(u32::MAX).saturating_add(1);
            let mut hits: Vec<(&str, PatternConfidence)> = Vec::new();
            for pattern in &self.patterns {
                if pattern.regex().is_match(line) {
                    hits.push((pattern.label().as_str(), pattern.confidence()));
                }
            }
            // A `942xxx` id is ModSecurity's SQL-injection CRS rule family:
            // its presence means the WAF itself already matched SQLi on this
            // request, so the line is a hit even when no raw `SQLI_PATTERNS`
            // signature survives in the logged (truncated/encoded) text.
            let modsec = crate::boundary::source_predicates::modsec_label(line);
            if hits.is_empty() && modsec.is_none() {
                continue;
            }

            let (worst_label, worst_confidence): (&str, PatternConfidence) = if hits.is_empty() {
                // ModSecurity-only hit: a CRS SQLi rule fired => high confidence.
                ("ModSecurity SQLi rule (942xxx)", PatternConfidence::High)
            } else {
                hits.sort_by_key(|(_, confidence)| {
                    std::cmp::Reverse(crate::boundary::source_predicates::severity_weight(
                        *confidence,
                    ))
                });
                let Some(first) = hits.first() else {
                    continue;
                };
                *first
            };

            let modsec_note = match modsec {
                Some(label) => format!(" ModSecurity rule match: {label}."),
                None => String::new(),
            };
            let mut matched_labels: Vec<&str> = hits.iter().map(|(label, _)| *label).collect();
            if let (true, Some(label)) = (matched_labels.is_empty(), modsec) {
                matched_labels.push(label);
            }
            findings.extend(crate::boundary::finding::from_source(
                (&self.rule_id, to_severity(worst_confidence)),
                "WAF log line matches a SQLi signature (T2 scored)",
                format!(
                    "confidence: {}, primary signature: {worst_label}, all \
                     signatures matched: {}.{modsec_note} Fix: block/parameterize the offending \
                     request path; treat this log line as a probable SQL injection attempt.",
                    worst_confidence.as_str(),
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
    use enforcer_domain::findings::ScanScope;
    use enforcer_validator::validator::{ValidationInput, Validator};

    use crate::boundary::fixture::{read_manifest_fixture, rel_path};

    use super::WafSqliSignatureValidator;

    /// Representative triple from the workpack: a UNION SELECT line
    /// scores a critical hit; a benign query stays under threshold (zero
    /// findings) — asserted on confidence/severity content, not a bare
    /// pass/fail bit, per the T2 labeled-corpus doctrine.
    #[test]
    fn cyberskills_waf_sqli() -> Result<(), Box<dyn std::error::Error>> {
        let validator = WafSqliSignatureValidator::new()?;

        let bad_source = read_manifest_fixture(
            "tests/fixtures/cyberskills/detect.waf.sqli-signature/bad/union_select.log.txt",
        )?;
        let file = rel_path("access.log")?;
        let bad_findings = validator.validate(ValidationInput {
            file: &file,
            source: enforcer_domain::boundary::validation::ValidationSource::from_text(&bad_source),
            scope: ScanScope::Files,
        });
        assert!(
            !bad_findings.is_empty(),
            "expected the UNION SELECT line to score a hit"
        );
        assert!(bad_findings[0].detail.as_str().contains("critical"));
        assert!(bad_findings[0].detail.as_str().contains("UNION-based"));

        let good_source = read_manifest_fixture(
            "tests/fixtures/cyberskills/detect.waf.sqli-signature/good/benign.log.txt",
        )?;
        let good_findings = validator.validate(ValidationInput {
            file: &file,
            source: enforcer_domain::boundary::validation::ValidationSource::from_text(
                &good_source,
            ),
            scope: ScanScope::Files,
        });
        assert!(
            good_findings.is_empty(),
            "expected the benign query log to score below threshold: {good_findings:#?}"
        );
        Ok(())
    }

    #[test]
    fn modsec_rule_id_enriches_the_finding_detail() -> Result<(), Box<dyn std::error::Error>> {
        let validator = WafSqliSignatureValidator::new()?;
        let file = rel_path("access.log")?;
        let line = r#"[id:"942100"] UNION SELECT username, password FROM users"#;
        let findings = validator.validate(ValidationInput {
            file: &file,
            source: enforcer_domain::boundary::validation::ValidationSource::from_text(line),
            scope: ScanScope::Files,
        });
        assert_eq!(findings.len(), 1);
        assert!(findings[0]
            .detail
            .as_str()
            .contains("SQL Injection via libinjection"));
        Ok(())
    }

    #[test]
    fn multiple_signatures_on_one_line_report_the_worst_severity(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let validator = WafSqliSignatureValidator::new()?;
        let file = rel_path("access.log")?;
        // "OR 1=1" (high) plus "-- " comment termination (medium): worst
        // severity reported is high, not medium.
        let line = "GET /login?user=admin' OR 1=1 -- ";
        let findings = validator.validate(ValidationInput {
            file: &file,
            source: enforcer_domain::boundary::validation::ValidationSource::from_text(line),
            scope: ScanScope::Files,
        });
        assert_eq!(findings.len(), 1);
        assert!(findings[0].detail.as_str().contains("confidence: high"));
        Ok(())
    }
}
