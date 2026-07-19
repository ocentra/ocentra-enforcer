//! Dart/Flutter widget-composition and UI-perf/style rules:
//! `DART-COMP-1.1`/`1.2` (one public widget per file, `{super.key}`
//! first), `DART-PERF-1.1`/`2.1` (list-builder perf, no `setState` in
//! `build()`), `DART-COLOR-1.1` (scored: hardcoded color literal),
//! `DART-NAV-2.*` (scored: imperative navigation), and `DART-L10N-2.1`
//! (scored: hardcoded user-facing string).

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::boundary::validation::{
    DartWidgetMultiplicity, ValidationMarker, ValidationSource,
};
use enforcer_domain::ids::{BuiltInDartRule, RuleId};
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};
use std::fmt;

use super::support::{first_line_containing, FindingSpec};

/// Count top-level public widget class declarations
/// (`class Foo extends StatelessWidget|StatefulWidget`, no leading
/// underscore) in `source`.
fn public_widget_multiplicity(source: ValidationSource<'_>) -> DartWidgetMultiplicity {
    match source
        .as_str()
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            trimmed.starts_with("class ")
                && !trimmed.starts_with("class _")
                && (trimmed.contains("extends StatelessWidget")
                    || trimmed.contains("extends StatefulWidget")
                    || trimmed.contains("extends ConsumerWidget")
                    || trimmed.contains("extends ConsumerStatefulWidget"))
        })
        .count()
    {
        0 => DartWidgetMultiplicity::None,
        1 => DartWidgetMultiplicity::One,
        _ => DartWidgetMultiplicity::Multiple,
    }
}

/// `DART-COMP-1.1` — one public widget per file.
#[derive(Debug)]
pub struct OnePublicWidgetPerFileValidator {
    rule_id: RuleId,
}

impl OnePublicWidgetPerFileValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: BuiltInDartRule::OnePublicWidgetPerFile.id(),
        })
    }
}

impl Validator for OnePublicWidgetPerFileValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        if !matches!(
            public_widget_multiplicity(input.source),
            DartWidgetMultiplicity::Multiple
        ) {
            return Vec::new();
        }
        vec![finding!(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Error,
                rule: BuiltInDartRule::OnePublicWidgetPerFile,
            },
            "this file declares more than one public widget class — split each public widget \
             into its own file.",
            &input,
            1_u32,
        )]
    }
}

/// `DART-COMP-1.2` — a widget constructor's first parameter must be
/// `{super.key`. Fires when a `const Foo(` constructor's parameter list
/// does not start with `{super.key`.
pub struct SuperKeyFirstParamValidator {
    rule_id: RuleId,
}

impl fmt::Debug for SuperKeyFirstParamValidator {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SuperKeyFirstParamValidator(REDACTED)")
    }
}

impl SuperKeyFirstParamValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: BuiltInDartRule::SuperKeyFirstParam.id(),
        })
    }
}

impl Validator for SuperKeyFirstParamValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        if matches!(
            public_widget_multiplicity(input.source),
            DartWidgetMultiplicity::None
        ) {
            return Vec::new();
        }
        for (idx, line) in input.source.as_str().lines().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("const ")
                && trimmed.contains('(')
                && !trimmed.contains("super.key")
            {
                // Only a constructor-shaped line (capitalized name right
                // after `const `) counts, not an arbitrary `const` value.
                let after_const = trimmed.trim_start_matches("const ");
                let starts_with_type_name = after_const
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_uppercase());
                if starts_with_type_name {
                    return vec![finding!(
                        &FindingSpec {
                            rule_id: &self.rule_id,
                            severity: Severity::Error,
                            rule: BuiltInDartRule::SuperKeyFirstParam,
                        },
                        "widget constructor does not declare `{super.key, ...}` — `super.key` \
                         must be the first named parameter.",
                        &input,
                        idx.saturating_add(1),
                    )];
                }
            }
        }
        Vec::new()
    }
}

/// `DART-PERF-1.1` — `ListView.builder` required for long/dynamic
/// lists; bans `ListView(children: items.map(...).toList())`.
#[derive(Debug)]
pub struct ListViewBuilderRequiredValidator {
    rule_id: RuleId,
}

impl ListViewBuilderRequiredValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: BuiltInDartRule::ListViewBuilderRequired.id(),
        })
    }
}

impl Validator for ListViewBuilderRequiredValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        let Some(line) = first_line_containing(
            input.source,
            ValidationMarker::from_static("ListView(children:"),
        ) else {
            return Vec::new();
        };
        if !input.source.as_str().contains(".map(") {
            return Vec::new();
        }
        vec![finding!(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Error,
                rule: BuiltInDartRule::ListViewBuilderRequired,
            },
            "a `ListView(children: ...)` is built from a mapped collection — use \
             `ListView.builder(itemCount: ..., itemBuilder: ...)` for long/dynamic lists.",
            &input,
            line,
        )]
    }
}

