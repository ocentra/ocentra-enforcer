//! The reusable fixture/parity harness: given a [`Validator`] plus its
//! fail/pass fixture file paths, assert it fires on the fail fixture and
//! stays silent on the pass fixture. This is the Rust-native replacement
//! for the ad-hoc `.mjs` detection-check/parity plumbing — every lang
//! validator crate calls [`run_fixture_parity`] from its own `cargo test`
//! instead of reimplementing this check.

use std::fs;

use enforcer_domain::boundary::validation::ValidationSourceText;
use enforcer_domain::findings::ScanScope;
use enforcer_domain::paths::{RelPath, RepoRoot};
use enforcer_domain::telemetry_types::FindingCount;

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
    repo_root: &RepoRoot,
    fail_fixture_path: &RelPath,
    pass_fixture_path: &RelPath,
) -> HarnessResult<()> {
    // CLONE-JUSTIFICATION: returned harness errors own the validator rule identity.
    let rule_id = validator.rule_id().clone();

    let fail_source = read_fixture(repo_root, fail_fixture_path)?;
    let fail_findings = validator.validate(ValidationInput {
        file: fail_fixture_path,
        source: fail_source.as_source(),
        scope: ScanScope::Files,
    });
    if fail_findings.is_empty() {
        return Err(HarnessError::DidNotFireOnFail {
            rule_id,
            // CLONE-JUSTIFICATION: the returned error outlives this borrowed fixture path.
            fixture: fail_fixture_path.clone(),
        });
    }
    if let Some(mismatch) = fail_findings
        .iter()
        .find(|finding| finding.rule_id != rule_id)
    {
        return Err(HarnessError::MismatchedRule {
            expected_rule_id: rule_id,
            // CLONE-JUSTIFICATION: the returned error owns the mismatched finding identity.
            actual_rule_id: mismatch.rule_id.clone(),
            // CLONE-JUSTIFICATION: the returned error outlives this borrowed fixture path.
            fixture: fail_fixture_path.clone(),
        });
    }

    let pass_source = read_fixture(repo_root, pass_fixture_path)?;
    let pass_findings = validator.validate(ValidationInput {
        file: pass_fixture_path,
        source: pass_source.as_source(),
        scope: ScanScope::Files,
    });
    if !pass_findings.is_empty() {
        let finding_count = u64::try_from(pass_findings.len())
            .map(FindingCount::new)
            .map_err(|source| HarnessError::FindingCountOverflow {
                // CLONE-JUSTIFICATION: the returned error outlives this borrowed fixture path.
                fixture: pass_fixture_path.clone(),
                source,
            })?;
        return Err(HarnessError::FiredOnPass {
            rule_id,
            // CLONE-JUSTIFICATION: the returned error outlives this borrowed fixture path.
            fixture: pass_fixture_path.clone(),
            finding_count,
        });
    }

    Ok(())
}

fn read_fixture(repo_root: &RepoRoot, rel_path: &RelPath) -> HarnessResult<ValidationSourceText> {
    let full = repo_root.resolve(rel_path);
    fs::read_to_string(&full)
        .map(ValidationSourceText::try_new)
        .map_err(|source| HarnessError::FixtureRead {
            // CLONE-JUSTIFICATION: the returned I/O error outlives this borrowed fixture path.
            path: rel_path.clone(),
            source,
        })
}

/// Build a [`RelPath`] for a fixture for use as `Finding::file`. Fixture
/// paths passed to this harness are repo-relative strings (the same shape
/// `RuleRecord.fixtures` carries), so a decode failure here means the
/// caller passed a malformed fixture path — that is a harness-usage bug,
/// surfaced as [`HarnessError::FixtureRead`] rather than a panic, keeping
/// this crate `unwrap`/`expect`-free per workspace lint policy.
#[cfg(test)]
mod tests {
    use enforcer_domain::boundary::decode_error::DecodeError;
    use enforcer_domain::findings::{Finding, FindingDetail, FindingLine, FindingTitle};
    use enforcer_domain::ids::RuleId;
    use enforcer_domain::paths::RepoRoot;
    use enforcer_domain::severity::Severity;
    use enforcer_domain::telemetry_types::SourceLine;

