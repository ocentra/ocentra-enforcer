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

use enforcer_core::error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};
use regex::Regex;

/// One `SQLI_PATTERNS` entry: pattern, human label, and severity, exactly
/// as the corpus script's L14-32 table declares (pattern-for-pattern,
/// order preserved).
struct SqliPattern {
    regex: &'static str,
    label: &'static str,
    severity: &'static str,
}

/// The 17-entry `SQLI_PATTERNS` table (agent.py L14-32), ported verbatim.
const SQLI_PATTERNS_SRC: &[SqliPattern] = &[
    SqliPattern {
        regex: r"(?i)\bUNION\s+(?:ALL\s+)?SELECT\b",
        label: "UNION-based",
        severity: "critical",
    },
    SqliPattern {
        regex: r#"(?i)\bOR\s+['"]?\d+['"]?\s*=\s*['"]?\d+"#,
        label: "Tautology (OR 1=1)",
        severity: "high",
    },
    SqliPattern {
        regex: r#"(?i)\bAND\s+['"]?\d+['"]?\s*=\s*['"]?\d+"#,
        label: "Tautology (AND 1=1)",
        severity: "high",
    },
    SqliPattern {
        regex: r"(?i)\bSLEEP\s*\(\s*\d+\s*\)",
        label: "Time-based blind (SLEEP)",
        severity: "critical",
    },
    SqliPattern {
        regex: r"(?i)\bBENCHMARK\s*\(",
        label: "Time-based blind (BENCHMARK)",
        severity: "critical",
    },
    SqliPattern {
        regex: r"(?i)\bWAITFOR\s+DELAY\b",
        label: "Time-based blind (WAITFOR)",
        severity: "critical",
    },
    SqliPattern {
        regex: r#"(?i)['"]\s*;\s*(?:DROP|DELETE|UPDATE|INSERT)\b"#,
        label: "Stacked query",
        severity: "critical",
    },
    SqliPattern {
        regex: r"(?i)\bINFORMATION_SCHEMA\b",
        label: "Schema enumeration",
        severity: "high",
    },
    SqliPattern {
        regex: r"(?i)\bLOAD_FILE\s*\(",
        label: "File read (LOAD_FILE)",
        severity: "critical",
    },
    SqliPattern {
        regex: r"(?i)\bINTO\s+(?:OUT|DUMP)FILE\b",
        label: "File write (INTO OUTFILE)",
        severity: "critical",
    },
    SqliPattern {
        regex: r"(?i)\bCONCAT\s*\(.*\bSELECT\b",
        label: "Nested SELECT in CONCAT",
        severity: "high",
    },
    SqliPattern {
        regex: r"(?i)\bGROUP_CONCAT\s*\(",
        label: "Data extraction (GROUP_CONCAT)",
        severity: "high",
    },
    SqliPattern {
        regex: r"(?i)\bEXTRACTVALUE\s*\(",
        label: "Error-based (EXTRACTVALUE)",
        severity: "high",
    },
    SqliPattern {
        regex: r"(?i)\bUPDATEXML\s*\(",
        label: "Error-based (UPDATEXML)",
        severity: "high",
    },
    SqliPattern {
        regex: r"(?i)(?:--|#|/\*)\s*$",
        label: "Comment termination",
        severity: "medium",
    },
    SqliPattern {
        regex: r"(?i)\bCHAR\s*\(\s*\d+(?:\s*,\s*\d+)*\s*\)",
        label: "CHAR() encoding bypass",
        severity: "medium",
    },
    SqliPattern {
        regex: r"(?i)0x[0-9a-f]{6,}",
        label: "Hex encoding bypass",
        severity: "medium",
    },
];

