//! Dart typed-DTO discipline: `DART-TYPE-1.1..1.6` (no `dynamic`, no
//! `Map<String,dynamic>` DTO/return, typed public signatures),
//! `DART-FALLBACK-1.1` / `DART-STYLE-2.2` (no silent `?? default` on a
//! required field / unjustified `response.data!`), and
//! `DART-FORMMAP-1.1` (form state as an untyped map).

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::boundary::validation::ValidationMarker;
use enforcer_domain::ids::{BuiltInDartRule, RuleId};
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

use super::support::{first_line_containing, first_line_containing_any, FindingSpec};

/// `DART-TYPE-1.1..1.6` — typed DTOs: a public function signature typed
/// `dynamic` (bare, or as `Map<String,dynamic>` on a public
/// parse/return signature) is untyped and flagged; a fully-typed nested
/// DTO class is clean.
#[derive(Debug)]
pub struct TypedDtoValidator {
    rule_id: RuleId,
}

impl TypedDtoValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: BuiltInDartRule::TypedDto.id(),
        })
    }
}

impl Validator for TypedDtoValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        let markers = [
            ValidationMarker::from_static("Map<String, dynamic>"),
            ValidationMarker::from_static("Map<String,dynamic>"),
            ValidationMarker::from_static("dynamic "),
        ];
        let Some(line) = first_line_containing_any(input.source, &markers) else {
            return Vec::new();
        };
        vec![finding!(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Error,
                rule: BuiltInDartRule::TypedDto,
            },
            "a public signature or DTO field is typed `dynamic`/`Map<String,dynamic>` — parse \
             into a typed class with typed fields instead.",
            &input,
            line,
        )]
    }
}

/// `DART-FALLBACK-1.1` / `DART-STYLE-2.2` (scored) — no `?? 0`/`?? ''`
/// coalescing a required field to a default at construction time, and no
/// bare `response.data!` unwrap missing a justifying comment on the
/// preceding line.
#[derive(Debug)]
pub struct SilentFallbackValidator {
    rule_id: RuleId,
}

impl SilentFallbackValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: BuiltInDartRule::SilentFallback.id(),
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
            rule: BuiltInDartRule::SilentFallback,
        };

        if let Some(line) = first_line_containing_any(
            input.source,
            &[
                ValidationMarker::from_static("?? 0"),
                ValidationMarker::from_static("?? ''"),
                ValidationMarker::from_static("?? \"\""),
            ],
        ) {
            return vec![finding!(
                &spec,
                "a required field falls back to a silent default (`?? 0`/`?? ''`) instead of \
                 validating then constructing — a missing value should fail construction, not \
                 silently substitute zero/empty.",
                &input,
                line,
            )];
        }

        if let Some(line) =
            first_line_containing(input.source, ValidationMarker::from_static(".data!"))
        {
            let lines: Vec<&str> = input.source.as_str().lines().collect();
            let idx = usize::try_from(line.value().get())
                .unwrap_or(usize::MAX)
                .saturating_sub(1);
            let has_justifying_comment = idx
                .checked_sub(1)
                .and_then(|previous| lines.get(previous))
                .is_some_and(|previous| previous.trim_start().starts_with("//"));
            if !has_justifying_comment {
                return vec![finding!(
                    &spec,
                    "bare `response.data!` unwrap with no justifying comment on the preceding \
                     line explaining why the value is guaranteed non-null.",
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
#[derive(Debug)]
pub struct FormStateMapValidator {
    rule_id: RuleId,
}

impl FormStateMapValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: BuiltInDartRule::FormStateMap.id(),
        })
    }
}

impl Validator for FormStateMapValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        let Some(line) = first_line_containing(
            input.source,
            ValidationMarker::from_static("Map<String, Object?>"),
        ) else {
            return Vec::new();
        };
        vec![finding!(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Warning,
                rule: BuiltInDartRule::FormStateMap,
            },
            "form/wizard state is a `Map<String, Object?>` — model it as a typed form-state \
             class so field access is compile-checked.",
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
