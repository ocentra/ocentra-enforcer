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
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

use super::support::{finding, FindingSpec};

const HARD_GATE_CODES: &[&str] = &["MISSING_VAR", "QUERYPARAM_REQ"];

/// `CF-TOOL-1.1` / `CF-CI-1.1` -- `.cflintrc` must be committed with every
/// hard-gate CFLint code set to `ERROR` severity. Fires on a `.cflintrc`
/// file where a tracked hard-gate code is present but NOT set to `ERROR`
/// (e.g. left at the CFLint default `WARNING`).
pub struct CflintrcHardGateValidator {
    rule_id: RuleId,
}

impl CflintrcHardGateValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "CF-TOOL-1.1".parse()?,
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
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(input.source);
        let Ok(value) = parsed else {
            return vec![finding(
                &FindingSpec {
                    rule_id: &self.rule_id,
                    severity: Severity::Error,
                    title: ".cflintrc is not valid JSON",
                },
                "`.cflintrc` could not be parsed as JSON -- it must be valid JSON declaring \
                 rule severities."
                    .to_owned(),
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
                return vec![finding(
                    &FindingSpec {
                        rule_id: &self.rule_id,
                        severity: Severity::Error,
                        title: ".cflintrc hard-gate code not set to ERROR",
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
pub struct PinnedDependencyValidator {
    rule_id: RuleId,
}

impl PinnedDependencyValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "CF-DEP-1.1".parse()?,
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
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(input.source);
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
                return vec![finding(
                    &FindingSpec {
                        rule_id: &self.rule_id,
                        severity: Severity::Error,
                        title: "box.json dependency uses a wildcard/range version",
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
pub struct TestboxBaseSpecValidator {
    rule_id: RuleId,
}

impl TestboxBaseSpecValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "CF-TEST-1.1".parse()?,
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
        if input.source.contains("extends=\"testbox.system.BaseSpec\"")
            || input.source.contains("extends=\"BaseSpec\"")
        {
            return Vec::new();
        }
        vec![finding(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Error,
                title: "TestBox spec does not extend BaseSpec",
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
pub struct CfformatCiStepValidator {
    rule_id: RuleId,
}

impl CfformatCiStepValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "CF-TOOL-2.1".parse()?,
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
        if input.source.contains("format:check") {
            return Vec::new();
        }
        vec![finding(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Warning,
                title: "CI has no cfformat format:check step (scored)",
            },
            "This CI workflow has no `format:check` (cfformat) step -- add one so formatting \
             drift is caught in CI."
                .to_owned(),
            &input,
            1,
        )]
    }
}

/// `CF-CI-2.1` (scored) -- TestBox coverage floor >= 70% wired as a
/// failing threshold.
pub struct CoverageFloorValidator {
    rule_id: RuleId,
}

impl CoverageFloorValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "CF-CI-2.1".parse()?,
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
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(input.source);
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
        vec![finding(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Warning,
                title: "TestBox coverage has no >=70% failing floor (scored)",
            },
            "This TestBox coverage config has no `coverage.failFloor` at/above 70 -- wire a \
             failing coverage floor plus pre-commit/pre-push hooks."
                .to_owned(),
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
