//! `MCM-TIME.1` (T1) — the clock-trust mechanics facet (h06, §8.7 of the
//! ingested money-critical/security-testing spec).
//!
//! Doctrine (§8.7): client wall-clock time is NEVER trusted in a money
//! path. A backend that reads `Date.now()`/`new Date()`/a client-supplied
//! timestamp field straight from the request body to drive an expiry or
//! settlement decision lets an attacker forge time itself — replay a
//! stale request with a spoofed future timestamp, or extend a window
//! indefinitely. The only safe pattern is server-side time
//! (`serverNow()`/`Date.now()` evaluated server-side with no client input
//! feeding it), an explicit skew-tolerance constant, and expiry that
//! FAILS CLOSED (treats an unparseable/missing time as expired, never as
//! valid).
//!
//! GENERIC across any value system — never a crypto-only slot/timestamp
//! notion (the optional crypto instance's slot-timing nuance is h07's
//! localnet-proof concern, not this facet's).
//!
//! Scoped by h01's money-critical classifier (consumed read-only) — this
//! module does not redefine what counts as money-critical.
//!
//! # Detection shape
//!
//! A line-scan `Validator` over TS/JS backend source:
//!
//! - A time-read expression sourced from the client
//!   (`req.body.timestamp`, `request.body.expiresAt`, or a bare
//!   `clientTime`/`clientNow` identifier) feeding an expiry/comparison
//!   context is flagged.
//! - A clean site uses `serverNow(`/a `Date.now()` call with no client
//!   timestamp field in the same source, declares an explicit skew
//!   constant (`skewTolerance`/`SKEW_TOLERANCE_MS`), and its expiry check
//!   fails closed (an `isExpired`/expiry helper whose fallback branch
//!   returns `true`/throws on missing/unparseable input, never `false`).

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};
use regex::Regex;

fn client_time_pattern() -> Result<Regex, DecodeError> {
    Regex::new(
        r"(?i)\b(?:req|request)\.body\.(?:timestamp|expiresAt|expiry|now)\b|\bclient(?:Time|Now)\b",
    )
    .map_err(|err| DecodeError::new("time.clientTimePattern", format!("invalid pattern: {err}")))
}

fn skew_tolerance_pattern() -> Result<Regex, DecodeError> {
    Regex::new(r"(?i)\bskew_?tolerance\b").map_err(|err| {
        DecodeError::new(
            "time.skewTolerancePattern",
            format!("invalid pattern: {err}"),
        )
    })
}

/// A fail-open expiry fallback: an expiry/`isExpired`-shaped helper whose
/// fallback path returns `false` (treats unknown time as NOT expired,
/// i.e. valid) rather than failing closed.
fn fail_open_expiry_pattern() -> Result<Regex, DecodeError> {
    Regex::new(r"(?i)\bexpir\w*\b.*\breturn\s+false\b|\breturn\s+false\b.*//\s*not\s+expired")
        .map_err(|err| {
            DecodeError::new(
                "time.failOpenExpiryPattern",
                format!("invalid pattern: {err}"),
            )
        })
}

/// `MCM-TIME.1` — T1 clock-trust mechanics gate.
///
/// Fires when the scanned source reads a client-supplied timestamp field
/// to drive time logic, or when an expiry helper's fallback is fail-open
/// (defaults to "not expired" rather than failing closed). A clean money
/// path uses server time, an explicit skew constant, and fails closed.
pub struct TimeValidator {
    rule_id: RuleId,
    client_time: Regex,
    skew_tolerance: Regex,
    fail_open_expiry: Regex,
}

impl TimeValidator {
    /// Build the validator, parsing its own `RuleId` literal and
    /// compiling its patterns at construction (parse-at-boundary).
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "MCM-TIME.1".parse()?,
            client_time: client_time_pattern()?,
            skew_tolerance: skew_tolerance_pattern()?,
            fail_open_expiry: fail_open_expiry_pattern()?,
        })
    }
}

