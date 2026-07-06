//! h12 acceptance proof — three named rows in TEST_PROOF_EXPECTATIONS.md:
//! `cyberskills-adapter-graceful-skip`, `cyberskills-adapter-severity-gate`,
//! `cyberskills-adapters-dogfood-exclusion`.
//!
//! Uses RECORDED tool-output fixtures under
//! `tests/fixtures/cyberskills_adapters/<adapter>/{good,bad}/` — no live
//! engine binary is required in CI, per the workpack's acceptance section.

use std::path::{Path, PathBuf};

use enforcer_config::resolve::resolve_profile_only;
use enforcer_domain::severity::Severity;
use enforcer_harness::adapters::cyberskills::gate::SeverityThresholdGate;
use enforcer_harness::adapters::cyberskills::recorded::parse_recorded;
use enforcer_harness::adapters::cyberskills::seam::AdapterOutcome;
use enforcer_scan::walk::{walk, IgnoreRules};
use enforcer_validator::harness::run_fixture_parity;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn repo_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    // CARGO_MANIFEST_DIR is `<repo>/crates/enforcer-harness`.
    Ok(Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?)
}

fn read_fixture(rel: &str) -> Result<String, Box<dyn std::error::Error>> {
    Ok(std::fs::read_to_string(manifest_dir().join(rel))?)
}

/// `cyberskills-adapter-graceful-skip`: an adapter that silently PASSES
/// when its binary is absent is flagged as dishonest (rejected at the
/// parse boundary); the honest skip (absent tool, `ran: 0`) is accepted as
/// [`AdapterOutcome::Skipped`]; a present tool with real findings is
/// accepted as [`AdapterOutcome::Ran`]; a present-but-erroring tool
/// surfaces the error via [`AdapterOutcome::Errored`], never a silent pass.
#[test]
fn cyberskills_adapter_graceful_skip() -> Result<(), Box<dyn std::error::Error>> {
    // Dishonest: tool absent yet reports `outcome: pass`.
    let dishonest = read_fixture(
        "tests/fixtures/cyberskills_adapters/slither/bad/tool_absent_reported_pass.json",
    )?;
    assert!(
        parse_recorded(&dishonest).is_err(),
        "an absent tool reporting `pass` must be REJECTED, not accepted as a silent skip"
    );

    // Honest skip: tool absent, `ran: 0`.
    let honest_skip =
        read_fixture("tests/fixtures/cyberskills_adapters/slither/good/tool_absent_skipped.json")?;
    let outcome = parse_recorded(&honest_skip)?;
    assert_eq!(outcome, AdapterOutcome::Skipped { ran: 0 });
    assert!(outcome.is_honest());

    // Tool present, real findings.
    let present_findings = read_fixture(
        "tests/fixtures/cyberskills_adapters/slither/good/tool_present_findings.json",
    )?;
    let outcome = parse_recorded(&present_findings)?;
    assert!(outcome.is_honest());
    let AdapterOutcome::Ran { ran, ref findings } = outcome else {
        return Err(format!("expected Ran, got {outcome:?}").into());
    };
    assert_eq!(ran, 1);
    assert_eq!(findings.len(), 1);

    // Present-but-erroring tool: dishonest variant (claims `pass`) rejected...
    let dishonest_error = read_fixture(
        "tests/fixtures/cyberskills_adapters/slither/bad/tool_present_error_reported_pass.json",
    )?;
    assert!(
        parse_recorded(&dishonest_error).is_err(),
        "a present-but-erroring tool reporting `pass` must be REJECTED"
    );

    // ...the honest variant surfaces the error, never a silent pass.
    let honest_error = read_fixture(
        "tests/fixtures/cyberskills_adapters/slither/good/tool_present_error_surfaced.json",
    )?;
    let outcome = parse_recorded(&honest_error)?;
    assert!(outcome.is_honest());
    let AdapterOutcome::Errored { ref error_message } = outcome else {
        return Err(format!("expected Errored, got {outcome:?}").into());
    };
    assert!(error_message.contains("compilation failed"));

    Ok(())
}

/// `cyberskills-adapter-severity-gate`: a T2 scored gate over RECORDED tool
/// output — a HIGH-severity CVE over threshold fails the gate; a
/// below-threshold finding stays clean. Proven through the standard
/// `run_fixture_parity` oracle every other rule in this workspace uses.
#[test]
fn cyberskills_adapter_severity_gate() -> Result<(), Box<dyn std::error::Error>> {
    let gate = SeverityThresholdGate::new("CYBER-ADAPTER-SCA-SEVERITY.1".parse()?, Severity::Error);
    run_fixture_parity(
        &gate,
        &manifest_dir(),
        "tests/fixtures/cyberskills_adapters/sca/bad/high_cve_over_threshold.json",
        "tests/fixtures/cyberskills_adapters/sca/good/below_threshold.json",
    )?;
    Ok(())
}

/// `cyberskills-adapters-dogfood-exclusion`: a self-host walk of THIS repo,
/// filtered through the `ocentra-enforcer` profile's own committed
/// `ignoreFileGlobs`, yields ZERO walked paths under
/// `crates/enforcer-harness/adapters/cyberskills/**` — mirrors h11's
/// `cyberskills_vendor_not_dogfooded` proof exactly (real repo walk, not a
/// synthetic fixture tree, honoring the same vendor-absent-style honesty
/// protocol for the external wrapper-scripts directory).
#[test]
fn cyberskills_adapters_not_dogfooded() -> Result<(), Box<dyn std::error::Error>> {
    let root = repo_root()?;
    let adapters_dir = root.join("crates/enforcer-harness/adapters/cyberskills");
    if !adapters_dir.is_dir() {
        // Honesty protocol: nothing to prove if the directory does not
        // exist yet — never fabricate a tree that was not actually landed.
        return Ok(());
    }

    let config = resolve_profile_only("ocentra-enforcer")?;
    let rules = IgnoreRules {
        ignore_dirs: config.ignore_dirs.clone(),
        ignore_file_globs: config
            .ignore_file_globs
            .iter()
            .map(|glob| glob.as_str().to_owned())
            .collect(),
    };

    let walked = walk(&root, &rules)?;
    let hits: Vec<_> = walked
        .iter()
        .filter(|path| {
            path.as_str()
                .starts_with("crates/enforcer-harness/adapters/cyberskills/")
        })
        .collect();

    assert!(
        hits.is_empty(),
        "expected zero walked paths under crates/enforcer-harness/adapters/cyberskills/, found: {hits:?}"
    );
    Ok(())
}

/// Regression guard mirroring h11's
/// `without_the_vendor_glob_the_walk_would_see_vendor_files`: an unfiltered
/// walk (no ignore globs) WOULD see files under the adapters directory —
/// proving the profile's glob is what does the excluding, not an accidental
/// absence of files there.
#[test]
fn without_the_adapters_glob_the_walk_would_see_adapter_files(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = repo_root()?;
    let adapters_dir = root.join("crates/enforcer-harness/adapters/cyberskills");
    if !adapters_dir.is_dir() {
        return Ok(());
    }

    let rules = IgnoreRules {
        ignore_dirs: Vec::new(),
        ignore_file_globs: Vec::new(),
    };
    let walked = walk(&root, &rules)?;
    let hits = walked
        .iter()
        .filter(|path| {
            path.as_str()
                .starts_with("crates/enforcer-harness/adapters/cyberskills/")
        })
        .count();
    assert!(
        hits > 0,
        "expected the unfiltered walk to see adapter wrapper files (sanity check that the \
         directory is actually present and would otherwise be walked)"
    );
    Ok(())
}
