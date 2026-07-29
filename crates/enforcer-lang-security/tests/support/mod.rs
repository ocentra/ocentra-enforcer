use std::path::Path;

use enforcer_domain::paths::{RelPath, RepoRoot};
use enforcer_validator::validator::Validator;

pub(crate) fn assert_fixture_parity(
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
