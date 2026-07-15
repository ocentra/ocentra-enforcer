//! Scan modes (f01): the caller-facing "what kind of run is this" selector
//! an AI agent (or the `enforcer scan --mode <m>` CLI / `enforcer_scan` MCP
//! tool, both `deps: f01`, neither owned here) picks inline while coding.
//!
//! **SKELETON BOUNDARY**: arc-15 laid a minimal 3-variant [`ScanMode`]
//! (`Files`/`Diff`/`All`) mapping 1:1 to [`crate::scope::ScopeRequest`]. f01
//! (this file) owns `src/modes.rs` outright and supersedes that skeleton
//! with the full named-mode surface the workpack calls for: `quick`,
//! `full`, `repo`, `workspace`, `scoped`, `diff`, `plan-scan`. No other
//! crate referenced the skeleton's `ScanMode`/`request_for_mode` (grep
//! confirmed at authoring time), so this is a same-file supersession, not a
//! breaking change to a consumed API.
//!
//! Each named mode maps to exactly one [`crate::scope::ScopeRequest`] (the
//! arc-15 tri-modal resolver's input) plus a [`TierFilter`] driving the
//! rule/tier subset the arc-15 fan-out ([`crate::engine::run`]) should
//! apply: `quick` = a named T1-only subset, `full` = every tier, the rest
//! default to T1 (the mechanical-enforcement floor) unless widened. Parsing
//! is boundary-validated: an unknown mode string or a malformed scope is a
//! typed [`ScanModeError`], never a silent default. The one true default —
//! when a caller supplies no scope at all — is [`ScanMode::Scoped`] (the
//! cwd crate/folder), never whole-repo.

use std::path::PathBuf;
use std::str::FromStr;

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::paths::{RelPath, RepoRoot};
use enforcer_domain::scan_types::{CommitRef, ScopeRequest};
use enforcer_domain::severity::Tier;


/// The named scan modes an agent/CLI/MCP caller selects. Each variant is a
/// caller *intent*; [`ScanRequest::resolve`] turns it into a concrete
/// [`crate::scope::ScopeRequest`] + [`TierFilter`] pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ScanMode {
    /// Fast, most-common T1 subset over the resolved scope.
    Quick,
    /// Everything the enforcer can do (every tier) over the resolved scope.
    Full,
    /// The whole repository tree (an explicit alias of `workspace`, kept
    /// distinct in the wire enum because callers spell it both ways).
    Repo,
    /// The whole workspace tree.
    Workspace,
    /// The current crate/folder only — the safe, narrow default.
    Scoped,
    /// Only files changed between two commit-ish endpoints.
    Diff,
    /// Validate a plan directory (`docs/plans/<name>/**`) rather than
    /// source code.
    PlanScan,
}

impl FromStr for ScanMode {
    type Err = ScanModeError;

    fn from_str(raw: &str) -> Result<Self, ScanModeError> {
        match raw {
            "quick" => Ok(Self::Quick),
            "full" => Ok(Self::Full),
            "repo" => Ok(Self::Repo),
            "workspace" => Ok(Self::Workspace),
            "scoped" => Ok(Self::Scoped),
            "diff" => Ok(Self::Diff),
            "plan-scan" => Ok(Self::PlanScan),
            other => Err(ScanModeError::UnknownMode {
                raw: other.to_owned(),
            }),
        }
    }
}

impl std::fmt::Display for ScanMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Quick => "quick",
            Self::Full => "full",
            Self::Repo => "repo",
            Self::Workspace => "workspace",
            Self::Scoped => "scoped",
            Self::Diff => "diff",
            Self::PlanScan => "plan-scan",
        };
        f.write_str(s)
    }
}

