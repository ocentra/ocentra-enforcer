//! `RUST-CAST-NO-AS-LOSSY` — no lossy numeric `as` cast; use
//! `TryFrom`/`try_into` and propagate the conversion error instead.
//!
//! Scope: flags `EXPR as u8/u16/u32/i8/i16/i32` — narrowing casts to the
//! small fixed-width integer types, which is where truncation silently
//! loses data. Casts to `usize`/`isize`/`u64`/`i64`/`f32`/`f64` and
//! pointer casts are out of scope for this rule (those either can't
//! truncate on common targets or aren't the "narrowing integer" case this
//! rule targets).

use syn::visit::{self, Visit};
use syn::{ExprCast, Type};

use enforcer_domain::findings::Finding;
use enforcer_domain::ids::{BuiltInRustRule, RuleId};
use enforcer_domain::paths::RelPath;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

/// The `RUST-CAST-NO-AS-LOSSY` `Validator`.
#[derive(Debug)]
pub struct CastLossyValidator {
    rule_id: RuleId,
}

impl CastLossyValidator {
    /// Build the validator, parsing its own `RuleId` literal at
    /// construction (parse-at-boundary).
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: BuiltInRustRule::CastLossy.id(),
        })
    }
}

impl Validator for CastLossyValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
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

const NARROW_INT_TYPES: &[&str] = &["u8", "u16", "u32", "i8", "i16", "i32"];

fn is_narrowing_target(ty: &Type) -> Option<&'static str> {
    let Type::Path(type_path) = ty else {
        return None;
    };
    let segment = type_path.path.segments.last()?;
    NARROW_INT_TYPES
        .iter()
        .find(|candidate| segment.ident == **candidate)
        .copied()
}

struct Visitor<'a> {
    rule_id: &'a RuleId,
    file: &'a RelPath,
    findings: Vec<Finding>,
}

impl<'ast> Visit<'ast> for Visitor<'_> {
    fn visit_expr_cast(&mut self, item: &'ast ExprCast) {
        if let Some(target) = is_narrowing_target(&item.ty) {
            let line = crate::boundary::finding::source_line(item);
            let Ok(finding) = crate::boundary::finding::from_source(
                (self.rule_id, Severity::Error),
                format!("lossy `as {target}` cast"),
                format!(
                    "Fix: replace this `as {target}` cast with `{target}::try_from(...)?` (or \
                     `.try_into()?`) and propagate the conversion error instead of silently \
                     truncating."
                ),
                self.file,
                line,
            ) else {
                return;
            };
            self.findings.push(finding);
        }
        visit::visit_expr_cast(self, item);
    }
}

#[cfg(test)]
mod tests {

    use crate::boundary::fixture::run_fixture_parity;

    use super::CastLossyValidator;

    #[test]
    fn fires_on_lossy_as_and_silent_on_try_from() -> Result<(), Box<dyn std::error::Error>> {
        let validator = CastLossyValidator::new()?;
        run_fixture_parity(
            &validator,
            "fixtures/cast-lossy/fail_as.rs",
            "fixtures/cast-lossy/pass_try_from.rs",
        )?;
        Ok(())
    }

    #[test]
    fn malformed_source_stays_silent() -> Result<(), Box<dyn std::error::Error>> {
        use enforcer_validator::validator::Validator;
        let validator = CastLossyValidator::new()?;
        let file: enforcer_domain::paths::RelPath =
            crate::boundary::fixture::source_file("crates/x/src/lib.rs")?;
        let findings = validator.validate(enforcer_validator::validator::ValidationInput {
            file: &file,
            source: enforcer_domain::boundary::validation::ValidationSource::from_text(
                "malformed rust {{{",
            ),
            scope: enforcer_domain::findings::ScanScope::Files,
        });
        assert!(findings.is_empty());
        Ok(())
    }
}
