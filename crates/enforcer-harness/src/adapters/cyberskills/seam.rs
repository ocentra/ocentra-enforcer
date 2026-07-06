//! The graceful-skip run-adapter seam every cyberskills engine adapter
//! shares: an HONEST three-way outcome (absent / errored / ran) that can
//! never collapse "tool missing" into "tool passed" — the exact failure
//! mode the h12 workpack calls out as dishonest and the a09 doctrine
//! forbids ("skipped != passed != failed").
//!
//! [`AdapterOutcome`] is the wire-adjacent shape every adapter (live or
//! recorded-fixture) produces; [`AdapterOutcome::is_honest`] is the
//! detection oracle the `cyberskills_adapter_graceful_skip` test drives.

use enforcer_domain::severity::Severity;

/// One finding an external engine reported, generic across engines (SCA
/// scanner CVE, static-analysis weakness, benchmark failure, ...).
///
/// Deliberately NOT `enforcer_domain::Finding` yet — this is the
/// engine-agnostic wire shape a RECORDED fixture or a live subprocess's
/// parsed stdout carries; [`crate::adapters::cyberskills::gate`] is what
/// maps it onto a real `Finding` behind a `RuleId`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineFinding {
    /// The engine's own identifier for what fired (a CVE id, a detector
    /// name, a CIS benchmark control id, ...).
    pub rule_id: String,
    /// The engine's own severity label (kept as the engine's raw string —
    /// e.g. `"High"`/`"Critical"` — normalized to `enforcer_domain::Severity`
    /// only at the gate, since engines disagree on vocabulary).
    pub severity: String,
    /// Repo/target-relative file the finding points at.
    pub file: String,
    /// 1-based line number, when the engine supplies one.
    #[serde(default = "default_line")]
    pub line: u32,
    /// Human-readable detail.
    pub message: String,
    /// Optional MITRE ATT&CK / CWE / OWASP citation the engine supplied.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub threat_id: Option<String>,
}

fn default_line() -> u32 {
    1
}

/// The three-way honest outcome of running one adapter against one target.
/// Never a bare bool — collapsing "absent" and "ran clean" into the same
/// `true` is exactly the dishonest-skip failure mode this seam exists to
/// prevent.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "outcome", rename_all = "lowercase")]
pub enum AdapterOutcome {
    /// The engine binary/lib was not found. This is a SKIP, not a pass —
    /// `ran` must be `0` and `findings` must be empty; a present, running
    /// tool is the only path that may report zero findings as "clean".
    Skipped {
        /// How many target items the engine would have covered, always `0`
        /// for a skip (kept explicit rather than omitted, so a skip and a
        /// zero-finding clean run are structurally distinguishable at the
        /// JSON level, not just by this enum tag).
        ran: u32,
    },
    /// The engine binary/lib was present but exited non-zero, produced
    /// unparseable output, or otherwise failed — surfaced as an error, NOT
    /// silently treated as clean.
    Errored {
        /// The engine's own failure detail (stderr tail, parse error, ...).
        error_message: String,
    },
    /// The engine ran to completion (present and did not error). `findings`
    /// may legitimately be empty (a real clean run) — that emptiness is
    /// only trustworthy because [`AdapterOutcome::Ran`] guarantees the tool
    /// actually executed.
    Ran {
        /// How many target items the engine covered.
        ran: u32,
        /// Findings the engine reported (possibly empty).
        findings: Vec<EngineFinding>,
    },
}

impl AdapterOutcome {
    /// `true` for every well-formed outcome value THIS type can construct.
    /// Because [`AdapterOutcome`] has no variant that conflates "absent"
    /// with "passed", any value of this type is honest by construction —
    /// the dishonest shape (`toolPresent: false` yet `outcome: "pass"`)
    /// cannot even be represented; see [`crate::adapters::cyberskills::recorded`]
    /// for the boundary check that rejects a raw fixture attempting that
    /// shape before it can become an [`AdapterOutcome`].
    pub fn is_honest(&self) -> bool {
        match self {
            AdapterOutcome::Skipped { ran } => *ran == 0,
            AdapterOutcome::Errored { .. } | AdapterOutcome::Ran { .. } => true,
        }
    }

    /// The findings this outcome carries, empty for `Skipped`/`Errored`.
    pub fn findings(&self) -> &[EngineFinding] {
        match self {
            AdapterOutcome::Ran { findings, .. } => findings,
            AdapterOutcome::Skipped { .. } | AdapterOutcome::Errored { .. } => &[],
        }
    }

    /// Map the engine's raw severity string onto the domain [`Severity`]
    /// scale. Unrecognized labels fail CLOSED to [`Severity::Error`] —
    /// an adapter that emits a severity word this mapping does not know
    /// must not silently downgrade to a warning.
    pub fn normalize_severity(raw: &str) -> Severity {
        match raw.to_ascii_lowercase().as_str() {
            "info" | "informational" | "low" => Severity::Info,
            "warning" | "medium" | "moderate" => Severity::Warning,
            _ => Severity::Error,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AdapterOutcome, EngineFinding};

    #[test]
    fn skipped_with_zero_ran_is_honest() {
        let outcome = AdapterOutcome::Skipped { ran: 0 };
        assert!(outcome.is_honest());
        assert!(outcome.findings().is_empty());
    }

    #[test]
    fn skipped_with_nonzero_ran_is_dishonest() {
        // Structurally representable (serde does not forbid it) but the
        // honesty oracle must still catch it — a skip that claims to have
        // covered targets is a contradiction in terms.
        let outcome = AdapterOutcome::Skipped { ran: 3 };
        assert!(!outcome.is_honest());
    }

    #[test]
    fn errored_is_honest_and_carries_no_findings() {
        let outcome = AdapterOutcome::Errored {
            error_message: "boom".to_owned(),
        };
        assert!(outcome.is_honest());
        assert!(outcome.findings().is_empty());
    }

    #[test]
    fn ran_with_findings_is_honest() {
        let outcome = AdapterOutcome::Ran {
            ran: 1,
            findings: vec![EngineFinding {
                rule_id: "X".to_owned(),
                severity: "High".to_owned(),
                file: "a.rs".to_owned(),
                line: 1,
                message: "m".to_owned(),
                threat_id: None,
            }],
        };
        assert!(outcome.is_honest());
        assert_eq!(outcome.findings().len(), 1);
    }

    #[test]
    fn severity_normalization_fails_closed_to_error_for_unknown_labels() {
        assert_eq!(
            AdapterOutcome::normalize_severity("Critical"),
            enforcer_domain::severity::Severity::Error
        );
        assert_eq!(
            AdapterOutcome::normalize_severity("nonsense-label"),
            enforcer_domain::severity::Severity::Error
        );
        assert_eq!(
            AdapterOutcome::normalize_severity("low"),
            enforcer_domain::severity::Severity::Info
        );
        assert_eq!(
            AdapterOutcome::normalize_severity("Medium"),
            enforcer_domain::severity::Severity::Warning
        );
    }

    #[test]
    fn outcome_wire_form_round_trips() -> Result<(), serde_json::Error> {
        let outcome = AdapterOutcome::Ran {
            ran: 1,
            findings: vec![],
        };
        let wire = serde_json::to_string(&outcome)?;
        let back: AdapterOutcome = serde_json::from_str(&wire)?;
        assert_eq!(back, outcome);
        Ok(())
    }
}