/// `DART-PERF-2.1` — no `setState(` call inside `build()`.
#[derive(Debug)]
pub struct SetStateInBuildValidator {
    rule_id: RuleId,
}

impl SetStateInBuildValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: BuiltInDartRule::SetStateInBuild.id(),
        })
    }
}

impl Validator for SetStateInBuildValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        let Some(build_start) =
            first_line_containing(input.source, ValidationMarker::from_static(" build("))
        else {
            return Vec::new();
        };
        let lines: Vec<&str> = input.source.as_str().lines().collect();
        let mut depth: i64 = 0;
        let mut opened = false;
        // BRAND-INVARIANT: SourceLine is positive and one-based; this checked
        // conversion creates only the zero-based iterator offset.
        for (idx, line) in lines.iter().enumerate().skip(
            usize::try_from(build_start.value().get())
                .unwrap_or(usize::MAX)
                .saturating_sub(1),
        ) {
            for ch in line.chars() {
                if ch == '{' {
                    depth += 1;
                    opened = true;
                } else if ch == '}' {
                    depth -= 1;
                }
            }
            if line.contains("setState(") {
                return vec![finding!(
                    &FindingSpec {
                        rule_id: &self.rule_id,
                        severity: Severity::Error,
                        rule: BuiltInDartRule::SetStateInBuild,
                    },
                    "`setState(...)` is called from inside `build()` — call it only from event \
                     handlers/callbacks, never from the build method itself.",
                    &input,
                    idx.saturating_add(1),
                )];
            }
            if opened && depth <= 0 {
                break;
            }
        }
        Vec::new()
    }
}

/// `DART-COLOR-1.1` (scored) — hardcoded color literal (`Color(0xFF...)`)
/// inside a widget instead of a theme reference.
#[derive(Debug)]
pub struct HardcodedColorValidator {
    rule_id: RuleId,
}

impl HardcodedColorValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: BuiltInDartRule::HardcodedColor.id(),
        })
    }
}

impl Validator for HardcodedColorValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        let Some(line) =
            first_line_containing(input.source, ValidationMarker::from_static("Color(0xFF"))
        else {
            return Vec::new();
        };
        vec![finding!(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Warning,
                rule: BuiltInDartRule::HardcodedColor,
            },
            "a hardcoded `Color(0xFF...)` literal appears in widget code — use \
             `Theme.of(context).colorScheme...` instead so the color follows the app theme.",
            &input,
            line,
        )]
    }
}

/// `DART-NAV-2.*` (scored) — imperative `Navigator.push` with a
/// hardcoded route instead of declarative GoRouter navigation.
#[derive(Debug)]
pub struct ImperativeNavigationValidator {
    rule_id: RuleId,
}

impl ImperativeNavigationValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: BuiltInDartRule::ImperativeNavigation.id(),
        })
    }
}

impl Validator for ImperativeNavigationValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        let Some(line) = first_line_containing(
            input.source,
            ValidationMarker::from_static("Navigator.push("),
        ) else {
            return Vec::new();
        };
        vec![finding!(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Warning,
                rule: BuiltInDartRule::ImperativeNavigation,
            },
            "`Navigator.push(...)` is used imperatively — prefer declarative GoRouter navigation \
             (`context.go('/named-route')`) with a named route.",
            &input,
            line,
        )]
    }
}

/// `DART-L10N-2.1` (scored) — hardcoded user-facing string literal in a
/// `Text(...)` widget instead of an l10n lookup.
#[derive(Debug)]
pub struct HardcodedUserStringValidator {
    rule_id: RuleId,
}

impl HardcodedUserStringValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: BuiltInDartRule::HardcodedUserString.id(),
        })
    }
}

impl Validator for HardcodedUserStringValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        for (idx, line) in input.source.as_str().lines().enumerate() {
            let trimmed = line.trim();
            if (trimmed.starts_with("Text('") || trimmed.starts_with("Text(\""))
                && !trimmed.contains("l10n.")
            {
                return vec![finding!(
                    &FindingSpec {
                        rule_id: &self.rule_id,
                        severity: Severity::Warning,
                        rule: BuiltInDartRule::HardcodedUserString,
                    },
                    "a `Text(...)` widget carries a hardcoded string literal — route it through \
                     `l10n.<key>` instead so it can be localized.",
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
        Box::new(OnePublicWidgetPerFileValidator::new()?),
        Box::new(SuperKeyFirstParamValidator::new()?),
        Box::new(ListViewBuilderRequiredValidator::new()?),
        Box::new(SetStateInBuildValidator::new()?),
        Box::new(HardcodedColorValidator::new()?),
        Box::new(ImperativeNavigationValidator::new()?),
        Box::new(HardcodedUserStringValidator::new()?),
    ])
}
