# Rust/MJS Parity Retirement Agent Router

<!-- agent-capsule -->
> Plan: `rust-mjs-parity-retirement-plan`
> Read order: `README.md` -> `ARCHITECTURE.md` -> `PLAN_STATE.md` -> `WORKPACK_INDEX.md` -> one assigned workpack -> `WORKER_CHECKLIST.md` -> matching proof row.
> Current entry gate: RM00 is boss-only; Luna stays read-only until the boss accepts its authority artifact and assigns an RM01 row range.
> Authority: exact Git objects and executable behavior, never branch names, prose, or AI judgment alone.
<!-- /agent-capsule -->

## Roles

- The primary boss owns authority decisions, shared capability-matrix schema, integration, exact-SHA aggregate proof, cutover, status, and retirement.
- The visible Luna manager schedules only dependency-ready packets, checks locks/mail, independently reproduces child evidence, and recommends acceptance.
- Audit children own one bounded read-only row family and immutable evidence packet.
- Repair children exist only after RM08 assigns one exact non-singleton capability. They own one disjoint Rust implementation/test packet.
- The independent reproducer uses a separate clean worktree and never repairs the candidate it judges.

## Frozen boundaries

- Public oracle: exact commit `267af94b701bd592e01a47649e3c18c26ee04239` currently observed at `origin/safety-main`.
- Common-fork provenance only: `d7162b6173e2c664547fcb9715ba135c435d0b1e`.
- Live private overlay: `9d21780f9a4f5a498fb16a6b1ae1c05ac2d83e36`, used only to prove its two exact allowlist behaviors.
- Candidate integration: `rust-build` at the exact SHA named by each packet.

No worker modifies, rebases, merges, fast-forwards, deletes, restores, or commits either frozen MJS authority. Neither frozen branch is a merge source. The private overlay never supplies a public pass.

## Singleton surfaces

Only the named boss/integrator may edit the authority manifest, capability schema/matrix, MCP/tool registry, installer defaults, CI/workflow selection, package/workspace manifests, plan state, aggregate/closure artifacts, cutover configuration, or branch retirement state. Children submit immutable evidence or disjoint patch commits.

## Small-model limits

Luna may inventory, run a frozen-vs-native oracle against exact fixtures, normalize observed outputs under an accepted schema, implement a boss-issued non-singleton repair, and report evidence. Luna must stop for ambiguous authority, schema design, missing comparable fixtures, shared-file changes, runtime selection, any request to weaken a rule, or any cutover/merge/deletion action.

## Mail

Every packet reports `START`, `BLOCKED`, or `DONE` using exact base/head/authority SHAs, owns, commands/run IDs, artifact hashes, `proves`, and `doesNotProve`. A completed audit is evidence, not self-declared parity.