/// Boundary/validation failure for a [`ScanMode`]/[`ScanRequest`]. Every
/// variant is a typed, non-defaulting rejection — a malformed mode or scope
/// never silently falls back to a default mode.
///
/// `enforcer-scan` does not depend on the `thiserror` crate directly (only
/// `enforcer-core` does), so this implements [`std::error::Error`] /
/// [`std::fmt::Display`] by hand, in the same structured-message spirit as
/// [`enforcer_domain::boundary::decode_error::DecodeError`] (which this type wraps and defers
/// to for scope/commit-ish validation failures).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScanModeError {
    /// The mode string did not match any named [`ScanMode`].
    UnknownMode {
        /// The raw, rejected mode string.
        raw: String,
    },
    /// `diff` mode was selected but no `base`/`head` pair was supplied.
    DiffRangeMissing,
    /// A mode other than `diff` was given a `base`/`head` pair, which would
    /// silently be ignored — rejected instead of dropped.
    DiffRangeUnexpected {
        /// The mode that unexpectedly carried a diff range.
        mode: ScanMode,
    },
    /// `plan-scan` mode was selected but no scope path was supplied (there
    /// is no whole-repo default for a plan target).
    PlanScanScopeMissing,
    /// The supplied scope path failed to normalize to a repo-relative path.
    Scope(DecodeError),
}

impl std::fmt::Display for ScanModeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownMode { raw } => write!(
                f,
                "unknown scan mode `{raw}`; expected one of: quick, full, repo, workspace, scoped, diff, plan-scan"
            ),
            Self::DiffRangeMissing => {
                write!(f, "scan mode `diff` requires both a `base` and `head` commit-ish")
            }
            Self::DiffRangeUnexpected { mode } => write!(
                f,
                "`base`/`head` was supplied but scan mode is `{mode}`, not `diff`"
            ),
            Self::PlanScanScopeMissing => write!(
                f,
                "scan mode `plan-scan` requires a `scope` path naming the plan directory"
            ),
            Self::Scope(inner) => write!(f, "scan request scope failed to resolve: {inner}"),
        }
    }
}

impl std::error::Error for ScanModeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Scope(inner) => Some(inner),
            _ => None,
        }
    }
}

impl From<DecodeError> for ScanModeError {
    fn from(inner: DecodeError) -> Self {
        Self::Scope(inner)
    }
}

/// The rule/tier subset a resolved mode drives over the arc-15 fan-out.
/// `None` means every tier (the `full` mode); `Some(tier)` is an inclusive
/// floor — every tier at or below the named mechanical-enforcement
/// strictness. [`Tier`] itself has no total order in `enforcer-domain`, so
/// this floor is expressed as an explicit allow-set rather than a `<=`
/// comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TierFilter {
    /// Every tier — no filtering.
    All,
    /// Only findings whose rule tier is in this explicit set.
    Only(Vec<Tier>),
}

impl TierFilter {
    /// Does `tier` pass this filter?
    #[must_use]
    pub fn allows(&self, tier: Tier) -> bool {
        match self {
            Self::All => true,
            Self::Only(tiers) => tiers.contains(&tier),
        }
    }
}

/// The caller-facing scan request: a [`ScanMode`] plus the optional scope
/// narrowing (`scoped`/`diff`/`plan-scan` inputs) needed to resolve it.
/// `serde`-deserializable so the MCP tool / CLI (neither owned here) can
/// decode a caller payload straight into this shape and call
/// [`ScanRequest::resolve`] at the boundary.
///
/// ROUNDTRIP-TEST: `tests/modes.rs::scan_mode_and_request_round_trip_through_the_external_wire_contract`
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanRequest {
    /// The named mode selected by the caller.
    pub mode: ScanMode,
    /// For `scoped`/`plan-scan`: the crate/folder/plan-dir to restrict to.
    /// Repo-relative or absolute; normalized during [`ScanRequest::resolve`].
    /// Ignored (must be absent) for `full`/`repo`/`workspace`/`diff`.
    // SERDE-DEFAULT-JUSTIFICATION: omission means no caller-selected scope;
    // resolution deliberately substitutes only the caller's current scope.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    /// For `diff`: the older commit-ish endpoint. Required iff `mode` is
    /// `diff`.
    // SERDE-DEFAULT-JUSTIFICATION: diff mode rejects a missing endpoint.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base: Option<String>,
    /// For `diff`: the newer commit-ish endpoint. Required iff `mode` is
    /// `diff`.
    // SERDE-DEFAULT-JUSTIFICATION: diff mode rejects a missing endpoint.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub head: Option<String>,
}

