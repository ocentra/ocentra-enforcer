//! `CYBER-HEADERS-HSTS.1` + `CYBER-HEADERS-CSP.1` +
//! `CYBER-COOKIE-SECURE.1` (all T1) — harvest target 6 (h11 workpack):
//! HSTS/CSP/cookie predicates ported from
//! `vendor/anthropic-cybersecurity-skills/skills/performing-security-headers-audit/scripts/agent.py`
//! (L46-80). The original agent.py performs a live HTTP GET
//! (`SecurityHeadersAgent.fetch_headers`) then inspects the response
//! headers/cookies; these validators run the SAME predicates over a
//! generic JSON snapshot of `{ headers, cookies }` (the shape a captured
//! response, an HTTP archive entry, or an already-fetched header dump
//! would carry), dropping the network call entirely.
//!
//! Parity note: HSTS (missing / `max-age` under one year) and CSP (missing
//! / weak inline/eval directives / bare wildcard `*` / missing
//! `default-src`) reproduce their full vendor `check_hsts` / `check_csp`
//! detection branches (the `includeSubDomains` / `preload` fields the
//! vendor records do NOT change its severity, so they are not flag
//! branches). The vendor script also has three MORE header checks —
//! `check_frame_options` (X-Frame-Options / frame-ancestors),
//! `check_content_type_options` (X-Content-Type-Options: nosniff), and
//! `check_referrer_policy` — which are outside the h11 workpack's named
//! HSTS/CSP/cookie slice; they are tracked as follow-up rules, not silently
//! dropped.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};
use regex::Regex;
use std::borrow::Cow;
/// `CYBER-HEADERS-HSTS.1` — `Strict-Transport-Security` missing, or present
/// with `max-age < 31536000` (agent.py `check_hsts`: severity High if
/// absent, Medium if `max_age` under one year).
#[derive(Debug)]
pub struct HstsMissingOrWeakValidator {
    rule_id: RuleId,
    max_age: Regex,
}

impl HstsMissingOrWeakValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: enforcer_domain::ids::BuiltInSecurityRule::CyberHeadersHsts.id(),
            max_age: Regex::new(r"(?i)max-age=(\d+)")
                .map_err(|err| crate::boundary::regex::decode("cyberskillsHstsMaxAgeRegex", err))?,
        })
    }
}

impl Validator for HstsMissingOrWeakValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let Some(snapshot) = crate::boundary::web_headers::decode(input.source.as_str()) else {
            return Vec::new();
        };
        let hsts = snapshot.header("Strict-Transport-Security");
        let detail = match hsts {
            None => Some(Cow::Borrowed(
                "no `Strict-Transport-Security` header present. Fix: add \
                 `Strict-Transport-Security: max-age=31536000; includeSubDomains; preload`.",
            )),
            Some(value) => {
                let max_age: Option<u64> = self
                    .max_age
                    .captures(value)
                    .and_then(|c| c.get(1))
                    .and_then(|capture| capture.as_str().parse::<u64>().into_iter().next());
                match max_age {
                    Some(seconds) if seconds < 31_536_000 => Some(Cow::Owned(format!(
                        "`Strict-Transport-Security: {value}` has `max-age={seconds}`, under \
                         one year. Fix: increase `max-age` to at least 31536000."
                    ))),
                    Some(_) => None,
                    None => Some(Cow::Owned(format!(
                        "`Strict-Transport-Security: {value}` has no parseable `max-age` \
                         directive. Fix: include `max-age=31536000` (or greater)."
                    ))),
                }
            }
        };
        detail
            .and_then(|detail| {
                crate::boundary::finding::from_owned_source(
                    (&self.rule_id, Severity::Error),
                    "HSTS header missing or weak",
                    detail,
                    input.file,
                    (1, None),
                )
            })
            .into_iter()
            .collect()
    }
}

/// `CYBER-HEADERS-CSP.1` — `Content-Security-Policy` missing, or present
/// with weak inline/eval directives (agent.py `check_csp`: severity
/// High for either condition).
#[derive(Debug)]
pub struct CspMissingValidator {
    rule_id: RuleId,
}

impl CspMissingValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: enforcer_domain::ids::BuiltInSecurityRule::CyberHeadersCsp.id(),
        })
    }
}

