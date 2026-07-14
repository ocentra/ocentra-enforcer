//! x04 — main-branch protection: EMITS the desired GitHub branch-protection
//! configuration for `main` and VERIFIES the live protection matches it,
//! failing closed (refusing to attest) whenever the protection is missing
//! or bypassable. RUST_ARCHITECTURE.md / workpack
//! `docs/plans/enforcer-selfhost-plan/workpacks/x04-main-branch-protection-ci.md`
//! is this module's charter.
//!
//! # Charter — settings, not parity
//!
//! This module owns configuring and verifying `main`'s protection
//! SETTINGS (required checks present + non-bypassable). It is distinct
//! from d11/d28, which validate CI PARITY (local vs. CI running the same
//! checks) — a different question from "is the trunk mechanically
//! guarded". It is also distinct from [`crate::ci::release_pipeline`]
//! (c10), which builds and gates the release itself and merely *depends*
//! on a protected `main` existing; this module is what makes that
//! assumption true.
//!
//! # Fail-closed by construction
//!
//! [`verify`] never attests protection on a partial match: any missing
//! required check, any bypass allowance (admin override, force-push,
//! deletion), or any required check that is not simultaneously "must be
//! up to date before merge" collapses the verdict to
//! [`Verdict::Refused`]. There is no partial-credit "mostly protected"
//! state — mirroring [`crate::ci::release_pipeline::gate_release`]'s
//! "never averaged, never mostly published" rule for the release gate.
//!
//! # Symbolic check contexts
//!
//! Required-check contexts are never hardcoded to a literal string in
//! this module. [`resolve_contexts`] derives the GitHub status-check
//! context from a workflow's declared `name:` and a job's declared id
//! (crossed with the job's OS matrix, when present) — the same
//! `"{workflow_name} / {job_id} ({matrix_value})"` shape GitHub itself
//! renders as the check-run context. This keeps the desired
//! configuration reconciled against the ACTUAL workflow/job names at
//! build time instead of drifting the way the legacy `docs/BRANCH_PROTECTION.md`
//! did (it named `Ocentra Enforcer / ocentra-enforcer (*)`, a pre-rename
//! context that was never applied as a real setting).

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

/// One CI workflow's declared identity, enough to derive every
/// status-check context it produces without hardcoding the rendered
/// string anywhere. `workflow_name` is the workflow's `name:` field;
/// `job_id` is the job's YAML key (not its `name:`, which GitHub does not
/// use for the check-run context); `matrix` is the job's OS matrix
/// values, in the order GitHub will render them, or empty when the job
/// has no matrix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowJob {
    /// The workflow's `name:` field (e.g. `"Rust CI"`).
    pub workflow_name: String,
    /// The job's YAML key (e.g. `"rust-ci"`).
    pub job_id: String,
    /// The job's matrix values, in declared order (e.g.
    /// `["ubuntu-latest", "windows-latest", "macos-latest"]`), or empty
    /// for a non-matrix job.
    pub matrix: Vec<String>,
}

/// Derive every GitHub status-check context a [`WorkflowJob`] produces.
/// Mirrors GitHub Actions' own rendering: `"{workflow_name} / {job_id}"`
/// for a non-matrix job, or one `"{workflow_name} / {job_id} ({value})"`
/// per matrix value. This is the ONLY place a context string is built —
/// callers reconcile against a workflow's actual declared name/job id/matrix
/// rather than writing a literal context anywhere else.
#[must_use]
pub fn resolve_contexts(job: &WorkflowJob) -> Vec<String> {
    if job.matrix.is_empty() {
        vec![format!("{} / {}", job.workflow_name, job.job_id)]
    } else {
        job.matrix
            .iter()
            .map(|value| format!("{} / {} ({value})", job.workflow_name, job.job_id))
            .collect()
    }
}

