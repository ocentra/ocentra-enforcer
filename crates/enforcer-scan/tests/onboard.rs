//! f02 proof: `cargo test -p enforcer-scan --test onboard` (named row
//! `onboard-scaffolds-enforce` in `TEST_PROOF_EXPECTATIONS.md`) over
//! `tests/fixtures/onboard/**`.
//!
//! Covers the three acceptance scenarios named in
//! `docs/plans/enforcer-selfhost-plan/workpacks/f02-onboard-and-autoindex.md`:
//! (a) fail-fixture -- a repo with no `.enforce/` has no baseline to
//! compare against; (b) pass-fixture -- `enforcer onboard` on a fresh repo
//! scaffolds `.enforce/` with a profile + baseline + registration, each
//! round-tripping through serde; (c) detection test -- a second onboard
//! run is idempotent (byte-identical serialized config, preserved
//! waivers).
//!
//! No `unwrap`/`expect`/`panic` (workspace lints): every test returns
//! `Result` and propagates via `?`.

use std::path::{Path, PathBuf};

use enforcer_domain::hashes::Sha256;
use enforcer_domain::paths::RepoRoot;
use enforcer_domain::telemetry_types::RecordSchemaVersion;
use enforcer_scan::onboard::{
    self, ConfigProvisioning, OnboardError, BASELINE_FILE, ENFORCE_DIR, PROJECT_CONFIG_FILE,
    REGISTRATION_FILE, REGISTRATION_VERSION,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Boundary-side wire mirror of `onboard::ProjectRegistration` -- the
/// decode half of the registration round-trip, deliberately declared HERE
/// (a test boundary) rather than in `src/onboard.rs`, which is
/// serialize-only per the "deserialize at the boundary" doctrine. Field
/// types are the same branded newtypes, so decoding re-validates the
/// brands.
#[derive(Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RegistrationWire {
    version: RecordSchemaVersion,
    project_id: Sha256,
    repo_root: RepoRoot,
}

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/onboard")
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let dest_path = dst.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_recursive(&entry.path(), &dest_path)?;
        } else {
            std::fs::copy(entry.path(), &dest_path)?;
        }
    }
    Ok(())
}

/// Copy a checked-in fixture tree into a fresh tempdir so mutating tests
/// (onboard writes `.enforce/`) never touch the committed fixture itself.
fn copy_fixture_into_temp(name: &str) -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    copy_dir_recursive(&fixtures_dir().join(name), temp.path())?;
    Ok(temp)
}

fn repo_root_of(path: &Path) -> Result<RepoRoot, Box<dyn std::error::Error>> {
    Ok(path.to_string_lossy().parse::<RepoRoot>()?)
}

/// (a) fail-fixture: a repo with no `.enforce/` has no baseline to compare
/// against -- `require_onboarded` must fail closed with a typed
/// "not onboarded" error, never silently defaulting to an empty baseline.
#[test]
fn not_onboarded_repo_fails_closed_with_typed_error() -> TestResult {
    let temp = copy_fixture_into_temp("not-onboarded")?;
    assert!(!temp.path().join(ENFORCE_DIR).exists());
    let root = repo_root_of(temp.path())?;
    let outcome = onboard::require_onboarded(&root);
    assert!(
        matches!(outcome, Err(OnboardError::NotOnboarded { .. })),
        "a never-onboarded repo must fail closed, not silently resolve to an empty baseline"
    );
    Ok(())
}

/// (b) pass-fixture: `enforcer onboard` on a fresh repo scaffolds
/// `.enforce/` with a profile + baseline + registration, each
/// round-tripping through serde, and grandfathers the fixture's known
/// violation into the baseline (ratchet-first).
#[test]
fn onboard_scaffolds_enforce_with_profile_baseline_and_registration() -> TestResult {
    let temp = copy_fixture_into_temp("fresh-repo")?;
    let root = repo_root_of(temp.path())?;

    let result = onboard::onboard(&root)?;
    assert_eq!(
        result.baseline.entry_count().get(),
        1,
        "the fixture's one unwrap() must be grandfathered into the baseline"
    );
    assert_eq!(
        result.config,
        ConfigProvisioning::WroteDefault,
        "no prior config existed; onboard must have written the default"
    );

    let enforce_dir = temp.path().join(ENFORCE_DIR);
    assert!(enforce_dir.join(PROJECT_CONFIG_FILE).exists());
    assert!(enforce_dir.join(BASELINE_FILE).exists());
    assert!(enforce_dir.join(REGISTRATION_FILE).exists());

    // Config round-trips through the f03 wire shape at the JSON boundary.
    let config_raw = std::fs::read_to_string(enforce_dir.join(PROJECT_CONFIG_FILE))?;
    let _config: enforcer_config::serde::WireProjectConfig = serde_json::from_str(&config_raw)?;

    // Baseline round-trips and grandfathers exactly the fixture's violation.
    let loaded_baseline = onboard::require_onboarded(&root)?;
    assert_eq!(loaded_baseline.entry_count().get(), 1);
    assert_eq!(loaded_baseline, result.baseline);

    // Registration round-trips through serde (decode side at THIS test
    // boundary -- see `RegistrationWire`) and carries the deterministic
    // project id, with both branded newtypes re-validating on decode.
    let reg_raw = std::fs::read_to_string(enforce_dir.join(REGISTRATION_FILE))?;
    let registration: RegistrationWire = serde_json::from_str(&reg_raw)?;
    assert_eq!(registration.version, REGISTRATION_VERSION);
    assert_eq!(registration.project_id, onboard::project_id(&root));
    assert_eq!(registration.project_id, result.project_id);
    assert_eq!(registration.repo_root, root);

    Ok(())
}

