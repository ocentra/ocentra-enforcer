//! `CYBER-DESERIALIZE.1` (T1) — harvested from the vendored
//! `exploiting-insecure-deserialization` cyberskill
//! (`vendor/anthropic-cybersecurity-skills/skills/exploiting-insecure-deserialization`).
//! The vendor skill is a live-target attack playbook: its `scripts/agent.py`
//! generates `ysoserial`/`ysoserial.net`/`PHPGGC` gadget-chain payloads and
//! replays them against a running application over HTTP — there is no
//! inline source-scanning predicate to port verbatim. This validator instead
//! implements the well-known DETERMINISTIC (Semgrep-style) source check for
//! the vulnerability class the skill exists to exploit: a per-line scan for
//! the insecure deserialization SINKS the skill's "Key Concepts"/workflow
//! table names for each ecosystem —
//!
//! - Python: `pickle.load(`/`pickle.loads(`/`cPickle.load(`/`cPickle.loads(`
//!   (Step 5 "Test Python Pickle Deserialization"), `marshal.loads(`
//!   (Ruby/Python `Marshal`-class analog named alongside pickle in the
//!   skill's "When to Use" list), and `yaml.load(` (Step 5's PyYAML
//!   `!!python/object/apply:os.system` payload) — UNLESS the call is the
//!   safe `yaml.safe_load(` spelling, or carries `Loader=...SafeLoader`,
//!   which is PyYAML's documented safe-loading idiom and does not construct
//!   arbitrary Python objects.
//! - Ruby: `Marshal.load(` and `YAML.load(` ("Pickle (Python), Marshal
//!   (Ruby), or YAML deserialization" — "When to Use").
//! - PHP: `unserialize(` (Step 3 "Test PHP Deserialization with PHPGGC";
//!   "PHP object injection via manipulated serialized string").
//! - Java: `ObjectInputStream` construction and `.readObject(` calls (Step 2
//!   "Test Java Deserialization with ysoserial"; `readObject` is listed as a
//!   deserialization "Magic Method" in "Key Concepts").
//! - .NET: `BinaryFormatter` (Step 4 "Test .NET Deserialization";
//!   `ysoserial.net -f BinaryFormatter`).
//!
//! Each sink is high-confidence RCE-capable on attacker-controlled input, so
//! every match is `Severity::Error`. Per the PREVENTION-gate doctrine, no
//! attempt is made to distinguish "trusted" vs "untrusted" call sites via
//! regex (that would require data-flow analysis this line-scan cannot do);
//! the one deliberate exception is PyYAML's SafeLoader, which the vendor
//! skill itself names as the safe alternative and which is trivially
//! detectable as a same-line token.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};
use regex::Regex;

use crate::boundary::pattern::{RemediationPattern, RemediationPatternSource as DeserSink};

/// Sinks that are dangerous unconditionally (no safe-argument spelling exists
/// for them, per the vendor skill's own workflow steps).
const ALWAYS_UNSAFE_SINKS: &[DeserSink] = &[
    DeserSink {
        regex: r"\b(?:pickle|cPickle)\.loads?\(",
        label: "Python pickle.load()/pickle.loads()/cPickle.load()/cPickle.loads()",
        fix: "never unpickle untrusted data; use a data-only format (JSON) or an \
              HMAC-signed payload verified before unpickling",
    },
    DeserSink {
        regex: r"\bmarshal\.loads?\(",
        label: "Python marshal.load()/marshal.loads()",
        fix: "marshal has no security guarantees for untrusted input; use JSON or another \
              data-only serialization format",
    },
    DeserSink {
        regex: r"\bMarshal\.load\(",
        label: "Ruby Marshal.load()",
        fix: "avoid Marshal.load on untrusted input; use JSON or a signed/allowlisted format \
              instead",
    },
    DeserSink {
        regex: r"\bYAML\.load\(",
        label: "Ruby YAML.load()",
        fix: "use YAML.safe_load (or Psych.safe_load) instead of YAML.load on untrusted input",
    },
    DeserSink {
        regex: r"\bunserialize\s*\(",
        label: "PHP unserialize()",
        fix: "use json_decode() instead of unserialize() on untrusted input, or pass \
              options => ['allowed_classes' => false] to disable object instantiation",
    },
    DeserSink {
        regex: r"\bObjectInputStream\b",
        label: "Java ObjectInputStream",
        fix: "avoid deserializing untrusted data with ObjectInputStream; add a validating \
              ObjectInputFilter (JEP 290) allowlist, or switch to a data-only format",
    },
    DeserSink {
        regex: r"\.readObject\s*\(",
        label: "Java .readObject()",
        fix: "do not call readObject() on untrusted input without an ObjectInputFilter \
              allowlist in place",
    },
    DeserSink {
        regex: r"\bBinaryFormatter\b",
        label: ".NET BinaryFormatter",
        fix: "BinaryFormatter is insecure by design and deprecated by Microsoft; use \
              System.Text.Json or another data-only serializer",
    },
];

