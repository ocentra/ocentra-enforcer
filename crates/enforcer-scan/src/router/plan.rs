//! The route plan (f05, the tested surface): a serializable, deterministic
//! `{ scope, languages[], rule_packs[], native_tools[] }` struct that
//! consumers (f01 scan-modes, the check/scan/run MCP tools, c04's
//! deny-hook) consume instead of hardcoding a language or a native tool.
//!
//! [`build_route_plan`] is the single entry point: given a walked path list
//! and the f03 [`ResolvedProjectTie`], it runs [`super::detect`],
//! [`super::scope`], and [`super::native_tie`] and folds their outputs into
//! one [`RoutePlanResponse`]. Fixtures assert on the emitted plan, never on side
//! effects — this module performs no I/O of its own beyond what the caller
//! already walked.

use std::collections::BTreeSet;

use crate::boundary::router::{NativeToolRouteResponse, RoutePlanResponse};
use enforcer_config::project_tie::ResolvedProjectTie;
use enforcer_domain::paths::RelPath;
use enforcer_domain::scan_types::{DetectedLanguage, RouteScope, RulePack};

use super::detect::detect_languages;
use super::identity::{detect_language_identities, DetectedLanguageRoute};
use super::native_tie::native_tools_for;
use super::scope::narrow;

/// Map a [`DetectedLanguage`] to the [`RulePack`]s it routes to (excluding
/// the universal [`RulePack::LiteralScanFloor`], which [`build_route_plan`]
/// attaches once, unconditionally, whenever any language is detected).
/// [`DetectedLanguage::Dart`], [`DetectedLanguage::Go`],
/// [`DetectedLanguage::Cfml`], and [`DetectedLanguage::Other`] have no
/// dedicated `enforcer-lang-*` pack landed yet — they route to zero rule
/// packs (still get the floor + their native tool where f03 has one).
fn rule_packs_for(language: DetectedLanguage) -> Vec<RulePack> {
    match language {
        DetectedLanguage::Rust => vec![RulePack::Rust, RulePack::Security],
        DetectedLanguage::TypeScript => vec![RulePack::TypeScript, RulePack::Security],
        DetectedLanguage::Python => vec![RulePack::Python, RulePack::Security],
        DetectedLanguage::Dart
        | DetectedLanguage::Go
        | DetectedLanguage::Cfml
        | DetectedLanguage::Other => Vec::new(),
    }
}

/// Build the identity-preserving route projection for a walked path list.
///
/// This is intentionally separate from [`build_route_plan`]. The latter is
/// the existing stable wire-compatible coarse plan consumed by current MCP
/// and CLI adapters; P1B supplies the canonical identity result that those
/// consumers will adopt in P1C without changing their wire enums here.
pub fn build_canonical_route_plan(
    paths: &[RelPath],
    scope: &RouteScope,
    include_unknown: bool,
) -> Vec<DetectedLanguageRoute> {
    let narrowed = narrow(paths, scope);
    let narrowed_paths: Vec<RelPath> = narrowed.into_iter().cloned().collect();
    detect_language_identities(&narrowed_paths, include_unknown)
}

// ROUNDTRIP-TEST: `tests/router.rs::route_plan_is_data_driven_and_round_trips_through_json`
// proves the complete DTO, including all nested route DTOs, round-trips.
//
// The full, serializable route plan: what scope it applies to, which
// languages were detected inside that scope, which enforcer rule packs
// each of those routes to, and which native tools (per f03's tie config)
// run alongside them. This is the one struct every consumer (f01, the
// check/scan/run MCP tools, c04) reads instead of hardcoding a language or
// tool.
// The scope this plan was narrowed to (default: whole repo).
// Every language detected within `scope`, in stable sorted order.
// The union of every detected language's rule packs, plus the
// universal literal-scan floor whenever `languages` is non-empty. In
// stable sorted order; never duplicated.
// The native tools selected to run per f03's tie config, one entry per
// detected language that has a mapped [`super::native_tie::NativeToolRouteResponse`].
// In stable sorted order.
// RoutePlanResponse is defined in crate::boundary::router.

// The wire-serializable projection of [`RouteScope`] carried on
// [`RoutePlanResponse`]. `RouteScope` itself is not `Serialize`/`Deserialize` (it
// borrows no data requiring that), but consumers reading a persisted or
// MCP-transmitted plan need a flat, self-describing shape — this mirrors
// [`RouteScope`]'s variants one-to-one.
// RouteScope is the canonical domain enum; its wire serde lives in
// enforcer-domain's scan boundary.
// Whole repository.
// Whole Cargo workspace.
// One Cargo crate, carrying its repo-relative root.
// One non-Cargo package, carrying its repo-relative root.
// An arbitrary folder, carrying its repo-relative root.
// A named monorepo domain, carrying its repo-relative root.
// A git diff range.
// No crate-local route-scope projection is maintained.

