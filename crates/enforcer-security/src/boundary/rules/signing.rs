//! `MCM-SIGNING.1` (T1) — the backend-signing mechanics facet (h06, §8.6
//! of the ingested money-critical/security-testing spec).
//!
//! Doctrine (§8.6): a backend MUST NOT sign or authorize a client-raw,
//! non-reconstructable, or unverified payload. Signing the caller's own
//! unmodified request body binds the server's trust to whatever the
//! client chose to send — an attacker can request a signature over any
//! payload shape they like. The only safe pattern is: canonically
//! serialize a payload the server itself reconstructed from trusted
//! request context (never the raw client body verbatim), and log a
//! correlation id at the sign site so the signing decision is
//! forensically traceable.
//!
//! GENERIC across any value system (fiat, Stripe, an internal ledger, or
//! the optional crypto/Anchor instance, per e-pack-crypto-blockchain,
//! which composes with this facet read-only) — never a crypto-only
//! marker set.
//!
//! Scoped by h01's money-critical classifier (consumed read-only via the
//! money-critical manifest/annotation this crate's [`super::money_critical`]
//! module maintains) — this module does not itself redefine what counts
//! as money-critical; it only mechanizes the SIGNING facet's shape over
//! source text.
//!
//! # Detection shape
//!
//! A line-scan `Validator` (mirrors [`super::no_bypass`]'s text-level
//! approach — target-language code, here TS/JS backend sign sites, is
//! scanned lexically rather than through a full frontend parse, since the
//! violation shape is a call-site pattern, not a structural AST property):
//!
//! - A call to a signing/authorization primitive (`sign(`, `.sign(`,
//!   `authorize(`, `signPayment(`) whose argument expression is the raw
//!   client request body identifier (`req.body`, `request.body`,
//!   `ctx.request.body`, or a bare `body`/`payload` identifier that was
//!   never reassigned through a `reconstruct`/`canonicalize`/`rebuild`
//!   call first) is flagged.
//! - A clean sign site: the signed argument is a canonically-serialized,
//!   server-reconstructed value (produced by a `canonicalize(`/
//!   `reconstructPayload(`/`buildSignable(`-shaped call), AND the same
//!   scanned source carries a correlation-id log call
//!   (`log*(...correlationId...)` / `logger.*(...correlationId...)`) at
//!   or near the sign site.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};
use regex::Regex;

/// A call to a signing/authorization primitive: `sign(`, `.sign(`,
/// `authorize(`, `signPayment(`, `authorizePayment(`.
fn sign_call_pattern() -> Result<Regex, DecodeError> {
    Regex::new(r"(?i)\b(?:sign|authorize|signPayment|authorizePayment)\s*\(").map_err(|err| {
        DecodeError::new("signing.signCallPattern", format!("invalid pattern: {err}"))
    })
}

/// The raw client request body reaching a sign call directly:
/// `req.body`, `request.body`, `ctx.request.body`, or a bare
/// `body`/`payload` identifier used as the call argument.
fn client_raw_pattern() -> Result<Regex, DecodeError> {
    Regex::new(r"(?i)\b(?:req|request|ctx\.request)\.body\b|\b(?:body|payload)\s*\)").map_err(
        |err| {
            DecodeError::new(
                "signing.clientRawPattern",
                format!("invalid pattern: {err}"),
            )
        },
    )
}

/// A canonical, server-reconstructed payload builder:
/// `canonicalize(`, `reconstructPayload(`, `buildSignable(`.
fn reconstructed_pattern() -> Result<Regex, DecodeError> {
    Regex::new(r"(?i)\b(?:canonicalize|reconstructPayload|buildSignable|rebuildPayload)\s*\(")
        .map_err(|err| {
            DecodeError::new(
                "signing.reconstructedPattern",
                format!("invalid pattern: {err}"),
            )
        })
}

/// A correlation-id log call anywhere in the scanned source.
fn correlation_log_pattern() -> Result<Regex, DecodeError> {
    Regex::new(r"(?i)\b(?:log|logger)\w*(?:\.\w+)?\s*\([^)]*correlationId").map_err(|err| {
        DecodeError::new(
            "signing.correlationLogPattern",
            format!("invalid pattern: {err}"),
        )
    })
}