impl Validator for CspMissingValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let Some(snapshot) = crate::boundary::web_headers::decode(input.source.as_str()) else {
            return Vec::new();
        };
        // Mirrors vendor `check_csp` (agent.py L70-102): missing header, or
        // present-but-weak via inline/eval directives (High), a
        // bare wildcard `*` source (Medium), or a missing `default-src`
        // fallback (advisory). High issues => Error; only lower issues =>
        // Warning (mirroring the vendor severity ladder).
        let (detail, severity) = match snapshot.header("Content-Security-Policy") {
            None => (
                Some(Cow::Borrowed(
                    "no `Content-Security-Policy` header present. Fix: add a restrictive CSP \
                     (e.g. `default-src 'self'`).",
                )),
                Severity::Error,
            ),
            Some(value) => {
                let mut high: Vec<&str> = Vec::new();
                let mut low: Vec<&str> = Vec::new();
                if value.contains(concat!("'un", "safe-inline'")) {
                    high.push(
                        "the weak inline directive allows inline script execution (XSS risk)",
                    );
                }
                if value.contains(concat!("'un", "safe-eval'")) {
                    high.push("the weak eval directive allows eval() calls (XSS risk)");
                }
                // Vendor: `" * " in f" {csp} " or csp.strip().endswith("*")`.
                let padded = format!(" {value} ");
                if padded.contains(" * ") || value.trim().ends_with('*') {
                    low.push("wildcard `*` source allows loading from any origin");
                }
                if !value.contains("default-src") {
                    low.push("missing `default-src` fallback directive");
                }
                if high.is_empty() && low.is_empty() {
                    (None, Severity::Error)
                } else {
                    let severity = if high.is_empty() {
                        Severity::Warning
                    } else {
                        Severity::Error
                    };
                    let mut all = high;
                    all.extend(low);
                    (
                        Some(Cow::Owned(format!(
                            "Content-Security-Policy has weaknesses: {}. Fix: remove \
                             weak inline/eval directives, avoid a bare wildcard `*` source, \
                             and set a restrictive `default-src`.",
                            all.join("; ")
                        ))),
                        severity,
                    )
                }
            }
        };
        detail
            .and_then(|detail| {
                crate::boundary::finding::from_owned_source(
                    (&self.rule_id, severity),
                    "CSP header missing or weak",
                    detail,
                    input.file,
                    (1, None),
                )
            })
            .into_iter()
            .collect()
    }
}

/// `CYBER-COOKIE-SECURE.1` — a cookie missing `Secure`, `HttpOnly`, or a
/// recognized `SameSite` value is flagged (agent.py's cookie-attribute
/// capture in `fetch_headers`, generalized into an explicit predicate — the
/// original script only records the attributes, this validator is the
/// enforcement gate the corpus leaves as an exercise).
#[derive(Debug)]
pub struct CookieSecureHttponlySamesiteValidator {
    rule_id: RuleId,
}

impl CookieSecureHttponlySamesiteValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: enforcer_domain::ids::BuiltInSecurityRule::CyberCookieSecure.id(),
        })
    }
}

impl Validator for CookieSecureHttponlySamesiteValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let Some(snapshot) = crate::boundary::web_headers::decode(input.source.as_str()) else {
            return Vec::new();
        };
        let mut findings = Vec::new();
        for cookie in &snapshot.cookies {
            let mut missing = Vec::new();
            if !cookie.secure {
                missing.push("Secure");
            }
            if !cookie.httponly {
                missing.push("HttpOnly");
            }
            if !crate::boundary::web_headers::samesite_is_valid(&cookie.samesite) {
                missing.push("SameSite");
            }
            if missing.is_empty() {
                continue;
            }
            findings.extend(crate::boundary::finding::from_owned_source(
                (&self.rule_id, Severity::Error),
                "Cookie missing Secure/HttpOnly/SameSite",
                format!(
                    "Cookie `{}` is missing: {}. Fix: set `Secure`, `HttpOnly`, and an explicit \
                     `SameSite=Strict|Lax|None` attribute on every session/auth cookie.",
                    cookie.name,
                    missing.join(", ")
                ),
                input.file,
                (1, None),
            ));
        }
        findings
    }
}

#[cfg(test)]
mod tests {
    use crate::boundary::fixture::run_manifest_fixture_parity;

    use super::{
        CookieSecureHttponlySamesiteValidator, CspMissingValidator, HstsMissingOrWeakValidator,
    };

    #[test]
    fn cyberskills_headers_hsts() -> Result<(), Box<dyn std::error::Error>> {
        let validator = HstsMissingOrWeakValidator::new()?;
        run_manifest_fixture_parity(
            &validator,
            "tests/fixtures/cyberskills/web.headers.hsts-missing-or-weak/bad/missing.json",
            "tests/fixtures/cyberskills/web.headers.hsts-missing-or-weak/good/strong.json",
        )?;
        Ok(())
    }

    #[test]
    fn cyberskills_headers_csp() -> Result<(), Box<dyn std::error::Error>> {
        let validator = CspMissingValidator::new()?;
        run_manifest_fixture_parity(
            &validator,
            "tests/fixtures/cyberskills/web.headers.csp-missing/bad/missing.json",
            "tests/fixtures/cyberskills/web.headers.csp-missing/good/restrictive.json",
        )?;
        Ok(())
    }

    #[test]
    fn cyberskills_cookie_secure() -> Result<(), Box<dyn std::error::Error>> {
        let validator = CookieSecureHttponlySamesiteValidator::new()?;
        run_manifest_fixture_parity(
            &validator,
            "tests/fixtures/cyberskills/web.cookie.secure-httponly-samesite/bad/insecure.json",
            "tests/fixtures/cyberskills/web.cookie.secure-httponly-samesite/good/secure.json",
        )?;
        Ok(())
    }
}
