//! The route plan (f05, the tested surface): a serializable, deterministic
//! `{ scope, languages[], rule_packs[], native_tools[] }` struct that
//! consumers (f01 scan-modes, the check/scan/run MCP tools, c04's
//! deny-hook) consume instead of hardcoding a language or a native tool.
//!
//! [`build_route_plan`] is the single entry point: given a walked path list
//! and the f03 [`ResolvedProjectTie`], it runs [`super::detect`],
//! [`super::scope`], and [`super::native_tie`] and folds their outputs into
//! one [`RoutePlan`]. Fixtures assert on the emitted plan, never on side
//! effects — this module performs no I/O of its own beyond what the caller
//! already walked.

use std::collections::BTreeSet;

use enforcer_config::project_tie::ResolvedProjectTie;
use enforcer_domain::paths::RelPath;
use serde::{Deserialize, Serialize};

use super::detect::{detect_languages, DetectedLanguage};
use super::native_tie::{native_tools_for, NativeToolRoute};
use super::scope::{narrow, RouteScope};

/// The enforcer rule pack a detected language routes to. One variant per
/// landed `enforcer-lang-*` family crate (arc-06..12), plus the always-on
/// literal-scan universal floor (arc-13). Deliberately does not
/// reimplement any pack — this is a selection key only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RulePack {
    /// `enforcer-lang-rust` (arc-06).
    Rust,
    /// `enforcer-lang-ts` (arc-07).
    TypeScript,
    /// `enforcer-lang-py` (arc-08).
    Python,
    /// `enforcer-lang-security` (arc-10), attached alongside every detected
    /// language's own pack — security rules are cross-cutting, not
    /// language-exclusive.
    Security,
    /// The literal-scan universal floor (arc-13) — the T2 floor every
    /// detected file gets, including [`DetectedLanguage::Other`] and
    /// otherwise-unrouted extensions. Never a T1 blocker on its own.
    LiteralScanFloor,
    /// The h11 cyberskills-corpus security-audit pack
    /// (`enforcer-lang-security::rules::cyberskills` +
    /// `enforcer-security::cyberskills`) — IaC/cloud/manifest/header
    /// predicates plus the scored WAF-SQLi matcher, harvested from the
    /// vendored `anthropic-cybersecurity-skills` corpus. Cross-cutting like
    /// [`RulePack::Security`] (attached alongside every detected
    /// language's own pack, not language-exclusive): [`build_route_plan`]
    /// attaches it once, unconditionally, whenever any language is
    /// detected, exactly like [`RulePack::LiteralScanFloor`]. Additive
    /// registration only (h11) — this variant does not replace or
    /// restructure [`RulePack::Security`] or any pre-existing routing
    /// rule.
    SecurityAudit,
}

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

/// The full, serializable route plan: what scope it applies to, which
/// languages were detected inside that scope, which enforcer rule packs
/// each of those routes to, and which native tools (per f03's tie config)
/// run alongside them. This is the one struct every consumer (f01, the
/// check/scan/run MCP tools, c04) reads instead of hardcoding a language or
/// tool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoutePlan {
    /// The scope this plan was narrowed to (default: whole repo).
    pub scope: RoutePlanScope,
    /// Every language detected within `scope`, in stable sorted order.
    pub languages: Vec<DetectedLanguage>,
    /// The union of every detected language's rule packs, plus the
    /// universal literal-scan floor whenever `languages` is non-empty. In
    /// stable sorted order; never duplicated.
    pub rule_packs: Vec<RulePack>,
    /// The native tools selected to run per f03's tie config, one entry per
    /// detected language that has a mapped [`super::native_tie::NativeToolRoute`].
    /// In stable sorted order.
    pub native_tools: Vec<NativeToolRoute>,
}

/// The wire-serializable projection of [`RouteScope`] carried on
/// [`RoutePlan`]. `RouteScope` itself is not `Serialize`/`Deserialize` (it
/// borrows no data requiring that), but consumers reading a persisted or
/// MCP-transmitted plan need a flat, self-describing shape — this mirrors
/// [`RouteScope`]'s variants one-to-one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind", content = "root")]
pub enum RoutePlanScope {
    /// Whole repository.
    Repo,
    /// Whole Cargo workspace.
    Workspace,
    /// One Cargo crate, carrying its repo-relative root.
    Crate(String),
    /// One non-Cargo package, carrying its repo-relative root.
    Package(String),
    /// An arbitrary folder, carrying its repo-relative root.
    Folder(String),
    /// A named monorepo domain, carrying its repo-relative root.
    Domain(String),
    /// A git diff range.
    Diff,
}

