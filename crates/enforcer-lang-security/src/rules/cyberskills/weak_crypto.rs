//! `CYBER-WEAK-CRYPTO.1` (T1) — harvest target: the `weak_hash`,
//! `weak_cipher`, `ecb_mode`, and `weak_random` entries of the
//! `WEAK_PATTERNS` table in
//! `vendor/anthropic-cybersecurity-skills/skills/performing-cryptographic-audit-of-application/scripts/agent.py`
//! (L26-62).
//!
//! The vendor patterns are broad bare-word matches (e.g.
//! `\b(MD5|md5|SHA1|sha1|SHA-1)\b(?!.*hmac)`, `\b(DES|3DES|RC4|RC2|Blowfish|
//! IDEA)\b`, `\b(ECB|MODE_ECB|ecb)\b`) tuned for a standalone audit report
//! where a human triages every hit; ported verbatim into a gate that blocks
//! commits, they would flag prose ("migrated away from MD5"), unrelated
//! identifiers, and non-cryptographic uses. This validator narrows each
//! vendor category to the concrete, high-confidence call-site shapes named
//! in the workpack spec (`hashlib.md5(`, `hashlib.sha1(`,
//! `MessageDigest.getInstance("MD5"/"SHA1")`, `createHash("md5"/"sha1")` for
//! `weak_hash`; `DES`/`DES3`/`ARC4` constructors and `Cipher.getInstance`/
//! `createCipher(iv)` call sites for `weak_cipher`; `MODE_ECB` and an
//! `"<algo>/ECB"` transformation string for `ecb_mode`) so only genuine API
//! usage triggers, per the PREVENTION-gate false-positive budget.
//!
//! `weak_random` is similarly narrowed: the vendor's bare
//! `\b(?:random\.random|math\.random|Math\.random|rand\(\)|srand)\b` fires
//! on ANY use of a non-crypto RNG, including harmless UI jitter or sampling.
//! This validator instead requires `Math.random()` to co-occur on the same
//! line with a secret/token/session/auth keyword — the spec's "insecure
//! randomness for secrets" case — and reports it as a `Warning`, not an
//! `Error`, since it is a narrower, context-dependent heuristic rather than
//! an unconditionally broken primitive.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};
use regex::Regex;

use crate::boundary::pattern::{LabelledPattern, LabelledPatternSource as WeakCryptoPattern};

/// Call-site patterns for the vendor's `weak_hash`, `weak_cipher`, and
/// `ecb_mode` categories (agent.py L27-44), narrowed from bare-word matches
/// to concrete API shapes per language.
const WEAK_CRYPTO_PATTERNS_SRC: &[WeakCryptoPattern] = &[
    // weak_hash: MD5/SHA1 (agent.py L27-32).
    WeakCryptoPattern {
        regex: r"hashlib\.md5\s*\(",
        label: "MD5 hash (Python hashlib.md5)",
    },
    WeakCryptoPattern {
        regex: r"hashlib\.sha1\s*\(",
        label: "SHA1 hash (Python hashlib.sha1)",
    },
    WeakCryptoPattern {
        regex: r#"MessageDigest\.getInstance\(\s*"MD5"\s*\)"#,
        label: "MD5 hash (Java MessageDigest)",
    },
    WeakCryptoPattern {
        regex: r#"MessageDigest\.getInstance\(\s*"SHA-?1"\s*\)"#,
        label: "SHA1 hash (Java MessageDigest)",
    },
    WeakCryptoPattern {
        regex: r#"createHash\(\s*["']md5["']\s*\)"#,
        label: "MD5 hash (Node crypto.createHash)",
    },
    WeakCryptoPattern {
        regex: r#"createHash\(\s*["']sha1["']\s*\)"#,
        label: "SHA1 hash (Node crypto.createHash)",
    },
    // weak_cipher: DES / 3DES / RC4 (agent.py L33-38).
    WeakCryptoPattern {
        regex: r"(?i)\b(?:DES3?|ARC4)\.new\s*\(",
        label: "Weak cipher (Python DES/3DES/RC4 constructor)",
    },
    WeakCryptoPattern {
        regex: r#"(?i)Cipher\.getInstance\(\s*"(?:DES|DESede|RC4)"#,
        label: "Weak cipher (Java DES/3DES/RC4 Cipher.getInstance)",
    },
    WeakCryptoPattern {
        regex: r#"(?i)createCipher(?:iv)?\(\s*["'](?:des(?:-ede3)?|rc4)"#,
        label: "Weak cipher (Node DES/3DES/RC4 createCipher(iv))",
    },
    // ecb_mode: ECB cipher mode (agent.py L39-44).
    WeakCryptoPattern {
        regex: r"\bMODE_ECB\b",
        label: "ECB cipher mode (Python MODE_ECB)",
    },
    WeakCryptoPattern {
        regex: r#"(?i)"[A-Za-z0-9]+/ECB"#,
        label: "ECB cipher mode (transformation string, e.g. \"AES/ECB\")",
    },
];

