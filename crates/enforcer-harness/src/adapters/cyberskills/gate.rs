//! Thin T1/T2 severity gates over an [`super::seam::AdapterOutcome`]: the
//! ENGINE stays external (a slither/mythril/sqlmap/SCA-scanner/... run), but
//! the pass/fail decision over its output is ours — a native Rust
//! `enforcer_validator::Validator`, d01-scaffolded like every other rule in
//! this workspace (registered in `crates/enforcer-rules/rules/`, proven by
//! `crates/enforcer-rules/tests/cyberskills_adapters_registry.rs`).
//!
//! [`SeverityThresholdGate`] is deliberately GENERIC over engines — it does
//! not assume slither, an SCA scanner, or any specific tool is installed;
//! it only requires an [`super::seam::AdapterOutcome`] (produced either by
//! [`super::recorded::parse_recorded`] from a fixture, or eventually by a
//! live subprocess adapter parsing real stdout through the same seam).

// BOUNDARY-INVARIANT: recorded adapter text is parsed before severity policy
// evaluates canonical findings; raw engine fields never enter the gate.
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

use super::recorded::parse_recorded;
use super::seam::AdapterOutcome;

/// A T2 scored gate: fails (emits a [`Finding`]) when ANY finding in the
/// engine's [`AdapterOutcome`] normalizes to a severity at or above
/// [`Self::threshold`]. `enforcer_domain::Severity` only has
/// Error/Warning/Info, so "at or above" is `Severity::Error` at the
/// default (strictest) threshold, or `Severity::Warning` when a caller
/// widens it. Skipped/errored outcomes never themselves trip this gate —
/// honesty about tool absence/failure is a SEPARATE concern from severity
/// scoring (see [`cyberskills_adapter_graceful_skip`] tests), though a
/// present-but-erroring engine's [`Finding`] is still surfaced by the
/// harness's own run-record, not silently dropped.
pub struct SeverityThresholdGate {
    rule_id: RuleId,
    threshold: Severity,
}

impl SeverityThresholdGate {
    /// Build a gate for `rule_id` (must already be registered in an
    /// `enforcer-rules` catalog) that fails closed at `threshold` or worse.
    pub fn new(rule_id: RuleId, threshold: Severity) -> Self {
        Self { rule_id, threshold }
    }

    /// Evaluate one [`AdapterOutcome`] directly (used by both the
    /// `Validator` impl below, which parses `input.source` as a recorded
    /// fixture first, and any caller that already has a live outcome in
    /// hand).
    pub fn evaluate(
        &self,
        outcome: &AdapterOutcome,
        file: &enforcer_domain::paths::RelPath,
    ) -> Vec<Finding> {
        outcome
            .findings()
            .iter()
            .filter(|finding| {
                AdapterOutcome::normalize_severity(finding.severity.as_str()) <= self.threshold
            })
            .filter_map(|finding| {
                finding.to_domain(self.rule_id.clone(), file.clone(), self.threshold)
            })
            .collect()
    }
}

