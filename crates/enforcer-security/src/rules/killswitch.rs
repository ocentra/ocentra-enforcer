//! `MCM-KILLSWITCH.1` (T1) — the kill-switch mechanics facet (h06, §8.9
//! of the ingested money-critical/security-testing spec).
//!
//! Doctrine (§8.9): a kill switch (emergency halt / circuit breaker /
//! pause-payments toggle) MUST be halt-all, atomic, authenticated,
//! audited, and replay-safe. An untested kill switch is forbidden — a
//! halt mechanism nobody has exercised is not a safety mechanism, it is
//! an unverified assumption sitting in the money path. Each of these
//! properties is independently checkable from the source text; missing
//! ANY of them is a fail.
//!
//! GENERIC across any value system — never a crypto-only pause notion.
//!
//! Scoped by h01's money-critical classifier (consumed read-only) — a
//! kill switch is definitionally money-critical; this module does not
//! redefine that classification.
//!
//! # Detection shape
//!
//! A line-scan `Validator` over TS/JS backend source declaring a
//! kill-switch primitive (`killSwitch(`, `circuitBreaker(`,
//! `emergencyHalt(`, `pausePayments(`):
//!
//! - Declares itself present via the kill-switch marker.
//! - Must be atomic: a transactional/lock-guarded call
//!   (`withLock(`/`transaction(`/`atomic(`) wraps the halt, or is
//!   flagged non-atomic.
//! - Must be authenticated: an auth/service-token verification call
//!   (`requireAuth(`/`authenticate(`/`verifyServiceToken(`) gates it, or
//!   is flagged unauthenticated.
//! - Must be audited: an audit-log call (`auditLog(`/`audit(`) records
//!   the halt, or is flagged unaudited.
//! - Must be replay-safe: an idempotency-key/nonce check
//!   (`idempotencyKey`/`replayGuard(`) guards it, or is flagged
//!   replay-unsafe.
//! - Must be tested: a co-located test marker comment
//!   (`// kill-switch-tested: <test-name>`) must be present in the same
//!   source — its absence is flagged as "untested kill switch is
//!   forbidden", independent of the mechanical properties above.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};
use regex::Regex;

fn kill_switch_pattern() -> Result<Regex, DecodeError> {
    Regex::new(r"(?i)\b(?:killSwitch|circuitBreaker|emergencyHalt|pausePayments)\b").map_err(
        |err| {
            DecodeError::new(
                "killswitch.killSwitchPattern",
                format!("invalid pattern: {err}"),
            )
        },
    )
}

fn atomic_pattern() -> Result<Regex, DecodeError> {
    Regex::new(r"(?i)\b(?:withLock|transaction|atomic)\s*\(").map_err(|err| {
        DecodeError::new(
            "killswitch.atomicPattern",
            format!("invalid pattern: {err}"),
        )
    })
}

fn authed_pattern() -> Result<Regex, DecodeError> {
    Regex::new(r"(?i)\b(?:requireAuth|authenticate|verifyServiceToken)\s*\(").map_err(|err| {
        DecodeError::new(
            "killswitch.authedPattern",
            format!("invalid pattern: {err}"),
        )
    })
}

fn audited_pattern() -> Result<Regex, DecodeError> {
    Regex::new(r"(?i)\b(?:auditLog|audit)\s*\(").map_err(|err| {
        DecodeError::new(
            "killswitch.auditedPattern",
            format!("invalid pattern: {err}"),
        )
    })
}

fn replay_safe_pattern() -> Result<Regex, DecodeError> {
    Regex::new(r"(?i)\bidempotencyKey\b|\breplayGuard\s*\(").map_err(|err| {
        DecodeError::new(
            "killswitch.replaySafePattern",
            format!("invalid pattern: {err}"),
        )
    })
}

