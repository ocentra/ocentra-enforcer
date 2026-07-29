//! CFML toolchain/CI hygiene rules: `box.json`/`.cflintrc` presence with
//! hard-gate `ERROR` severity (`CF-TOOL-1.1`/`CF-CI-1.1`), pinned deps
//! (`CF-DEP-1.1`), TestBox spec parity (`CF-TEST-1.1`), cfformat CI step
//! (`CF-TOOL-2.1`, scored), and TestBox coverage floor (`CF-CI-2.1`,
//! scored).
//!
//! These validators scan manifest/CI TEXT (JSON/YAML-shaped), not CFML
//! source -- their fixtures are `.json`/`.yaml`, mirroring
//! `enforcer-lang-dart::rules::toolchain`'s posture for `pubspec.yaml`/CI.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::ids::{BuiltInCfmlRule, RuleId};
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

use super::boundary::decode_json;
use super::support::FindingSpec;

const HARD_GATE_CODES: &[&str] = &["MISSING_VAR", "QUERYPARAM_REQ"];

/// `CF-TOOL-1.1` / `CF-CI-1.1` -- `.cflintrc` must be committed with every
/// hard-gate CFLint code set to `ERROR` severity. Fires on a `.cflintrc`
/// file where a tracked hard-gate code is present but NOT set to `ERROR`
/// (e.g. left at the CFLint default `WARNING`).
#[derive(Debug)]
pub struct CflintrcHardGateValidator {
    rule_id: RuleId,
}

impl CflintrcHardGateValidator {
    /// Construct the CFLint configuration validator.
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: BuiltInCfmlRule::CflintrcHardGate.id(),
        })
    }
}

impl Validator for CflintrcHardGateValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        if !input.file.as_str().ends_with(".cflintrc") {
            return Vec::new();
        }
        let parsed = decode_json(input.source);
        let Ok(value) = parsed else {
            return vec![finding!(
                &FindingSpec {
                    rule_id: &self.rule_id,
                    severity: Severity::Error,
                    rule: BuiltInCfmlRule::CflintrcHardGate,
                },
                "`.cflintrc` could not be parsed as JSON -- it must be valid JSON declaring \
                 rule severities.",
                &input,
                1,
            )];
        };
        let rule_field = value.get("rule").and_then(|r| r.as_array());
        for code in HARD_GATE_CODES {
            let severity = rule_field
                .and_then(|rules| {
                    rules
                        .iter()
                        .find(|entry| entry.get("code").and_then(|c| c.as_str()) == Some(*code))
                })
                .and_then(|entry| entry.get("severity"))
                .and_then(|s| s.as_str());
            if severity != Some("ERROR") {
                return vec![finding!(
                    &FindingSpec {
                        rule_id: &self.rule_id,
                        severity: Severity::Error,
                        rule: BuiltInCfmlRule::CflintrcHardGate,
                    },
                    format!(
                        "`.cflintrc` must set the hard-gate CFLint code `{code}` to `ERROR` \
                         severity; it is missing or set to a lower severity."
                    ),
                    &input,
                    1,
                )];
            }
        }
        Vec::new()
    }
}

/// `CF-DEP-1.1` -- pinned deps in `box.json`: no `"*"`/`"^"` wildcard
/// version range.
#[derive(Debug)]
pub struct PinnedDependencyValidator {
    rule_id: RuleId,
}

impl PinnedDependencyValidator {
    /// Construct the pinned-dependency validator.
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: BuiltInCfmlRule::PinnedDependency.id(),
        })
    }
}

impl Validator for PinnedDependencyValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        if !input.file.as_str().ends_with("box.json") {
            return Vec::new();
        }
        let parsed = decode_json(input.source);
        let Ok(value) = parsed else {
            return Vec::new();
        };
        let Some(deps) = value.get("dependencies").and_then(|d| d.as_object()) else {
            return Vec::new();
        };
        for (name, version) in deps {
            let Some(version_str) = version.as_str() else {
                continue;
            };
            if version_str.contains('*') || version_str.starts_with('^') {
                return vec![finding!(
                    &FindingSpec {
                        rule_id: &self.rule_id,
                        severity: Severity::Error,
                        rule: BuiltInCfmlRule::PinnedDependency,
                    },
                    format!(
                        "`box.json` dependency `{name}` is pinned to `{version_str}`, a \
                         wildcard/range version -- pin an exact version (e.g. `1.2.0`)."
                    ),
                    &input,
                    1,
                )];
            }
        }
        Vec::new()
    }
}