impl From<&RouteScope> for RoutePlanScope {
    fn from(scope: &RouteScope) -> Self {
        match scope {
            RouteScope::Repo => RoutePlanScope::Repo,
            RouteScope::Workspace => RoutePlanScope::Workspace,
            RouteScope::Crate(root) => RoutePlanScope::Crate(root.clone()),
            RouteScope::Package(root) => RoutePlanScope::Package(root.clone()),
            RouteScope::Folder(root) => RoutePlanScope::Folder(root.clone()),
            RouteScope::Domain(root) => RoutePlanScope::Domain(root.clone()),
            RouteScope::Diff => RoutePlanScope::Diff,
        }
    }
}

/// Build the [`RoutePlan`] for a walked, repo-relative path list, narrowed
/// to `scope`, with native tools attached per `tie` (f03's resolved
/// `.enforce/config` view).
///
/// Stages, matching the workpack's three-stage charter:
/// 1. narrow `paths` to `scope` ([`super::scope::narrow`]) — this can never
///    widen the input, only shrink it.
/// 2. detect languages within the narrowed set
///    ([`super::detect::detect_languages`]).
/// 3. route each detected language to its [`RulePack`]s and, per `tie`, its
///    [`NativeToolRoute`] ([`super::native_tie::native_tools_for`]).
///
/// An empty or fully-unknown narrowed set yields an honest empty plan
/// (`languages`/`rule_packs`/`native_tools` all empty) — never a false
/// route into a T1 pack or native tool that was not actually detected.
pub fn build_route_plan(
    paths: &[RelPath],
    scope: &RouteScope,
    tie: &ResolvedProjectTie,
) -> RoutePlan {
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

    let mut native_tools: Vec<NativeToolRoute> = Vec::new();
    for language in &languages {
        native_tools.extend(native_tools_for(*language, tie));
    }
    native_tools.sort_by_key(|route| route.tool);
    native_tools.dedup_by_key(|route| route.tool);

    RoutePlan {
        scope: RoutePlanScope::from(scope),
        languages,
        rule_packs: rule_packs.into_iter().collect(),
        native_tools,
    }
}

#[cfg(test)]
mod tests {
    use super::{build_route_plan, RoutePlanScope, RulePack};
    use crate::router::detect::DetectedLanguage;
    use crate::router::scope::RouteScope;
    use enforcer_config::project_tie::{NativeMode, NativeTool, ProjectConfig, ResolvedProjectTie};
    use std::str::FromStr;

    fn rel(path: &str) -> Result<enforcer_domain::paths::RelPath, Box<dyn std::error::Error>> {
        Ok(enforcer_domain::paths::RelPath::from_str(path)?)
    }

    fn default_tie() -> Result<ResolvedProjectTie, Box<dyn std::error::Error>> {
        Ok(ResolvedProjectTie::resolve(
            &ProjectConfig::default(),
            "<test>",
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
        assert_eq!(plan.scope, RoutePlanScope::Repo);
        assert!(plan.languages.contains(&DetectedLanguage::Rust));
        assert!(plan.languages.contains(&DetectedLanguage::TypeScript));
        assert!(plan.rule_packs.contains(&RulePack::Rust));
        assert!(plan.rule_packs.contains(&RulePack::TypeScript));
        assert!(plan.rule_packs.contains(&RulePack::LiteralScanFloor));
        assert!(plan
            .native_tools
            .iter()
            .any(|route| route.tool == NativeTool::Cargo));
        assert!(plan
            .native_tools
            .iter()
            .any(|route| route.tool == NativeTool::Tsc));
        Ok(())
    }

    #[test]
    fn python_only_folder_does_not_leak_rust_pack() -> Result<(), Box<dyn std::error::Error>> {
        let paths = vec![rel("pyproject.toml")?, rel("app/main.py")?];
        let tie = default_tie()?;
        let plan = build_route_plan(&paths, &RouteScope::Repo, &tie);
        assert_eq!(plan.languages, vec![DetectedLanguage::Python]);
        assert!(!plan.rule_packs.contains(&RulePack::Rust));
        assert!(plan.rule_packs.contains(&RulePack::Python));
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
        let scope = RouteScope::Crate("crates/enforcer-scan".to_owned());
        let plan = build_route_plan(&paths, &scope, &tie);
        assert_eq!(plan.languages, vec![DetectedLanguage::Rust]);
        assert!(!plan.languages.contains(&DetectedLanguage::TypeScript));
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
        assert!(plan.rule_packs.contains(&RulePack::SecurityAudit));
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
        let tie = enforcer_config::project_tie::parse_project_tie(&raw, "<test>")?;
        let paths = vec![rel("Cargo.toml")?, rel("src/lib.rs")?];
        let plan = build_route_plan(&paths, &RouteScope::Repo, &tie);
        let cargo_route = plan
            .native_tools
            .iter()
            .find(|route| route.tool == NativeTool::Cargo)
            .ok_or("expected a cargo native tool route")?;
        assert_eq!(cargo_route.tie.mode, NativeMode::Override);
        Ok(())
    }
}