fn tested_pattern() -> Result<Regex, DecodeError> {
    Regex::new(r"(?i)//\s*kill-switch-tested\s*:").map_err(|err| {
        DecodeError::new(
            "killswitch.testedPattern",
            format!("invalid pattern: {err}"),
        )
    })
}

/// `MCM-KILLSWITCH.1` — T1 kill-switch mechanics gate.
///
/// Fires once per missing required property (atomic / authed / audited /
/// replay-safe / tested) on any source declaring a kill-switch
/// primitive. A clean kill switch is halt-all, atomic, authenticated,
/// audited, replay-safe, AND tested (a co-located
/// `// kill-switch-tested: <name>` marker).
pub struct KillSwitchValidator {
    rule_id: RuleId,
    kill_switch: Regex,
    atomic: Regex,
    authed: Regex,
    audited: Regex,
    replay_safe: Regex,
    tested: Regex,
}

impl KillSwitchValidator {
    /// Build the validator, parsing its own `RuleId` literal and
    /// compiling its patterns at construction (parse-at-boundary).
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "MCM-KILLSWITCH.1".parse()?,
            kill_switch: kill_switch_pattern()?,
            atomic: atomic_pattern()?,
            authed: authed_pattern()?,
            audited: audited_pattern()?,
            replay_safe: replay_safe_pattern()?,
            tested: tested_pattern()?,
        })
    }
}

impl Validator for KillSwitchValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        if !self.kill_switch.is_match(input.source) {
            return Vec::new();
        }

        let mut missing = Vec::new();
        if !self.atomic.is_match(input.source) {
            missing.push("atomic (no withLock/transaction/atomic wrapper)");
        }
        if !self.authed.is_match(input.source) {
            missing.push("authenticated (no requireAuth/authenticate/verifyServiceToken)");
        }
        if !self.audited.is_match(input.source) {
            missing.push("audited (no auditLog/audit call)");
        }
        if !self.replay_safe.is_match(input.source) {
            missing.push("replay-safe (no idempotencyKey/replayGuard)");
        }
        if !self.tested.is_match(input.source) {
            missing.push("tested (no `// kill-switch-tested: <name>` marker)");
        }

        if missing.is_empty() {
            return Vec::new();
        }

        let line_number = input
            .source
            .lines()
            .enumerate()
            .find(|(_, text)| self.kill_switch.is_match(text))
            .map(|(idx, _)| (idx as u32).saturating_add(1))
            .unwrap_or(1);

        vec![Finding {
            rule_id: self.rule_id.clone(),
            severity: Severity::Error,
            title: "kill switch is not halt-all/atomic/authed/audited/replay-safe/tested (T1)"
                .to_owned(),
            detail: format!(
                "kill switch is missing: {}. Doctrine (§8.9): a kill switch MUST be halt-all, \
                 atomic, authenticated, audited, and replay-safe; an untested kill switch is \
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

    use super::KillSwitchValidator;

    fn manifest_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    fn rel(path: &str) -> Result<RelPath, Box<dyn std::error::Error>> {
        Ok(path.parse()?)
    }

    #[test]
    fn mcm_killswitch() -> Result<(), Box<dyn std::error::Error>> {
        let validator = KillSwitchValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/money_critical_mechanics/killswitch/bad/killswitch_untested.ts",
            "tests/fixtures/money_critical_mechanics/killswitch/good/killswitch_full.ts",
        )?;
        Ok(())
    }

    #[test]
    fn nonatomic_killswitch_is_flagged() -> Result<(), Box<dyn std::error::Error>> {
        let validator = KillSwitchValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/money_critical_mechanics/killswitch/bad/killswitch_nonatomic.ts",
            "tests/fixtures/money_critical_mechanics/killswitch/good/killswitch_full.ts",
        )?;
        Ok(())
    }

    #[test]
    fn silent_on_source_without_kill_switch() -> Result<(), Box<dyn std::error::Error>> {
        let validator = KillSwitchValidator::new()?;
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