impl Default for ScanRequest {
    /// The no-arg default: `scoped` with no explicit scope path (resolved
    /// against the caller's cwd by [`ScanRequest::resolve`]) — never
    /// whole-repo.
    fn default() -> Self {
        Self {
            mode: ScanMode::Scoped,
            scope: None,
            base: None,
            head: None,
        }
    }
}

/// A [`ScanRequest`] resolved against a repository root into the concrete
/// inputs [`crate::engine::run`] and [`crate::walk::walk`] consume.
/// This is an internal execution plan, not a serialized request payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedScanPlan {
    /// The mode this resolution was computed for.
    pub mode: ScanMode,
    /// The tri-modal scope request feeding [`crate::scope::resolve`].
    pub scope_request: ScopeRequest,
    /// The tier subset this mode drives over the fan-out's findings.
    pub tier_filter: TierFilter,
}

impl ScanRequest {
    /// Resolve this request against `repo_root`, given the caller's current
    /// working directory expressed as a repo-relative path (used only when
    /// `mode` is `scoped` and no explicit `scope` was supplied).
    ///
    /// # Errors
    /// Returns [`ScanModeError`] if: the mode requires a scope/diff range
    /// that is missing; a mode that does not take a diff range was given
    /// one; or a supplied scope/commit-ish string fails to parse. Never
    /// silently substitutes a default for a malformed input.
    pub fn resolve(
        &self,
        repo_root: &RepoRoot,
        cwd_scope: &RelPath,
    ) -> Result<ResolvedScanPlan, ScanModeError> {
        self.reject_unexpected_diff_range()?;
        let (scope_request, tier_filter) = match self.mode {
            ScanMode::Quick => (
                self.scoped_request(repo_root, cwd_scope)?,
                tier_only(&[Tier::T1]),
            ),
            ScanMode::Full => (ScopeRequest::All, TierFilter::All),
            ScanMode::Repo | ScanMode::Workspace => (ScopeRequest::All, tier_only(&[Tier::T1])),
            ScanMode::Scoped => (
                self.scoped_request(repo_root, cwd_scope)?,
                tier_only(&[Tier::T1]),
            ),
            ScanMode::Diff => (self.diff_request()?, tier_only(&[Tier::T1])),
            ScanMode::PlanScan => (
                self.explicit_scope_request(repo_root)?
                    .ok_or(ScanModeError::PlanScanScopeMissing)?,
                tier_only(&[Tier::T1]),
            ),
        };
        Ok(ResolvedScanPlan {
            mode: self.mode,
            scope_request,
            tier_filter,
        })
    }

    /// `scoped`/`quick`: an explicit `scope` if given, else the caller's cwd.
    fn scoped_request(
        &self,
        repo_root: &RepoRoot,
        cwd_scope: &RelPath,
    ) -> Result<ScopeRequest, ScanModeError> {
        match self.explicit_scope_request(repo_root)? {
            Some(request) => Ok(request),
            None => Ok(ScopeRequest::Paths(vec![PathBuf::from(cwd_scope.as_str())])),
        }
    }

    /// Build a `Paths` request from an explicit `scope` string, if present.
    fn explicit_scope_request(
        &self,
        _repo_root: &RepoRoot,
    ) -> Result<Option<ScopeRequest>, ScanModeError> {
        match &self.scope {
            Some(raw) => {
                validate_scope_input(raw)?;
                Ok(Some(ScopeRequest::Paths(vec![PathBuf::from(raw)])))
            }
            None => Ok(None),
        }
    }

    fn diff_request(&self) -> Result<ScopeRequest, ScanModeError> {
        let base = self
            .base
            .as_deref()
            .ok_or(ScanModeError::DiffRangeMissing)?;
        let head = self
            .head
            .as_deref()
            .ok_or(ScanModeError::DiffRangeMissing)?;
        let base: CommitRef = base.parse().map_err(ScanModeError::Scope)?;
        let head: CommitRef = head.parse().map_err(ScanModeError::Scope)?;
        Ok(ScopeRequest::Diff { base, head })
    }

