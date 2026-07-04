//! The reusable fixture/parity harness: given a [`Validator`] plus its
//! fail/pass fixture file paths, assert it fires on the fail fixture and
//! stays silent on the pass fixture. This is the Rust-native replacement
//! for the ad-hoc `.mjs` detection-check/parity plumbing — every lang
//! validator crate calls [`run_fixture_parity`] from its own `cargo test`
//! instead of reimplementing this check.

use std::fs;
use std::path::Path;

use enforcer_domain::findings::ScanScope;
use enforcer_domain::paths::RelPath;

use crate::error::{HarnessError, HarnessResult};
use crate::validator::{ValidationInput, Validator};

/// Run the fail/pass parity oracle for one validator.
///
/// - Reads `fail_fixture_path` and `pass_fixture_path` from disk.
/// - Runs `validator` against the fail fixture; fails closed if it produces
///   zero findings, or if any produced finding's `rule_id` does not match
///   `validator.rule_id()`.
/// - Runs `validator` against the pass fixture; fails closed if it produces
///   ANY finding at all.
///
/// Both fixture paths are resolved relative to `repo_root` (typically
/// `CARGO_MANIFEST_DIR` of the calling crate), so callers pass the same
/// repo-relative paths a `RuleRecord.fixtures` entry would carry.
pub fn run_fixture_parity(
    validator: &dyn Validator,
    repo_root: &Path,
    fail_fixture_path: &str,
    pass_fixture_path: &str,
) -> HarnessResult<()> {
    let rule_id = validator.rule_id().to_string();

    let fail_source = read_fixture(repo_root, fail_fixture_path)?;
    let fail_file = fixture_rel_path(fail_fixture_path)?;
    let fail_findings = validator.validate(ValidationInput {
        file: &fail_file,
        source: &fail_source,
        scope: ScanScope::Files,
    });
    if fail_findings.is_empty() {
        return Err(HarnessError::DidNotFireOnFail {
            rule_id,
            fixture: fail_fixture_path.to_owned(),
        });
    }
    if let Some(mismatch) = fail_findings
        .iter()
        .find(|finding| finding.rule_id.as_str() != rule_id)
    {
        return Err(HarnessError::DidNotFireOnFail {
            rule_id: format!(
                "{rule_id} (fixture produced mismatched ruleId `{}`)",
                mismatch.rule_id.as_str()
            ),
            fixture: fail_fixture_path.to_owned(),
        });
    }

    let pass_source = read_fixture(repo_root, pass_fixture_path)?;
    let pass_file = fixture_rel_path(pass_fixture_path)?;
    let pass_findings = validator.validate(ValidationInput {
        file: &pass_file,
        source: &pass_source,
        scope: ScanScope::Files,
    });
    if !pass_findings.is_empty() {
        return Err(HarnessError::FiredOnPass {
            rule_id,
            fixture: pass_fixture_path.to_owned(),
            finding_count: pass_findings.len(),
        });
    }

    Ok(())
}

fn read_fixture(repo_root: &Path, rel: &str) -> HarnessResult<String> {
    let full = repo_root.join(rel);
    fs::read_to_string(&full).map_err(|source| HarnessError::FixtureRead {
        path: full.display().to_string(),
        source,
    })
}

/// Build a [`RelPath`] for a fixture for use as `Finding::file`. Fixture
/// paths passed to this harness are repo-relative strings (the same shape
/// `RuleRecord.fixtures` carries), so a decode failure here means the
/// caller passed a malformed fixture path — that is a harness-usage bug,
/// surfaced as [`HarnessError::FixtureRead`] rather than a panic, keeping
/// this crate `unwrap`/`expect`-free per workspace lint policy.
fn fixture_rel_path(rel: &str) -> HarnessResult<RelPath> {
    rel.parse()
        .map_err(
            |decode_error: enforcer_core::error::DecodeError| HarnessError::FixtureRead {
                path: rel.to_owned(),
                source: std::io::Error::new(std::io::ErrorKind::InvalidInput, decode_error),
            },
        )
}

