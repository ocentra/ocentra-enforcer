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
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

/// One manifest-shape rule: fires when `absent_marker` is missing from the
/// manifest (a "this must be present" rule) or when `forbidden_marker` is
/// present (a "this must be absent" rule). Exactly one of the two is set
/// per rule.
enum Shape {
    /// The manifest MUST contain this marker; absence trips the rule.
    MustContain(&'static str),
    /// The manifest MUST NOT contain any of these markers; presence trips
    /// the rule (first match reported).
    MustNotContain(&'static [&'static str]),
}

struct ManifestShapeValidator {
    rule_id: RuleId,
    title: &'static str,
    shape: Shape,
}

impl Validator for ManifestShapeValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        match self.shape {
            Shape::MustContain(marker) => {
                if input.source.contains(marker) {
                    return Vec::new();
                }
                vec![self.finding(input, format!("missing required `{marker}`"))]
            }
            Shape::MustNotContain(markers) => {
                for line in input.source.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with('#') {
                        continue;
                    }
                    if let Some(marker) = markers.iter().find(|m| line.contains(**m)) {
                        return vec![self.finding(input, format!("forbidden `{marker}`"))];
                    }
                }
                Vec::new()
            }
        }
    }
}

impl ManifestShapeValidator {
    fn finding(&self, input: ValidationInput<'_>, detail: String) -> Finding {
        Finding {
            rule_id: self.rule_id.clone(),
            severity: Severity::Error,
            title: self.title.to_owned(),
            detail,
            file: input.file.clone(),
            line: 1,
            snippet: None,
        }
    }
}

/// Build every PY-keyed `generic-scanner` validator this crate registers
/// (the PY-5 manifest-shape remainder, 8 rules).
pub fn all() -> Result<Vec<Box<dyn Validator>>, DecodeError> {
    Ok(vec![
        Box::new(ManifestShapeValidator {
            rule_id: "PY-5.1".parse()?,
            title: "pyproject.toml is required for Python projects",
            shape: Shape::MustContain("[project]"),
        }),
        Box::new(ManifestShapeValidator {
            rule_id: "PY-5.2".parse()?,
            title: "Ruff configuration is required",
            shape: Shape::MustContain("[tool.ruff]"),
        }),
        Box::new(ManifestShapeValidator {
            rule_id: "PY-5.3".parse()?,
            title: "Pyright or mypy configuration is required",
            shape: Shape::MustContain("[tool.pyright]"),
        }),
        Box::new(ManifestShapeValidator {
            rule_id: "PY-5.4".parse()?,
            title: "Python type checker strict mode is required",
            shape: Shape::MustContain("strict = true"),
        }),
        Box::new(ManifestShapeValidator {
            rule_id: "PY-5.7".parse()?,
            title: "Python lockfile is required",
            shape: Shape::MustContain("[tool.enforcer.lockfile]"),
        }),
        Box::new(ManifestShapeValidator {
            rule_id: "PY-5.8".parse()?,
            title: "Unpinned Python requirements are forbidden",
            shape: Shape::MustNotContain(&[">=", "~=", "*"]),
        }),
        Box::new(ManifestShapeValidator {
            rule_id: "PY-5.9".parse()?,
            title: "Python git dependencies are forbidden",
            shape: Shape::MustNotContain(&["git+", "{ git ="]),
        }),
        Box::new(ManifestShapeValidator {
            rule_id: "PY-5.10".parse()?,
            title: "Python local path dependencies require waiver",
            shape: Shape::MustNotContain(&["{ path =", "-e ."]),
        }),
    ])
}
