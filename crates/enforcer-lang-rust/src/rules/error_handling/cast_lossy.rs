//! `RUST-CAST-NO-AS-LOSSY` — no lossy numeric `as` cast; use
//! `TryFrom`/`try_into` and propagate the conversion error instead.
//!
//! Scope: flags `EXPR as u8/u16/u32/i8/i16/i32` — narrowing casts to the
//! small fixed-width integer types, which is where truncation silently
//! loses data. Casts to `usize`/`isize`/`u64`/`i64`/`f32`/`f64` and
//! pointer casts are out of scope for this rule (those either can't
//! truncate on common targets or aren't the "narrowing integer" case this
//! rule targets).

use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{ExprCast, Type};

use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::RelPath;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

/// The `RUST-CAST-NO-AS-LOSSY` `Validator`.
pub struct CastLossyValidator {
    rule_id: RuleId,
}

impl CastLossyValidator {
    /// Build the validator, parsing its own `RuleId` literal at
    /// construction (parse-at-boundary).
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: "RUST-CAST-NO-AS-LOSSY".parse()?,
        })
    }
}

impl Validator for CastLossyValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let Ok(file) = syn::parse_file(input.source) else {
            return Vec::new();
        };
        let mut visitor = Visitor {
            rule_id: self.rule_id.clone(),
            file: input.file.clone(),
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
    let name = segment.ident.to_string();
    NARROW_INT_TYPES
        .iter()
        .find(|candidate| **candidate == name)
        .copied()
}

struct Visitor {
    rule_id: RuleId,
    file: RelPath,
    findings: Vec<Finding>,
}

impl<'ast> Visit<'ast> for Visitor {
    fn visit_expr_cast(&mut self, item: &'ast ExprCast) {
        if let Some(target) = is_narrowing_target(&item.ty) {
            let line = u32::try_from(item.span().start().line.max(1)).unwrap_or(u32::MAX);
            self.findings.push(Finding {
                rule_id: self.rule_id.clone(),
                severity: Severity::Error,
                title: format!("lossy `as {target}` cast"),
                detail: format!(
                    "Fix: replace this `as {target}` cast with `{target}::try_from(...)?` (or \
                     `.try_into()?`) and propagate the conversion error instead of silently \
                     truncating."
                ),
                file: self.file.clone(),
                line,
                snippet: None,
            });
        }
        visit::visit_expr_cast(self, item);
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use enforcer_validator::harness::run_fixture_parity;

    use super::CastLossyValidator;

    fn manifest_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn fires_on_lossy_as_and_silent_on_try_from() -> Result<(), Box<dyn std::error::Error>> {
        let validator = CastLossyValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "fixtures/cast-lossy/fail_as.rs",
            "fixtures/cast-lossy/pass_try_from.rs",
        )?;
        Ok(())
    }

    #[test]
    fn unparseable_source_stays_silent() -> Result<(), Box<dyn std::error::Error>> {
        use enforcer_validator::validator::Validator;
        let validator = CastLossyValidator::new()?;
        let file: enforcer_domain::paths::RelPath = "crates/x/src/lib.rs".parse()?;
        let findings = validator.validate(enforcer_validator::validator::ValidationInput {
            file: &file,
            source: "not valid rust {{{",
            scope: enforcer_domain::findings::ScanScope::Files,
        });
        assert!(findings.is_empty());
        Ok(())
    }
}
