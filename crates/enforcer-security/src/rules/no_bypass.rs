//! `H00-1.1` — the no-bypass meta-check.
//!
//! Doctrine: `enforcer-config` is the SINGLE declarative control-plane
//! (owner/exempt globs, allow-regex, per-rule toggles, `cfg(test)`
//! skipping) that both the CLI and UI surfaces read. There is never an
//! inline-disable — the enforcer ships NO inline-suppress escape hatch.
//! The ONLY legitimate exemption path is a declarative, committed, gated
//! waiver read from `enforcer-config` (owner + reason + ruleId), never an
//! inline comment.
//!
//! This validator bans, across every scanned line regardless of source
//! language, the family of inline lint-disable / validation-bypass
//! directives:
//!
//! - Rust: `#[allow(...)]` on an enforcer-governed lint, `clippy::allow`
//!   on the deny wall.
//! - JS/TS: `// eslint-disable`, `// eslint-disable-next-line`,
//!   `@ts-ignore`, `@ts-expect-error`.
//! - Python: `# noqa`, `# type: ignore`.
//! - Ad-hoc: any comment containing `enforcer-disable`/`enforcer-ignore`/
//!   `enforcer-bypass` (the generic ad-hoc suppress-comment shape this
//!   crate itself must never grow).
//!
//! Deliberately NOT comment-guarded: unlike most line-scan validators in
//! this workspace (see `enforcer-lang-security`'s `text_scan` module
//! doc), the violation shape here IS the comment/attribute itself, so
//! skipping comment-only lines would defeat the whole check.
//!
//! The only way to legitimately suppress a rule is a declarative waiver
//! entry in `enforcer-config` (owner + reason + ruleId) — this validator
//! does not itself read that config (that wiring is `enforcer-config` +
//! the scan orchestrator's job, arc-14+); it only bans the inline escape
//! hatch at the source-text level, unconditionally.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};
use regex::Regex;

/// One inline-suppress directive shape this check bans, paired with the
/// human-readable name used in the finding detail.
struct BannedPattern {
    name: &'static str,
    pattern: Regex,
}

/// The no-bypass meta-check `Validator`: bans every known inline-suppress
/// directive shape across all scanned source text.
pub struct NoBypassValidator {
    rule_id: RuleId,
    banned: Vec<BannedPattern>,
}

impl NoBypassValidator {
    pub fn new() -> Result<Self, DecodeError> {
        let specs: &[(&str, &str)] = &[
            (
                "rust #[allow(...)] on an enforcer-governed lint",
                r#"#\s*!?\s*\[\s*allow\s*\("#,
            ),
            (
                "clippy::allow on the deny wall",
                r"clippy\s*::\s*allow\s*\(",
            ),
            (
                "eslint-disable directive",
                r"//\s*eslint-disable(?:-next-line|-line)?",
            ),
            ("TypeScript @ts-ignore", r"@ts-ignore\b"),
            ("TypeScript @ts-expect-error", r"@ts-expect-error\b"),
            ("Python `# noqa` suppress comment", r"#\s*noqa\b"),
            (
                "Python `# type: ignore` suppress comment",
                r"#\s*type\s*:\s*ignore\b",
            ),
            (
                "ad-hoc enforcer-disable/ignore/bypass suppress comment",
                r"enforcer-(?:disable|ignore|bypass)\b",
            ),
        ];
        let mut banned = Vec::with_capacity(specs.len());
        for (name, source) in specs {
            let pattern = Regex::new(source).map_err(|err| {
                DecodeError::new("noBypass.pattern", format!("invalid pattern {name}: {err}"))
            })?;
            banned.push(BannedPattern { name, pattern });
        }
        Ok(Self {
            rule_id: "H00-1.1".parse()?,
            banned,
        })
    }
}

impl Validator for NoBypassValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let mut findings = Vec::new();
        for (idx, text) in input.source.lines().enumerate() {
            let line_number = (idx as u32).saturating_add(1);
            for candidate in &self.banned {
                if candidate.pattern.is_match(text) {
                    findings.push(Finding {
                        rule_id: self.rule_id.clone(),
                        severity: Severity::Error,
                        title: "Inline validation-bypass directives are forbidden".to_owned(),
                        detail: format!(
                            "found {}: the only legitimate exemption path is a declarative, \
                             committed, gated waiver in enforcer-config (owner+reason+ruleId), \
                             never an inline comment. Fix: remove the inline suppression and, \
                             if truly needed, add an `enforcer-config` waiver instead.",
                            candidate.name
                        ),
                        file: input.file.clone(),
                        line: line_number,
                        snippet: Some(text.trim().to_owned()),
                    });
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

    use super::NoBypassValidator;
    use enforcer_validator::validator::{ValidationInput, Validator};

    fn manifest_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    fn rel(path: &str) -> Result<RelPath, Box<dyn std::error::Error>> {
        Ok(path.parse()?)
    }

    #[test]
    fn fires_on_rust_inline_allow() -> Result<(), Box<dyn std::error::Error>> {
        let validator = NoBypassValidator::new()?;
        let file = rel("crates/x/src/lib.rs")?;
        let findings = validator.validate(ValidationInput {
            file: &file,
            source: "#[allow(clippy::unwrap_used)]\nfn f() {}",
            scope: ScanScope::Files,
        });
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id.as_str(), "H00-1.1");
        Ok(())
    }

    #[test]
    fn fires_on_eslint_disable() -> Result<(), Box<dyn std::error::Error>> {
        let validator = NoBypassValidator::new()?;
        let file = rel("src/app.ts")?;
        let findings = validator.validate(ValidationInput {
            file: &file,
            source: "// eslint-disable-next-line no-eval\neval(x);",
            scope: ScanScope::Files,
        });
        assert_eq!(findings.len(), 1);
        Ok(())
    }

    #[test]
    fn fires_on_python_noqa_and_type_ignore() -> Result<(), Box<dyn std::error::Error>> {
        let validator = NoBypassValidator::new()?;
        let file = rel("src/app.py")?;
        let findings = validator.validate(ValidationInput {
            file: &file,
            source: "import os  # noqa\nx: int = f()  # type: ignore",
            scope: ScanScope::Files,
        });
        assert_eq!(findings.len(), 2);
        Ok(())
    }

    #[test]
    fn silent_on_clean_source() -> Result<(), Box<dyn std::error::Error>> {
        let validator = NoBypassValidator::new()?;
        let file = rel("crates/x/src/lib.rs")?;
        let findings = validator.validate(ValidationInput {
            file: &file,
            source: "fn f() {\n    // a normal comment\n    1 + 1;\n}",
            scope: ScanScope::Files,
        });
        assert!(findings.is_empty());
        Ok(())
    }

    #[test]
    fn no_bypass_fixture_parity() -> Result<(), Box<dyn std::error::Error>> {
        let validator = NoBypassValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/no-bypass/bad/sample.rs",
            "tests/fixtures/no-bypass/good/sample.rs",
        )?;
        Ok(())
    }
}
