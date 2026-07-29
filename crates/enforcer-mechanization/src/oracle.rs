//! The fail-closed parity oracle: a rule is only ACCEPTED if its record
//! shape is well-formed, a validator implementation is supplied, and that
//! validator fires on the declared fail fixture while staying silent on
//! the declared pass fixture.
//!
//! This is the d01 port of the `.mjs` contract-coverage / contract-load
//! scripts (`scripts/check-source-core-contract-*.mjs`): those scripts
//! refused to accept a rule/contract lacking full linkage coverage. This
//! oracle applies the same fail-closed posture to the Rust rule pipeline,
//! reusing [`enforcer_validator::harness::run_fixture_parity`] as the
//! reusable base rather than reimplementing fixture I/O here.

use enforcer_domain::paths::RepoRoot;

use enforcer_rules::registry::{RuleRecord, RuleRegistry};
use enforcer_validator::harness::run_fixture_parity;
use enforcer_validator::validator::Validator;

use crate::error::{MechanizationError, MechanizationResult};

/// Accept or reject one candidate rule.
///
/// - `record` is the rule as scaffolded (see [`crate::scaffold::scaffold_rule`])
///   or otherwise assembled.
/// - `validator`, if present, is the concrete implementation to test. `None`
///   means "no validator wired yet" — always rejected; a rule record without
///   an implementation to prove is never accepted, regardless of what its
///   fixtures contain.
/// - `repo_root` is the root fixture paths in `record.fixtures` are resolved
///   relative to (typically `CARGO_MANIFEST_DIR` of the calling crate).
///
/// Returns `Ok(())` only when: the record's shape passes
/// `RuleRegistry::from_records` validation, a validator was supplied, that
/// validator's `rule_id()` matches `record.rule_id`, it fires on the fail
/// fixture, and it stays silent on the pass fixture.
pub fn accept_rule(
    record: &RuleRecord,
    validator: Option<&dyn Validator>,
    repo_root: &RepoRoot,
) -> MechanizationResult<()> {
    // Re-verify record shape independently of however the caller built it —
    // the oracle does not trust the scaffolder, it proves the record.
    RuleRegistry::from_records(vec![Clone::clone(record)]).map_err(|source| {
        MechanizationError::RecordRejected {
            rule_id: Clone::clone(&record.rule_id),
            source,
        }
    })?;

    let Some(validator) = validator else {
        return Err(MechanizationError::MissingValidator {
            rule_id: Clone::clone(&record.rule_id),
        });
    };

    if validator.rule_id() != &record.rule_id {
        return Err(MechanizationError::ValidatorRuleMismatch {
            declared: Clone::clone(&record.rule_id),
            implemented: Clone::clone(validator.rule_id()),
        });
    }

    run_fixture_parity(
        validator,
        repo_root,
        &record.fixtures.fail,
        &record.fixtures.pass,
    )
    .map_err(|source| MechanizationError::ParityFailed {
        rule_id: Clone::clone(&record.rule_id),
        source,
    })
}