/// `MCM-SIGNING.1` — T1 backend-signing mechanics gate.
///
/// Fires when a sign/authorize call site's argument expression reads
/// straight from the raw client request body (or a `body`/`payload`
/// identifier never passed through a reconstruction call), OR when the
/// scanned source has no correlation-id log call backing any sign site.
/// A clean sign site canonicalizes a server-reconstructed payload and
/// logs a correlation id.
pub struct SigningValidator {
    rule_id: RuleId,
    sign_call: Regex,
    client_raw: Regex,
    reconstructed: Regex,
    correlation_log: Regex,
}

impl SigningValidator {
    /// Build the validator, parsing its own `RuleId` literal and
    /// compiling its patterns at construction (parse-at-boundary).
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "MCM-SIGNING.1".parse()?,
            sign_call: sign_call_pattern()?,
            client_raw: client_raw_pattern()?,
            reconstructed: reconstructed_pattern()?,
            correlation_log: correlation_log_pattern()?,
        })
    }
}

impl Validator for SigningValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let mut findings = Vec::new();
        let has_correlation_log = self.correlation_log.is_match(input.source.as_str());

        for (idx, text) in input.source.as_str().lines().enumerate() {
            let line_number = u32::try_from(idx).unwrap_or(u32::MAX).saturating_add(1);
            if !self.sign_call.is_match(text) {
                continue;
            }

            let signs_client_raw =
                self.client_raw.is_match(text) && !self.reconstructed.is_match(text);
            let missing_correlation = !has_correlation_log;

            if !signs_client_raw && !missing_correlation {
                continue;
            }

            // The `!signs_client_raw && !missing_correlation` guard above
            // already excludes the `(false, false)` case, so only the
            // three violation shapes below are ever rendered.
            let reason = if signs_client_raw && missing_correlation {
                "signs a client-raw/non-reconstructed payload AND has no correlation-id log at \
                 the sign site"
            } else if signs_client_raw {
                "signs a client-raw/non-reconstructed payload directly"
            } else {
                "has no correlation-id log backing this sign site"
            };

            findings.extend(canonical_finding! {
                rule_id: self.rule_id.clone(),
                severity: Severity::Error,
                title: "backend signs a client-raw or unlogged payload (T1)".to_owned(),
                detail: format!(
                    "sign/authorize call {reason}. Doctrine (§8.6): a backend MUST NOT sign a \
                     client-raw, non-reconstructable, or unverified payload — binding trust to \
                     whatever the client chose to send lets an attacker request a signature over \
                     any payload shape. Fix: canonically serialize a payload the server itself \
                     reconstructed from trusted request context (`canonicalize(...)` / \
                     `reconstructPayload(...)`), and log a correlation id at the sign site.",
                ),
                file: input.file.clone(),
                line: line_number,
                snippet: Some(text.trim().to_owned()),
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
    use enforcer_validator::harness::run_fixture_parity;
    use enforcer_validator::validator::{ValidationInput, Validator};

    use super::SigningValidator;

    fn manifest_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    fn rel(path: &str) -> Result<RelPath, Box<dyn std::error::Error>> {
        Ok(path.parse()?)
    }

    #[test]
    fn mcm_signing() -> Result<(), Box<dyn std::error::Error>> {
        let validator = SigningValidator::new()?;
        run_fixture_parity(
            &validator,
            &enforcer_domain::paths::RepoRoot::try_from(manifest_dir().as_path())?,
            &"tests/fixtures/money_critical_mechanics/signing/bad/sign_client_raw.ts".parse()?,
            &"tests/fixtures/money_critical_mechanics/signing/good/sign_reconstructed.ts"
                .parse()?,
        )?;
        Ok(())
    }

    #[test]
    fn fires_on_bare_body_identifier_sign_call() -> Result<(), Box<dyn std::error::Error>> {
        let validator = SigningValidator::new()?;
        let file = rel("src/pay.ts")?;
        let findings = validator.validate(ValidationInput {
            file: &file,
            source: enforcer_domain::boundary::validation::ValidationSource::from_text(
                "logger.info('sign', { correlationId });\nconst sig = sign(body);\n",
            ),
            scope: ScanScope::Files,
        });
        assert_eq!(findings.len(), 1);
        Ok(())
    }

    #[test]
    fn silent_on_non_signing_source() -> Result<(), Box<dyn std::error::Error>> {
        let validator = SigningValidator::new()?;
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
