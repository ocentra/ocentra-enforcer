//! Dart typed-DTO discipline: `DART-TYPE-1.1..1.6` (no `dynamic`, no
//! `Map<String,dynamic>` DTO/return, typed public signatures),
//! `DART-FALLBACK-1.1` / `DART-STYLE-2.2` (no silent `?? default` on a
//! required field / unjustified `response.data!`), and
//! `DART-FORMMAP-1.1` (form state as an untyped map).

use enforcer_core::error::DecodeError;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

use super::support::{finding, first_line_containing, first_line_containing_any, FindingSpec};

/// `DART-TYPE-1.1..1.6` — typed DTOs: a public function signature typed
/// `dynamic` (bare, or as `Map<String,dynamic>` on a public
/// parse/return signature) is untyped and flagged; a fully-typed nested
/// DTO class is clean.
pub struct TypedDtoValidator {
    rule_id: RuleId,
}

impl TypedDtoValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "DART-TYPE-1.1".parse()?,
        })
    }
}

impl Validator for TypedDtoValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        let markers = ["Map<String, dynamic>", "Map<String,dynamic>", "dynamic "];
        let Some(line) = first_line_containing_any(input.source, &markers) else {
            return Vec::new();
        };
        vec![finding(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Error,
                title: "untyped `dynamic`/`Map<String,dynamic>` DTO or signature",
            },
            "a public signature or DTO field is typed `dynamic`/`Map<String,dynamic>` — parse \
             into a typed class with typed fields instead."
                .to_owned(),
            &input,
            line,
        )]
    }
}

/// `DART-FALLBACK-1.1` / `DART-STYLE-2.2` (scored) — no `?? 0`/`?? ''`
/// coalescing a required field to a default at construction time, and no
/// bare `response.data!` unwrap missing a justifying comment on the
/// preceding line.
pub struct SilentFallbackValidator {
    rule_id: RuleId,
}

impl SilentFallbackValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "DART-FALLBACK-1.1".parse()?,
        })
    }
}

impl Validator for SilentFallbackValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        let spec = FindingSpec {
            rule_id: &self.rule_id,
            severity: Severity::Warning,
            title: "silent default fallback on a required field (scored)",
        };

        if let Some(line) = first_line_containing_any(input.source, &["?? 0", "?? ''", "?? \"\""]) {
            return vec![finding(
                &spec,
                "a required field falls back to a silent default (`?? 0`/`?? ''`) instead of \
                 validating then constructing — a missing value should fail construction, not \
                 silently substitute zero/empty."
                    .to_owned(),
                &input,
                line,
            )];
        }

        if let Some(line) = first_line_containing(input.source, ".data!") {
            let lines: Vec<&str> = input.source.lines().collect();
            let idx = (line as usize).saturating_sub(1);
            let has_justifying_comment = idx > 0 && lines[idx - 1].trim_start().starts_with("//");
            if !has_justifying_comment {
                return vec![finding(
                    &spec,
                    "bare `response.data!` unwrap with no justifying comment on the preceding \
                     line explaining why the value is guaranteed non-null."
                        .to_owned(),
                    &input,
                    line,
                )];
            }
        }

        Vec::new()
    }
}

/// `DART-FORMMAP-1.1` (scored) — wizard/form state carried as an untyped
/// `Map<String, Object?>` instead of a typed form-state class.
pub struct FormStateMapValidator {
    rule_id: RuleId,
}

impl FormStateMapValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "DART-FORMMAP-1.1".parse()?,
        })
    }
}

impl Validator for FormStateMapValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        let Some(line) = first_line_containing(input.source, "Map<String, Object?>") else {
            return Vec::new();
        };
        vec![finding(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Warning,
                title: "form/wizard state carried as an untyped map (scored)",
            },
            "form/wizard state is a `Map<String, Object?>` — model it as a typed form-state \
             class so field access is compile-checked."
                .to_owned(),
            &input,
            line,
        )]
    }
}

/// Build every validator this module registers.
pub fn all() -> Result<Vec<Box<dyn Validator>>, DecodeError> {
    Ok(vec![
        Box::new(TypedDtoValidator::new()?),
        Box::new(SilentFallbackValidator::new()?),
        Box::new(FormStateMapValidator::new()?),
    ])
}
