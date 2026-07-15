//! Dart naming/style rules: `DART-NAME-1.1` (snake_case filename matches
//! its public widget), `DART-IMP-1.1` (scored: ungrouped import order),
//! and `DART-STYLE-2.1` (scored: string interpolation over
//! concatenation).

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

use super::support::{finding, FindingSpec};

/// Convert a `PascalCase`/`camelCase` widget name to its expected
/// `snake_case` filename stem (e.g. `OrderCard` -> `order_card`).
fn expected_snake_case_stem(widget_name: &str) -> String {
    let mut out = String::new();
    for (idx, ch) in widget_name.chars().enumerate() {
        if ch.is_ascii_uppercase() {
            if idx != 0 {
                out.push('_');
            }
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}

/// `DART-NAME-1.1` — the file's snake_case stem must match its public
/// widget class name (`order_card.dart` for `class OrderCard`).
pub struct SnakeCaseFilenameValidator {
    rule_id: RuleId,
}

impl SnakeCaseFilenameValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "DART-NAME-1.1".parse()?,
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

        let Some(widget_name) = input.source.lines().find_map(|line| {
            let trimmed = line.trim_start();
            trimmed
                .strip_prefix("class ")
                .and_then(|rest| rest.split_whitespace().next())
                .filter(|name| name.chars().next().is_some_and(|c| c.is_ascii_uppercase()))
        }) else {
            return Vec::new();
        };

        let expected = expected_snake_case_stem(widget_name);
        if stem == expected {
            return Vec::new();
        }
        vec![finding(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Error,
                title: "filename does not match its widget in snake_case",
            },
            format!(
                "file `{file_name}` declares `class {widget_name}` but the filename stem is not \
                 its snake_case form (`{expected}.dart`)."
            ),
            &input,
            1,
        )]
    }
}

/// The three canonical Dart import groups, in required order: `dart:`,
/// `package:`, then relative (`./`/`../`/bare-filename) imports.
fn import_group(line: &str) -> Option<u8> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with("import ") {
        return None;
    }
    if trimmed.contains("'dart:") {
        Some(0)
    } else if trimmed.contains("'package:") {
        Some(1)
    } else {
        Some(2)
    }
}

/// `DART-IMP-1.1` (scored) — imports must be grouped `dart:` ->
/// `package:` -> relative, not interleaved.
pub struct UngroupedImportsValidator {
    rule_id: RuleId,
}

impl UngroupedImportsValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "DART-IMP-1.1".parse()?,
        })
    }
}

impl Validator for UngroupedImportsValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        let mut max_seen: i8 = -1;
        for (idx, line) in input.source.lines().enumerate() {
            let Some(group) = import_group(line) else {
                continue;
            };
            let group = group as i8;
            if group < max_seen {
                return vec![finding(
                    &FindingSpec {
                        rule_id: &self.rule_id,
                        severity: Severity::Warning,
                        title: "ungrouped/interleaved import order (scored)",
                    },
                    "imports are interleaved instead of grouped `dart:` -> `package:` -> \
                     relative — reorder them into the three canonical groups."
                        .to_owned(),
                    &input,
                    (idx as u32).saturating_add(1),
                )];
            }
            max_seen = group;
        }
        Vec::new()
    }
}

/// `DART-STYLE-2.1` (scored) — string interpolation preferred over `+`
/// concatenation.
pub struct StringConcatenationValidator {
    rule_id: RuleId,
}

impl StringConcatenationValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "DART-STYLE-2.1".parse()?,
        })
    }
}

impl Validator for StringConcatenationValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        for (idx, line) in input.source.lines().enumerate() {
            let trimmed = line.trim();
            let looks_like_string_concat =
                (trimmed.contains("' + ") || trimmed.contains("\" + ")) && trimmed.contains('\'');
            if looks_like_string_concat {
                return vec![finding(
                    &FindingSpec {
                        rule_id: &self.rule_id,
                        severity: Severity::Warning,
                        title: "string concatenation instead of interpolation (scored)",
                    },
                    "a string is built with `+` concatenation — use `'...$expr...'` \
                     interpolation instead."
                        .to_owned(),
                    &input,
                    (idx as u32).saturating_add(1),
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
