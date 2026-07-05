//! Dart/Flutter security rules `DART-SEC-1.1..1.6`: hardcoded secrets,
//! insecure token/PII storage, plaintext HTTP, disabled TLS
//! verification, bare `print` diagnostics, and unguarded debug output.

use enforcer_core::error::DecodeError;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

use super::support::{finding, first_line_containing, first_line_containing_any, FindingSpec};

/// `DART-SEC-1.1` — no hardcoded API key/token literal
/// (`const apiKey = 'sk-...'`-shaped assignment).
pub struct HardcodedSecretValidator {
    rule_id: RuleId,
}

impl HardcodedSecretValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "DART-SEC-1.1".parse()?,
        })
    }
}

impl Validator for HardcodedSecretValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        let Some(line) = first_line_containing_any(input.source, &["apiKey = '", "apiKey = \""])
        else {
            return Vec::new();
        };
        vec![finding(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Error,
                title: "hardcoded API key/token literal",
            },
            "an API key/token is hardcoded as a string literal — load it from \
             `String.fromEnvironment`/a build-time secret, never commit it to source."
                .to_owned(),
            &input,
            line,
        )]
    }
}

/// `DART-SEC-1.2` — tokens/PII must go to `flutter_secure_storage`, not
/// `SharedPreferences`.
pub struct InsecureStorageValidator {
    rule_id: RuleId,
}

impl InsecureStorageValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "DART-SEC-1.2".parse()?,
        })
    }
}

impl Validator for InsecureStorageValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        let Some(line) = first_line_containing_any(
            input.source,
            &[
                "prefs.setString('auth_token'",
                "prefs.setString(\"auth_token\"",
            ],
        ) else {
            return Vec::new();
        };
        vec![finding(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Error,
                title: "token/PII written to SharedPreferences",
            },
            "a token/PII value is written to `SharedPreferences` — use \
             `flutter_secure_storage`'s `FlutterSecureStorage().write(...)` for anything \
             sensitive."
                .to_owned(),
            &input,
            line,
        )]
    }
}

/// `DART-SEC-1.3` — HTTPS only: a bare `http://` URI (excluding
/// `localhost`/`127.0.0.1` for local dev) is flagged.
pub struct PlaintextHttpValidator {
    rule_id: RuleId,
}

impl PlaintextHttpValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "DART-SEC-1.3".parse()?,
        })
    }
}

impl Validator for PlaintextHttpValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        for (idx, line) in input.source.lines().enumerate() {
            if line.contains("http://")
                && !line.contains("localhost")
                && !line.contains("127.0.0.1")
            {
                return vec![finding(
                    &FindingSpec {
                        rule_id: &self.rule_id,
                        severity: Severity::Error,
                        title: "plaintext HTTP URI",
                    },
                    "a network call targets a plaintext `http://` URI — use `https://` for any \
                     non-local endpoint."
                        .to_owned(),
                    &input,
                    (idx as u32).saturating_add(1),
                )];
            }
        }
        Vec::new()
    }
}

/// `DART-SEC-1.4` — never disable SSL/TLS certificate verification
/// (`badCertificateCallback = (...) => true`).
pub struct DisabledTlsValidator {
    rule_id: RuleId,
}

impl DisabledTlsValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "DART-SEC-1.4".parse()?,
        })
    }
}

impl Validator for DisabledTlsValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        let Some(line) = first_line_containing(input.source, "badCertificateCallback") else {
            return Vec::new();
        };
        if !input.source.contains("=> true") {
            return Vec::new();
        }
        vec![finding(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Error,
                title: "SSL certificate verification disabled",
            },
            "`badCertificateCallback` unconditionally returns `true`, disabling TLS certificate \
             verification — never override this to accept all certificates."
                .to_owned(),
            &input,
            line,
        )]
    }
}

/// `DART-SEC-1.5` (scored) — bare `print(...)` used for diagnostics
/// instead of a monitoring logger.
pub struct BarePrintValidator {
    rule_id: RuleId,
}

impl BarePrintValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "DART-SEC-1.5".parse()?,
        })
    }
}

impl Validator for BarePrintValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        let Some(line) = first_line_containing(input.source, "print(") else {
            return Vec::new();
        };
        vec![finding(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Warning,
                title: "bare print() for diagnostics (scored)",
            },
            "bare `print(...)` is used for diagnostics — route through a monitoring logger \
             instead so output is structured and filterable in production."
                .to_owned(),
            &input,
            line,
        )]
    }
}

/// `DART-SEC-1.6` (scored) — a debug-output block with no `kDebugMode`
/// guard.
pub struct UnguardedDebugOutputValidator {
    rule_id: RuleId,
}

impl UnguardedDebugOutputValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "DART-SEC-1.6".parse()?,
        })
    }
}

impl Validator for UnguardedDebugOutputValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        let Some(line) = first_line_containing(input.source, "debugPrint(") else {
            return Vec::new();
        };
        if input.source.contains("kDebugMode") {
            return Vec::new();
        }
        vec![finding(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Warning,
                title: "unguarded debug output (scored)",
            },
            "a `debugPrint(...)` call has no `kDebugMode` guard anywhere in the file — wrap \
             debug-only output in `if (kDebugMode) { ... }`."
                .to_owned(),
            &input,
            line,
        )]
    }
}

/// Build every validator this module registers.
pub fn all() -> Result<Vec<Box<dyn Validator>>, DecodeError> {
    Ok(vec![
        Box::new(HardcodedSecretValidator::new()?),
        Box::new(InsecureStorageValidator::new()?),
        Box::new(PlaintextHttpValidator::new()?),
        Box::new(DisabledTlsValidator::new()?),
        Box::new(BarePrintValidator::new()?),
        Box::new(UnguardedDebugOutputValidator::new()?),
    ])
}
