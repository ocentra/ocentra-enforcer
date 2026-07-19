//! `RUST-ARCH-1.1` (T2) — `main.rs` should only parse args and call
//! `run()`/equivalent; business-logic functions do not belong there.
//!
//! Path-scoped: only fires when [`ValidationInput::file`] ends in
//! `main.rs`. Flags any `fn` item defined directly in that file other
//! than `fn main`.

use syn::visit::Visit;
use syn::ItemFn;

use enforcer_domain::findings::Finding;
use enforcer_domain::ids::{BuiltInRustRule, RuleId};
use enforcer_domain::paths::RelPath;
use enforcer_domain::rules_types::RulePredicateResult;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

/// The `RUST-ARCH-1.1` `Validator`.
#[derive(Debug)]
pub struct ArchMainThinValidator {
    rule_id: RuleId,
}

impl ArchMainThinValidator {
    /// Build the validator, parsing its own `RuleId` literal at
    /// construction (parse-at-boundary).
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: BuiltInRustRule::ArchMainThin.id(),
        })
    }
}

fn is_main_rs(path: &RelPath) -> RulePredicateResult {
    if path.as_str().ends_with("main.rs") {
        RulePredicateResult::Matched
    } else {
        RulePredicateResult::NotMatched
    }
}

impl Validator for ArchMainThinValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        if is_main_rs(input.file) == RulePredicateResult::NotMatched {
            return Vec::new();
        }
        let Ok(file) = syn::parse_file(input.source.as_str()) else {
            return Vec::new();
        };
        let mut visitor = Visitor {
            rule_id: &self.rule_id,
            file: input.file,
            findings: Vec::new(),
        };
        visitor.visit_file(&file);
        visitor.findings
    }
}

struct Visitor<'a> {
    rule_id: &'a RuleId,
    file: &'a RelPath,
    findings: Vec<Finding>,
}

impl<'ast> Visit<'ast> for Visitor<'_> {
    fn visit_item_fn(&mut self, item: &'ast ItemFn) {
        if item.sig.ident != "main" {
            let line = crate::boundary::finding::source_line(item);
            let Ok(finding) = crate::boundary::finding::from_source(
                (self.rule_id, Severity::Warning),
                format!(
                    "business-logic `fn {}` defined in `main.rs`",
                    item.sig.ident
                ),
                format!(
                    "Fix: move `fn {}` out of `main.rs` into a lib module; `main.rs` should \
                     only parse args and call `run()`.",
                    item.sig.ident
                ),
                self.file,
                line,
            ) else {
                return;
            };
            self.findings.push(finding);
        }
        // Intentionally do not recurse further — nested fns inside `main`
        // itself are a separate (unaddressed) concern for this rule.
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use enforcer_domain::findings::ScanScope;
    use enforcer_domain::paths::RelPath;
    use enforcer_validator::validator::{ValidationInput, Validator};

    use super::ArchMainThinValidator;

    fn manifest_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    fn read_fixture(rel: &str) -> std::io::Result<String> {
        fs::read_to_string(manifest_dir().join(rel))
    }

    #[test]
    fn fires_on_logic_in_main_rs() -> Result<(), Box<dyn std::error::Error>> {
        let validator = ArchMainThinValidator::new()?;
        let source = read_fixture("fixtures/arch-main-thin/fail_logic_in_main.rs")?;
        let file: RelPath = crate::boundary::fixture::source_file("crates/x/src/main.rs")?;
        let findings = validator.validate(ValidationInput {
            file: &file,
            source: enforcer_domain::boundary::validation::ValidationSource::from_text(&source),
            scope: ScanScope::Files,
        });
        assert_eq!(findings.len(), 1);
        assert!(findings
            .iter()
            .all(|f| f.rule_id.as_str() == "RUST-ARCH-1.1"));
        Ok(())
    }

    #[test]
    fn silent_on_thin_main() -> Result<(), Box<dyn std::error::Error>> {
        let validator = ArchMainThinValidator::new()?;
        let source = read_fixture("fixtures/arch-main-thin/pass_thin_main.rs")?;
        let file: RelPath = crate::boundary::fixture::source_file("crates/x/src/main.rs")?;
        let findings = validator.validate(ValidationInput {
            file: &file,
            source: enforcer_domain::boundary::validation::ValidationSource::from_text(&source),
            scope: ScanScope::Files,
        });
        assert!(findings.is_empty());
        Ok(())
    }

    #[test]
    fn silent_outside_main_rs_even_with_extra_fns() -> Result<(), Box<dyn std::error::Error>> {
        let validator = ArchMainThinValidator::new()?;
        let source = read_fixture("fixtures/arch-main-thin/fail_logic_in_main.rs")?;
        let file: RelPath = crate::boundary::fixture::source_file("crates/x/src/lib.rs")?;
        let findings = validator.validate(ValidationInput {
            file: &file,
            source: enforcer_domain::boundary::validation::ValidationSource::from_text(&source),
            scope: ScanScope::Files,
        });
        assert!(findings.is_empty());
        Ok(())
    }

    #[test]
    fn malformed_source_stays_silent() -> Result<(), Box<dyn std::error::Error>> {
        let validator = ArchMainThinValidator::new()?;
        let file: RelPath = crate::boundary::fixture::source_file("crates/x/src/main.rs")?;
        let findings = validator.validate(ValidationInput {
            file: &file,
            source: enforcer_domain::boundary::validation::ValidationSource::from_text(
                "malformed rust {{{",
            ),
            scope: ScanScope::Files,
        });
        assert!(findings.is_empty());
        Ok(())
    }
}
