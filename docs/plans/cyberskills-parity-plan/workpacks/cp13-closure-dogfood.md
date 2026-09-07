# CP13 - Corpus Closure and Exact-SHA Dogfood

<!-- agent-capsule -->
> Agent Capsule
> Plan: `cyberskills-parity-plan`
> Doc: `CP13 Corpus Closure and Exact-SHA Dogfood`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `proof/cyberskills/cp13/**`, `proof/dogfood-manifest.json`
- deps: `cp01`, `cp02`, `cp03`, `cp04`, `cp05`, `cp06`, `cp07`, `cp08`, `cp09`, `cp10`, `cp11`, `cp12`
- tier: `P5 T1`

> Owner class: boss assembles; an independent gatekeeper verifies.
> Batch limit: terminal milestone only.
> Depends on: all component-bearing workpacks required by the final ledger.

## Where We Are

No exact integration SHA proves complete component disposition, Rust self-dogfood, CLI/MCP parity, substantive CI, and frozen-authority comparison together.

## Where We Want To Be

Assemble terminal evidence and have an independent gatekeeper reproduce it before any merge, cutover, or legacy-retirement decision.

## Objective

Prove truthful closure of the 817-skill corpus and Enforcer's self-enforcement before any merge/cutover/legacy-retirement decision.

## Requirement Checklist

### Ledger closure

- [ ] Exactly 817 canonical source identities reconcile as 816 readable plus one `sourceUnavailable` protected deletion with tracked blob `df48fa4149dd25956e730443d3582693a3f825a8`; it cannot be counted as covered, retained, decomposed, implemented, or proved until an explicit owner decision.
- [ ] Every skill has at least one reviewed component and zero unexplained source sections claimed in scope.
- [ ] Every native component has executable rule/evidence parity.
- [ ] Every external component names a real approved engine or remains honestly blocked/deferred.
- [ ] Every advisory/manual component has enforced retention and reason.
- [ ] Derived totals reconcile; no hand totals or mutually exclusive whole-skill shortcut remains.
- [ ] `notProved` is non-empty wherever coverage is narrowed.

## Runtime closure

- [ ] Rust CLI and MCP expose the same CyberSkills behavior.
- [ ] Native, external, advisory-retention, parser-error, tool-error, and skip outcomes render honestly.
- [ ] Rust scans its own changed crates with non-zero ran counts.
- [ ] Grammar/syntax ownership is singular and dependency policy is green.
- [ ] External runner policy, secrets, resource limits, and provenance are green.
- [ ] Frozen-MJS comparisons cover the behaviors still used as migration authority.

## Acceptance And Proof

### Terminal gates

- [ ] Workspace build/test, all-target clippy, fmt check, deny, audit, and relevant platform jobs.
- [ ] Enforcer mutation-risk and strict verify on the exact integration SHA.
- [ ] Detached-parent introduced-findings proof.
- [ ] MCP stdio and CLI smoke on the built artifact.
- [ ] CI jobs on the exact SHA are substantive; path filters did not skip source validation.
- [ ] Gatekeeper independently reproduces the plan proof rows and diff ownership.
- [ ] `proof/cyberskills/cp13/closure.json` binds `integrationSha`, `treeSha`, `baseSha`, clean-worktree identity, source-identity counts, unavailable-source record, policy/config digests, tool versions, commands/run IDs/exit codes, artifact hashes, built CLI/MCP artifact SHA, same-SHA substantive CI job IDs, and independent reproduction result.
- [ ] The independent gatekeeper uses a separate clean worktree and reproduces the identical integration/tree SHA; missing fields, mismatched CI SHA, docs-only/path-skipped source validation, or mismatched artifact hashes fail closure.

## Non-claims

CP13 green permits a merge/cutover decision; it does not itself merge, delete branches, remove the frozen checkout, or retire safety infrastructure. Those remain explicit owner-authorized operations after ordinary protected-branch validation.

## Parallel Ownership Notes

CP13 runs alone on the assembled integration SHA. It does not overlap implementation workers, and the independent gatekeeper uses a separate clean worktree and lane.