/// The desired branch-protection configuration for `main`, as authored in
/// `.github/BRANCH_PROTECTION.md` and applied by [`emit_payload`]. Every
/// field here maps 1:1 onto a bit of GitHub's branch-protection REST
/// schema so the emitted payload and the verified live state are always
/// compared on the same shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesiredProtection {
    /// The CI jobs whose resolved contexts are marked required. Never a
    /// literal string list — always [`WorkflowJob`]s resolved via
    /// [`resolve_contexts`].
    pub required_jobs: Vec<WorkflowJob>,
    /// Require branches to be up to date with `main` before merge
    /// (`required_status_checks.strict`). Must be `true` — a red/pending
    /// required check on a stale branch must not be merge-eligible.
    pub require_up_to_date: bool,
    /// Require pull requests before any change lands on `main`.
    pub require_pull_request: bool,
}

impl DesiredProtection {
    /// The pass-fixture-shaped configuration this repo actually wants for
    /// `main`: PRs required, the resolved CI job(s) required, branches
    /// must be up to date, no bypass allowance anywhere. Every fail
    /// fixture is a deliberate deviation FROM this baseline (missing
    /// checks, or the checks present but bypassable).
    #[must_use]
    pub fn baseline(required_jobs: Vec<WorkflowJob>) -> Self {
        Self {
            required_jobs,
            require_up_to_date: true,
            require_pull_request: true,
        }
    }

    /// Every resolved required-check context this configuration demands,
    /// flattened across all its [`WorkflowJob`]s. Deterministic order
    /// (job declaration order, then matrix order) so the emitted payload
    /// is stable across runs.
    #[must_use]
    pub fn required_contexts(&self) -> Vec<String> {
        let mut seen = BTreeSet::new();
        self.required_jobs
            .iter()
            .flat_map(resolve_contexts)
            .filter(|context| seen.insert(context.clone()))
            .collect()
    }
}

/// The GitHub branch-protection REST payload this module emits (the
/// request body for `PUT /repos/{owner}/{repo}/branches/{branch}/protection`,
/// applied via `gh api`). Field names/shapes mirror GitHub's schema
/// directly so [`emit_payload`]'s output can be sent to `gh api` verbatim
/// and so [`LiveProtectionState`] (the read-back shape) lines up field for
/// field with what was sent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GhApiPayload {
    /// `required_status_checks` block, or `None` when no checks exist to
    /// require (always a fail-fixture state — see [`verify`]).
    pub required_status_checks: Option<RequiredStatusChecks>,
    /// `enforce_admins`: `true` means admins are NOT allowed to bypass the
    /// required checks. Note the API's own naming is the inverse of the
    /// "override allowed" phrasing this crate's fixtures use in prose:
    /// `enforce_admins = true` is the SAFE / non-bypassable setting.
    pub enforce_admins: bool,
    /// `true` requires a pull request (and, transitively, review flow)
    /// before a change can reach `main`.
    pub required_pull_request: bool,
    /// `true` blocks force-pushes to the protected branch.
    pub allow_force_pushes: bool,
    /// `true` allows the branch to be deleted. Always `false` for `main`.
    pub allow_deletions: bool,
}

/// The `required_status_checks` sub-object of the GitHub branch-protection
/// schema.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequiredStatusChecks {
    /// `strict`: require branches to be up to date before merge. Maps to
    /// [`DesiredProtection::require_up_to_date`].
    pub strict: bool,
    /// The required check contexts, resolved via [`resolve_contexts`] —
    /// never a hand-authored literal.
    pub contexts: Vec<String>,
}

/// Build the `gh api` request payload for a [`DesiredProtection`]. This is
/// the emitter half: the ONLY function that turns the desired
/// configuration into the literal JSON body `gh api
/// --method PUT repos/{owner}/{repo}/branches/main/protection --input -`
/// would send. [`verify`]'s pass path is exactly "the live state read back
/// equals this payload's shape".
#[must_use]
pub fn emit_payload(desired: &DesiredProtection) -> GhApiPayload {
    let contexts = desired.required_contexts();
    GhApiPayload {
        required_status_checks: if contexts.is_empty() {
            None
        } else {
            Some(RequiredStatusChecks {
                strict: desired.require_up_to_date,
                contexts,
            })
        },
        enforce_admins: true,
        required_pull_request: desired.require_pull_request,
        allow_force_pushes: false,
        allow_deletions: false,
    }
}

