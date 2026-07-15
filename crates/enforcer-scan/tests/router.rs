//! f05 proof: `router-detect-route-plan` + `router-scope-narrowing`.
//!
//! Runs every fixture mini-repo under `tests/fixtures/router/<case>/`
//! through [`enforcer_scan::walk::walk`], [`enforcer_scan::router::detect`],
//! [`enforcer_scan::router::scope`], and
//! [`enforcer_scan::router::plan::build_route_plan`], asserting the emitted
//! [`enforcer_scan::router::plan::RoutePlanDto`] matches what each fixture's
//! name promises. Fixtures assert on the emitted plan, never on side
//! effects (T1 deterministic, no network, no native-tool invocation).

use enforcer_config::project_tie::{load_project_tie, ResolvedProjectTie};
use enforcer_config::serde::{WireEnforcerScope, WireNativeMode, WireNativeTool};
use enforcer_scan::router::detect::DetectedLanguage;
use enforcer_scan::router::plan::{build_route_plan, RoutePlanDto, RoutePlanScope, RulePack};
use enforcer_scan::router::scope::RouteScope;
use enforcer_scan::walk::{walk, IgnoreRules};
use std::collections::BTreeSet;
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

    assert_eq!(
        plan.languages,
        vec![DetectedLanguage::Rust, DetectedLanguage::TypeScript],
        "mixed manifests must detect exactly Rust and TypeScript"
    );
    assert_eq!(
        plan.rule_packs,
        vec![
            RulePack::Rust,
            RulePack::TypeScript,
            RulePack::Security,
            RulePack::LiteralScanFloor,
            RulePack::SecurityAudit,
        ],
        "mixed repositories must route both language packs plus the cross-cutting packs"
    );
    assert_eq!(
        plan.native_tools.iter().map(|route| route.tool).collect::<Vec<_>>(),
        vec![WireNativeTool::Cargo, WireNativeTool::Tsc],
        "mixed repositories must attach the native tool for each detected language"
    );
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
    assert_eq!(
        plan.rule_packs,
        vec![
            RulePack::Rust,
            RulePack::Security,
            RulePack::LiteralScanFloor,
            RulePack::SecurityAudit,
        ],
        "a Rust-only repository must route its language pack plus cross-cutting packs"
    );
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
    assert_eq!(
        plan.rule_packs,
        vec![
            RulePack::Python,
            RulePack::Security,
            RulePack::LiteralScanFloor,
            RulePack::SecurityAudit,
        ],
        "a Python-only folder must route its language pack plus cross-cutting packs"
    );
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
    // Both cross-cutting packs (the universal literal-scan floor and h11's
    // security-audit pack) attach; no T1 language-exclusive pack does.
    assert_eq!(
        plan.rule_packs,
        vec![RulePack::LiteralScanFloor, RulePack::SecurityAudit],
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
        .find(|r| r.tool == WireNativeTool::Cargo)
        .ok_or("expected a cargo native tool route in the plan")?;
    assert_eq!(cargo_route.tie.mode, WireNativeMode::Override);
    assert_eq!(cargo_route.tie.scope, WireEnforcerScope::Scoped);
    Ok(())
}

