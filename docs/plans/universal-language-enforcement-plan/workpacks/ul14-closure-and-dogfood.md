# UL14 - Closure and Exact-SHA Dogfood

<!-- agent-capsule -->
> Agent Capsule
> Plan: `universal-language-enforcement-plan`
> Doc: `UL14 Closure and Exact-SHA Dogfood`
> Kind: terminal boss and independent-gatekeeper workpack.
> Read when: the active policy profile's required capability rows are accepted.
> Stop rule: diagnose failures; do not repair product code inside closure.
> Proves: required language/tool/rule capabilities are honest and green on one exact integration SHA.
> Does not prove: every language has every capability or all future policy.
> Proof rule: local, CI, artifact, and independent-reproduction SHAs must match.
<!-- /agent-capsule -->

- owns: `proof/universal-language/ul14/closure.json`, `proof/universal-language/ul14/gatekeeper.json`, terminal proof scripts/tests, and boss-only plan status
- deps: profile-derived required subset of `UL01-UL13`, mechanically recorded; `UL07` always required
- tier: `P0 terminal exact-SHA proof`

> Owner class: boss plus independent gatekeeper; no Luna implementation repair.
> Batch limit: one candidate integration SHA.

## Where We Are

Focused parser or rule tests can be green while routing, required tools, CI, or unavailable states remain unproved. “As applicable” could omit inconvenient rows without a mechanical selector.

## Where We Want To Be

The selected doctrine/product profile mechanically derives every required capability row. One clean candidate SHA passes local author-time/MCP/hook/CI/dogfood routes, and an independent worktree reproduces the decisive commands and artifacts.

## Owns

- machine-readable closure/gatekeeper artifacts and terminal validation only;
- no product repair, registry migration, profile weakening, or capability reclassification.

## Objective

Bind the universal-language claim to executable evidence without pretending unsupported rows are complete.

## Requirement Checklist

- [ ] Closure selection is derived from active profile and capability matrix, not hand-picked.
- [ ] Artifact records base/integration/tree SHA, clean status, policy/config digests, language/tool versions, commands, run IDs, exit codes, findings, artifact hashes, and CI job IDs/SHA.
- [ ] Required missing/version-mismatch/timeout/malformed providers fail closure.
- [ ] Every language claim names L0-L5 level and `notProved`.
- [ ] CLI/MCP/hook/CI use the same adapter/rule identities and candidate artifact.
- [ ] Independent gatekeeper uses a separate clean worktree and identical SHA.
- [ ] Docs-only/path-skipped source jobs cannot satisfy substantive gates.
- [ ] Mutation-risk, strict verify, workspace/impacted CI, security/dependency, and dogfood pass.

## Acceptance And Proof

Validate closure schemas/hashes, run all profile-derived gates through Enforcer, confirm CI commit/artifact SHA equality, and reproduce from the gatekeeper worktree. Any failure reopens the owning workpack; closure does not patch it.

## Stop conditions

Stop on dirty/inherited residue, changed candidate SHA, stale/unpinned tool evidence, missing capability, mismatched CI/artifact SHA, unavailable required provider, or a request to weaken policy during closure.

## Parallel Ownership Notes

Closure runs alone. The gatekeeper is independent and read-only except for its own immutable reproduction artifact.