/// The live branch-protection state as read back from `gh api
/// repos/{owner}/{repo}/branches/{branch}/protection` (or a captured
/// fixture standing in for it). Deliberately the SAME shape as
/// [`GhApiPayload`] plus the one field the read API reports that the write
/// API does not need in the request (`checks_are_currently_passing`),
/// which the merge-red fixture exercises.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LiveProtectionState {
    /// Mirrors [`GhApiPayload::required_status_checks`]; `None` means no
    /// required checks are configured at all.
    pub required_status_checks: Option<RequiredStatusChecks>,
    /// Mirrors [`GhApiPayload::enforce_admins`].
    pub enforce_admins: bool,
    /// Mirrors [`GhApiPayload::required_pull_request`].
    pub required_pull_request: bool,
    /// Mirrors [`GhApiPayload::allow_force_pushes`].
    pub allow_force_pushes: bool,
    /// Mirrors [`GhApiPayload::allow_deletions`].
    pub allow_deletions: bool,
    /// Whether every required check's most recent run on the tip of
    /// `main`'s candidate merge is green. `false` (or `None`, meaning
    /// pending) simulates a red/pending required check; the merge-red
    /// fail fixture sets this to `Some(false)`.
    pub required_checks_passing: Option<bool>,
}

/// Why [`verify`] refused to attest protection. Every variant names the
/// specific gap so the CI log (and the fixture's assertion) can identify
/// exactly which requirement failed, mirroring
/// [`crate::ci::release_pipeline::ReleaseGateVerdict::Blocked`]'s "name
/// the failing asset" shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefusalReason {
    /// No required status checks are configured at all.
    NoRequiredChecks,
    /// Required checks exist but the desired context set is not fully
    /// covered by the live contexts (a context is missing).
    MissingRequiredContext {
        /// The desired context that the live state does not require.
        missing: String,
    },
    /// `enforce_admins` is `false`: admins can override the required
    /// checks.
    AdminOverrideAllowed,
    /// `allow_force_pushes` is `true`: force-push to the protected branch
    /// is not blocked.
    ForcePushAllowed,
    /// `allow_deletions` is `true`: the protected branch could be
    /// deleted.
    DeletionAllowed,
    /// `required_status_checks.strict` is `false`: branches are not
    /// required to be up to date before merge, so a stale branch's
    /// already-green (but now-outdated) check can merge without re-running
    /// against `main`'s tip.
    NotRequiredUpToDate,
    /// Pull requests are not required before merging to `main`.
    PullRequestNotRequired,
    /// A required check is red or still pending yet the live state
    /// reports the branch as merge-eligible regardless — the "merge when
    /// red" gap this module exists to close.
    RedCheckMergeEligible,
}

impl RefusalReason {
    /// Stable machine-readable code for CI and installer consumers. The
    /// descriptive text in [`Self::message`] may evolve, but this code is
    /// the reporting contract callers can key automation on.
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::NoRequiredChecks => "no_required_checks",
            Self::MissingRequiredContext { .. } => "missing_required_context",
            Self::AdminOverrideAllowed => "admin_override_allowed",
            Self::ForcePushAllowed => "force_push_allowed",
            Self::DeletionAllowed => "deletion_allowed",
            Self::NotRequiredUpToDate => "not_required_up_to_date",
            Self::PullRequestNotRequired => "pull_request_not_required",
            Self::RedCheckMergeEligible => "red_check_merge_eligible",
        }
    }

    /// Human-readable detail for CI logs and installer reports.
    #[must_use]
    pub fn message(&self) -> String {
        match self {
            Self::NoRequiredChecks => "main has no required status checks".to_owned(),
            Self::MissingRequiredContext { missing } => {
                format!("main is missing required status check: {missing}")
            }
            Self::AdminOverrideAllowed => {
                "main allows administrators to override required checks".to_owned()
            }
            Self::ForcePushAllowed => "main allows force pushes".to_owned(),
            Self::DeletionAllowed => "main allows deletion".to_owned(),
            Self::NotRequiredUpToDate => {
                "main does not require branches to be up to date before merge".to_owned()
            }
            Self::PullRequestNotRequired => {
                "main does not require pull requests before merge".to_owned()
            }
            Self::RedCheckMergeEligible => {
                "required checks are red or pending while the merge remains eligible".to_owned()
            }
        }
    }
}

