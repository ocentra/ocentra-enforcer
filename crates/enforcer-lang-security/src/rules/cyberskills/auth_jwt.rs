//! `CYBER-AUTH-JWT.1` (T1) — harvested from the vendored cyberskills JWT
//! trio: `testing-jwt-token-security`, `performing-jwt-none-algorithm-attack`,
//! and `exploiting-jwt-algorithm-confusion-attack`
//! (`vendor/anthropic-cybersecurity-skills/skills/...`). All three vendor
//! skills are live-target attack agents (`scripts/agent.py` intercepts a
//! captured JWT, forges a new one, and replays it against
//! `--target-url`/`--base-url` over HTTP with `requests`) — there is no
//! inline source-scanning predicate to port verbatim. This validator instead
//! implements the well-known DETERMINISTIC checks the three skills exist to
//! probe for, applied directly to application source instead of a live
//! token/endpoint:
//!
//! 1. **Algorithm-none acceptance** (`performing-jwt-none-algorithm-attack`,
//!    `testing-jwt-token-security` Step 2 "Test Algorithm None Attack"): a
//!    verify/decode call (or header literal) configured with
//!    `algorithms: ["none"]` / `algorithm: "none"` / `alg: "none"`
//!    (case-insensitive on the value) means the JWT library will accept an
//!    attacker-forged unsigned token outright — Critical (`Severity::Error`).
//! 2. Unverified `jwt.decode()` calls (no verify option present) are
//!    deliberately SKIPPED: absence of a verify flag is not reliably
//!    distinguishable from "verification happens elsewhere in the call
//!    chain" via per-line regex, so flagging it here would be a
//!    false-positive magnet.
//! 3. **Hardcoded short HMAC/JWT secret** (`testing-jwt-token-security`
//!    Step 4 "Brute-Force HMAC Secret" / Scenario 2 "Weak HMAC Secret",
//!    `exploiting-jwt-algorithm-confusion-attack`'s RS256->HS256 confusion
//!    both rely on the signing secret being guessable or the public key
//!    being reachable): a string literal shorter than 16 characters passed
//!    as the secret/key argument to `jwt.sign(...)` / `jwt.verify(...)` is
//!    both a weak-entropy secret AND, being a literal, a leaked one the
//!    moment the repo is cloned — High (`Severity::Error`).

use enforcer_core::error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};
use regex::Regex;

/// `CYBER-AUTH-JWT.1` — JWT `alg:"none"` acceptance and hardcoded
/// short signing-secret detector.
pub struct JwtSecurityValidator {
    rule_id: RuleId,
    none_algorithm: Regex,
    short_hardcoded_secret: Regex,
}

impl JwtSecurityValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "CYBER-AUTH-JWT.1".parse()?,
            // Matches `algorithms: ["none"]`, `algorithm: "none"`, and the
            // bare header field `alg: "none"` (including the JSON-quoted-key
            // spelling `"alg": "none"`), any quote style, optional array
            // brackets, `:` or `=` assignment, case-insensitive on both the
            // key spelling and the "none" value (vendor tries
            // "none"/"None"/"NONE"/"nOnE" variants in `test_alg_none`).
            none_algorithm: Regex::new(
                r#"(?i)\b(?:algorithms?|alg)\b['"]?\s*[:=]\s*\[?\s*['"]none['"]\s*\]?"#,
            )
            .map_err(|err| DecodeError::new("cyberskillsJwtNoneAlgorithm", err.to_string()))?,
            // Matches `jwt.sign(payload, "secret")` / `jwt.verify(token,
            // 'key')` (and the `jsonwebtoken.` alias) where the secret
            // argument is a quoted literal of 1-15 characters (i.e. under
            // 16 — the vendor's "minimum key length of 256 bits" advice
            // implies a bare short literal is already too weak). The first
            // argument is restricted to a simple identifier/property-access
            // shape so an object-literal payload containing its own comma
            // does not desynchronize which argument is "the secret".
            short_hardcoded_secret: Regex::new(
                r#"(?i)\b(?:jwt|jsonwebtoken)\.(?:sign|verify)\s*\(\s*[\w.\[\]$]+\s*,\s*['"]([^'"]{1,15})['"]"#,
            )
            .map_err(|err| DecodeError::new("cyberskillsJwtShortSecret", err.to_string()))?,
        })
    }
}

impl Validator for JwtSecurityValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let mut findings = Vec::new();
        for (index, line) in input.source.lines().enumerate() {
            let line_number = u32::try_from(index).unwrap_or(u32::MAX).saturating_add(1);

            if self.none_algorithm.is_match(line) {
                findings.push(Finding {
                    rule_id: self.rule_id.clone(),
                    severity: Severity::Error,
                    title: "JWT verification accepts the \"none\" algorithm".to_owned(),
                    detail: "a JWT verify/decode call (or header literal) allows \
                             `alg`/`algorithm(s)` = \"none\", which lets an attacker strip the \
                             signature entirely and forge arbitrary claims (CVE class: JWT \
                             algorithm-none bypass). Fix: enforce an explicit allowlist of \
                             expected signing algorithms (e.g. `algorithms: [\"HS256\"]`) and \
                             never include \"none\"."
                        .to_owned(),
                    file: input.file.clone(),
                    line: line_number,
                    snippet: Some(line.to_owned()),
                });
            }

            if let Some(captures) = self.short_hardcoded_secret.captures(line) {
                let secret_len = captures
                    .get(1)
                    .map(|m| m.as_str().chars().count())
                    .unwrap_or(0);
                findings.push(Finding {
                    rule_id: self.rule_id.clone(),
                    severity: Severity::Error,
                    title: "hardcoded short JWT signing secret".to_owned(),
                    detail: format!(
                        "a {secret_len}-character string literal is passed as the HMAC/JWT \
                         signing secret. A short, hardcoded secret is both brute-forceable \
                         offline (hashcat/john, mode 16500) and permanently leaked to every \
                         clone/fork/CI log. Fix: use a high-entropy secret (32+ bytes) loaded \
                         at runtime from an environment variable or secret store, never a \
                         literal in source."
                    ),
                    file: input.file.clone(),
                    line: line_number,
                    snippet: None,
                });
            }
        }
        findings
    }
}

#[cfg(test)]
mod tests {
    use enforcer_validator::harness::run_fixture_parity;

    use super::JwtSecurityValidator;

    #[test]
    fn cyberskills_auth_jwt() -> Result<(), Box<dyn std::error::Error>> {
        let validator = JwtSecurityValidator::new()?;
        run_fixture_parity(
            &validator,
            &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")),
            "tests/fixtures/cyberskills/auth.jwt-insecure/bad/none_alg.js",
            "tests/fixtures/cyberskills/auth.jwt-insecure/good/verified.js",
        )?;
        Ok(())
    }
}
