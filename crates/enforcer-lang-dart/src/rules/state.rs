//! Dart/Flutter state-management rules: `DART-STATE-1.1` (no
//! `ChangeNotifier` in new code), `DART-RIVERPOD-1.1` (Riverpod 2.x
//! `NotifierProvider`, ban legacy `StateNotifierProvider`),
//! `DART-STATE-1.2` (no `ref.read` in `build()`), `DART-STATE-1.3`
//! (scored: detail page mutates a list provider), and
//! `DART-INITSTATE-1.1` (scored: data fetch in `initState`).

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

use super::support::{finding, first_line_containing, FindingSpec};

/// `DART-STATE-1.1` — no `ChangeNotifier` in new code (Riverpod
/// `Notifier`/`AsyncNotifier` supersedes it).
pub struct NoChangeNotifierValidator {
    rule_id: RuleId,
}

impl NoChangeNotifierValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "DART-STATE-1.1".parse()?,
        })
    }
}

impl Validator for NoChangeNotifierValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        let Some(line) = first_line_containing(input.source, "extends ChangeNotifier") else {
            return Vec::new();
        };
        vec![finding(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Error,
                title: "ChangeNotifier used in new code",
            },
            "class extends `ChangeNotifier` — new code must use a Riverpod 2.x \
             `Notifier`/`AsyncNotifier` instead."
                .to_owned(),
            &input,
            line,
        )]
    }
}

/// `DART-RIVERPOD-1.1` — Riverpod 2.x `NotifierProvider` required; ban
/// legacy 1.x `StateNotifierProvider`.
pub struct LegacyStateNotifierProviderValidator {
    rule_id: RuleId,
}

impl LegacyStateNotifierProviderValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "DART-RIVERPOD-1.1".parse()?,
        })
    }
}

impl Validator for LegacyStateNotifierProviderValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        let Some(line) = first_line_containing(input.source, "StateNotifierProvider") else {
            return Vec::new();
        };
        vec![finding(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Error,
                title: "legacy StateNotifierProvider used",
            },
            "`StateNotifierProvider` is legacy Riverpod 1.x — use a `NotifierProvider` (Riverpod \
             2.x) instead."
                .to_owned(),
            &input,
            line,
        )]
    }
}

/// `DART-STATE-1.2` — no `ref.read` inside `build()`; use `ref.watch` so
/// the widget rebuilds on provider change.
pub struct RefReadInBuildValidator {
    rule_id: RuleId,
}

impl RefReadInBuildValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "DART-STATE-1.2".parse()?,
        })
    }
}

impl Validator for RefReadInBuildValidator {
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
            if line.contains("ref.read(") {
                return vec![finding(
                    &FindingSpec {
                        rule_id: &self.rule_id,
                        severity: Severity::Error,
                        title: "ref.read used inside build()",
                    },
                    "`ref.read(...)` is called inside `build()` — use `ref.watch(...)` so the \
                     widget rebuilds when the provider changes."
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

/// `DART-STATE-1.3` (scored) — a detail widget's `build()` mutates a
/// list provider directly (`ref.read(listProvider.notifier).update(...)`)
/// instead of emitting an event/navigating back with a result.
pub struct DetailMutatesListProviderValidator {
    rule_id: RuleId,
}

impl DetailMutatesListProviderValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "DART-STATE-1.3".parse()?,
        })
    }
}

impl Validator for DetailMutatesListProviderValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        let Some(line) = first_line_containing(input.source, "listProvider.notifier).update(")
        else {
            return Vec::new();
        };
        vec![finding(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Warning,
                title: "detail widget mutates a list provider directly (scored)",
            },
            "a detail widget mutates a list provider directly via \
             `ref.read(listProvider.notifier).update(...)` — emit an event or navigate back with \
             a result instead of reaching into shared list state."
                .to_owned(),
            &input,
            line,
        )]
    }
}

/// `DART-INITSTATE-1.1` (scored) — a data fetch kicked off from
/// `initState` (`fetch().then(...)` inside `initState`) instead of a
/// provider/`FutureBuilder`.
pub struct DataFetchInInitStateValidator {
    rule_id: RuleId,
}

impl DataFetchInInitStateValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "DART-INITSTATE-1.1".parse()?,
        })
    }
}

impl Validator for DataFetchInInitStateValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        let Some(init_start) = first_line_containing(input.source, "initState(") else {
            return Vec::new();
        };
        let lines: Vec<&str> = input.source.lines().collect();
        let mut depth: i64 = 0;
        let mut opened = false;
        for (idx, line) in lines
            .iter()
            .enumerate()
            .skip((init_start as usize).saturating_sub(1))
        {
            for ch in line.chars() {
                if ch == '{' {
                    depth += 1;
                    opened = true;
                } else if ch == '}' {
                    depth -= 1;
                }
            }
            if line.contains(".then(") {
                return vec![finding(
                    &FindingSpec {
                        rule_id: &self.rule_id,
                        severity: Severity::Warning,
                        title: "data fetch kicked off from initState (scored)",
                    },
                    "`initState` kicks off a data fetch with `.then(...)` — use a provider or \
                     `FutureBuilder` to drive the fetch instead of an imperative side-effect."
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