/// One failed expectation in the serializable installer/CI report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportedRefusal {
    /// Stable code for machines and CI annotations.
    pub code: String,
    /// Concrete explanation for a human reading the CI log.
    pub message: String,
}

/// Stable report of the protection verification contract for `main`.
///
/// This is deliberately separate from [`Verdict`]: callers can serialize it
/// directly into a CI artifact without parsing Rust debug output, while the
/// in-process verdict remains convenient for gates that need an exit code.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BranchProtectionReport {
    /// The only protected branch this workpack evaluates.
    pub branch: String,
    /// Contexts the installer expects this repository to require.
    pub expected_contexts: Vec<String>,
    /// Contexts observed in the captured or live protection response.
    pub observed_contexts: Vec<String>,
    /// `true` only when every protection expectation is satisfied.
    pub attested: bool,
    /// Process-compatible status: zero iff [`Self::attested`] is `true`.
    pub exit_code: i32,
    /// Every unmet expectation, in deterministic verifier order.
    pub refusals: Vec<ReportedRefusal>,
}

/// The verifier's verdict: either it attests protection is in place and
/// non-bypassable, or it refuses — fail-closed, naming every gap found (not
/// just the first one), so one fixture run surfaces the complete list of
/// what must be fixed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// Live state matches (or exceeds) the desired configuration on every
    /// axis: required checks present and covering the desired contexts,
    /// no bypass allowance, up-to-date required, PRs required, and no
    /// required check is red/pending while still merge-eligible.
    Attested,
    /// At least one gap was found. Never emitted for a single gap in
    /// isolation when more exist — carries every [`RefusalReason`] found
    /// in this run.
    Refused(Vec<RefusalReason>),
}

impl Verdict {
    /// `true` iff every requirement passed. Mirrors
    /// [`crate::ci::release_pipeline::ReleaseGateVerdict::may_publish`]'s
    /// all-or-nothing shape: there is no partial-credit "mostly attested"
    /// verdict.
    #[must_use]
    pub fn is_attested(&self) -> bool {
        matches!(self, Self::Attested)
    }

    /// Process exit-code-shaped: `0` when attested, non-zero (the count
    /// of distinct refusal reasons, always >= 1) when refused. Callers map
    /// this onto the process exit code directly rather than re-deriving
    /// "was it refused" from the variant a second time.
    #[must_use]
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::Attested => 0,
            Self::Refused(reasons) => reasons.len().max(1) as i32,
        }
    }
}

/// Verify a captured (or live) [`LiveProtectionState`] against a
/// [`DesiredProtection`], fail-closed. Every gap is collected — this never
/// short-circuits on the first failure — so a single fixture run reports
/// the complete list of what is missing or bypassable, per
/// RUST_ARCHITECTURE.md's fail-closed binding for this workpack.
#[must_use]
pub fn verify(desired: &DesiredProtection, live: &LiveProtectionState) -> Verdict {
    let mut reasons = Vec::new();

    match &live.required_status_checks {
        None => reasons.push(RefusalReason::NoRequiredChecks),
        Some(live_checks) => {
            let desired_contexts = desired.required_contexts();
            if desired_contexts.is_empty() {
                // The desired configuration itself declares no required
                // jobs -- treat as equivalent to "no required checks" so a
                // misconfigured desired-state can never silently attest.
                reasons.push(RefusalReason::NoRequiredChecks);
            }
            for context in &desired_contexts {
                if !live_checks.contexts.contains(context) {
                    reasons.push(RefusalReason::MissingRequiredContext {
                        missing: context.clone(),
                    });
                }
            }
            if desired.require_up_to_date && !live_checks.strict {
                reasons.push(RefusalReason::NotRequiredUpToDate);
            }
        }
    }

    if !live.enforce_admins {
        reasons.push(RefusalReason::AdminOverrideAllowed);
    }
    if live.allow_force_pushes {
        reasons.push(RefusalReason::ForcePushAllowed);
    }
    if live.allow_deletions {
        reasons.push(RefusalReason::DeletionAllowed);
    }
    if desired.require_pull_request && !live.required_pull_request {
        reasons.push(RefusalReason::PullRequestNotRequired);
    }
    // Merge-red: a required check that is red or pending must never be
    // reported merge-eligible. `Some(true)` is the only accepting value;
    // `Some(false)` (red) and `None` (pending/unknown) both refuse.
    if live.required_status_checks.is_some() && live.required_checks_passing != Some(true) {
        reasons.push(RefusalReason::RedCheckMergeEligible);
    }

    if reasons.is_empty() {
        Verdict::Attested
    } else {
        Verdict::Refused(reasons)
    }
}