/// `CYBER-WEAK-CRYPTO.1` — flags weak/broken cryptographic primitives
/// (MD5/SHA1 hashing, DES/3DES/RC4 ciphers, ECB mode) as errors, and
/// `Math.random()` used to build a secret/token as a warning.
#[derive(Debug)]
pub struct WeakCryptoValidator {
    rule_id: RuleId,
    patterns: Vec<LabelledPattern>,
    math_random: Regex,
    token_context: Regex,
}

impl WeakCryptoValidator {
    pub fn new() -> Result<Self, DecodeError> {
        let mut patterns = Vec::with_capacity(WEAK_CRYPTO_PATTERNS_SRC.len());
        for entry in WEAK_CRYPTO_PATTERNS_SRC {
            patterns.push(LabelledPattern::compile_source(
                "cyberskillsWeakCryptoPattern",
                entry,
            )?);
        }
        let math_random = Regex::new(r"Math\.random\s*\(\s*\)").map_err(|err| {
            crate::boundary::regex::decode("cyberskillsWeakCryptoMathRandom", err)
        })?;
        // No `\b` before the keyword: real call sites name these values with
        // camelCase identifiers (`resetToken`, `authToken`, `sessionSecret`),
        // where a word-boundary assertion would never fire before the
        // capitalized keyword since there is no non-word transition there.
        let token_context = Regex::new(
            r"(?i)(?:token|secret|password|passwd|session|api[_-]?key|auth)",
        )
        .map_err(|err| crate::boundary::regex::decode("cyberskillsWeakCryptoTokenContext", err))?;
        Ok(Self {
            rule_id: enforcer_domain::ids::BuiltInSecurityRule::CyberWeakCrypto.id(),
            patterns,
            math_random,
            token_context,
        })
    }
}

impl Validator for WeakCryptoValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let mut findings = Vec::new();
        for (index, line) in input.source.as_str().lines().enumerate() {
            let line_number = u32::try_from(index).unwrap_or(u32::MAX).saturating_add(1);
            let mut matched_labels: Vec<&str> = Vec::new();
            for pattern in &self.patterns {
                if pattern.regex().is_match(line)
                    && !matched_labels.contains(&pattern.label().as_str())
                {
                    matched_labels.push(pattern.label().as_str());
                }
            }
            if !matched_labels.is_empty() {
                findings.extend(crate::boundary::finding::from_source(
                    (&self.rule_id, Severity::Error),
                    "Weak or broken cryptographic primitive",
                    format!(
                        "Line uses a weak/broken cryptographic primitive: {}. Fix: use SHA-256 or \
                         SHA-3 for hashing, AES-256-GCM or ChaCha20-Poly1305 for encryption, and \
                         GCM/CTR (never ECB) as the cipher mode.",
                        matched_labels.join(", ")
                    ),
                    input.file,
                    (line_number, Some(line)),
                ));
            }
            if self.math_random.is_match(line) && self.token_context.is_match(line) {
                findings.extend(crate::boundary::finding::from_source(
                    (&self.rule_id, Severity::Warning),
                    "Insecure randomness used for a security-sensitive value",
                    "Line uses Math.random() alongside a token/secret/session/auth \
                             keyword. Math.random() is not cryptographically secure and its \
                             output is predictable. Fix: use crypto.getRandomValues() or \
                             crypto.randomBytes() (JS/Node), the secrets module (Python), or \
                             SecureRandom (Java) instead.",
                    input.file,
                    (line_number, Some(line)),
                ));
            }
        }
        findings
    }
}

#[cfg(test)]
mod tests {
    use crate::boundary::fixture::run_manifest_fixture_parity;

    use super::WeakCryptoValidator;

    #[test]
    fn cyberskills_weak_crypto() -> Result<(), Box<dyn std::error::Error>> {
        let v = WeakCryptoValidator::new()?;
        run_manifest_fixture_parity(
            &v,
            "tests/fixtures/cyberskills/crypto.weak-algorithm/bad/weak.py",
            "tests/fixtures/cyberskills/crypto.weak-algorithm/good/strong.py",
        )?;
        Ok(())
    }
}
