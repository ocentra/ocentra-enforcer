//! Dart/Flutter widget-composition and UI-perf/style rules:
//! `DART-COMP-1.1`/`1.2` (one public widget per file, `{super.key}`
//! first), `DART-PERF-1.1`/`2.1` (list-builder perf, no `setState` in
//! `build()`), `DART-COLOR-1.1` (scored: hardcoded color literal),
//! `DART-NAV-2.*` (scored: imperative navigation), and `DART-L10N-2.1`
//! (scored: hardcoded user-facing string).

use enforcer_core::error::DecodeError;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

use super::support::{finding, first_line_containing, FindingSpec};

/// Count top-level public widget class declarations
/// (`class Foo extends StatelessWidget|StatefulWidget`, no leading
/// underscore) in `source`.
fn public_widget_class_count(source: &str) -> usize {
    source
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
}

/// `DART-COMP-1.1` — one public widget per file.
pub struct OnePublicWidgetPerFileValidator {
    rule_id: RuleId,
}

impl OnePublicWidgetPerFileValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "DART-COMP-1.1".parse()?,
        })
    }
}

impl Validator for OnePublicWidgetPerFileValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        if public_widget_class_count(input.source) <= 1 {
            return Vec::new();
        }
        vec![finding(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Error,
                title: "more than one public widget declared in this file",
            },
            "this file declares more than one public widget class — split each public widget \
             into its own file."
                .to_owned(),
            &input,
            1,
        )]
    }
}

/// `DART-COMP-1.2` — a widget constructor's first parameter must be
/// `{super.key`. Fires when a `const Foo(` constructor's parameter list
/// does not start with `{super.key`.
pub struct SuperKeyFirstParamValidator {
    rule_id: RuleId,
}

impl SuperKeyFirstParamValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "DART-COMP-1.2".parse()?,
        })
    }
}

impl Validator for SuperKeyFirstParamValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        if public_widget_class_count(input.source) == 0 {
            return Vec::new();
        }
        for (idx, line) in input.source.lines().enumerate() {
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
                    return vec![finding(
                        &FindingSpec {
                            rule_id: &self.rule_id,
                            severity: Severity::Error,
                            title: "widget constructor missing {super.key} first param",
                        },
                        "widget constructor does not declare `{super.key, ...}` — `super.key` \
                         must be the first named parameter."
                            .to_owned(),
                        &input,
                        (idx as u32).saturating_add(1),
                    )];
                }
            }
        }
        Vec::new()
    }
}

/// `DART-PERF-1.1` — `ListView.builder` required for long/dynamic
/// lists; bans `ListView(children: items.map(...).toList())`.
pub struct ListViewBuilderRequiredValidator {
    rule_id: RuleId,
}

impl ListViewBuilderRequiredValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "DART-PERF-1.1".parse()?,
        })
    }
}

impl Validator for ListViewBuilderRequiredValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        let Some(line) = first_line_containing(input.source, "ListView(children:") else {
            return Vec::new();
        };
        if !input.source.contains(".map(") {
            return Vec::new();
        }
        vec![finding(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Error,
                title: "ListView(children: ...map...) over a dynamic collection",
            },
            "a `ListView(children: ...)` is built from a mapped collection — use \
             `ListView.builder(itemCount: ..., itemBuilder: ...)` for long/dynamic lists."
                .to_owned(),
            &input,
            line,
        )]
    }
}

/// `DART-PERF-2.1` — no `setState(` call inside `build()`.
pub struct SetStateInBuildValidator {
    rule_id: RuleId,
}

impl SetStateInBuildValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "DART-PERF-2.1".parse()?,
        })
    }
}

impl Validator for SetStateInBuildValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        let Some(build_start) = first_line_containing(input.source, " build(") else {
            return Vec::new();
        };
        let lines: Vec<&str> = input.source.lines().collect();
        let mut depth: i64 = 0;
        let mut opened = false;
        for (idx, line) in lines
            .iter()
            .enumerate()
            .skip((build_start as usize).saturating_sub(1))
        {
            for ch in line.chars() {
                if ch == '{' {
                    depth += 1;
                    opened = true;
                } else if ch == '}' {
                    depth -= 1;
                }
            }
            if line.contains("setState(") {
                return vec![finding(
                    &FindingSpec {
                        rule_id: &self.rule_id,
                        severity: Severity::Error,
                        title: "setState() called inside build()",
                    },
                    "`setState(...)` is called from inside `build()` — call it only from event \
                     handlers/callbacks, never from the build method itself."
                        .to_owned(),
                    &input,
                    (idx as u32).saturating_add(1),
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
pub struct HardcodedColorValidator {
    rule_id: RuleId,
}

impl HardcodedColorValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "DART-COLOR-1.1".parse()?,
        })
    }
}

impl Validator for HardcodedColorValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        let Some(line) = first_line_containing(input.source, "Color(0xFF") else {
            return Vec::new();
        };
        vec![finding(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Warning,
                title: "hardcoded color literal in widget (scored)",
            },
            "a hardcoded `Color(0xFF...)` literal appears in widget code — use \
             `Theme.of(context).colorScheme...` instead so the color follows the app theme."
                .to_owned(),
            &input,
            line,
        )]
    }
}

/// `DART-NAV-2.*` (scored) — imperative `Navigator.push` with a
/// hardcoded route instead of declarative GoRouter navigation.
pub struct ImperativeNavigationValidator {
    rule_id: RuleId,
}

impl ImperativeNavigationValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "DART-NAV-2.1".parse()?,
        })
    }
}

impl Validator for ImperativeNavigationValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        let Some(line) = first_line_containing(input.source, "Navigator.push(") else {
            return Vec::new();
        };
        vec![finding(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Warning,
                title: "imperative Navigator.push instead of declarative routing (scored)",
            },
            "`Navigator.push(...)` is used imperatively — prefer declarative GoRouter navigation \
             (`context.go('/named-route')`) with a named route."
                .to_owned(),
            &input,
            line,
        )]
    }
}

/// `DART-L10N-2.1` (scored) — hardcoded user-facing string literal in a
/// `Text(...)` widget instead of an l10n lookup.
pub struct HardcodedUserStringValidator {
    rule_id: RuleId,
}

impl HardcodedUserStringValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "DART-L10N-2.1".parse()?,
        })
    }
}

impl Validator for HardcodedUserStringValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        for (idx, line) in input.source.lines().enumerate() {
            let trimmed = line.trim();
            if (trimmed.starts_with("Text('") || trimmed.starts_with("Text(\""))
                && !trimmed.contains("l10n.")
            {
                return vec![finding(
                    &FindingSpec {
                        rule_id: &self.rule_id,
                        severity: Severity::Warning,
                        title: "hardcoded user-facing string (scored)",
                    },
                    "a `Text(...)` widget carries a hardcoded string literal — route it through \
                     `l10n.<key>` instead so it can be localized."
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
        Box::new(OnePublicWidgetPerFileValidator::new()?),
        Box::new(SuperKeyFirstParamValidator::new()?),
        Box::new(ListViewBuilderRequiredValidator::new()?),
        Box::new(SetStateInBuildValidator::new()?),
        Box::new(HardcodedColorValidator::new()?),
        Box::new(ImperativeNavigationValidator::new()?),
        Box::new(HardcodedUserStringValidator::new()?),
    ])
}