impl Validator for TimeValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let mut findings = Vec::new();
        let has_skew_tolerance = self.skew_tolerance.is_match(input.source.as_str());

        for (idx, text) in input.source.as_str().lines().enumerate() {
            let line_number = u32::try_from(idx).unwrap_or(u32::MAX).saturating_add(1);

            if self.client_time.is_match(text) {
                findings.extend(canonical_finding! {
                    rule_id: self.rule_id.clone(),
                    severity: Severity::Error,
                    title: "money path trusts client-supplied clock time (T1)".to_owned(),
                    detail: "client wall-clock time is read to drive a money-path time decision. \
                              Doctrine (§8.7): client time is NEVER trusted in a money path — an \
                              attacker can forge time (replay with a spoofed timestamp, extend a \
                              window indefinitely). Fix: use server-side time only \
                              (`serverNow()`/server-evaluated `Date.now()`), with an explicit skew \
                              constant and fail-closed expiry."
                        .to_owned(),
                    file: input.file.clone(),
                    line: line_number,
                    snippet: Some(text.trim().to_owned()),
                });
                continue;
            }

            if self.fail_open_expiry.is_match(text) {
                findings.extend(canonical_finding! {
                    rule_id: self.rule_id.clone(),
                    severity: Severity::Error,
                    title: "expiry check fails open (T1)".to_owned(),
                    detail: "an expiry/isExpired-shaped check's fallback treats unknown/missing \
                              time as NOT expired. Doctrine (§8.7): expiry MUST fail closed \
                              (treat unparseable/missing time as expired), never fail open. Fix: \
                              invert the fallback so missing/unparseable time is treated as \
                              expired."
                        .to_owned(),
                    file: input.file.clone(),
                    line: line_number,
                    snippet: Some(text.trim().to_owned()),
                });
            }
        }

        if !findings.is_empty() && !has_skew_tolerance {
            // Missing skew tolerance compounds an already-flagged clock
            // issue; note it on the first finding rather than emitting a
            // whole-file duplicate finding per line.
            if let Some(first) = findings.first_mut() {
                let detail = format!(
                    "{} Additionally, no explicit skew-tolerance constant \
                     (`skewTolerance`/`SKEW_TOLERANCE_MS`) was found in this source.",
                    first.detail.as_str()
                );
                if let Ok(detail) = enforcer_domain::findings::FindingDetail::new(detail) {
                    first.detail = detail;
                }
            }
        }

        findings
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use enforcer_domain::findings::ScanScope;
    use enforcer_domain::paths::RelPath;
    use enforcer_validator::harness::run_fixture_parity;
    use enforcer_validator::validator::{ValidationInput, Validator};

    use super::TimeValidator;

    fn manifest_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    fn rel(path: &str) -> Result<RelPath, Box<dyn std::error::Error>> {
        Ok(path.parse()?)
    }

    #[test]
    fn mcm_time() -> Result<(), Box<dyn std::error::Error>> {
        let validator = TimeValidator::new()?;
        run_fixture_parity(
            &validator,
            &enforcer_domain::paths::RepoRoot::try_from(manifest_dir().as_path())?,
            &"tests/fixtures/money_critical_mechanics/time/bad/client_clock.ts".parse()?,
            &"tests/fixtures/money_critical_mechanics/time/good/server_clock.ts".parse()?,
        )?;
        Ok(())
    }

    #[test]
    fn fires_on_client_now_identifier() -> Result<(), Box<dyn std::error::Error>> {
        let validator = TimeValidator::new()?;
        let file = rel("src/pay.ts")?;
        let findings = validator.validate(ValidationInput {
            file: &file,
            source: enforcer_domain::boundary::validation::ValidationSource::from_text(
                "const expiry = clientNow + windowMs;\n",
            ),
            scope: ScanScope::Files,
        });
        assert_eq!(findings.len(), 1);
        Ok(())
    }

    #[test]
    fn silent_on_non_time_source() -> Result<(), Box<dyn std::error::Error>> {
        let validator = TimeValidator::new()?;
        let file = rel("src/util.ts")?;
        let findings = validator.validate(ValidationInput {
            file: &file,
            source: enforcer_domain::boundary::validation::ValidationSource::from_text(
                "function add(a: number, b: number) { return a + b; }\n",
            ),
            scope: ScanScope::Files,
        });
        assert!(findings.is_empty());
        Ok(())
    }
}
