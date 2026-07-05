//! f05 proof: `router-detect-route-plan` + `router-scope-narrowing`.
//!
//! Runs every fixture mini-repo under `tests/fixtures/router/<case>/`
//! through [`enforcer_scan::walk::walk`], [`enforcer_scan::router::detect`],
//! [`enforcer_scan::router::scope`], and
//! [`enforcer_scan::router::plan::build_route_plan`], asserting the emitted
//! [`enforcer_scan::router::plan::RoutePlan`] matches what each fixture's
//! name promises. Fixtures assert on the emitted plan, never on side
//! effects (T1 deterministic, no network, no native-tool invocation).

use enforcer_config::project_tie::{
    load_project_tie, EnforcerScope, NativeMode, NativeTool, ResolvedProjectTie,
};
use enforcer_scan::router::detect::DetectedLanguage;
use enforcer_scan::router::plan::{build_route_plan, RoutePlanScope, RulePack};
use enforcer_scan::router::scope::RouteScope;
use enforcer_scan::walk::{walk, IgnoreRules};
use std::path::{Path, PathBuf};

fn fixture_root(case: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/router")
        .join(case)
}

fn walked_paths(
    root: &Path,
) -> Result<Vec<enforcer_domain::paths::RelPath>, Box<dyn std::error::Error>> {
    Ok(walk(root, &IgnoreRules::default())?)
}

/// Load the f03 tie from `root/.enforce/config` when it exists on disk.
/// Real `.enforce/` directories are gitignored repo-wide (they are the
/// on-disk artifact a real onboarded project writes, not committed source),
/// so fixtures that need a non-default tie instead commit a flat
/// `enforce.config.json` file (mirroring f03's own fixture convention) and
/// this helper falls back to loading that when no real `.enforce/config`
/// is present.
fn tie_for(root: &Path) -> Result<ResolvedProjectTie, Box<dyn std::error::Error>> {
    let real_config = root.join(".enforce/config");
    if real_config.exists() {
        return Ok(load_project_tie(&real_config)?);
    }
    let flat_fixture = root.join("enforce.config.json");
    Ok(load_project_tie(&flat_fixture)?)
}

/// Fail fixture (mixed repo, missing-ts assertion) inverted into a pass
/// assertion: a mixed `Cargo.toml`+`package.json` repo routes BOTH rust and
/// ts packs and BOTH their native tools. The would-be fail case (a plan
/// missing ts when `package.json` is present) is asserted negatively right
/// here — `assert!(...ts pack present...)` — rather than as a separate
/// fixture directory, since the router has no "leave ts out" code path to
/// exercise as a distinct fixture; the assertion IS the fail-guard.
#[test]
fn mixed_repo_routes_rust_and_ts_packs_and_native_tools() -> Result<(), Box<dyn std::error::Error>>
{
    let root = fixture_root("mixed_rust_ts");
    let paths = walked_paths(&root)?;
    let tie = tie_for(&root)?;
    let plan = build_route_plan(&paths, &RouteScope::Repo, &tie);

    assert!(
        plan.languages.contains(&DetectedLanguage::Rust),
        "fail-guard: rust must be detected when Cargo.toml is present"
    );
    assert!(
        plan.languages.contains(&DetectedLanguage::TypeScript),
        "fail-guard: ts must be detected when package.json is present, never dropped"
    );
    assert!(plan.rule_packs.contains(&RulePack::Rust));
    assert!(plan.rule_packs.contains(&RulePack::TypeScript));
    assert!(plan.rule_packs.contains(&RulePack::LiteralScanFloor));
    assert!(plan
        .native_tools
        .iter()
        .any(|r| r.tool == NativeTool::Cargo));
    assert!(plan.native_tools.iter().any(|r| r.tool == NativeTool::Tsc));
    Ok(())
}

#[test]
fn rust_only_repo_routes_rust_only_never_leaks_other_packs(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = fixture_root("rust_only");
    let paths = walked_paths(&root)?;
    let tie = tie_for(&root)?;
    let plan = build_route_plan(&paths, &RouteScope::Repo, &tie);

    assert_eq!(plan.languages, vec![DetectedLanguage::Rust]);
    assert!(plan.rule_packs.contains(&RulePack::Rust));
    assert!(
        !plan.rule_packs.contains(&RulePack::TypeScript),
        "fail-guard: a rust-only repo must never route to the ts pack"
    );
    assert!(
        !plan.rule_packs.contains(&RulePack::Python),
        "fail-guard: a rust-only repo must never route to the python pack"
    );
    Ok(())
}