#[cfg(test)]
mod tests {
    use enforcer_domain::findings::Finding;
    use enforcer_domain::ids::RuleId;
    use enforcer_domain::severity::Severity;

    use super::run_fixture_parity;
    use crate::error::HarnessError;
    use crate::validator::{ValidationInput, Validator};

    /// A validator that correctly fires only on fixtures containing the
    /// literal marker `FORBIDDEN`.
    struct MarkerValidator {
        rule_id: RuleId,
    }

    impl Validator for MarkerValidator {
        fn rule_id(&self) -> &RuleId {
            &self.rule_id
        }

        fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
            if input.source.contains("FORBIDDEN") {
                vec![Finding {
                    rule_id: self.rule_id.clone(),
                    severity: Severity::Error,
                    title: "forbidden marker present".to_owned(),
                    detail: "found the literal marker FORBIDDEN".to_owned(),
                    file: input.file.clone(),
                    line: 1,
                    snippet: None,
                }]
            } else {
                Vec::new()
            }
        }
    }

    /// A validator that never fires — used to prove the harness catches a
    /// broken validator (the "negative test where a broken validator is
    /// caught" the workpack requires).
    struct SilentValidator {
        rule_id: RuleId,
    }

    impl Validator for SilentValidator {
        fn rule_id(&self) -> &RuleId {
            &self.rule_id
        }

        fn validate(&self, _input: ValidationInput<'_>) -> Vec<Finding> {
            Vec::new()
        }
    }

    /// A validator that fires on everything — the opposite kind of broken
    /// validator: it must be caught on the PASS fixture.
    struct AlwaysFiresValidator {
        rule_id: RuleId,
    }

    impl Validator for AlwaysFiresValidator {
        fn rule_id(&self) -> &RuleId {
            &self.rule_id
        }

        fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
            vec![Finding {
                rule_id: self.rule_id.clone(),
                severity: Severity::Error,
                title: "always fires".to_owned(),
                detail: "this validator is broken and fires unconditionally".to_owned(),
                file: input.file.clone(),
                line: 1,
                snippet: None,
            }]
        }
    }

    fn manifest_dir() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn correct_validator_passes_both_directions() -> Result<(), Box<dyn std::error::Error>> {
        let validator = MarkerValidator {
            rule_id: "RR-99.1".parse()?,
        };
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "fixtures/sample/fail.txt",
            "fixtures/sample/pass.txt",
        )?;
        Ok(())
    }

    #[test]
    fn broken_silent_validator_is_caught_on_fail_fixture() -> Result<(), Box<dyn std::error::Error>>
    {
        let validator = SilentValidator {
            rule_id: "RR-99.1".parse()?,
        };
        let outcome = run_fixture_parity(
            &validator,
            &manifest_dir(),
            "fixtures/sample/fail.txt",
            "fixtures/sample/pass.txt",
        );
        assert!(matches!(
            outcome,
            Err(HarnessError::DidNotFireOnFail { .. })
        ));
        Ok(())
    }

    #[test]
    fn broken_always_fires_validator_is_caught_on_pass_fixture(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let validator = AlwaysFiresValidator {
            rule_id: "RR-99.1".parse()?,
        };
        let outcome = run_fixture_parity(
            &validator,
            &manifest_dir(),
            "fixtures/sample/fail.txt",
            "fixtures/sample/pass.txt",
        );
        assert!(matches!(outcome, Err(HarnessError::FiredOnPass { .. })));
        Ok(())
    }

    #[test]
    fn missing_fixture_reports_read_error() -> Result<(), Box<dyn std::error::Error>> {
        let validator = MarkerValidator {
            rule_id: "RR-99.1".parse()?,
        };
        let outcome = run_fixture_parity(
            &validator,
            &manifest_dir(),
            "fixtures/sample/does-not-exist.txt",
            "fixtures/sample/pass.txt",
        );
        assert!(matches!(outcome, Err(HarnessError::FixtureRead { .. })));
        Ok(())
    }
}
