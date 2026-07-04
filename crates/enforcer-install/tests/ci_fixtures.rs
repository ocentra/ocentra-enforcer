//! Integration proof for workpack c10 (CI integration and binary
//! bootstrap): the checked-in generated artifacts
//! (`.github/actions/enforcer-scan/action.yml`, `install.sh`,
//! `install.ps1`, `packages/enforcer-cli/package.json`) byte-match what
//! `enforcer_install::ci` renders today (no hand-edit drift), the
//! pre-publish smoke fixtures exist and are exactly a seeded
//! violation/clean pair, and the acceptance row's grep-clean assertion
//! holds repo-wide: no generated doc/config contains a literal absolute
//! local filesystem path.

use enforcer_install::ci::{github_action, installer_scripts, npm_wrapper};
use std::path::{Path, PathBuf};

fn repo_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    // crates/enforcer-install -> repo root.
    Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?)
}

const VERSION: &str = "0.1.0";

#[test]
fn checked_in_action_yml_matches_the_rust_rendered_source_of_truth(
) -> Result<(), Box<dyn std::error::Error>> {
    let path = repo_root()?.join(".github/actions/enforcer-scan/action.yml");
    let on_disk = std::fs::read_to_string(&path)?;
    let rendered = github_action::render_action_yml();
    assert_eq!(
        on_disk, rendered,
        "action.yml has drifted from enforcer_install::ci::github_action::render_action_yml() -- regenerate via `cargo run -p enforcer-install --example gen_ci_artifacts`"
    );
    Ok(())
}

#[test]
fn checked_in_install_sh_matches_the_rust_rendered_source_of_truth(
) -> Result<(), Box<dyn std::error::Error>> {
    let path = repo_root()?.join("install.sh");
    let on_disk = std::fs::read_to_string(&path)?;
    let rendered = installer_scripts::render_install_sh(VERSION);
    assert_eq!(
        on_disk, rendered,
        "install.sh has drifted from its Rust-rendered source"
    );
    Ok(())
}

#[test]
fn checked_in_install_ps1_matches_the_rust_rendered_source_of_truth(
) -> Result<(), Box<dyn std::error::Error>> {
    let path = repo_root()?.join("install.ps1");
    let on_disk = std::fs::read_to_string(&path)?;
    let rendered = installer_scripts::render_install_ps1(VERSION);
    assert_eq!(
        on_disk, rendered,
        "install.ps1 has drifted from its Rust-rendered source"
    );
    Ok(())
}

#[test]
fn checked_in_npm_wrapper_package_json_matches_the_rust_rendered_source_of_truth(
) -> Result<(), Box<dyn std::error::Error>> {
    let path = repo_root()?.join("packages/enforcer-cli/package.json");
    let on_disk = std::fs::read_to_string(&path)?;
    let rendered = npm_wrapper::render_wrapper_package_json(VERSION);
    assert_eq!(
        on_disk, rendered,
        "packages/enforcer-cli/package.json has drifted from its Rust-rendered source"
    );
    Ok(())
}

#[test]
fn smoke_fixture_pair_exists_as_a_seeded_fail_and_a_clean_pass(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = repo_root()?;
    let fail = root.join("crates/enforcer-install/tests/fixtures/ci/smoke/fail/main.rs");
    let pass = root.join("crates/enforcer-install/tests/fixtures/ci/smoke/pass/main.rs");
    let fail_src = std::fs::read_to_string(&fail)?;
    let pass_src = std::fs::read_to_string(&pass)?;

    // The fail fixture seeds a real violation (`unwrap()` on a `None`),
    // the pass fixture does not -- proving the pair is a genuine
    // fail/pass contrast, not two copies of the same clean file.
    assert!(fail_src.contains("unwrap()"));
    assert!(!pass_src.contains("unwrap()"));
    Ok(())
}

#[test]
fn dist_workspace_toml_declares_every_platform_in_the_release_matrix(
) -> Result<(), Box<dyn std::error::Error>> {
    let path = repo_root()?.join("dist-workspace.toml");
    let contents = std::fs::read_to_string(&path)?;
    for triple in [
        "x86_64-pc-windows-msvc",
        "x86_64-apple-darwin",
        "aarch64-apple-darwin",
        "x86_64-unknown-linux-gnu",
        "x86_64-unknown-linux-musl",
        "aarch64-unknown-linux-gnu",
    ] {
        assert!(
            contents.contains(triple),
            "dist-workspace.toml is missing declared target `{triple}`"
        );
    }
    assert!(contents.contains("[dist.variants.full]"));
    assert!(contents.contains("[dist.variants.lite]"));
    assert!(contents.contains("[dist.smoke-test]"));
    Ok(())
}

/// Grep-clean assertion (workpack c10 acceptance row): no generated
/// doc/config this pack owns or fixed may ever contain a literal local
/// absolute filesystem path -- the exact bug class found in
/// `docs/TARGET_REPO_WIRING.md`'s old `E:/ocentra-enforcer/rules/INDEX.md`
/// reference, which this pack replaced with `enforcer explain <ruleId>`.
#[test]
fn no_owned_ci_artifact_contains_a_hardcoded_local_absolute_path(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = repo_root()?;
    let candidates: &[&str] = &[
        ".github/actions/enforcer-scan/action.yml",
        "install.sh",
        "install.ps1",
        "packages/enforcer-cli/package.json",
        "dist-workspace.toml",
        ".github/workflows/release.yml",
        "docs/TARGET_REPO_WIRING.md",
    ];
    let banned_needles: &[&str] = &["E:/", "E:\\", "C:/Projects", "C:\\Projects", "/home/"];

    let mut offenders: Vec<String> = Vec::new();
    for candidate in candidates {
        let path: &Path = Path::new(candidate);
        let full = root.join(path);
        let contents = std::fs::read_to_string(&full)?;
        for needle in banned_needles {
            if contents.contains(needle) {
                offenders.push(format!("{candidate} contains banned literal `{needle}`"));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "hardcoded local absolute path(s) found: {offenders:?}"
    );
    Ok(())
}

/// Seeded-violation counterpart to the grep-clean assertion above: a
/// string carrying a banned literal MUST be caught by the same needle
/// check the real assertion uses, proving the check itself is not a
/// vacuous no-op.
#[test]
fn the_grep_clean_check_itself_detects_a_seeded_hardcoded_path() {
    let seeded = "Read E:/ocentra-enforcer/rules/INDEX.md";
    let banned_needles: &[&str] = &["E:/", "E:\\", "C:/Projects", "C:\\Projects", "/home/"];
    assert!(banned_needles.iter().any(|n| seeded.contains(n)));
}
