//! `MCM-BOUNDARY.1` (T1) — the untrusted-internal-boundary mechanics
//! facet (h06, §8.8 of the ingested money-critical/security-testing
//! spec).
//!
//! BOUNDARY-INVARIANT: this module accepts raw source text only through
//! [`ValidationInput`] and emits typed [`Finding`] values; it never treats an
//! internal-network signal or header as an authorization decision.
//! boundaryOwnerNote: the Track H boundary validator owns this source-pattern
//! interpretation and its regular-expression transport boundary.
//!
//! Doctrine (§8.8): internal APIs are HOSTILE. Cloudflare/AWS/gateway
//! topology and "it's on the internal network" give ZERO security —
//! internal headers (`X-Internal`, `X-Internal-Auth`, `X-Forwarded-*`)
//! are attacker-controllable the moment any request can reach the
//! service directly (a misconfigured route, a compromised sibling
//! service, a pod-to-pod path that bypasses the edge). An internal
//! endpoint with no authentication of its own, or one that trusts an
//! internal header as an authorization signal, is a fail — full stop.
//!
//! GENERIC across any value system and any internal topology — never
//! assumes a specific cloud/gateway vendor gives real isolation.
//!
//! Scoped by h01's money-critical classifier (consumed read-only) — this
//! module does not redefine what counts as money-critical; an internal
//! endpoint touching money-critical logic is exactly the shape this
//! facet targets.
//!
//! # Detection shape
//!
//! A line-scan `Validator` over TS/JS backend source:
//!
//! - An internal-route handler (`router.internal(`, `app.internal(`, or
//!   a route path containing `/internal/`) declared with no auth
//!   middleware/guard call (`requireAuth(`, `authenticate(`,
//!   `verifyServiceToken(`) anywhere in the same source is flagged.
//!   Trusting an internal header directly as a boolean truth
//!   (`req.headers['x-internal']`/`req.headers.get('x-internal')`
//!   feeding an `if`/boolean context, without a matching
//!   `verifyServiceToken(`/`authenticate(` call gating it) is flagged
//!   independently.
//! - A clean site: the internal route handler's source also calls an
//!   auth/service-token verification primitive, and any internal header
//!   read is only used after that verification, never as the sole trust
//!   signal.

use enforcer_core::error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};
use regex::Regex;

fn internal_route_pattern() -> Result<Regex, DecodeError> {
    Regex::new(r#"(?i)\b(?:router|app)\.internal\s*\(|['"`]/internal/"#).map_err(|err| {
        DecodeError::new(
            "boundary.internalRoutePattern",
            format!("invalid pattern: {err}"),
        )
    })
}

fn auth_guard_pattern() -> Result<Regex, DecodeError> {
    Regex::new(r"(?i)\b(?:requireAuth|authenticate|verifyServiceToken)\s*\(").map_err(|err| {
        DecodeError::new(
            "boundary.authGuardPattern",
            format!("invalid pattern: {err}"),
        )
    })
}

fn trusted_internal_header_pattern() -> Result<Regex, DecodeError> {
    Regex::new(r#"(?i)headers(?:\.get\(\s*['"`]x-internal|\[\s*['"`]x-internal)"#).map_err(|err| {
        DecodeError::new(
            "boundary.trustedInternalHeaderPattern",
            format!("invalid pattern: {err}"),
        )
    })
}

/// `MCM-BOUNDARY.1` — T1 untrusted-internal-boundary mechanics gate.
///
/// Fires when an internal-route declaration's source has no
/// auth/service-token verification call anywhere in it, OR when an
/// internal header is read and used without a matching verification call
/// present in the same source. A clean internal endpoint authenticates
/// itself and never trusts a bare internal header as the sole signal.
pub struct BoundaryValidator {
    rule_id: RuleId,
    internal_route: Regex,
    auth_guard: Regex,
    trusted_header: Regex,
}

impl BoundaryValidator {
    /// Build the validator, parsing its own `RuleId` literal and
    /// compiling its patterns at construction (parse-at-boundary).
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "MCM-BOUNDARY.1".parse()?,
            internal_route: internal_route_pattern()?,
            auth_guard: auth_guard_pattern()?,
            trusted_header: trusted_internal_header_pattern()?,
        })
    }
}

impl Validator for BoundaryValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let has_auth_guard = self.auth_guard.is_match(input.source);
        let mut findings = Vec::new();

        for (idx, text) in input.source.lines().enumerate() {
            // CAST-JUSTIFICATION: `enumerate` produces a source-line index and
            // `saturating_add` preserves a valid one-based finding location.
            let line_number = (idx as u32).saturating_add(1);

            if self.internal_route.is_match(text) && !has_auth_guard {
                findings.push(Finding {
                    rule_id: self.rule_id.clone(),
                    severity: Severity::Error,
                    title: "unauthenticated internal endpoint (T1)".to_owned(),
                    detail: "an internal-route declaration has no auth/service-token \
                              verification call anywhere in this source. Doctrine (§8.8): \
                              internal APIs are HOSTILE — topology gives ZERO security. An \
                              unauthenticated internal endpoint is a fail, full stop. Fix: gate \
                              this route with `requireAuth(...)`/`authenticate(...)`/ \
                              `verifyServiceToken(...)`."
                        .to_owned(),
                    file: input.file.clone(),
                    line: line_number,
                    snippet: Some(text.trim().to_owned()),
                });
            }

            if self.trusted_header.is_match(text) && !has_auth_guard {
                findings.push(Finding {
                    rule_id: self.rule_id.clone(),
                    severity: Severity::Error,
                    title: "internal header trusted without verification (T1)".to_owned(),
                    detail: "an `x-internal`-shaped header is read with no matching auth/ \
                              service-token verification call in this source. Doctrine (§8.8): \
                              internal headers are attacker-controllable the moment any request \
                              can reach the service directly — never trust one as the sole \
                              authorization signal. Fix: verify the caller \
                              (`verifyServiceToken(...)`/`authenticate(...)`) before honoring any \
                              internal header."
                        .to_owned(),
                    file: input.file.clone(),
                    line: line_number,
                    snippet: Some(text.trim().to_owned()),
                });
            }
        }
        findings
    }
}
