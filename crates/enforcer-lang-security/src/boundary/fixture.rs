//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! Negative invalid-input coverage rejects malformed, corrupt, and unsupported payloads.
//! Test-fixture filesystem boundary for canonical path conversion.

use std::path::Path;

use enforcer_domain::findings::ScanScope;
use enforcer_domain::paths::{RelPath, RepoRoot};
use enforcer_validator::validator::{ValidationInput, Validator};

pub(crate) fn run_fixture_parity(
    validator: &dyn Validator,
    manifest_dir: &Path,
    fail_fixture: &str,
    pass_fixture: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let repo_root = RepoRoot::try_from(manifest_dir)?;
    let fail_fixture = RelPath::try_from(String::from(fail_fixture))?;
    let pass_fixture = RelPath::try_from(String::from(pass_fixture))?;
    enforcer_validator::harness::run_fixture_parity(
        validator,
        &repo_root,
        &fail_fixture,
        &pass_fixture,
    )?;
    Ok(())
}

/// Run a crate-local fixture pair from the manifest directory boundary.
pub(crate) fn run_manifest_fixture_parity(
    validator: &dyn Validator,
    fail_fixture: &str,
    pass_fixture: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    run_fixture_parity(
        validator,
        Path::new(env!("CARGO_MANIFEST_DIR")),
        fail_fixture,
        pass_fixture,
    )
}

/// Read a crate-local fixture from the manifest directory.
pub(crate) fn read_manifest_fixture(
    fixture: &'static str,
) -> Result<String, Box<dyn std::error::Error>> {
    Ok(std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join(fixture),
    )?)
}

/// Decode a static fixture-relative path into the canonical repository path type.
pub(crate) fn rel_path(
    path: &'static str,
) -> Result<RelPath, enforcer_domain::boundary::decode_error::DecodeError> {
    RelPath::try_from(String::from(path))
}

/// Validate raw fixture source at the test boundary and return its finding count.
pub(crate) fn finding_count(
    validator: &dyn Validator,
    file: &'static str,
    source: &str,
) -> Result<usize, Box<dyn std::error::Error>> {
    let file = rel_path(file)?;
    Ok(validator
        .validate(ValidationInput {
            file: &file,
            source: enforcer_domain::boundary::validation::ValidationSource::from_text(source),
            scope: ScanScope::Files,
        })
        .len())
}
