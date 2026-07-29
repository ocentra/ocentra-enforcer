//! Dart toolchain/dependency-hygiene rules: `DART-TOOL-1.1..1.3`
//! (`analysis_options.yaml` present + strict, CI runs
//! `dart analyze --fatal-infos` + `dart format --set-exit-if-changed`),
//! `DART-DEP-1.1` (`pubspec.lock` committed + pinned `^` deps, ban
//! `foo: any`), and `DART-GEN-1.1` (never hand-edit generated
//! `.g.dart`/`.freezed.dart`).
//!
//! These validators inspect project-manifest/CI-config TEXT (a project
//! fixture's `analysis_options.yaml`, `pubspec.yaml`, CI workflow file,
//! or a generated-file diff) — they never shell out to `dart analyze`/
//! `dart format` themselves; that native invocation is
//! `enforcer-harness`'s (arc-18) run-adapter concern, not this crate's.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::boundary::validation::ValidationMarker;
use enforcer_domain::ids::{BuiltInDartRule, RuleId};
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

use super::support::{first_line_containing, FindingSpec};

/// `DART-TOOL-1.1` — a project must carry a strict `analysis_options.yaml`
/// (`include: package:flutter_lints/flutter.yaml` or a `strict-mode:`
/// block); this validator reads a manifest-marker fixture text standing
/// in for "this project's toolchain config", firing when the strict
/// marker is absent.
#[derive(Debug)]
pub struct StrictAnalysisOptionsValidator {
    rule_id: RuleId,
}

impl StrictAnalysisOptionsValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: BuiltInDartRule::StrictAnalysisOptions.id(),
        })
    }
}

impl Validator for StrictAnalysisOptionsValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        if input.source.as_str().contains("strict-mode:")
            || input.source.as_str().contains("flutter_lints")
        {
            return Vec::new();
        }
        vec![finding!(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Error,
                rule: BuiltInDartRule::StrictAnalysisOptions,
            },
            "no strict `analysis_options.yaml` configuration found (missing `strict-mode:` / a \
             `flutter_lints` include) — every Dart project needs a strict analyzer config.",
            &input,
            1_u32,
        )]
    }
}

/// `DART-TOOL-1.2` — CI must run `dart analyze --fatal-infos`.
#[derive(Debug)]
pub struct CiRunsAnalyzeValidator {
    rule_id: RuleId,
}

impl CiRunsAnalyzeValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: BuiltInDartRule::CiRunsAnalyze.id(),
        })
    }
}

impl Validator for CiRunsAnalyzeValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        if input.source.as_str().contains("dart analyze --fatal-infos") {
            return Vec::new();
        }
        vec![finding!(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Error,
                rule: BuiltInDartRule::CiRunsAnalyze,
            },
            "the CI workflow has no `dart analyze --fatal-infos` step — analyzer warnings/infos \
             must gate CI, not just errors.",
            &input,
            1_u32,
        )]
    }
}

/// `DART-TOOL-1.3` — CI must run `dart format --set-exit-if-changed`.
#[derive(Debug)]
pub struct CiRunsFormatCheckValidator {
    rule_id: RuleId,
}

impl CiRunsFormatCheckValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: BuiltInDartRule::CiRunsFormatCheck.id(),
        })
    }
}

impl Validator for CiRunsFormatCheckValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        if input
            .source
            .as_str()
            .contains("dart format --set-exit-if-changed")
        {
            return Vec::new();
        }
        vec![finding!(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Error,
                rule: BuiltInDartRule::CiRunsFormatCheck,
            },
            "the CI workflow has no `dart format --set-exit-if-changed` step — formatting drift \
             must fail CI.",
            &input,
            1_u32,
        )]
    }
}

/// `DART-DEP-1.1` — `pubspec.lock` must be committed (checked at the
/// repo-shape level elsewhere); here, `pubspec.yaml` dependency entries
/// must be pinned (`^x.y.z`), never `foo: any`.
#[derive(Debug)]
pub struct UnpinnedDependencyValidator {
    rule_id: RuleId,
}

impl UnpinnedDependencyValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: BuiltInDartRule::UnpinnedDependency.id(),
        })
    }
}

impl Validator for UnpinnedDependencyValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        let Some(line) =
            first_line_containing(input.source, ValidationMarker::from_static(": any"))
        else {
            return Vec::new();
        };
        vec![finding!(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Error,
                rule: BuiltInDartRule::UnpinnedDependency,
            },
            "a `pubspec.yaml` dependency is constrained to `any` — pin it to a caret range \
             (`^1.2.0`) instead.",
            &input,
            line,
        )]
    }
}

/// `DART-GEN-1.1` — never hand-edit a generated `.g.dart`/`.freezed.dart`
/// file. This validator inspects a generated file's own text: a
/// generated file must start with its standard `// GENERATED CODE` (or
/// `// coverage:ignore-file` codegen) marker; a hand-edit that strips
/// the marker (while the filename still ends `.g.dart`/`.freezed.dart`)
/// is caught by the fixture pairing itself — the marker's ABSENCE is
/// what this validator fires on.
#[derive(Debug)]
pub struct HandEditedGeneratedFileValidator {
    rule_id: RuleId,
}

impl HandEditedGeneratedFileValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: BuiltInDartRule::HandEditedGeneratedFile.id(),
        })
    }
}

impl Validator for HandEditedGeneratedFileValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        let path = input.file.as_str();
        let is_generated = path.ends_with(".g.dart") || path.ends_with(".freezed.dart");
        if !is_generated {
            return Vec::new();
        }
        if input.source.as_str().contains("GENERATED CODE") {
            return Vec::new();
        }
        vec![finding!(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Error,
                rule: BuiltInDartRule::HandEditedGeneratedFile,
            },
            "this `.g.dart`/`.freezed.dart` file has no `// GENERATED CODE` marker — it looks \
             hand-edited; regenerate it with `build_runner` instead of editing it directly.",
            &input,
            1_u32,
        )]
    }
}

/// Build every validator this module registers.
pub fn all() -> Result<Vec<Box<dyn Validator>>, DecodeError> {
    Ok(vec![
        Box::new(StrictAnalysisOptionsValidator::new()?),
        Box::new(CiRunsAnalyzeValidator::new()?),
        Box::new(CiRunsFormatCheckValidator::new()?),
        Box::new(UnpinnedDependencyValidator::new()?),
        Box::new(HandEditedGeneratedFileValidator::new()?),
    ])
}