impl Validator for SeverityThresholdGate {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    /// Treats `input.source` as a RECORDED adapter-outcome JSON document
    /// (the CI-test posture the workpack requires: no live engine needed).
    /// Malformed/dishonest recorded JSON is itself a parity gap, not a
    /// silent empty result — it fails the SAME way `run_fixture_parity`
    /// expects a rule's fail fixture to fail: by emitting a `Finding`, here
    /// carrying the parse rejection as the detail.
    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        match parse_recorded(input.source.as_str()) {
            Ok(outcome) => self.evaluate(&outcome, input.file),
            Err(err) => domain_finding!(
                self.rule_id.clone(),
                Severity::Error,
                "cyberskills adapter output rejected".to_owned(),
                err.to_string(),
                input.file.clone(),
                1,
            )
            .into_iter()
            .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use enforcer_domain::boundary::validation::ValidationSource;
    use enforcer_domain::harness_types::{
        HarnessDiagnosticMessage, HarnessDiagnosticPath, HarnessExternalRuleId,
        HarnessExternalSeverity, HarnessSourceLine, HarnessThreatId,
    };
    use enforcer_domain::paths::{RelPath, RepoRoot};
    use enforcer_domain::severity::Severity;
    use enforcer_validator::harness::run_fixture_parity;
    use enforcer_validator::validator::{ValidationInput, Validator};

    use super::SeverityThresholdGate;
    use crate::adapters::cyberskills::seam::{AdapterOutcome, EngineFindingEnvelope};

    fn manifest_dir() -> Result<RepoRoot, Box<dyn std::error::Error>> {
        Ok(RepoRoot::try_from(std::path::Path::new(env!(
            "CARGO_MANIFEST_DIR"
        )))?)
    }

    fn gate() -> Result<SeverityThresholdGate, Box<dyn std::error::Error>> {
        Ok(SeverityThresholdGate::new(
            "CYBER-ADAPTER-SCA-SEVERITY.1".parse()?,
            Severity::Error,
        ))
    }

    #[test]
    fn severity_gate_fixture_parity() -> Result<(), Box<dyn std::error::Error>> {
        let fail: RelPath =
            "tests/fixtures/cyberskills_adapters/sca/bad/high_cve_over_threshold.json".parse()?;
        let pass: RelPath =
            "tests/fixtures/cyberskills_adapters/sca/good/below_threshold.json".parse()?;
        run_fixture_parity(&gate()?, &manifest_dir()?, &fail, &pass)?;
        Ok(())
    }

    #[test]
    fn gate_fires_on_high_severity_finding() -> Result<(), Box<dyn std::error::Error>> {
        let gate = gate()?;
        let file: enforcer_domain::paths::RelPath = "package-lock.json".parse()?;
        let outcome = AdapterOutcome::Ran {
            ran: 1,
            findings: vec![EngineFindingEnvelope {
                rule_id: HarnessExternalRuleId::from_adapter("CVE-2024-12345"),
                severity: HarnessExternalSeverity::from_adapter("High"),
                file: HarnessDiagnosticPath::from_adapter("package-lock.json"),
                line: HarnessSourceLine::from_external(1),
                message: HarnessDiagnosticMessage::from_adapter("vulnerable"),
                threat_id: Some(HarnessThreatId::from_adapter("CWE-1321")),
            }],
        };
        let findings = gate.evaluate(&outcome, &file);
        assert_eq!(findings.len(), 1);
        assert_eq!(
            findings[0].detail.as_str(),
            "CVE-2024-12345 (High): vulnerable [CWE-1321]"
        );
        Ok(())
    }

    #[test]
    fn gate_stays_silent_on_low_severity_finding() -> Result<(), Box<dyn std::error::Error>> {
        let gate = gate()?;
        let file: enforcer_domain::paths::RelPath = "package-lock.json".parse()?;
        let outcome = AdapterOutcome::Ran {
            ran: 1,
            findings: vec![EngineFindingEnvelope {
                rule_id: HarnessExternalRuleId::from_adapter("CVE-2023-99999"),
                severity: HarnessExternalSeverity::from_adapter("Low"),
                file: HarnessDiagnosticPath::from_adapter("package-lock.json"),
                line: HarnessSourceLine::from_external(1),
                message: HarnessDiagnosticMessage::from_adapter("minor"),
                threat_id: None,
            }],
        };
        assert!(gate.evaluate(&outcome, &file).is_empty());
        Ok(())
    }

    #[test]
    fn gate_stays_silent_on_honest_skip() -> Result<(), Box<dyn std::error::Error>> {
        let gate = gate()?;
        let file: enforcer_domain::paths::RelPath = "package-lock.json".parse()?;
        let outcome = AdapterOutcome::Skipped { ran: 0 };
        assert!(gate.evaluate(&outcome, &file).is_empty());
        Ok(())
    }

    #[test]
    fn validator_rejects_malformed_source_as_a_finding_not_a_panic(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let gate = gate()?;
        let file: enforcer_domain::paths::RelPath = "adapter-output.json".parse()?;
        let findings = gate.validate(ValidationInput {
            file: &file,
            source: ValidationSource::from_text("{not json"),
            scope: enforcer_domain::findings::ScanScope::Files,
        });
        assert_eq!(findings.len(), 1);
        assert!(
            findings[0]
                .detail
                .as_str()
                .to_lowercase()
                .contains("decode")
                || findings[0].title.as_str().contains("rejected")
        );
        Ok(())
    }
}
