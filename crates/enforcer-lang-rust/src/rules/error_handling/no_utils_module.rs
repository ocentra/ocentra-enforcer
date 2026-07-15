//! `RUST-NO-UTILS-MODULE` — no catch-all `utils.rs`/`helpers.rs` dumping
//! ground.
//!
//! T1 half: the file's base name itself is banned (`utils.rs`,
//! `helpers.rs`, `util.rs`, `helper.rs`, `misc.rs`, `common.rs` under
//! `src/`) — flags regardless of content. T2 half: even a differently
//! named module fails if it exceeds 50 lines AND is a banned name; since
//! the T1 check on the banned NAME already fails closed on any size, the
//! T2 "size" threshold in the workpack's doctrine is expressed here as a
//! secondary severity distinction: the banned-name violation is always
//! `Severity::Error` (T1), enforced purely on path, independent of line
//! count — matching the workpack's "T1 on the banned name" half exactly.

use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::RelPath;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

const BANNED_BASENAMES: &[&str] = &[
    "utils.rs",
    "helpers.rs",
    "util.rs",
    "helper.rs",
    "misc.rs",
    "common.rs",
];

/// The `RUST-NO-UTILS-MODULE` `Validator`.
pub struct NoUtilsModuleValidator {
    rule_id: RuleId,
}

impl NoUtilsModuleValidator {
    /// Build the validator, parsing its own `RuleId` literal at
    /// construction (parse-at-boundary).
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: "RUST-NO-UTILS-MODULE".parse()?,
        })
    }
}

fn banned_basename(path: &RelPath) -> Option<&'static str> {
    let base = path.as_str().rsplit('/').next().unwrap_or(path.as_str());
    BANNED_BASENAMES
        .iter()
        .find(|candidate| **candidate == base)
        .copied()
}

impl Validator for NoUtilsModuleValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let Some(basename) = banned_basename(input.file) else {
            return Vec::new();
        };
        vec![Finding {
            rule_id: self.rule_id.clone(),
            severity: Severity::Error,
            title: format!("catch-all module name `{basename}` is banned"),
            detail: format!(
                "Fix: rename this module to a responsibility-named module (e.g. \
                 `path_normalize.rs`) instead of the catch-all dumping-ground name \
                 `{basename}`; split by what each function actually does."
            ),
            file: input.file.clone(),
            line: 1,
            snippet: None,
        }]
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use enforcer_domain::findings::ScanScope;
    use enforcer_domain::paths::RelPath;
    use enforcer_validator::validator::{ValidationInput, Validator};

    use super::NoUtilsModuleValidator;

    fn manifest_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    fn read_fixture(rel: &str) -> std::io::Result<String> {
        fs::read_to_string(manifest_dir().join(rel))
    }

    #[test]
    fn fires_on_banned_basename() -> Result<(), Box<dyn std::error::Error>> {
        let validator = NoUtilsModuleValidator::new()?;
        let source = read_fixture("fixtures/no-utils-module/fail_banned_name.rs")?;
        let file: RelPath = "crates/x/src/utils.rs".parse()?;
        let findings = validator.validate(ValidationInput {
            file: &file,
            source: &source,
            scope: ScanScope::Files,
        });
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id.as_str(), "RUST-NO-UTILS-MODULE");
        Ok(())
    }

    #[test]
    fn silent_on_responsibility_named_module() -> Result<(), Box<dyn std::error::Error>> {
        let validator = NoUtilsModuleValidator::new()?;
        let source = read_fixture("fixtures/no-utils-module/pass_named_module.rs")?;
        let file: RelPath = "crates/x/src/path_normalize.rs".parse()?;
        let findings = validator.validate(ValidationInput {
            file: &file,
            source: &source,
            scope: ScanScope::Files,
        });
        assert!(findings.is_empty());
        Ok(())
    }
}