    use super::run_fixture_parity;
    use crate::error::HarnessError;
    use crate::validator::{ValidationInput, Validator};

    /// A validator that correctly fires only on fixtures containing the
    /// literal marker `FORBIDDEN`.
    struct MarkerValidator {
        rule_id: RuleId,
        title: FindingTitle,
        detail: FindingDetail,
        line: SourceLine,
    }

    impl Validator for MarkerValidator {
        fn rule_id(&self) -> &RuleId {
            &self.rule_id
        }

        fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
            if input.source.as_str().contains("FORBIDDEN") {
                vec![Finding {
                    rule_id: self.rule_id.clone(),
                    severity: Severity::Error,
                    title: self.title.clone(),
                    detail: self.detail.clone(),
                    file: input.file.clone(),
                    line: FindingLine::known(self.line),
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
        title: FindingTitle,
        detail: FindingDetail,
        line: SourceLine,
    }

    impl Validator for AlwaysFiresValidator {
        fn rule_id(&self) -> &RuleId {
            &self.rule_id
        }

        fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
            vec![Finding {
                rule_id: self.rule_id.clone(),
                severity: Severity::Error,
                title: self.title.clone(),
                detail: self.detail.clone(),
                file: input.file.clone(),
                line: FindingLine::known(self.line),
                snippet: None,
            }]
        }
    }

    fn manifest_root() -> Result<RepoRoot, DecodeError> {
        RepoRoot::try_from(std::path::Path::new(env!("CARGO_MANIFEST_DIR")))
    }

    fn marker_validator() -> Result<MarkerValidator, DecodeError> {
        Ok(MarkerValidator {
            rule_id: "RR-99.1".parse()?,
            title: FindingTitle::new("forbidden marker present".to_owned())?,
            detail: FindingDetail::new("found the literal marker FORBIDDEN".to_owned())?,
            line: SourceLine::try_new(std::num::NonZeroU32::MIN),
        })
    }

    fn always_fires_validator() -> Result<AlwaysFiresValidator, DecodeError> {
        Ok(AlwaysFiresValidator {
            rule_id: "RR-99.1".parse()?,
            title: FindingTitle::new("always fires".to_owned())?,
            detail: FindingDetail::new(
                "this validator is broken and fires unconditionally".to_owned(),
            )?,
            line: SourceLine::try_new(std::num::NonZeroU32::MIN),
        })
    }

    #[test]
    fn correct_validator_passes_both_directions() -> Result<(), Box<dyn std::error::Error>> {
        let validator = marker_validator()?;
        run_fixture_parity(
            &validator,
            &manifest_root()?,
            &"fixtures/sample/fail.txt".parse()?,
            &"fixtures/sample/pass.txt".parse()?,
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
            &manifest_root()?,
            &"fixtures/sample/fail.txt".parse()?,
            &"fixtures/sample/pass.txt".parse()?,
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
        let validator = always_fires_validator()?;
        let outcome = run_fixture_parity(
            &validator,
            &manifest_root()?,
            &"fixtures/sample/fail.txt".parse()?,
            &"fixtures/sample/pass.txt".parse()?,
        );
        assert!(matches!(outcome, Err(HarnessError::FiredOnPass { .. })));
        Ok(())
    }

    #[test]
    fn missing_fixture_reports_read_error() -> Result<(), Box<dyn std::error::Error>> {
        let validator = marker_validator()?;
        let outcome = run_fixture_parity(
            &validator,
            &manifest_root()?,
            &"fixtures/sample/does-not-exist.txt".parse()?,
            &"fixtures/sample/pass.txt".parse()?,
        );
        assert!(matches!(outcome, Err(HarnessError::FixtureRead { .. })));
        Ok(())
    }
}
