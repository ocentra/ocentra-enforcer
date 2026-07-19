//! Dart/Flutter state-management rules: `DART-STATE-1.1` (no
//! `ChangeNotifier` in new code), `DART-RIVERPOD-1.1` (Riverpod 2.x
//! `NotifierProvider`, ban legacy `StateNotifierProvider`),
//! `DART-STATE-1.2` (no `ref.read` in `build()`), `DART-STATE-1.3`
//! (scored: detail page mutates a list provider), and
//! `DART-INITSTATE-1.1` (scored: data fetch in `initState`).

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::boundary::validation::ValidationMarker;
use enforcer_domain::ids::{BuiltInDartRule, RuleId};
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

use super::support::{first_line_containing, FindingSpec};

/// `DART-STATE-1.1` — no `ChangeNotifier` in new code (Riverpod
/// `Notifier`/`AsyncNotifier` supersedes it).
#[derive(Debug)]
pub struct NoChangeNotifierValidator {
    rule_id: RuleId,
}

impl NoChangeNotifierValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: BuiltInDartRule::NoChangeNotifier.id(),
        })
    }
}

impl Validator for NoChangeNotifierValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        let Some(line) = first_line_containing(
            input.source,
            ValidationMarker::from_static("extends ChangeNotifier"),
        ) else {
            return Vec::new();
        };
        vec![finding!(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Error,
                rule: BuiltInDartRule::NoChangeNotifier,
            },
            "class extends `ChangeNotifier` — new code must use a Riverpod 2.x \
             `Notifier`/`AsyncNotifier` instead.",
            &input,
            line,
        )]
    }
}

/// `DART-RIVERPOD-1.1` — Riverpod 2.x `NotifierProvider` required; ban
/// legacy 1.x `StateNotifierProvider`.
#[derive(Debug)]
pub struct LegacyStateNotifierProviderValidator {
    rule_id: RuleId,
}

impl LegacyStateNotifierProviderValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: BuiltInDartRule::LegacyStateNotifierProvider.id(),
        })
    }
}

impl Validator for LegacyStateNotifierProviderValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        let Some(line) = first_line_containing(
            input.source,
            ValidationMarker::from_static("StateNotifierProvider"),
        ) else {
            return Vec::new();
        };
        vec![finding!(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Error,
                rule: BuiltInDartRule::LegacyStateNotifierProvider,
            },
            "`StateNotifierProvider` is legacy Riverpod 1.x — use a `NotifierProvider` (Riverpod \
             2.x) instead.",
            &input,
            line,
        )]
    }
}

/// `DART-STATE-1.2` — no `ref.read` inside `build()`; use `ref.watch` so
/// the widget rebuilds on provider change.
#[derive(Debug)]
pub struct RefReadInBuildValidator {
    rule_id: RuleId,
}

impl RefReadInBuildValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: BuiltInDartRule::RefReadInBuild.id(),
        })
    }
}

impl Validator for RefReadInBuildValidator {
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
            if line.contains("ref.read(") {
                return vec![finding!(
                    &FindingSpec {
                        rule_id: &self.rule_id,
                        severity: Severity::Error,
                        rule: BuiltInDartRule::RefReadInBuild,
                    },
                    "`ref.read(...)` is called inside `build()` — use `ref.watch(...)` so the \
                     widget rebuilds when the provider changes.",
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

/// `DART-STATE-1.3` (scored) — a detail widget's `build()` mutates a
/// list provider directly (`ref.read(listProvider.notifier).update(...)`)
/// instead of emitting an event/navigating back with a result.
#[derive(Debug)]
pub struct DetailMutatesListProviderValidator {
    rule_id: RuleId,
}

impl DetailMutatesListProviderValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: BuiltInDartRule::DetailMutatesListProvider.id(),
        })
    }
}

impl Validator for DetailMutatesListProviderValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        let Some(line) = first_line_containing(
            input.source,
            ValidationMarker::from_static("listProvider.notifier).update("),
        ) else {
            return Vec::new();
        };
        vec![finding!(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Warning,
                rule: BuiltInDartRule::DetailMutatesListProvider,
            },
            "a detail widget mutates a list provider directly via \
             `ref.read(listProvider.notifier).update(...)` — emit an event or navigate back with \
             a result instead of reaching into shared list state.",
            &input,
            line,
        )]
    }
}

/// `DART-INITSTATE-1.1` (scored) — a data fetch kicked off from
/// `initState` (`fetch().then(...)` inside `initState`) instead of a
/// provider/`FutureBuilder`.
#[derive(Debug)]
pub struct DataFetchInInitStateValidator {
    rule_id: RuleId,
}

impl DataFetchInInitStateValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: BuiltInDartRule::DataFetchInInitState.id(),
        })
    }
}

impl Validator for DataFetchInInitStateValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        let Some(init_start) =
            first_line_containing(input.source, ValidationMarker::from_static("initState("))
        else {
            return Vec::new();
        };
        let lines: Vec<&str> = input.source.as_str().lines().collect();
        let mut depth: i64 = 0;
        let mut opened = false;
        // BRAND-INVARIANT: SourceLine is positive and one-based; this checked
        // conversion creates only the zero-based iterator offset.
        for (idx, line) in lines.iter().enumerate().skip(
            usize::try_from(init_start.value().get())
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
            if line.contains(".then(") {
                return vec![finding!(
                    &FindingSpec {
                        rule_id: &self.rule_id,
                        severity: Severity::Warning,
                        rule: BuiltInDartRule::DataFetchInInitState,
                    },
                    "`initState` kicks off a data fetch with `.then(...)` — use a provider or \
                     `FutureBuilder` to drive the fetch instead of an imperative side-effect.",
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

/// Build every validator this module registers.
pub fn all() -> Result<Vec<Box<dyn Validator>>, DecodeError> {
    Ok(vec![
        Box::new(NoChangeNotifierValidator::new()?),
        Box::new(LegacyStateNotifierProviderValidator::new()?),
        Box::new(RefReadInBuildValidator::new()?),
        Box::new(DetailMutatesListProviderValidator::new()?),
        Box::new(DataFetchInInitStateValidator::new()?),
    ])
}