/// The `MODSEC_RULE_MAP` 942xxx lexicon (agent.py L34-55), ported verbatim.
/// `942xxx` is ModSecurity's SQL-injection CRS rule family, so a log line
/// carrying one of these ids is a WAF-confirmed SQLi hit: the id both
/// TRIGGERS a finding (even when no raw `SQLI_PATTERNS` signature survives
/// in the logged text) and enriches its detail with the rule description.
const MODSEC_RULE_MAP: &[(&str, &str)] = &[
    ("942100", "SQL Injection via libinjection"),
    ("942110", "SQL Injection (common keywords)"),
    ("942120", "SQL Injection operator"),
    ("942130", "SQL Injection tautology"),
    ("942140", "SQL Injection (DB names)"),
    ("942150", "SQL Injection (functions)"),
    ("942160", "SQL Injection blind test (sleep/benchmark)"),
    ("942170", "SQL Injection (UNION query)"),
    ("942180", "SQL Injection bypass (basic auth)"),
    ("942190", "SQL Injection (MSSQL exec)"),
    ("942200", "SQL Injection (MySQL comment/space obfuscation)"),
    ("942210", "SQL Injection (chained)"),
    ("942220", "SQL Injection (integer overflow)"),
    ("942230", "SQL Injection (conditional)"),
    ("942240", "SQL Injection (MySQL charset switch)"),
    ("942250", "SQL Injection (MATCH AGAINST)"),
    ("942260", "SQL Injection bypass (basic auth 2)"),
    ("942270", "SQL Injection (common DB names)"),
    ("942280", "SQL Injection (pg_sleep/waitfor)"),
    ("942290", "SQL Injection (MongoDB)"),
];

fn modsec_label(line: &str) -> Option<&'static str> {
    let id_regex = Regex::new(r#"id:"?(\d{6})"?"#).ok()?;
    let id = id_regex.captures(line)?.get(1)?.as_str();
    MODSEC_RULE_MAP
        .iter()
        .find(|(rule_id, _)| *rule_id == id)
        .map(|(_, label)| *label)
}

/// Severity weight used to pick the WORST hit on a line when multiple
/// patterns fire (mirrors the intuitive critical > high > medium ranking
/// the corpus script's severity labels imply).
fn severity_weight(label: &str) -> u8 {
    match label {
        "critical" => 3,
        "high" => 2,
        "medium" => 1,
        _ => 0,
    }
}

fn to_severity(label: &str) -> Severity {
    match label {
        "critical" | "high" => Severity::Error,
        "medium" => Severity::Warning,
        _ => Severity::Info,
    }
}

/// `CYBER-WAF-SQLI.1` — T2 scored matcher over WAF/access-log lines.
pub struct WafSqliSignatureValidator {
    rule_id: RuleId,
    patterns: Vec<(Regex, &'static str, &'static str)>,
}