/// `CF-TEST-1.1` -- one TestBox spec per component, extending
/// `testbox.system.BaseSpec`. This validator's scope is the SPEC file
/// itself: a `*Test.cfc`/`*Spec.cfc` file that does not `extends=` a
/// `BaseSpec` is a violation.
#[derive(Debug)]
pub struct TestboxBaseSpecValidator {
    rule_id: RuleId,
}

impl TestboxBaseSpecValidator {
    /// Construct the TestBox base-spec validator.
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: BuiltInCfmlRule::TestboxBaseSpec.id(),
        })
    }
}

impl Validator for TestboxBaseSpecValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        let path = input.file.as_str();
        let is_spec_file = path.ends_with("Test.cfc") || path.ends_with("Spec.cfc");
        if !is_spec_file {
            return Vec::new();
        }
        if input
            .source
            .as_str()
            .contains("extends=\"testbox.system.BaseSpec\"")
            || input.source.as_str().contains("extends=\"BaseSpec\"")
        {
            return Vec::new();
        }
        vec![finding!(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Error,
                rule: BuiltInCfmlRule::TestboxBaseSpec,
            },
            format!(
                "`{path}` is a TestBox spec but does not `extends=\"testbox.system.BaseSpec\"` \
                 -- every spec must extend BaseSpec."
            ),
            &input,
            1,
        )]
    }
}

/// `CF-TOOL-2.1` (scored) -- cfformat `format:check` step present in CI.
#[derive(Debug)]
pub struct CfformatCiStepValidator {
    rule_id: RuleId,
}

impl CfformatCiStepValidator {
    /// Construct the CFFormat CI-step validator.
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: BuiltInCfmlRule::CfformatCiStep.id(),
        })
    }
}

impl Validator for CfformatCiStepValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        let path = input.file.as_str();
        let is_ci_file = path.contains(".github/workflows/") || path.ends_with("ci.yml");
        if !is_ci_file {
            return Vec::new();
        }
        if input.source.as_str().contains("format:check") {
            return Vec::new();
        }
        vec![finding!(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Warning,
                rule: BuiltInCfmlRule::CfformatCiStep,
            },
            "This CI workflow has no `format:check` (cfformat) step -- add one so formatting \
             drift is caught in CI.",
            &input,
            1,
        )]
    }
}

/// `CF-CI-2.1` (scored) -- TestBox coverage floor >= 70% wired as a
/// failing threshold.
#[derive(Debug)]
pub struct CoverageFloorValidator {
    rule_id: RuleId,
}

impl CoverageFloorValidator {
    /// Construct the coverage-floor validator.
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: BuiltInCfmlRule::CoverageFloor.id(),
        })
    }
}

impl Validator for CoverageFloorValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        let path = input.file.as_str();
        let is_coverage_config = path.contains("testbox") && path.ends_with(".json");
        if !is_coverage_config {
            return Vec::new();
        }
        let parsed = decode_json(input.source);
        let Ok(value) = parsed else {
            return Vec::new();
        };
        let has_fail_floor = value
            .get("coverage")
            .and_then(|c| c.get("failFloor"))
            .and_then(|f| f.as_f64())
            .map(|f| f >= 70.0)
            .unwrap_or(false);
        if has_fail_floor {
            return Vec::new();
        }
        vec![finding!(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Warning,
                rule: BuiltInCfmlRule::CoverageFloor,
            },
            "This TestBox coverage config has no `coverage.failFloor` at/above 70 -- wire a \
             failing coverage floor plus pre-commit/pre-push hooks.",
            &input,
            1,
        )]
    }
}

/// Build every validator this module registers.
pub fn all() -> Result<Vec<Box<dyn Validator>>, DecodeError> {
    Ok(vec![
        Box::new(CflintrcHardGateValidator::new()?),
        Box::new(PinnedDependencyValidator::new()?),
        Box::new(TestboxBaseSpecValidator::new()?),
        Box::new(CfformatCiStepValidator::new()?),
        Box::new(CoverageFloorValidator::new()?),
    ])
}
