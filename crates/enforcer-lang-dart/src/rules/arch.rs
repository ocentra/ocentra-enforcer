//! Dart/Flutter layer-boundary + null-safety-discipline rules:
//! `DART-ARCH-1.1..1.4`, `DART-DOMAIN-1.1`, `DART-BANG-1.1`, and
//! `DART-FREEZED-1.1`. All are lightweight line/keyword-oriented text
//! detectors over the file's import block or field declarations —
//! mirroring `enforcer-lang-common::rules::fsm`'s dominant shape rather
//! than a full AST parse (this crate has no tree-sitter/AST dependency).

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::boundary::validation::ValidationMarker;
use enforcer_domain::ids::{BuiltInDartRule, RuleId};
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

use super::support::{first_line_containing, first_line_containing_any, FindingSpec};

/// `DART-ARCH-1.1..1.4` / `DART-DOMAIN-1.1` — layer/import boundaries.
///
/// Fires when a file under a `data/` directory imports a `presentation/`
/// path, OR when a file under a `domain/` directory imports
/// `package:flutter/...`. Both are layer-boundary escapes: `data` must
/// route reads back through `domain`, and `domain` must stay pure Dart.
#[derive(Debug)]
pub struct LayerBoundaryValidator {
    rule_id: RuleId,
}

impl LayerBoundaryValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: BuiltInDartRule::LayerBoundary.id(),
        })
    }
}

impl Validator for LayerBoundaryValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        let path = input.file.as_str();
        let spec = FindingSpec {
            rule_id: &self.rule_id,
            severity: Severity::Error,
            rule: BuiltInDartRule::LayerBoundary,
        };

        if path.contains("/data/") || path.contains("data/") {
            if let Some(line) = first_line_containing(
                input.source,
                ValidationMarker::from_static("/presentation/"),
            ) {
                return vec![finding!(
                    &spec,
                    "a `data/` file imports a `presentation/` path — data must route reads back \
                     through `domain/`, never reach into presentation directly.",
                    &input,
                    line,
                )];
            }
        }

        if path.contains("/domain/") || path.contains("domain/") {
            if let Some(line) = first_line_containing(
                input.source,
                ValidationMarker::from_static("package:flutter/"),
            ) {
                return vec![finding!(
                    &spec,
                    "a `domain/` file imports `package:flutter/...` — domain must stay pure Dart \
                     with no Flutter dependency.",
                    &input,
                    line,
                )];
            }
        }

        Vec::new()
    }
}

/// `DART-BANG-1.1` — no unchecked null-assertion `!` on a nullable
/// expression, and no unguarded `as` cast with no preceding `is` type
/// check in the same file. Both markers are common unsafe shapes:
/// `foo['id']!` unwrapping a possibly-null map lookup, and `x as Order`
/// with no `is Order` guard anywhere above it.
#[derive(Debug)]
pub struct NoUncheckedBangOrCastValidator {
    rule_id: RuleId,
}

impl NoUncheckedBangOrCastValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: BuiltInDartRule::UncheckedBangOrCast.id(),
        })
    }
}

impl Validator for NoUncheckedBangOrCastValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        let spec = FindingSpec {
            rule_id: &self.rule_id,
            severity: Severity::Error,
            rule: BuiltInDartRule::UncheckedBangOrCast,
        };

        // `']!` / `')!` catches the common `map['key']!` / `fn(...)!`
        // unwrap shape without tripping on Dart's `!=` operator.
        if let Some(line) = first_line_containing_any(
            input.source,
            &[
                ValidationMarker::from_static("']!"),
                ValidationMarker::from_static("\")!"),
            ],
        ) {
            return vec![finding!(
                &spec,
                "unchecked null-assertion `!` on a nullable lookup — use `tryParse`/a null-aware \
                 accessor with an explicit fallback instead.",
                &input,
                line,
            )];
        }

        if let Some(line) =
            first_line_containing(input.source, ValidationMarker::from_static(" as "))
        {
            if !input.source.as_str().contains(" is ") {
                return vec![finding!(
                    &spec,
                    "unguarded `as` cast with no preceding `is` type check anywhere in the file.",
                    &input,
                    line,
                )];
            }
        }

        Vec::new()
    }
}

/// `DART-FREEZED-1.1` — immutable entities via `@freezed`: a `class`
/// declaration with a mutable (non-`final`) field and a setter is
/// flagged; a `@freezed`-annotated class is exempt (freezed generates
/// immutable, `final`-field data classes).
#[derive(Debug)]
pub struct FreezedEntityValidator {
    rule_id: RuleId,
}

impl FreezedEntityValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: BuiltInDartRule::FreezedEntity.id(),
        })
    }
}

impl Validator for FreezedEntityValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        if input.source.as_str().contains("@freezed") {
            return Vec::new();
        }
        if !input.source.as_str().contains("class ") {
            return Vec::new();
        }
        let Some(line) = first_line_containing(input.source, ValidationMarker::from_static("set "))
        else {
            return Vec::new();
        };
        vec![finding!(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Error,
                rule: BuiltInDartRule::FreezedEntity,
            },
            "class declares a setter over a mutable field — entities should be immutable \
             `@freezed` value classes, not mutable objects with setters.",
            &input,
            line,
        )]
    }
}

/// Build every validator this module registers.
pub fn all() -> Result<Vec<Box<dyn Validator>>, DecodeError> {
    Ok(vec![
        Box::new(LayerBoundaryValidator::new()?),
        Box::new(NoUncheckedBangOrCastValidator::new()?),
        Box::new(FreezedEntityValidator::new()?),
    ])
}
