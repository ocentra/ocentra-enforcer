# CP07 - One External Engine Pilot

<!-- agent-capsule -->
> Agent Capsule
> Plan: `cyberskills-parity-plan`
> Doc: `CP07 One External Engine Pilot`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-harness/src/adapters/cyberskills/<approved-engine>/**`, `crates/enforcer-harness/tests/fixtures/cyberskills_adapters/<approved-engine>/**`, `proof/cyberskills/cp07/**`
- deps: `CP06`
- tier: `P4 T2`

> Owner class: Luna may implement fixtures/mapping under boss review; Sol approves engine policy.
> Batch limit: exactly one engine and one output protocol.
> Depends on: CP06.

## Where We Are

The shared engine module is not proved against a real third-party executable or stable machine-readable output protocol.

## Where We Want To Be

Prove exactly one reproducible, low-risk engine through both recorded and optional live adapters before mapping multiple skills.

## Objective

Prove the external-engine module against one useful, reproducible, low-risk engine before mapping a family of skills. Selection favors an engine installable in CI with machine-readable output and no credentials or privileged network access.

## Selection gate

The boss records candidate comparison: corpus demand, license, install reproducibility, pinned version, platform support, output stability, runtime, network/credential need, fixture availability, and overlap with native predicates. No engine is selected by popularity alone.

## Requirement Checklist

- [ ] Engine ID, pinned version/range, executable discovery, output schema, and license are recorded.
- [ ] Target and arguments are typed and allowlisted.
- [ ] A recorded adapter test runs without the engine.
- [ ] An optional live test runs only when the exact engine is available and reports honest skip otherwise.
- [ ] Recorded and live normalization agree on the same captured artifact.
- [ ] Unavailable, non-zero exit, timeout, malformed output, unknown severity, and findings-above/below-threshold paths are proved.
- [ ] Provenance distinguishes recorded from live evidence.
- [ ] Only narrowed engine-output gating is claimed; the engine's entire methodology is not reimplemented or certified.

## Acceptance And Proof

Run focused adapter tests, full `enforcer-harness` tests, clippy/fmt, Enforcer checks, and exact-SHA optional CI showing whether the live engine job ran or skipped.

## Parallel Ownership Notes

The boss replaces `<approved-engine>` with one exact engine path. CP07 consumes the shared runner read-only and cannot edit another engine or the generic seam.