/// Build the [`RoutePlanResponse`] for a walked, repo-relative path list, narrowed
/// to `scope`, with native tools attached per `tie` (f03's resolved
/// `.enforce/config` view).
///
/// Stages, matching the workpack's three-stage charter:
/// 1. narrow `paths` to `scope` ([`super::scope::narrow`]) — this can never
///    widen the input, only shrink it.
/// 2. detect languages within the narrowed set
///    ([`super::detect::detect_languages`]).
/// 3. route each detected language to its [`RulePack`]s and, per `tie`, its
///    [`NativeToolRouteResponse`] ([`super::native_tie::native_tools_for`]).
///
/// An empty or fully-unknown narrowed set yields an honest empty plan
/// (`languages`/`rule_packs`/`native_tools` all empty) — never a false
/// route into a T1 pack or native tool that was not actually detected.
pub fn build_route_plan(
    paths: &[RelPath],
    scope: &RouteScope,
    tie: &ResolvedProjectTie,
) -> RoutePlanResponse {
    let narrowed = narrow(paths, scope);
    let narrowed_paths: Vec<RelPath> = narrowed.into_iter().cloned().collect();
    let languages_set = detect_languages(&narrowed_paths);
    let languages: Vec<DetectedLanguage> = languages_set.iter().copied().collect();

    let mut rule_packs: BTreeSet<RulePack> = BTreeSet::new();
    for language in &languages {
        rule_packs.extend(rule_packs_for(*language));
    }
    if !languages.is_empty() {
        rule_packs.insert(RulePack::LiteralScanFloor);
        // h11: the cyberskills security-audit pack is cross-cutting, like
        // `RulePack::Security` — attached for any detected repo, not
        // gated on a specific language.
        rule_packs.insert(RulePack::SecurityAudit);
    }

    let mut native_tools: Vec<NativeToolRouteResponse> = Vec::new();
    for language in &languages {
        native_tools.extend(native_tools_for(*language, tie));
    }
    native_tools.sort_by_key(|route| route.tool);
    native_tools.dedup_by_key(|route| route.tool);

    RoutePlanResponse {
        // CLONE-JUSTIFICATION: the response owns its canonical scope after
        // the borrowed planning input is released.
        scope: scope.clone(),
        languages,
        rule_packs: rule_packs.into_iter().collect(),
        native_tools,
    }
}

#[cfg(test)]
mod tests {
    use super::build_route_plan;
    use enforcer_config::project_tie::{ProjectConfig, ResolvedProjectTie};
    use enforcer_config::serde::{WireNativeMode, WireNativeTool};
    use enforcer_domain::config_types::{ConfigJson, ConfigSource};
    use enforcer_domain::scan_types::{DetectedLanguage, RouteScope, RulePack};
    use std::str::FromStr;

    fn rel(literal: &str) -> Result<enforcer_domain::paths::RelPath, Box<dyn std::error::Error>> {
        Ok(enforcer_domain::paths::RelPath::from_str(literal)?)
    }

    fn default_tie() -> Result<ResolvedProjectTie, Box<dyn std::error::Error>> {
        Ok(ResolvedProjectTie::resolve(
            &ProjectConfig::default(),
            &ConfigSource::from_owned("<test>".to_owned()),
        )?)
    }

    #[test]
    fn mixed_repo_routes_rust_and_typescript_with_native_tools(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let paths = vec![
            rel("Cargo.toml")?,
            rel("src/lib.rs")?,
            rel("package.json")?,
            rel("web/index.ts")?,
        ];
        let tie = default_tie()?;
        let plan = build_route_plan(&paths, &RouteScope::Repo, &tie);
        assert_eq!(plan.scope, RouteScope::Repo);
        assert_eq!(
            plan.languages,
            vec![DetectedLanguage::Rust, DetectedLanguage::TypeScript]
        );
        assert_eq!(
            plan.rule_packs,
            vec![
                RulePack::Rust,
                RulePack::TypeScript,
                RulePack::Security,
                RulePack::LiteralScanFloor,
                RulePack::SecurityAudit,
            ]
        );
        assert_eq!(
            plan.native_tools
                .iter()
                .map(|route| route.tool)
                .collect::<Vec<_>>(),
            vec![WireNativeTool::Cargo, WireNativeTool::Tsc]
        );
        Ok(())
    }

