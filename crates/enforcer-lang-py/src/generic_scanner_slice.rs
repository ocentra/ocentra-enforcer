//! The PY SLICE of the shared `generic-scanner` engine: 8 PY-5 manifest
//! rules (pyproject/ruff-config/typecheck-config/strict/lockfile/pinning/
//! git-dep/path-dep). Per the workpack's shared-engine boundary note, this
//! crate owns ONLY these PY-keyed rules; the `generic-scanner` engine
//! itself and the cross-language partition spec belong to arc-09. This
//! module does not import or re-implement arc-09's engine -- it is a
//! same-shape, PY-scoped sibling that this crate is free to own until the
//! shared engine lands and these rules migrate to consume it.
//!
//! All 8 rules inspect `pyproject.toml` / `requirements.txt` MANIFEST text
//! (never `.py` source), so each fixture pair here is a manifest snippet.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::{Finding, FindingTitle};
use enforcer_domain::ids::{BuiltInPythonRule, RuleId};
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

use crate::boundary::finding::{finding, static_title, PythonFindingSpec};
use crate::boundary::source::PythonMarkers;

/// One manifest-shape rule: fires when `absent_marker` is missing from the
/// manifest (a "this must be present" rule) or when `forbidden_marker` is
/// present (a "this must be absent" rule). Exactly one of the two is set
/// per rule.
#[derive(Debug, Clone, Copy)]
enum Shape {
    /// The manifest MUST contain this marker; absence trips the rule.
    MustContain(PythonMarkers),
    /// The manifest MUST NOT contain any of these markers; presence trips
    /// the rule (first match reported).
    MustNotContain(PythonMarkers),
}

struct ManifestShapeValidator {
    rule_id: RuleId,
    title: FindingTitle,
    shape: Shape,
}

impl Validator for ManifestShapeValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        match self.shape {
            Shape::MustContain(marker) => {
                if marker.any_in(input.source) {
                    return Vec::new();
                }
                let Some(marker) = marker.iter().next() else {
                    return Vec::new();
                };
                finding(
                    &PythonFindingSpec {
                        rule_id: &self.rule_id,
                        severity: Severity::Error,
                        title: self.title.as_str(),
                    },
                    format!("missing required `{marker}`"),
                    &input,
                    1,
                )
            }
            Shape::MustNotContain(markers) => {
                for line in input.source.as_str().lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with('#') {
                        continue;
                    }
                    if let Some(marker) = markers.iter().find(|marker| line.contains(marker)) {
                        return finding(
                            &PythonFindingSpec {
                                rule_id: &self.rule_id,
                                severity: Severity::Error,
                                title: self.title.as_str(),
                            },
                            format!("forbidden `{marker}`"),
                            &input,
                            1,
                        );
                    }
                }
                Vec::new()
            }
        }
    }
}

/// Build every PY-keyed `generic-scanner` validator this crate registers
/// (the PY-5 manifest-shape remainder, 8 rules).
pub fn all() -> Result<Vec<Box<dyn Validator>>, DecodeError> {
    Ok(vec![
        Box::new(ManifestShapeValidator {
            rule_id: BuiltInPythonRule::Py5Rule1.id(),
            title: static_title("pyproject.toml is required for Python projects")?,
            shape: Shape::MustContain(PythonMarkers::new(&["[project]"])),
        }),
        Box::new(ManifestShapeValidator {
            rule_id: BuiltInPythonRule::Py5Rule2.id(),
            title: static_title("Ruff configuration is required")?,
            shape: Shape::MustContain(PythonMarkers::new(&["[tool.ruff]"])),
        }),
        Box::new(ManifestShapeValidator {
            rule_id: BuiltInPythonRule::Py5Rule3.id(),
            title: static_title("Pyright or mypy configuration is required")?,
            shape: Shape::MustContain(PythonMarkers::new(&["[tool.pyright]"])),
        }),
        Box::new(ManifestShapeValidator {
            rule_id: BuiltInPythonRule::Py5Rule4.id(),
            title: static_title("Python type checker strict mode is required")?,
            shape: Shape::MustContain(PythonMarkers::new(&["strict = true"])),
        }),
        Box::new(ManifestShapeValidator {
            rule_id: BuiltInPythonRule::Py5Rule7.id(),
            title: static_title("Python lockfile is required")?,
            shape: Shape::MustContain(PythonMarkers::new(&["[tool.enforcer.lockfile]"])),
        }),
        Box::new(ManifestShapeValidator {
            rule_id: BuiltInPythonRule::Py5Rule8.id(),
            title: static_title("Unpinned Python requirements are forbidden")?,
            shape: Shape::MustNotContain(PythonMarkers::new(&[">=", "~=", "*"])),
        }),
        Box::new(ManifestShapeValidator {
            rule_id: BuiltInPythonRule::Py5Rule9.id(),
            title: static_title("Python git dependencies are forbidden")?,
            shape: Shape::MustNotContain(PythonMarkers::new(&["git+", "{ git ="])),
        }),
        Box::new(ManifestShapeValidator {
            rule_id: BuiltInPythonRule::Py5Rule10.id(),
            title: static_title("Python local path dependencies require waiver")?,
            shape: Shape::MustNotContain(PythonMarkers::new(&["{ path =", "-e ."])),
        }),
    ])
}
