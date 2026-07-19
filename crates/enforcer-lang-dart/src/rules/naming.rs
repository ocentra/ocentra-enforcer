//! Dart naming/style rules: `DART-NAME-1.1` (snake_case filename matches
//! its public widget), `DART-IMP-1.1` (scored: ungrouped import order),
//! and `DART-STYLE-2.1` (scored: string interpolation over
//! concatenation).

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::boundary::validation::{
    DartFilenameStem, DartImportGroup, DartWidgetName, ValidationSource,
};
use enforcer_domain::ids::{BuiltInDartRule, RuleId};
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

use super::support::FindingSpec;

/// Convert a `PascalCase`/`camelCase` widget name to its expected
/// `snake_case` filename stem (e.g. `OrderCard` -> `order_card`).
fn expected_snake_case_stem(widget_name: &DartWidgetName) -> Option<DartFilenameStem> {
    let mut out = String::new();
    for (idx, ch) in widget_name.as_str().chars().enumerate() {
        if ch.is_ascii_uppercase() {
            if idx != 0 {
                out.push('_');
            }
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push(ch);
        }
    }
    match DartFilenameStem::try_new(out) {
        Ok(stem) => Some(stem),
        Err(_) => None,
    }
}

/// `DART-NAME-1.1` — the file's snake_case stem must match its public
/// widget class name (`order_card.dart` for `class OrderCard`).
#[derive(Debug)]
pub struct SnakeCaseFilenameValidator {
    rule_id: RuleId,
}

impl SnakeCaseFilenameValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: BuiltInDartRule::SnakeCaseFilename.id(),
        })
    }
}

impl Validator for SnakeCaseFilenameValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        let path = input.file.as_str();
        let Some(file_name) = path.rsplit('/').next() else {
            return Vec::new();
        };
        let Some(stem) = file_name.strip_suffix(".dart") else {
            return Vec::new();
        };

        let Some(widget_name) = input.source.as_str().lines().find_map(|line| {
            let trimmed = line.trim_start();
            trimmed
                .strip_prefix("class ")
                .and_then(|rest| rest.split_whitespace().next())
                .filter(|name| name.chars().next().is_some_and(|c| c.is_ascii_uppercase()))
                .and_then(|name| {
                    // ALLOC-JUSTIFICATION: the validated class name is owned by the
                    // domain brand before it is transformed into a filename stem.
                    match DartWidgetName::try_new(name.to_owned()) {
                        Ok(widget_name) => Some(widget_name),
                        Err(_) => None,
                    }
                })
        }) else {
            return Vec::new();
        };

        let Some(expected) = expected_snake_case_stem(&widget_name) else {
            return Vec::new();
        };
        if stem == expected.as_str() {
            return Vec::new();
        }
        vec![finding!(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Error,
                rule: BuiltInDartRule::SnakeCaseFilename,
            },
            format!(
                "file `{file_name}` declares `class {}` but the filename stem is not its snake_case \
                 form (`{}.dart`).",
                widget_name.as_str(),
                expected.as_str()
            ),
            &input,
            1_u32,
        )]
    }
}

/// The three canonical Dart import groups, in required order: `dart:`,
/// `package:`, then relative (`./`/`../`/bare-filename) imports.
fn import_group(line: ValidationSource<'_>) -> Option<DartImportGroup> {
    let trimmed = line.as_str().trim_start();
    if !trimmed.starts_with("import ") {
        return None;
    }
    if trimmed.contains("'dart:") {
        Some(DartImportGroup::Dart)
    } else if trimmed.contains("'package:") {
        Some(DartImportGroup::Package)
    } else {
        Some(DartImportGroup::Relative)
    }
}

/// `DART-IMP-1.1` (scored) — imports must be grouped `dart:` ->
/// `package:` -> relative, not interleaved.
#[derive(Debug)]
pub struct UngroupedImportsValidator {
    rule_id: RuleId,
}

impl UngroupedImportsValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: BuiltInDartRule::UngroupedImports.id(),
        })
    }
}

impl Validator for UngroupedImportsValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        let mut max_seen = None;
        for (idx, line) in input.source.as_str().lines().enumerate() {
            let Some(group) = import_group(ValidationSource::from_text(line)) else {
                continue;
            };
            if max_seen.is_some_and(|seen| group < seen) {
                return vec![finding!(
                    &FindingSpec {
                        rule_id: &self.rule_id,
                        severity: Severity::Warning,
                        rule: BuiltInDartRule::UngroupedImports,
                    },
                    "imports are interleaved instead of grouped `dart:` -> `package:` -> \
                     relative — reorder them into the three canonical groups.",
                    &input,
                    idx.saturating_add(1),
                )];
            }
            max_seen = Some(group);
        }
        Vec::new()
    }
}

/// `DART-STYLE-2.1` (scored) — string interpolation preferred over `+`
/// concatenation.
#[derive(Debug)]
pub struct StringConcatenationValidator {
    rule_id: RuleId,
}

impl StringConcatenationValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: BuiltInDartRule::StringConcatenation.id(),
        })
    }
}

impl Validator for StringConcatenationValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        for (idx, line) in input.source.as_str().lines().enumerate() {
            let trimmed = line.trim();
            let looks_like_string_concat =
                (trimmed.contains("' + ") || trimmed.contains("\" + ")) && trimmed.contains('\'');
            if looks_like_string_concat {
                return vec![finding!(
                    &FindingSpec {
                        rule_id: &self.rule_id,
                        severity: Severity::Warning,
                        rule: BuiltInDartRule::StringConcatenation,
                    },
                    "a string is built with `+` concatenation — use `'...$expr...'` \
                     interpolation instead.",
                    &input,
                    idx.saturating_add(1),
                )];
            }
        }
        Vec::new()
    }
}

/// Build every validator this module registers.
pub fn all() -> Result<Vec<Box<dyn Validator>>, DecodeError> {
    Ok(vec![
        Box::new(SnakeCaseFilenameValidator::new()?),
        Box::new(UngroupedImportsValidator::new()?),
        Box::new(StringConcatenationValidator::new()?),
    ])
}

#[cfg(test)]
mod tests {
    use super::{DartFilenameStem, DartWidgetName};

    #[test]
    fn dart_name_brands_reject_invalid_input() {
        for invalid in ["", "widget", "Widget-Card"] {
            assert!(DartWidgetName::try_new(invalid.to_owned()).is_err());
        }
        for invalid_stem in ["", "OrderCard", "order-card"] {
            assert!(DartFilenameStem::try_new(invalid_stem.to_owned()).is_err());
        }
    }
}