    #[test]
    fn python_only_folder_does_not_leak_rust_pack() -> Result<(), Box<dyn std::error::Error>> {
        let paths = vec![rel("pyproject.toml")?, rel("app/main.py")?];
        let tie = default_tie()?;
        let plan = build_route_plan(&paths, &RouteScope::Repo, &tie);
        assert_eq!(plan.languages, vec![DetectedLanguage::Python]);
        assert_eq!(
            plan.rule_packs,
            vec![
                RulePack::Python,
                RulePack::Security,
                RulePack::LiteralScanFloor,
                RulePack::SecurityAudit,
            ]
        );
        Ok(())
    }

    #[test]
    fn crate_scope_narrows_plan_to_that_crate_only() -> Result<(), Box<dyn std::error::Error>> {
        let paths = vec![
            rel("crates/enforcer-scan/Cargo.toml")?,
            rel("crates/enforcer-scan/src/lib.rs")?,
            rel("web/package.json")?,
            rel("web/index.ts")?,
        ];
        let tie = default_tie()?;
        let scope = RouteScope::Crate("crates/enforcer-scan".parse()?);
        let plan = build_route_plan(&paths, &scope, &tie);
        assert_eq!(plan.languages, vec![DetectedLanguage::Rust]);
        assert_eq!(
            plan.rule_packs,
            vec![
                RulePack::Rust,
                RulePack::Security,
                RulePack::LiteralScanFloor,
                RulePack::SecurityAudit,
            ]
        );
        Ok(())
    }

    #[test]
    fn unknown_extension_yields_literal_scan_floor_only_never_a_t1_pack(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let paths = vec![rel("script.rb")?];
        let tie = default_tie()?;
        let plan = build_route_plan(&paths, &RouteScope::Repo, &tie);
        assert_eq!(plan.languages, vec![DetectedLanguage::Other]);
        // Both cross-cutting packs (the universal literal-scan floor and
        // h11's security-audit pack) attach; no language-exclusive T1
        // pack (`Rust`/`TypeScript`/`Python`) does.
        assert_eq!(
            plan.rule_packs,
            vec![RulePack::LiteralScanFloor, RulePack::SecurityAudit]
        );
        Ok(())
    }

    #[test]
    fn any_detected_repo_attaches_the_h11_security_audit_pack(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let paths = vec![rel("Cargo.toml")?, rel("src/lib.rs")?];
        let tie = default_tie()?;
        let plan = build_route_plan(&paths, &RouteScope::Repo, &tie);
        assert_eq!(
            plan.rule_packs,
            vec![
                RulePack::Rust,
                RulePack::Security,
                RulePack::LiteralScanFloor,
                RulePack::SecurityAudit,
            ]
        );
        Ok(())
    }

    #[test]
    fn empty_repo_yields_honest_empty_plan() -> Result<(), Box<dyn std::error::Error>> {
        let paths: Vec<enforcer_domain::paths::RelPath> = Vec::new();
        let tie = default_tie()?;
        let plan = build_route_plan(&paths, &RouteScope::Repo, &tie);
        assert!(plan.languages.is_empty());
        assert!(plan.rule_packs.is_empty());
        assert!(plan.native_tools.is_empty());
        Ok(())
    }

    #[test]
    fn native_override_mode_still_selects_the_tool_route_present(
    ) -> Result<(), Box<dyn std::error::Error>> {
        // f03's `Override` mode means our checks stand down for that tool,
        // but the tool itself remains the selected native tool for the
        // language — the route plan still names it (consumers read
        // `runs_enforcer_checks`/`runs_native_tool` off the tie itself).
        let raw = serde_json::json!({
            "native": { "cargo": { "mode": "override" } }
        })
        .to_string();
        let tie = enforcer_config::project_tie::parse_project_tie(
            &ConfigJson::from_owned(raw),
            &ConfigSource::from_owned("<test>".to_owned()),
        )?;
        let paths = vec![rel("Cargo.toml")?, rel("src/lib.rs")?];
        let plan = build_route_plan(&paths, &RouteScope::Repo, &tie);
        let cargo_route = plan
            .native_tools
            .iter()
            .find(|route| route.tool == WireNativeTool::Cargo)
            .ok_or("expected a cargo native tool route")?;
        assert_eq!(cargo_route.tie.mode, WireNativeMode::Override);
        Ok(())
    }
}
