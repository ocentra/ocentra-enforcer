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

use crate::boundary::modes::{validate_scope_input, ScanRequest};
use enforcer_domain::paths::{RelPath, RepoRoot};
use enforcer_domain::scan_types::{
    CommitRef, ResolvedScanPlan, ScanMode, ScanModeError, ScopeRequest, TierFilter,
    TierFilterDecision,
};
use enforcer_domain::severity::Tier;
use std::path::PathBuf;

/// The caller-facing scan request: a [`ScanMode`] plus the optional scope
/// narrowing (`scoped`/`diff`/`plan-scan` inputs) needed to resolve it.
/// `serde`-deserializable so the MCP tool / CLI (neither owned here) can
/// decode a caller payload straight into this shape and call
/// [`ScanRequest::resolve`] at the boundary.
///
/// ROUNDTRIP-TEST: `tests/modes.rs::scan_mode_and_request_round_trip_through_the_external_wire_contract`
/// The named mode selected by the caller.
/// For `scoped`/`plan-scan`: the crate/folder/plan-dir to restrict to.
/// Repo-relative or absolute; normalized during [`ScanRequest::resolve`].
/// Ignored (must be absent) for `full`/`repo`/`workspace`/`diff`.
// SERDE-DEFAULT-JUSTIFICATION: omission means no caller-selected scope;
// resolution deliberately substitutes only the caller's current scope.
/// For `diff`: the older commit-ish endpoint. Required iff `mode` is
/// `diff`.
// SERDE-DEFAULT-JUSTIFICATION: diff mode rejects a missing endpoint.
/// For `diff`: the newer commit-ish endpoint. Required iff `mode` is
/// `diff`.
// SERDE-DEFAULT-JUSTIFICATION: diff mode rejects a missing endpoint.
/// against the caller's cwd by [`ScanRequest::resolve`]) — never
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

/// Return whether a resolved scan tier filter includes `tier`.
#[must_use]
pub fn tier_filter_allows(filter: &TierFilter, tier: Tier) -> TierFilterDecision {
    match filter {
        TierFilter::All => TierFilterDecision::Allowed,
        TierFilter::Only(tiers) if tiers.contains(&tier) => TierFilterDecision::Allowed,
        TierFilter::Only(_) => TierFilterDecision::Rejected,
    }
}

/// Validate a caller-supplied scope string's shape (non-empty, no
/// root-escaping `..`) at THIS boundary, before it reaches
/// [`crate::scope::resolve`]. An absolute (drive-letter/UNC) scope is
/// accepted here — the tri-modal resolver strips the repo-root prefix
/// downstream — so this only rejects structurally-empty or escaping input,
/// never a silent default.
#[cfg(test)]
mod tests {
    use super::{tier_filter_allows, ScanRequest};
    use enforcer_domain::paths::{RelPath, RepoRoot};
    use enforcer_domain::scan_types::ScopeRequest;
    use enforcer_domain::scan_types::{ScanMode, ScanModeError, TierFilter, TierFilterDecision};
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
        let outcome = crate::boundary::modes::decode_scan_mode_json("\"bogus-mode\"");
        assert_eq!(
            outcome.err().map(|error| error.to_string()),
            Some(
                "invalid type: string \"bogus-mode\", expected internally tagged enum ScanModeWire at line 1 column 12"
                    .to_owned()
            )
        );
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
        assert!(matches!(resolved.scope_request, ScopeRequest::Paths(_)));
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
        assert_eq!(
            tier_filter_allows(&resolved.tier_filter, Tier::T1),
            TierFilterDecision::Allowed
        );
        assert_eq!(
            tier_filter_allows(&resolved.tier_filter, Tier::T2),
            TierFilterDecision::Rejected
        );
        assert_eq!(
            tier_filter_allows(&resolved.tier_filter, Tier::T3),
            TierFilterDecision::Rejected
        );
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
        assert!(matches!(resolved.scope_request, ScopeRequest::Diff { .. }));
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
        assert!(matches!(resolved.scope_request, ScopeRequest::Paths(_)));
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
        assert!(matches!(outcome, Err(ScanModeError::Scope(_))));
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
        assert!(matches!(outcome, Err(ScanModeError::Scope(_))));
        Ok(())
    }
}