/// A fresh onboard on a repo with zero violations produces an empty (but
/// present) baseline -- onboarding a clean tree is meaningful (scaffolds
/// `.enforce/`), not merely a no-op shortcut.
#[test]
fn onboard_on_clean_tree_produces_a_present_but_empty_baseline() -> TestResult {
    let temp = copy_fixture_into_temp("not-onboarded")?;
    let root = repo_root_of(temp.path())?;
    let result = onboard::onboard(&root)?;
    assert!(result.baseline.is_empty());
    let baseline = onboard::require_onboarded(&root)?;
    assert!(baseline.is_empty());
    Ok(())
}

/// (c) detection test: re-running onboard is idempotent. An existing
/// `.enforce/config` (with a hand-added waiver, simulating a post-onboard
/// user edit) must be preserved BYTE-IDENTICAL, never overwritten with a
/// fresh default -- dropping a waiver on a second run is exactly the
/// failure this test guards against.
#[test]
fn second_onboard_run_preserves_existing_config_byte_identical() -> TestResult {
    let temp = copy_fixture_into_temp("fresh-repo")?;
    let root = repo_root_of(temp.path())?;

    let first = onboard::onboard(&root)?;
    assert_eq!(first.config, ConfigProvisioning::WroteDefault);

    let config_path = temp.path().join(ENFORCE_DIR).join(PROJECT_CONFIG_FILE);
    let edited = serde_json::json!({
        "native": {},
        "policy": {
            "ruleToggles": {
                "RR-1.1": {
                    "enabled": false,
                    "waiver": {
                        "ruleId": "RR-1.1",
                        "owner": "platform-team",
                        "reason": "tracked in TICKET-42"
                    }
                }
            }
        }
    });
    let edited_bytes = serde_json::to_vec_pretty(&edited)?;
    std::fs::write(&config_path, &edited_bytes)?;

    let second = onboard::onboard(&root)?;
    assert_eq!(
        second.config,
        ConfigProvisioning::PreservedExisting,
        "an existing config must be preserved, never reported as (re)written"
    );

    let after_bytes = std::fs::read(&config_path)?;
    assert_eq!(
        after_bytes, edited_bytes,
        "a second onboard run must preserve the hand-edited config byte-identical, waiver included"
    );

    // The registration record is a pure function of repo_root -- byte
    // identical across repeated runs against the same root.
    let registration_path = temp.path().join(ENFORCE_DIR).join(REGISTRATION_FILE);
    let reg_bytes_first = std::fs::read(&registration_path)?;
    onboard::onboard(&root)?;
    let reg_bytes_second = std::fs::read(&registration_path)?;
    assert_eq!(reg_bytes_first, reg_bytes_second);

    Ok(())
}

/// A malformed pre-existing `.enforce/config` must fail onboarding rather
/// than being silently replaced with a fresh default.
#[test]
fn malformed_existing_config_fails_onboarding_rather_than_being_replaced() -> TestResult {
    let temp = copy_fixture_into_temp("fresh-repo")?;
    let root = repo_root_of(temp.path())?;
    let enforce_dir = temp.path().join(ENFORCE_DIR);
    std::fs::create_dir_all(&enforce_dir)?;
    std::fs::write(enforce_dir.join(PROJECT_CONFIG_FILE), b"{ not json")?;

    let outcome = onboard::onboard(&root);
    assert!(
        matches!(outcome, Err(OnboardError::ConfigLoad(_))),
        "a malformed existing config must fail onboarding with the typed config-load error, \
         not be silently replaced"
    );

    let raw = std::fs::read_to_string(enforce_dir.join(PROJECT_CONFIG_FILE))?;
    assert_eq!(
        raw, "{ not json",
        "the malformed file must remain untouched"
    );
    Ok(())
}

/// Two fresh onboards of identical fixture content produce byte-identical
/// baseline files (the baseline's entries are repo-relative and content-
/// derived only, independent of the absolute tempdir path each copy lives
/// under).
#[test]
fn two_fresh_onboards_of_identical_content_are_byte_identical() -> TestResult {
    let first_temp = copy_fixture_into_temp("fresh-repo")?;
    let second_temp = copy_fixture_into_temp("fresh-repo")?;
    let first_root = repo_root_of(first_temp.path())?;
    let second_root = repo_root_of(second_temp.path())?;

    onboard::onboard(&first_root)?;
    onboard::onboard(&second_root)?;

    let baseline_first = std::fs::read(first_temp.path().join(ENFORCE_DIR).join(BASELINE_FILE))?;
    let baseline_second = std::fs::read(second_temp.path().join(ENFORCE_DIR).join(BASELINE_FILE))?;
    assert_eq!(baseline_first, baseline_second);

    Ok(())
}