/// python-only folder -> plan routes python only (fail-guard: leaking the
/// rust pack is asserted absent).
#[test]
fn python_only_folder_routes_python_only() -> Result<(), Box<dyn std::error::Error>> {
    let root = fixture_root("python_only");
    let paths = walked_paths(&root)?;
    let tie = tie_for(&root)?;
    let plan = build_route_plan(&paths, &RouteScope::Repo, &tie);

    assert_eq!(plan.languages, vec![DetectedLanguage::Python]);
    assert!(plan.rule_packs.contains(&RulePack::Python));
    assert!(
        !plan.rule_packs.contains(&RulePack::Rust),
        "fail-guard: python-only folder must not leak the rust pack"
    );
    Ok(())
}

/// crate scope -> plan narrows to that crate (fail-guard: a repo-wide leak
/// pulling in the sibling `other/` TypeScript package is asserted absent).
#[test]
fn crate_scope_narrows_to_single_crate_not_repo_wide() -> Result<(), Box<dyn std::error::Error>> {
    let root = fixture_root("crate_scope");
    let paths = walked_paths(&root)?;
    let tie = tie_for(&root)?;
    let scope = RouteScope::Crate("crates/inner".to_owned());
    let plan = build_route_plan(&paths, &scope, &tie);

    assert_eq!(plan.scope, RoutePlanScope::Crate("crates/inner".to_owned()));
    assert_eq!(plan.languages, vec![DetectedLanguage::Rust]);
    assert!(
        !plan.languages.contains(&DetectedLanguage::TypeScript),
        "fail-guard: crate scope must not widen back to repo-wide and pick up the sibling ts package"
    );
    Ok(())
}

/// unknown ext -> plan carries literal-scan T2 only, no T1 blocker
/// (fail-guard: no bogus T1 pack is asserted absent).
#[test]
fn unknown_extension_yields_literal_scan_floor_only() -> Result<(), Box<dyn std::error::Error>> {
    let root = fixture_root("unknown_only");
    let paths = walked_paths(&root)?;
    let tie = tie_for(&root)?;
    let plan = build_route_plan(&paths, &RouteScope::Repo, &tie);

    assert_eq!(plan.languages, vec![DetectedLanguage::Other]);
    assert_eq!(
        plan.rule_packs,
        vec![RulePack::LiteralScanFloor],
        "fail-guard: an unrouted extension must never emit a T1 language pack"
    );
    assert!(plan.native_tools.is_empty());
    Ok(())
}

/// An empty/unknown repo (no walkable files at all) yields an honest empty
/// plan — not a false whole-repo route.
#[test]
fn empty_repo_yields_honest_empty_plan() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let paths = walked_paths(temp.path())?;
    let tie = tie_for(temp.path())?;
    let plan = build_route_plan(&paths, &RouteScope::Repo, &tie);

    assert!(plan.languages.is_empty());
    assert!(plan.rule_packs.is_empty());
    assert!(plan.native_tools.is_empty());
    Ok(())
}

/// The f03 `.enforce/config` tie override (`cargo` -> `override`) is
/// honored on the emitted route plan's native-tool entry, not silently
/// re-defaulted to `Augment`.
#[test]
fn f03_tie_override_is_honored_on_the_route_plan() -> Result<(), Box<dyn std::error::Error>> {
    let root = fixture_root("tie_override");
    let paths = walked_paths(&root)?;
    let tie = tie_for(&root)?;
    let plan = build_route_plan(&paths, &RouteScope::Repo, &tie);

    let cargo_route = plan
        .native_tools
        .iter()
        .find(|r| r.tool == NativeTool::Cargo)
        .ok_or("expected a cargo native tool route in the plan")?;
    assert_eq!(cargo_route.tie.mode, NativeMode::Override);
    assert_eq!(cargo_route.tie.scope, EnforcerScope::Scoped);
    Ok(())
}
