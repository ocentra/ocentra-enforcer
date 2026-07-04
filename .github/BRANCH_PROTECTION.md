# Branch Protection — `main`

This is the settings-as-doc source of truth for this repo's OWN `main` branch
protection. `crates/enforcer-install/src/ci/branch_protection.rs`'s emitter
applies this configuration (via `gh api`) and its verifier checks the live
GitHub state against it, failing closed if protection is missing or
bypassable. Do not hand-edit GitHub's branch-protection settings without
updating this file first — the verifier treats this file's described state
as ground truth, not whatever happens to be configured in the GitHub UI.

## Why this exists

The enforcer's whole doctrine is "only green, proven trees advance" — yet
until this workpack, the trunk of the enforcer's OWN repo was unprotected: a
direct push, a force-push, an admin override, or a merge with a red/pending
required check could land on `main` with zero mechanical resistance. This
document + the `branch_protection` module close that gap on this repo's own
`main`.

## Required status checks

The required-check contexts below are declared SYMBOLICALLY in
`branch_protection.rs` (as a `WorkflowJob { workflow_name, job_id, matrix }`)
and resolved at build/verify time via `resolve_contexts()` — never
hardcoded as a literal string anywhere in this repo. They are reconciled
against each workflow's actual declared `name:` and job id, so a workflow
rename or job-id rename is a compile-time-visible drift, not a
silently-stale protection setting (this is exactly the failure mode the
legacy `docs/BRANCH_PROTECTION.md` fell into: it named
`Ocentra Enforcer / ocentra-enforcer (*)` WITH a parenthesized-matrix
suffix, a pre-rename context that was never actually applied as a setting,
had drifted from the real workflow, AND assumed a matrix that job has never
had).

Today's resolved required contexts, covering BOTH workflows that
push-trigger on `main`:

- From `.github/workflows/ci.yml` (`Rust CI`'s `rust-ci` job, matrixed over
  `ubuntu-latest` / `windows-latest` / `macos-latest` — fmt/clippy/test/
  deny/audit):
  - `Rust CI / rust-ci (ubuntu-latest)`
  - `Rust CI / rust-ci (windows-latest)`
  - `Rust CI / rust-ci (macos-latest)`
- From `.github/workflows/ocentra-enforcer.yml` (`Ocentra Enforcer`'s
  `ocentra-enforcer` job — no matrix, single `ubuntu-latest` run of the
  self-scan/dogfood gate):
  - `Ocentra Enforcer / ocentra-enforcer`

If either workflow file's name, job id, or matrix ever changes, update the
corresponding `WorkflowJob` passed to `DesiredProtection::baseline` in
`branch_protection.rs` (and its test/fixture reconciliation) in the same
change — never let this list drift from the real workflow files.

## Branch rules (non-bypassable)

- Require a pull request before any change lands on `main`. No direct
  pushes.
- Require the resolved status checks above to be green before merge.
- Require branches to be up to date with `main` before merge
  (`required_status_checks.strict = true`) — a stale branch's already-green
  check does not satisfy the gate; it must re-run against `main`'s current
  tip.
- Disallow admin override of the required checks (`enforce_admins = true`).
  There is no "administrators can bypass" allowance.
- Block force-pushes to `main` (`allow_force_pushes = false`).
- Block deletion of `main` (`allow_deletions = false`).
- A pull request whose required check(s) are red or still pending is NOT
  merge-eligible under any circumstance ("merge when red" is not a thing on
  this branch).

## Relationship to `pr_ready` / the release gate

`main` being protected under the rules above is a precondition the
`pr_ready` merge flow consults before a merge proceeds, and it is also the
precondition RELEASE_POLICY's "release-cut-from-green-`main`" gate
(`crates/enforcer-install/src/ci/release_pipeline.rs`, workpack c10)
presumes. Neither flow re-implements the branch-protection check itself —
both defer to this module's verifier as the single source of truth for
"is `main` actually guarded right now."

## The one sanctioned escape: break-glass save-state push

There is exactly ONE documented exception, and it is categorically
DIFFERENT from the merge gate above:

- An emergency `--no-verify`-class push is allowed **only** as a
  save-state / snapshot push to a **non-`main` ref** (e.g. a personal
  branch, a WIP tag, or a lane branch) when a hook or check is blocking an
  otherwise-urgent need to not lose work.
- This escape explicitly **does not merge to `main`** and **does not**
  satisfy or bypass any of the required-status-checks guarding `main`. It
  preserves work in progress on a side ref; it never substitutes for the
  merge gate.
- Nothing about this escape changes, weakens, or is checked by
  `branch_protection.rs`'s verifier — the verifier only ever looks at
  `main`'s live protection settings, which this escape never touches.

If a change created via this escape needs to reach `main`, it still goes
through a pull request and still has to pass every required check above —
there is no back door from a break-glass push directly onto the protected
trunk.
