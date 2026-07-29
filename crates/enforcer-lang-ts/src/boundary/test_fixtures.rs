//! Decode fixture path spellings at the test filesystem boundary.
//!
//! BOUNDARY-INVARIANT: raw fixture spellings convert to canonical repository
//! root and relative-path brands before filesystem access.
//! boundaryOwnerNote: enforcer-lang-ts owns its fixture filesystem boundary.
//! Negative invalid-input coverage is provided by the fixture harness, which
//! rejects escaping and missing fixture paths before validator execution.

use enforcer_domain::paths::{RelPath, RepoRoot};
use enforcer_validator::validator::Validator;

/// Decode the crate fixture root into a canonical repository root.
pub(crate) fn fixture_root(
) -> Result<RepoRoot, enforcer_domain::boundary::decode_error::DecodeError> {
    env!("CARGO_MANIFEST_DIR").parse()
}

/// Decode one fixture spelling into a canonical relative path.
pub(crate) fn fixture_path(
    path: &'static str,
) -> Result<RelPath, enforcer_domain::boundary::decode_error::DecodeError> {
    path.parse()
}

/// Prove one validator against its real fail and pass fixtures.
pub(crate) fn run_fixture_parity(
    validator: &dyn Validator,
    fail_fixture: &str,
    pass_fixture: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let root = fixture_root()?;
    let fail: RelPath = fail_fixture.parse()?;
    let pass: RelPath = pass_fixture.parse()?;
    enforcer_validator::harness::run_fixture_parity(validator, &root, &fail, &pass)?;
    Ok(())
}