/// The requirement-checklist calls the emitted ROUTE PLAN "the tested
/// surface" and a "serializable `serde` struct", and the workpack mandates
/// keeping it data-driven for the Tauri UI. That contract is only real if
/// the plan actually survives the JSON hop the UI (and the check/scan/run
/// MCP tools) read it over. This proves the full plan round-trips through
/// `serde_json` unchanged AND that the wire shape is the self-describing,
/// camelCase form a TS consumer expects — a `kind`-tagged scope and
/// camelCase language/pack ids (`typeScript`, `literalScanFloor`), never
/// Rust enum debug names.
#[test]
fn route_plan_is_data_driven_and_round_trips_through_json() -> Result<(), Box<dyn std::error::Error>>
{
    let root = fixture_root("mixed_rust_ts");
    let paths = walked_paths(&root)?;
    let tie = tie_for(&root)?;
    let plan = build_route_plan(&paths, &RouteScope::Repo, &tie);

    // Core contract: serialize -> deserialize is the identity on a plan.
    let json = serde_json::to_string(&plan)?;
    let restored: RoutePlanDto = serde_json::from_str(&json)?;
    assert_eq!(
        plan, restored,
        "a RoutePlanDto must survive a serde_json round-trip byte-for-byte-equivalently"
    );

    // Wire-shape contract the Tauri UI depends on: a flat, self-describing
    // JSON object with a discriminated scope and camelCase id tokens.
    let value: serde_json::Value = serde_json::from_str(&json)?;
    assert_eq!(
        value["scope"]["kind"], "repo",
        "scope must serialize as a `kind`-tagged discriminated union for the UI"
    );
    let rule_packs = value["rulePacks"]
        .as_array()
        .ok_or("`rulePacks` must serialize as a JSON array")?;
    assert!(
        rule_packs.iter().any(|p| p == "typeScript"),
        "rule-pack ids must be camelCase tokens (`typeScript`), got {rule_packs:?}"
    );
    assert!(
        rule_packs.iter().any(|p| p == "literalScanFloor"),
        "the universal floor id must serialize as `literalScanFloor`, got {rule_packs:?}"
    );
    let languages = value["languages"]
        .as_array()
        .ok_or("`languages` must serialize as a JSON array")?;
    assert!(
        languages.iter().any(|l| l == "typeScript"),
        "detected-language ids must be camelCase tokens, got {languages:?}"
    );
    let native_tools = value["nativeTools"]
        .as_array()
        .ok_or("`nativeTools` must serialize as a JSON array")?;
    assert!(
        native_tools
            .iter()
            .any(|t| t["tool"] == "cargo" && t["tie"]["mode"].is_string()),
        "each native-tool entry must carry its tool id plus a resolved tie, got {native_tools:?}"
    );
    Ok(())
}

/// Parity over the router's route-plan case ids (the Rust-native analog of
/// d01's reverse-orphan sweep, scoped to f05's owned surface): every case
/// this suite declares must have exactly one on-disk fixture directory, and
/// every on-disk fixture directory must be a declared case. A fixture added
/// without a driving test (an orphan route-plan id) — or a case whose
/// fixture was deleted (a dangling one) — fails closed here instead of going
/// silently unproven. Each declared case is then driven end-to-end
/// (detect -> scope -> plan -> JSON round-trip) so the id -> fixture -> plan
/// chain is proven intact for all of them, not just the ones with a bespoke
/// assertion above.
#[test]
fn every_router_fixture_case_is_declared_and_proven() -> Result<(), Box<dyn std::error::Error>> {
    // The canonical registry of route-plan cases this suite proves. Each
    // name is a subdirectory of `tests/fixtures/router/` exercised by a
    // detection test in this file.
    const DECLARED_CASES: &[&str] = &[
        "mixed_rust_ts",
        "rust_only",
        "python_only",
        "crate_scope",
        "unknown_only",
        "tie_override",
    ];

    let fixtures_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/router");
    let on_disk: BTreeSet<String> = std::fs::read_dir(&fixtures_dir)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect();
    let declared: BTreeSet<String> = DECLARED_CASES.iter().map(|c| (*c).to_owned()).collect();
    assert_eq!(
        on_disk, declared,
        "route-plan fixture cases must be in parity with the declared set — a dir present \
         on disk but not declared is an orphan fixture; a declared case with no dir is dangling"
    );

    // Forward chain: every declared id resolves to a fixture that produces a
    // real, serializable plan.
    for case in DECLARED_CASES {
        let root = fixture_root(case);
        let paths = walked_paths(&root)?;
        let tie = tie_for(&root)?;
        let plan = build_route_plan(&paths, &RouteScope::Repo, &tie);
        let json = serde_json::to_string(&plan).map_err(|e| format!("{case}: serialize: {e}"))?;
        let restored: RoutePlanDto =
            serde_json::from_str(&json).map_err(|e| format!("{case}: deserialize: {e}"))?;
        assert_eq!(
            plan, restored,
            "case `{case}`: plan must round-trip unchanged"
        );
    }
    Ok(())
}