    fn reject_unexpected_diff_range(&self) -> Result<(), ScanModeError> {
        if self.mode != ScanMode::Diff && (self.base.is_some() || self.head.is_some()) {
            return Err(ScanModeError::DiffRangeUnexpected { mode: self.mode });
        }
        Ok(())
    }
}

fn tier_only(tiers: &[Tier]) -> TierFilter {
    TierFilter::Only(tiers.to_vec())
}

/// Validate a caller-supplied scope string's shape (non-empty, no
/// root-escaping `..`) at THIS boundary, before it reaches
/// [`crate::scope::resolve`]. An absolute (drive-letter/UNC) scope is
/// accepted here — the tri-modal resolver strips the repo-root prefix
/// downstream — so this only rejects structurally-empty or escaping input,
/// never a silent default.
fn validate_scope_input(raw: &str) -> Result<(), DecodeError> {
    let normalized = enforcer_core::platform::normalize_separators(raw);
    let trimmed = normalized.trim_start_matches('/');
    if trimmed.is_empty() {
        return Err(DecodeError::new("scanRequest.scope", "must not be empty"));
    }
    if is_drive_or_unc_absolute(trimmed) {
        return Ok(());
    }
    let mut depth: i32 = 0;
    for segment in trimmed.split('/') {
        match segment {
            ".." => {
                depth -= 1;
                if depth < 0 {
                    return Err(DecodeError::new(
                        "scanRequest.scope",
                        "`..` segment escapes the repository root",
                    ));
                }
            }
            "" | "." => {}
            _ => depth += 1,
        }
    }
    Ok(())
}

fn is_drive_or_unc_absolute(normalized: &str) -> bool {
    normalized.starts_with("//")
        || (normalized.len() >= 3
            && normalized.as_bytes()[0].is_ascii_alphabetic()
            && normalized.as_bytes()[1] == b':'
            && normalized.as_bytes()[2] == b'/')
}

#[cfg(test)]
mod tests {
    use super::{ScanMode, ScanModeError, ScanRequest, TierFilter};
    use enforcer_domain::scan_types::ScopeRequest;
    use enforcer_domain::paths::{RelPath, RepoRoot};
    use enforcer_domain::severity::Tier;
    use std::str::FromStr;

    fn repo_root() -> Result<RepoRoot, Box<dyn std::error::Error>> {
        Ok(r"C:\Projects\enforcer".parse()?)
    }

    fn cwd() -> Result<RelPath, Box<dyn std::error::Error>> {
        Ok("crates/enforcer-scan".parse()?)
    }

    #[test]
    fn mode_round_trips_through_display_and_from_str() -> Result<(), ScanModeError> {
        for mode in [
            ScanMode::Quick,
            ScanMode::Full,
            ScanMode::Repo,
            ScanMode::Workspace,
            ScanMode::Scoped,
            ScanMode::Diff,
            ScanMode::PlanScan,
        ] {
            let rendered = mode.to_string();
            let parsed = ScanMode::from_str(&rendered)?;
            assert_eq!(parsed, mode);
        }
        Ok(())
    }

    #[test]
    fn unknown_mode_string_is_rejected_not_defaulted() {
        let outcome = ScanMode::from_str("bogus-mode");
        assert!(matches!(outcome, Err(ScanModeError::UnknownMode { .. })));
    }

    #[test]
    fn serde_boundary_rejects_unknown_mode() {
        let outcome: Result<ScanMode, _> = serde_json::from_str("\"bogus-mode\"");
        assert!(outcome.is_err(), "malformed mode must not silently decode");
    }

    #[test]
    fn no_arg_default_is_scoped_never_whole_repo() {
        let request = ScanRequest::default();
        assert_eq!(request.mode, ScanMode::Scoped);
        assert!(request.scope.is_none());
    }

    #[test]
    fn default_scoped_resolves_to_cwd_paths_not_all() -> Result<(), Box<dyn std::error::Error>> {
        let request = ScanRequest::default();
        let resolved = request.resolve(&repo_root()?, &cwd()?)?;
        assert_eq!(resolved.mode, ScanMode::Scoped);
        assert!(matches!(
            resolved.scope_request,
            ScopeRequest::Paths(_)
        ));
        Ok(())
    }