/// Verify `main` and return the stable report consumed by installer and CI
/// callers. This function performs no network or GitHub mutation; callers may
/// pass a `gh api` read-back or a captured fixture as [`LiveProtectionState`].
#[must_use]
pub fn verify_and_report(
    desired: &DesiredProtection,
    live: &LiveProtectionState,
) -> BranchProtectionReport {
    let verdict = verify(desired, live);
    let refusals = match &verdict {
        Verdict::Attested => Vec::new(),
        Verdict::Refused(reasons) => reasons
            .iter()
            .map(|reason| ReportedRefusal {
                code: reason.code().to_owned(),
                message: reason.message(),
            })
            .collect(),
    };
    let observed_contexts = live
        .required_status_checks
        .as_ref()
        .map_or_else(Vec::new, |checks| checks.contexts.clone());

    BranchProtectionReport {
        branch: "main".to_owned(),
        expected_contexts: desired.required_contexts(),
        observed_contexts,
        attested: verdict.is_attested(),
        exit_code: verdict.exit_code(),
        refusals,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        emit_payload, resolve_contexts, verify, DesiredProtection, GhApiPayload,
        LiveProtectionState, RefusalReason, Verdict, WorkflowJob,
    };

    fn enforcer_job() -> WorkflowJob {
        WorkflowJob {
            workflow_name: "Rust CI".to_owned(),
            job_id: "rust-ci".to_owned(),
            matrix: vec![
                "ubuntu-latest".to_owned(),
                "windows-latest".to_owned(),
                "macos-latest".to_owned(),
            ],
        }
    }

    fn desired() -> DesiredProtection {
        DesiredProtection::baseline(vec![enforcer_job()])
    }

    fn pass_live() -> LiveProtectionState {
        let payload = emit_payload(&desired());
        LiveProtectionState {
            required_status_checks: payload.required_status_checks,
            enforce_admins: payload.enforce_admins,
            required_pull_request: payload.required_pull_request,
            allow_force_pushes: payload.allow_force_pushes,
            allow_deletions: payload.allow_deletions,
            required_checks_passing: Some(true),
        }
    }

    #[test]
    fn resolve_contexts_renders_one_context_per_matrix_value() {
        let contexts = resolve_contexts(&enforcer_job());
        assert_eq!(
            contexts,
            vec![
                "Rust CI / rust-ci (ubuntu-latest)".to_owned(),
                "Rust CI / rust-ci (windows-latest)".to_owned(),
                "Rust CI / rust-ci (macos-latest)".to_owned(),
            ]
        );
    }

    #[test]
    fn resolve_contexts_non_matrix_job_has_no_parenthesized_suffix() {
        let job = WorkflowJob {
            workflow_name: "Docs".to_owned(),
            job_id: "lint".to_owned(),
            matrix: vec![],
        };
        assert_eq!(resolve_contexts(&job), vec!["Docs / lint".to_owned()]);
    }

    #[test]
    fn emit_payload_never_hardcodes_a_stale_context_it_always_derives_from_the_job(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let payload = emit_payload(&desired());
        let Some(checks) = payload.required_status_checks.clone() else {
            return Err("baseline desired protection always has required jobs".into());
        };
        assert_eq!(checks.contexts, resolve_contexts(&enforcer_job()));
        assert!(checks.strict);
        assert!(payload.enforce_admins);
        assert!(!payload.allow_force_pushes);
        assert!(!payload.allow_deletions);
        assert!(payload.required_pull_request);
        Ok(())
    }

    #[test]
    fn emit_payload_with_no_required_jobs_emits_none_for_required_status_checks() {
        let empty = DesiredProtection::baseline(vec![]);
        let payload = emit_payload(&empty);
        assert_eq!(payload.required_status_checks, None);
    }

    #[test]
    fn verify_attests_the_pass_fixture_state() {
        let verdict = verify(&desired(), &pass_live());
        assert_eq!(verdict, Verdict::Attested);
        assert!(verdict.is_attested());
        assert_eq!(verdict.exit_code(), 0);
    }

    #[test]
    fn verify_refuses_when_no_required_checks_are_configured(
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Fail fixture 1: no required checks at all.
        let mut live = pass_live();
        live.required_status_checks = None;
        let verdict = verify(&desired(), &live);
        assert!(!verdict.is_attested());
        match verdict {
            Verdict::Refused(reasons) => {
                assert!(reasons.contains(&RefusalReason::NoRequiredChecks));
                Ok(())
            }
            Verdict::Attested => Err("expected Refused".into()),
        }
    }

    #[test]
    fn verify_refuses_when_admin_override_allowed_or_force_push_allowed(
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Fail fixture 2: required checks present but bypassable.
        let mut live = pass_live();
        live.enforce_admins = false;
        live.allow_force_pushes = true;
        let verdict = verify(&desired(), &live);
        match verdict {
            Verdict::Refused(reasons) => {
                assert!(reasons.contains(&RefusalReason::AdminOverrideAllowed));
                assert!(reasons.contains(&RefusalReason::ForcePushAllowed));
                assert_eq!(reasons.len(), 2);
                Ok(())
            }
            Verdict::Attested => Err("expected Refused".into()),
        }
    }

    #[test]
    fn verify_refuses_when_required_check_is_red_but_still_merge_eligible() {
        // Fail fixture 3: red/pending required check that is nonetheless
        // reported merge-eligible.
        let mut live = pass_live();
        live.required_checks_passing = Some(false);
        let verdict = verify(&desired(), &live);
        assert_eq!(
            verdict,
            Verdict::Refused(vec![RefusalReason::RedCheckMergeEligible])
        );

        // Pending (unknown) must refuse too, not just explicit red.
        let mut pending_live = pass_live();
        pending_live.required_checks_passing = None;
        let pending_verdict = verify(&desired(), &pending_live);
        assert_eq!(
            pending_verdict,
            Verdict::Refused(vec![RefusalReason::RedCheckMergeEligible])
        );
    }

    #[test]
    fn verify_refuses_when_not_required_up_to_date_before_merge(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut live = pass_live();
        if let Some(checks) = live.required_status_checks.as_mut() {
            checks.strict = false;
        }
        let verdict = verify(&desired(), &live);
        match verdict {
            Verdict::Refused(reasons) => {
                assert!(reasons.contains(&RefusalReason::NotRequiredUpToDate));
                Ok(())
            }
            Verdict::Attested => Err("expected Refused".into()),
        }
    }

    #[test]
    fn verify_refuses_when_live_contexts_are_missing_a_desired_context(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut live = pass_live();
        if let Some(checks) = live.required_status_checks.as_mut() {
            checks.contexts.retain(|c| !c.contains("macos-latest"));
        }
        let verdict = verify(&desired(), &live);
        match verdict {
            Verdict::Refused(reasons) => {
                assert!(reasons.iter().any(|r| matches!(
                    r,
                    RefusalReason::MissingRequiredContext { missing }
                        if missing.contains("macos-latest")
                )));
                Ok(())
            }
            Verdict::Attested => Err("expected Refused".into()),
        }
    }

    #[test]
    fn ghapi_payload_and_live_state_round_trip_through_json_with_identical_shape(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let payload = emit_payload(&desired());
        let json = serde_json::to_string(&payload)?;
        let round_tripped: GhApiPayload = serde_json::from_str(&json)?;
        assert_eq!(payload, round_tripped);

        // The live-state read-back shape must accept the emitted payload's
        // required_status_checks verbatim (same field names/types), which
        // is the contract that makes "emitted payload -> pass fixture"
        // meaningful rather than two independently-typed schemas that
        // happen to look similar.
        let Some(checks) = payload.required_status_checks.as_ref() else {
            return Err("baseline emits required checks".into());
        };
        assert_eq!(checks.contexts, resolve_contexts(&enforcer_job()));
        Ok(())
    }
}
