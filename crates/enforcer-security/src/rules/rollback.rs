//! `MCM-ROLLBACK.1` (T2) — the rollback/compensation mechanics facet
//! (h06, §8.10 of the ingested money-critical/security-testing spec).
//!
//! Doctrine (§8.10): a rollback/compensation path MUST be idempotent,
//! replay-safe, atomic, and exactly-once. A rollback that re-applies its
//! reversal on every retry (no idempotency key, no "already rolled back"
//! guard) can double-refund, double-release, or otherwise duplicate the
//! compensating effect — the exact double-spend shape rollback exists to
//! prevent, turned back on itself. An untested rollback is forbidden,
//! same as an untested kill switch.
//!
//! This is a T2 SCORED heuristic (mirrors [`super::economic`]'s
//! scored-shape) — score + confidence are carried in the finding detail.
//!
//! GENERIC across any value system — never a crypto-only rollback
//! notion.
//!
//! Scoped by h01's money-critical classifier (consumed read-only).
//!
//! # Detection shape
//!
//! A line-scan `Validator` over TS/JS backend source declaring a
//! rollback/compensation primitive (`rollback(`, `compensate(`,
//! `reverseTransaction(`):
//!
//! - Non-idempotent: no idempotency-key check / "already rolled back"
//!   guard (`idempotencyKey`/`alreadyRolledBack`/`isCompensated`) found
//!   in the same source — scores toward the finding.
//! - Non-atomic: no transactional wrapper (`withLock(`/`transaction(`/
//!   `atomic(`) found — scores toward the finding.
//! - Untested: no co-located test marker
//!   (`// rollback-tested: <test-name>`) — scores toward the finding,
//!   independent of the mechanical properties (mirrors the kill-switch
//!   facet's "untested X is forbidden" rule).
//! - Crossing the threshold flags the rollback; a fully idempotent,
//!   atomic, tested rollback stays clean.

use enforcer_core::error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};
use regex::Regex;

fn rollback_pattern() -> Result<Regex, DecodeError> {
    Regex::new(r"(?i)\b(?:rollback|compensate|reverseTransaction)\s*\(")
        .map_err(|err| DecodeError::new("rollback.rollbackPattern", format!("invalid pattern: {err}")))
}

fn idempotent_guard_pattern() -> Result<Regex, DecodeError> {
    Regex::new(r"(?i)\bidempotencyKey\b|\balreadyRolledBack\b|\bisCompensated\b")
        .map_err(|err| DecodeError::new("rollback.idempotentGuardPattern", format!("invalid pattern: {err}")))
}

fn atomic_pattern() -> Result<Regex, DecodeError> {
    Regex::new(r"(?i)\b(?:withLock|transaction|atomic)\s*\(")
        .map_err(|err| DecodeError::new("rollback.atomicPattern", format!("invalid pattern: {err}")))
}

fn tested_pattern() -> Result<Regex, DecodeError> {
    Regex::new(r"(?i)//\s*rollback-tested\s*:")
        .map_err(|err| DecodeError::new("rollback.testedPattern", format!("invalid pattern: {err}")))
}

/// Score >= this threshold crosses into a flagged non-idempotent-rollback
/// finding.
const ROLLBACK_THRESHOLD: i32 = 40;

/// `MCM-ROLLBACK.1` — T2 scored rollback/compensation mechanics gate.
///
/// Scores a rollback/compensation declaration against missing
/// idempotency guard, missing atomic wrapper, and missing test marker.
/// Crossing [`ROLLBACK_THRESHOLD`] flags the finding; an idempotent,
/// atomic, tested rollback stays clean.
pub struct RollbackValidator {
    rule_id: RuleId,
    rollback: Regex,
    idempotent_guard: Regex,
    atomic: Regex,
    tested: Regex,
}

impl RollbackValidator {
    /// Build the validator, parsing its own `RuleId` literal and
    /// compiling its patterns at construction (parse-at-boundary).
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "MCM-ROLLBACK.1".parse()?,
            rollback: rollback_pattern()?,
            idempotent_guard: idempotent_guard_pattern()?,
            atomic: atomic_pattern()?,
            tested: tested_pattern()?,
        })
    }
}

impl Validator for RollbackValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        if !self.rollback.is_match(input.source) {
            return Vec::new();
        }

        let mut score = 0i32;
        let mut missing = Vec::new();
        if !self.idempotent_guard.is_match(input.source) {
            score += 40;
            missing.push("idempotent/replay-safe guard (no idempotencyKey/alreadyRolledBack/isCompensated)");
        }
        if !self.atomic.is_match(input.source) {
            score += 30;
            missing.push("atomic (no withLock/transaction/atomic wrapper)");
        }
        if !self.tested.is_match(input.source) {
            score += 30;
            missing.push("tested (no `// rollback-tested: <name>` marker)");
        }

        if score < ROLLBACK_THRESHOLD {
            return Vec::new();
        }

        let confidence = if score >= ROLLBACK_THRESHOLD.saturating_mul(2) {
            "high"
        } else {
            "ambiguous (unsure -> treated as non-idempotent rollback)"
        };

        let line_number = input
            .source
            .lines()
            .enumerate()
            .find(|(_, text)| self.rollback.is_match(text))
            .map(|(idx, _)| (idx as u32).saturating_add(1))
            .unwrap_or(1);

        vec![Finding {
            rule_id: self.rule_id.clone(),
            severity: Severity::Warning,
            title: "rollback/compensation is not idempotent/atomic/tested (T2 scored)".to_owned(),
            detail: format!(
                "rollback is missing: {} (score {score}, threshold {ROLLBACK_THRESHOLD}, \
                 confidence: {confidence}). Doctrine (§8.10): a rollback/compensation path MUST \
                 be idempotent, replay-safe, atomic, and exactly-once; an untested rollback is \
                 forbidden. Fix: add the missing property/properties above.",
                missing.join(", ")
            ),
            file: input.file.clone(),
            line: line_number,
            snippet: None,
        }]
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use enforcer_domain::findings::ScanScope;
    use enforcer_domain::paths::RelPath;
    use enforcer_validator::harness::run_fixture_parity;
    use enforcer_validator::validator::{ValidationInput, Validator};

    use super::RollbackValidator;

    fn manifest_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    fn rel(path: &str) -> Result<RelPath, Box<dyn std::error::Error>> {
        Ok(path.parse()?)
    }

    #[test]
    fn mcm_rollback() -> Result<(), Box<dyn std::error::Error>> {
        let validator = RollbackValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/money_critical_mechanics/rollback/bad/rollback_nonidempotent.ts",
            "tests/fixtures/money_critical_mechanics/rollback/good/rollback_exactly_once.ts",
        )?;
        Ok(())
    }

    #[test]
    fn silent_on_source_without_rollback() -> Result<(), Box<dyn std::error::Error>> {
        let validator = RollbackValidator::new()?;
        let file = rel("src/util.ts")?;
        let findings = validator.validate(ValidationInput {
            file: &file,
            source: "function add(a: number, b: number) { return a + b; }\n",
            scope: ScanScope::Files,
        });
        assert!(findings.is_empty());
        Ok(())
    }
}