/// `CYBER-DESERIALIZE.1` — insecure deserialization sink detector across
/// Python, Ruby, PHP, Java, and .NET.
#[derive(Debug)]
pub struct InsecureDeserializationValidator {
    rule_id: RuleId,
    dangerous_sinks: Vec<RemediationPattern>,
    python_yaml_load: Regex,
    yaml_safe_loader_guard: Regex,
}

impl InsecureDeserializationValidator {
    pub fn new() -> Result<Self, DecodeError> {
        let mut dangerous_sinks = Vec::with_capacity(ALWAYS_UNSAFE_SINKS.len());
        for sink in ALWAYS_UNSAFE_SINKS {
            dangerous_sinks.push(RemediationPattern::compile_source(
                "cyberskillsInsecureDeserSink",
                sink,
            )?);
        }
        Ok(Self {
            rule_id: enforcer_domain::ids::BuiltInSecurityRule::CyberDeserialize.id(),
            dangerous_sinks,
            // `yaml.load(` (lowercase module, as in `import yaml`). Deliberately
            // does NOT match `yaml.safe_load(` — the substring after `yaml.` in
            // that spelling is `safe_load(`, not `load(`.
            python_yaml_load: Regex::new(r"\byaml\.load\s*\(").map_err(|err| {
                crate::boundary::regex::decode("cyberskillsInsecureDeserYamlLoad", err)
            })?,
            // Same-line guard: `Loader=yaml.SafeLoader` / `Loader=SafeLoader`
            // (any spacing/quoting) is PyYAML's documented safe-loading idiom.
            yaml_safe_loader_guard: Regex::new(r"\bSafeLoader\b").map_err(|err| {
                crate::boundary::regex::decode("cyberskillsInsecureDeserYamlSafeLoaderGuard", err)
            })?,
        })
    }
}

impl Validator for InsecureDeserializationValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let mut findings = Vec::new();
        for (index, line) in input.source.as_str().lines().enumerate() {
            let line_number = u32::try_from(index).unwrap_or(u32::MAX).saturating_add(1);

            for pattern in &self.dangerous_sinks {
                if pattern.regex().is_match(line) {
                    let label = pattern.label().as_str();
                    let fix = pattern.fix().as_str();
                    findings.extend(crate::boundary::finding::from_source(
                        (&self.rule_id, Severity::Error),
                        "insecure deserialization sink",
                        format!(
                            "{label} deserializes its input without any integrity or type \
                             check, letting an attacker who controls the serialized payload \
                             achieve remote code execution via a gadget chain (see OWASP \
                             A08:2021 — Software and Data Integrity Failures). Fix: {fix}."
                        ),
                        input.file,
                        (line_number, Some(line)),
                    ));
                }
            }

            // Python `yaml.load(...)` is dangerous UNLESS the same line carries a
            // SafeLoader guard (`yaml.safe_load(` / `Loader=yaml.SafeLoader`).
            if self.python_yaml_load.is_match(line) && !self.yaml_safe_loader_guard.is_match(line) {
                findings.extend(crate::boundary::finding::from_source(
                    (&self.rule_id, Severity::Error),
                    "insecure deserialization sink",
                    "Python yaml.load() without a SafeLoader can construct \
                             arbitrary Python objects from the input document (PyYAML's \
                             own documented attack payload is \
                             `!!python/object/apply:os.system [...]`), letting an attacker \
                             who controls the YAML document achieve remote code execution. \
                             Fix: use yaml.safe_load(...) or pass Loader=yaml.SafeLoader.",
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

    use super::InsecureDeserializationValidator;

    #[test]
    fn cyberskills_insecure_deser() -> Result<(), Box<dyn std::error::Error>> {
        let validator = InsecureDeserializationValidator::new()?;
        run_manifest_fixture_parity(
            &validator,
            "tests/fixtures/cyberskills/web.insecure-deserialization/bad/deser.py",
            "tests/fixtures/cyberskills/web.insecure-deserialization/good/safe.py",
        )?;
        Ok(())
    }
}
