//! Canonical fixture-path decoding for crate unit tests.
//!
//! BOUNDARY-INVARIANT: fixture path literals are untrusted test-boundary
//! input and must decode into validated repository-relative paths before any
//! filesystem access. Malformed or escaping paths are rejected.

use enforcer_domain::paths::{RelPath, RepoRoot};
use enforcer_validator::validator::Validator;

pub(crate) fn source_file(
    raw: &str,
) -> Result<RelPath, enforcer_domain::boundary::decode_error::DecodeError> {
    raw.parse()
}

/// Decode fixture literals and run the shared parity harness.
pub(crate) fn run_fixture_parity(
    validator: &dyn Validator,
    fail_fixture: &str,
    pass_fixture: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let repo_root: RepoRoot = env!("CARGO_MANIFEST_DIR").parse()?;
    let fail_fixture: RelPath = fail_fixture.parse()?;
    let pass_fixture: RelPath = pass_fixture.parse()?;
    enforcer_validator::harness::run_fixture_parity(
        validator,
        &repo_root,
        &fail_fixture,
        &pass_fixture,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::source_file;

    #[test]
    fn rejects_invalid_fixture_path_that_escapes_repository_root() {
        assert!(matches!(
            source_file("../outside.rs"),
            Err(error)
                if error.path == "relPath"
                    && error.reason == "invalid relative path: `..` segment escapes the repository root"
        ));
    }
}
