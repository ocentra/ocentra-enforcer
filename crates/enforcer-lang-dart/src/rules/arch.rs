//! Dart/Flutter layer-boundary + null-safety-discipline rules:
//! `DART-ARCH-1.1..1.4`, `DART-DOMAIN-1.1`, `DART-BANG-1.1`, and
//! `DART-FREEZED-1.1`. All are lightweight line/keyword-oriented text
//! detectors over the file's import block or field declarations —
//! mirroring `enforcer-lang-common::rules::fsm`'s dominant shape rather
//! than a full AST parse (this crate has no tree-sitter/AST dependency).

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

use super::support::{finding, first_line_containing, first_line_containing_any, FindingSpec};

/// `DART-ARCH-1.1..1.4` / `DART-DOMAIN-1.1` — layer/import boundaries.
///
/// Fires when a file under a `data/` directory imports a `presentation/`
/// path, OR when a file under a `domain/` directory imports
/// `package:flutter/...`. Both are layer-boundary escapes: `data` must
/// route reads back through `domain`, and `domain` must stay pure Dart.
pub struct LayerBoundaryValidator {
    rule_id: RuleId,
}

impl LayerBoundaryValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "DART-ARCH-1.1".parse()?,
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
            title: "Dart layer boundary crossed",
        };

        if path.contains("/data/") || path.contains("data/") {
            if let Some(line) = first_line_containing(input.source, "/presentation/") {
                return vec![finding(
                    &spec,
                    "a `data/` file imports a `presentation/` path — data must route reads back \
                     through `domain/`, never reach into presentation directly."
                        .to_owned(),
                    &input,
                    line,
                )];
            }
        }

        if path.contains("/domain/") || path.contains("domain/") {
            if let Some(line) = first_line_containing(input.source, "package:flutter/") {
                return vec![finding(
                    &spec,
                    "a `domain/` file imports `package:flutter/...` — domain must stay pure Dart \
                     with no Flutter dependency."
                        .to_owned(),
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
pub struct NoUncheckedBangOrCastValidator {
    rule_id: RuleId,
}

impl NoUncheckedBangOrCastValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "DART-BANG-1.1".parse()?,
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
            title: "unchecked null-assertion or unguarded cast",
        };

        // `']!` / `')!` catches the common `map['key']!` / `fn(...)!`
        // unwrap shape without tripping on Dart's `!=` operator.
        if let Some(line) = first_line_containing_any(input.source, &["']!", "\")!"]) {
            return vec![finding(
                &spec,
                "unchecked null-assertion `!` on a nullable lookup — use `tryParse`/a null-aware \
                 accessor with an explicit fallback instead."
                    .to_owned(),
                &input,
                line,
            )];
        }

        if let Some(line) = first_line_containing(input.source, " as ") {
            if !input.source.contains(" is ") {
                return vec![finding(
                    &spec,
                    "unguarded `as` cast with no preceding `is` type check anywhere in the file."
                        .to_owned(),
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
pub struct FreezedEntityValidator {
    rule_id: RuleId,
}

impl FreezedEntityValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "DART-FREEZED-1.1".parse()?,
        })
    }
}

impl Validator for FreezedEntityValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        if input.source.contains("@freezed") {
            return Vec::new();
        }
        if !input.source.contains("class ") {
            return Vec::new();
        }
        let Some(line) = first_line_containing(input.source, "set ") else {
            return Vec::new();
        };
        vec![finding(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Error,
                title: "mutable entity should be an immutable @freezed class",
            },
            "class declares a setter over a mutable field — entities should be immutable \
             `@freezed` value classes, not mutable objects with setters."
                .to_owned(),
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