impl WafSqliSignatureValidator {
    pub fn new() -> Result<Self, DecodeError> {
        let mut patterns = Vec::with_capacity(SQLI_PATTERNS_SRC.len());
        for entry in SQLI_PATTERNS_SRC {
            let regex = Regex::new(entry.regex)
                .map_err(|err| DecodeError::new("cyberskillsWafSqliPattern", err.to_string()))?;
            patterns.push((regex, entry.label, entry.severity));
        }
        Ok(Self {
            rule_id: "CYBER-WAF-SQLI.1".parse()?,
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
        for (index, line) in input.source.lines().enumerate() {
            let line_number = u32::try_from(index).unwrap_or(u32::MAX).saturating_add(1);
            let mut hits: Vec<(&str, &str)> = Vec::new();
            for (regex, label, severity) in &self.patterns {
                if regex.is_match(line) {
                    hits.push((label, severity));
                }
            }
            // A `942xxx` id is ModSecurity's SQL-injection CRS rule family:
            // its presence means the WAF itself already matched SQLi on this
            // request, so the line is a hit even when no raw `SQLI_PATTERNS`
            // signature survives in the logged (truncated/encoded) text.
            let modsec = modsec_label(line);
            if hits.is_empty() && modsec.is_none() {
                continue;
            }

            let (worst_label, worst_severity): (&str, &str) = if hits.is_empty() {
                // ModSecurity-only hit: a CRS SQLi rule fired => high confidence.
                ("ModSecurity SQLi rule (942xxx)", "high")
            } else {
                hits.sort_by_key(|(_, severity)| std::cmp::Reverse(severity_weight(severity)));
                hits[0]
            };

            let modsec_note = modsec
                .map(|label| format!(" ModSecurity rule match: {label}."))
                .unwrap_or_default();
            let mut matched_labels: Vec<&str> = hits.iter().map(|(label, _)| *label).collect();
            if let (true, Some(label)) = (matched_labels.is_empty(), modsec) {
                matched_labels.push(label);
            }
            findings.push(Finding {
                rule_id: self.rule_id.clone(),
                severity: to_severity(worst_severity),
                title: "WAF log line matches a SQLi signature (T2 scored)".to_owned(),
                detail: format!(
                    "confidence: {worst_severity}, primary signature: {worst_label}, all \
                     signatures matched: {}.{modsec_note} Fix: block/parameterize the offending \
                     request path; treat this log line as a probable SQL injection attempt.",
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

    use enforcer_domain::findings::ScanScope;
    use enforcer_domain::paths::RelPath;
    use enforcer_validator::validator::{ValidationInput, Validator};

    use super::WafSqliSignatureValidator;

    fn manifest_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    fn rel(path: &str) -> Result<RelPath, Box<dyn std::error::Error>> {
        Ok(path.parse()?)
    }

    /// Representative triple from the workpack: a UNION SELECT line
    /// scores a critical hit; a benign query stays under threshold (zero
    /// findings) — asserted on confidence/severity content, not a bare
    /// pass/fail bit, per the T2 labeled-corpus doctrine.
    #[test]
    fn cyberskills_waf_sqli() -> Result<(), Box<dyn std::error::Error>> {
        let validator = WafSqliSignatureValidator::new()?;

        let bad_path = manifest_dir()
            .join("tests/fixtures/cyberskills/detect.waf.sqli-signature/bad/union_select.log.txt");
        let bad_source = std::fs::read_to_string(bad_path)?;
        let file = rel("access.log")?;
        let bad_findings = validator.validate(ValidationInput {
            file: &file,
            source: &bad_source,
            scope: ScanScope::Files,
        });
        assert!(
            !bad_findings.is_empty(),
            "expected the UNION SELECT line to score a hit"
        );
        assert!(bad_findings[0].detail.contains("critical"));
        assert!(bad_findings[0].detail.contains("UNION-based"));

        let good_path = manifest_dir()
            .join("tests/fixtures/cyberskills/detect.waf.sqli-signature/good/benign.log.txt");
        let good_source = std::fs::read_to_string(good_path)?;
        let good_findings = validator.validate(ValidationInput {
            file: &file,
            source: &good_source,
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
        let file = rel("access.log")?;
        let line = r#"[id:"942100"] UNION SELECT username, password FROM users"#;
        let findings = validator.validate(ValidationInput {
            file: &file,
            source: line,
            scope: ScanScope::Files,
        });
        assert_eq!(findings.len(), 1);
        assert!(findings[0]
            .detail
            .contains("SQL Injection via libinjection"));
        Ok(())
    }

    #[test]
    fn multiple_signatures_on_one_line_report_the_worst_severity(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let validator = WafSqliSignatureValidator::new()?;
        let file = rel("access.log")?;
        // "OR 1=1" (high) plus "-- " comment termination (medium): worst
        // severity reported is high, not medium.
        let line = "GET /login?user=admin' OR 1=1 -- ";
        let findings = validator.validate(ValidationInput {
            file: &file,
            source: line,
            scope: ScanScope::Files,
        });
        assert_eq!(findings.len(), 1);
        assert!(findings[0].detail.contains("confidence: high"));
        Ok(())
    }
}
