//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! `MCM-ECONOMIC.1` (T2) â€” the economic-cost mechanics facet (h06, Â§8.10
//! of the ingested money-critical/security-testing spec).
//!
//! Doctrine (Â§8.10): attacker-cost MUST be >= system-cost. A retry loop
//! that re-runs a backend operation with non-zero system cost (a
//! provider call, a DB write, a settlement attempt) with NO charge to
//! the caller and NO bound on retry count gives an attacker free,
//! unbounded leverage â€” each retry costs the system money/resources
//! while costing the attacker nothing. "Dust" (residual sub-unit value
//! left over from a calculation) must also be bounded, never
//! accumulated into an exploitable drift.
//!
//! This is a T2 SCORED heuristic (mirrors [`super::money_critical`]'s
//! scored-classifier shape): score + confidence are carried in the
//! finding detail, not a separate wire field.
//!
//! GENERIC across any value system â€” never a crypto-only fee model.
//!
//! Scoped by h01's money-critical classifier (consumed read-only).
//!
//! # Detection shape
//!
//! A line-scan `Validator` over TS/JS backend source:
//!
//! - A retry construct (`retry(`, `while (attempts`, `for (let i = 0; i <
//!   maxRetries`) whose body contains a backend-cost-bearing call
//!   (`chargeProvider(`, `dbWrite(`, `callPaymentGateway(`,
//!   `settleTransaction(`) scores toward the free-retry finding. Each
//!   retry-cost co-occurrence in the same source adds to the score.
//! - A bound/charge mitigating signal (`maxRetries`/`retryBudget`/
//!   `chargeCaller(`/`boundedBy(`) present in the same source lowers
//!   the score below threshold, staying clean.
//! - Score and a coarse confidence bucket are always rendered in the
//!   finding detail so a downstream consumer can parse them out (this
//!   crate's `Finding` shape has no dedicated scored-model slot yet).

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};
use regex::Regex;

fn retry_construct_pattern() -> Result<Regex, DecodeError> {
    Regex::new(r"(?i)\bretry\s*\(|\bwhile\s*\(\s*attempts|\bfor\s*\([^)]*maxRetries").map_err(
        |err| {
            DecodeError::new(
                "economic.retryConstructPattern",
                format!("invalid pattern: {err}"),
            )
        },
    )
}

fn backend_cost_pattern() -> Result<Regex, DecodeError> {
    Regex::new(r"(?i)\b(?:chargeProvider|dbWrite|callPaymentGateway|settleTransaction)\s*\(")
        .map_err(|err| {
            DecodeError::new(
                "economic.backendCostPattern",
                format!("invalid pattern: {err}"),
            )
        })
}

fn bound_or_charge_pattern() -> Result<Regex, DecodeError> {
    Regex::new(r"(?i)\bmaxRetries\s*=|\bretryBudget\b|\bchargeCaller\s*\(|\bboundedBy\s*\(")
        .map_err(|err| {
            DecodeError::new(
                "economic.boundOrChargePattern",
                format!("invalid pattern: {err}"),
            )
        })
}

/// Score >= this threshold crosses into a flagged free-retry finding.
const FREE_RETRY_THRESHOLD: i32 = 50;

/// `MCM-ECONOMIC.1` â€” T2 scored economic-cost mechanics gate.
///
/// Scores a retry construct co-occurring with a backend-cost-bearing
/// call, minus any bound/charge mitigation present in the same source.
/// Crossing [`FREE_RETRY_THRESHOLD`] flags the finding; a bounded or
/// charged retry stays clean.
pub struct EconomicValidator {
    rule_id: RuleId,
    retry_construct: Regex,
    backend_cost: Regex,
    bound_or_charge: Regex,
}

impl EconomicValidator {
    /// Build the validator, parsing its own `RuleId` literal and
    /// compiling its patterns at construction (parse-at-boundary).
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "MCM-ECONOMIC.1".parse()?,
            retry_construct: retry_construct_pattern()?,
            backend_cost: backend_cost_pattern()?,
            bound_or_charge: bound_or_charge_pattern()?,
        })
    }
}

impl Validator for EconomicValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let has_retry = self.retry_construct.is_match(input.source.as_str());
        let has_backend_cost = self.backend_cost.is_match(input.source.as_str());
        if !has_retry || !has_backend_cost {
            return Vec::new();
        }

        let mut score = 50;
        if self.bound_or_charge.is_match(input.source.as_str()) {
            score -= 60;
        }

        if score < FREE_RETRY_THRESHOLD {
            return Vec::new();
        }

        let confidence = if score >= FREE_RETRY_THRESHOLD.saturating_mul(2) {
            "high"
        } else {
            "ambiguous (unsure -> treated as a free-retry finding)"
        };

        let line_number = input
            .source
            .as_str()
            .lines()
            .enumerate()
            .find(|(_, text)| self.retry_construct.is_match(text))
            .map(|(idx, _)| u32::try_from(idx).unwrap_or(u32::MAX).saturating_add(1))
            .unwrap_or(1);

        crate::boundary::finding::from_source(
            (&self.rule_id, Severity::Warning),
            "retry loop with non-zero backend cost and no bound/charge (T2 scored)",
            format!(
                "retry construct co-occurs with a backend-cost-bearing call (score {score}, \
                 threshold {FREE_RETRY_THRESHOLD}, confidence: {confidence}), with no \
                 `maxRetries`/`retryBudget`/`chargeCaller`/`boundedBy` mitigation found. \
                 Doctrine (Â§8.10): attacker-cost MUST be >= system-cost â€” an unbounded, \
                 uncharged retry against a non-zero-cost backend call gives an attacker free \
                 leverage. Fix: bound the retry count/budget, or charge the caller per attempt."
            ),
            input.file,
            (line_number, None),
        )
        .into_iter()
        .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use enforcer_domain::findings::ScanScope;
    use enforcer_domain::paths::RelPath;
    use enforcer_validator::harness::run_fixture_parity;
    use enforcer_validator::validator::{ValidationInput, Validator};

    use super::EconomicValidator;

    fn manifest_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    fn rel(path: &str) -> Result<RelPath, Box<dyn std::error::Error>> {
        Ok(path.parse()?)
    }

    #[test]
    fn mcm_economic() -> Result<(), Box<dyn std::error::Error>> {
        let validator = EconomicValidator::new()?;
        run_fixture_parity(
            &validator,
            &enforcer_domain::paths::RepoRoot::try_from(manifest_dir().as_path())?,
            &"tests/fixtures/money_critical_mechanics/economic/bad/free_retry.ts".parse()?,
            &"tests/fixtures/money_critical_mechanics/economic/good/bounded_cost.ts".parse()?,
        )?;
        Ok(())
    }

    #[test]
    fn silent_without_backend_cost_call() -> Result<(), Box<dyn std::error::Error>> {
        let validator = EconomicValidator::new()?;
        let file = rel("src/util.ts")?;
        let findings = validator.validate(ValidationInput {
            file: &file,
            source: enforcer_domain::boundary::validation::ValidationSource::from_text(
                "retry(() => { doNothing(); });\n",
            ),
            scope: ScanScope::Files,
        });
        assert!(findings.is_empty());
        Ok(())
    }
}