    #[test]
    fn full_mode_resolves_to_all_scope_and_all_tiers() -> Result<(), Box<dyn std::error::Error>> {
        let request = ScanRequest {
            mode: ScanMode::Full,
            ..ScanRequest::default()
        };
        let resolved = request.resolve(&repo_root()?, &cwd()?)?;
        assert_eq!(resolved.scope_request, ScopeRequest::All);
        assert_eq!(resolved.tier_filter, TierFilter::All);
        Ok(())
    }

    #[test]
    fn quick_mode_is_t1_only() -> Result<(), Box<dyn std::error::Error>> {
        let request = ScanRequest {
            mode: ScanMode::Quick,
            ..ScanRequest::default()
        };
        let resolved = request.resolve(&repo_root()?, &cwd()?)?;
        assert!(resolved.tier_filter.allows(Tier::T1));
        assert!(!resolved.tier_filter.allows(Tier::T2));
        assert!(!resolved.tier_filter.allows(Tier::T3));
        Ok(())
    }

    #[test]
    fn diff_mode_without_range_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
        let request = ScanRequest {
            mode: ScanMode::Diff,
            ..ScanRequest::default()
        };
        let outcome = request.resolve(&repo_root()?, &cwd()?);
        assert!(matches!(outcome, Err(ScanModeError::DiffRangeMissing)));
        Ok(())
    }

    #[test]
    fn diff_mode_with_range_resolves_to_diff_scope() -> Result<(), Box<dyn std::error::Error>> {
        let request = ScanRequest {
            mode: ScanMode::Diff,
            base: Some("main".to_owned()),
            head: Some("HEAD".to_owned()),
            ..ScanRequest::default()
        };
        let resolved = request.resolve(&repo_root()?, &cwd()?)?;
        assert!(matches!(
            resolved.scope_request,
            ScopeRequest::Diff { .. }
        ));
        Ok(())
    }

    #[test]
    fn non_diff_mode_with_range_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
        let request = ScanRequest {
            mode: ScanMode::Full,
            base: Some("main".to_owned()),
            head: Some("HEAD".to_owned()),
            ..ScanRequest::default()
        };
        let outcome = request.resolve(&repo_root()?, &cwd()?);
        assert!(matches!(
            outcome,
            Err(ScanModeError::DiffRangeUnexpected { .. })
        ));
        Ok(())
    }

    #[test]
    fn plan_scan_without_scope_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
        let request = ScanRequest {
            mode: ScanMode::PlanScan,
            ..ScanRequest::default()
        };
        let outcome = request.resolve(&repo_root()?, &cwd()?);
        assert!(matches!(outcome, Err(ScanModeError::PlanScanScopeMissing)));
        Ok(())
    }

    #[test]
    fn plan_scan_with_scope_resolves_to_that_path() -> Result<(), Box<dyn std::error::Error>> {
        let request = ScanRequest {
            mode: ScanMode::PlanScan,
            scope: Some("docs/plans/enforcer-selfhost-plan".to_owned()),
            ..ScanRequest::default()
        };
        let resolved = request.resolve(&repo_root()?, &cwd()?)?;
        assert!(matches!(
            resolved.scope_request,
            ScopeRequest::Paths(_)
        ));
        Ok(())
    }

    #[test]
    fn empty_scope_string_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
        let request = ScanRequest {
            mode: ScanMode::Scoped,
            scope: Some(String::new()),
            ..ScanRequest::default()
        };
        let outcome = request.resolve(&repo_root()?, &cwd()?);
        assert!(outcome.is_err());
        Ok(())
    }

    #[test]
    fn escaping_scope_string_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
        let request = ScanRequest {
            mode: ScanMode::Scoped,
            scope: Some("../../etc/passwd".to_owned()),
            ..ScanRequest::default()
        };
        let outcome = request.resolve(&repo_root()?, &cwd()?);
        assert!(outcome.is_err());
        Ok(())
    }
}
