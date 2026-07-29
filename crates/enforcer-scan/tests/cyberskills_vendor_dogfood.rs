//! h11 acceptance proof: `cyberskills_vendor_not_dogfooded` — a self-host
//! walk of THIS repository, filtered through the `ocentra-enforcer`
//! profile's own committed `ignoreFileGlobs` (arc-03), yields ZERO walked
//! paths under `vendor/anthropic-cybersecurity-skills/**`. The vendored
//! corpus (817 Python-backed skills) must never enter our own dogfood
//! scan — "dogfood must not drown in vendored Python" (workpack
//! "Dogfood exclusion (fail-closed)").
//!
//! This is a real walk of the actual repo root (not a synthetic fixture
//! tree), because the acceptance criterion is specifically about the
//! REAL vendored directory this repo carries — a synthetic stand-in would
//! not prove the actual profile config + actual vendor path line up.

use std::path::{Path, PathBuf};

use enforcer_config::resolve::resolve_profile_only;
use enforcer_domain::config_types::ConfigProfileName;
use enforcer_scan::walk::{walk, IgnoreRules};

fn repo_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    // CARGO_MANIFEST_DIR is `<repo>/crates/enforcer-scan`.
    Ok(Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?)
}

#[test]
fn cyberskills_vendor_not_dogfooded() -> Result<(), Box<dyn std::error::Error>> {
    let root = repo_root()?;
    let vendor_dir = root.join("vendor/anthropic-cybersecurity-skills");
    if !vendor_dir.is_dir() {
        // Vendor-absent (L12 honesty protocol): nothing to prove here —
        // do not fabricate a corpus that is not actually vendored.
        return Ok(());
    }

    let profile_name = ConfigProfileName::new("ocentra-enforcer".to_owned())?;
    let config = resolve_profile_only(&profile_name)?;

    let rules = IgnoreRules::new(config.ignore_dirs, config.ignore_file_globs);

    let walked = walk(&root, &rules)?;
    let vendor_hits: Vec<_> = walked
        .iter()
        .filter(|path| path.as_str().starts_with("vendor/"))
        .collect();

    assert!(
        vendor_hits.is_empty(),
        "expected zero walked paths under vendor/, found: {vendor_hits:?}"
    );
    Ok(())
}

/// Regression guard named directly for the acceptance criterion: a scan
/// that DID walk `vendor/**` (i.e. omitted the `vendor/*` ignore glob)
/// would fail this test — asserted by constructing `IgnoreRules` WITHOUT
/// the vendor glob and confirming the walk THEN finds vendor paths (proves
/// the glob is the thing doing the excluding, not an accidental absence of
/// vendor files).
#[test]
fn without_the_vendor_glob_the_walk_would_see_vendor_files(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = repo_root()?;
    let vendor_dir = root.join("vendor/anthropic-cybersecurity-skills");
    if !vendor_dir.is_dir() {
        return Ok(());
    }

    let rules = IgnoreRules::default();
    let walked = walk(&root, &rules)?;
    let vendor_hits = walked
        .iter()
        .filter(|path| path.as_str().starts_with("vendor/"))
        .count();
    assert!(
        vendor_hits > 0,
        "expected the unfiltered walk to see vendor files (sanity check that the corpus is \
         actually present and would otherwise be walked)"
    );
    Ok(())
}
