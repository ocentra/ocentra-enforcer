# CP10 - External Engine Mapping Waves

<!-- agent-capsule -->
> Agent Capsule
> Plan: `cyberskills-parity-plan`
> Doc: `CP10 External Engine Mapping Waves`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-harness/src/adapters/cyberskills/<approved-engine>/**`, `crates/enforcer-harness/tests/fixtures/cyberskills_adapters/<approved-engine>/**`, `proof/cyberskills/cp10/<batch-id>/**`
- deps: `CP07`, `CP08`
- tier: `P3 T2`

> Owner class: Luna-safe after the engine exists.
> Batch limit: one engine and no more than 10 skills.
> Depends on: CP07 and approved CP08 components.

## Where We Are

Engine-deferred skills are not yet mapped to real typed engine capabilities, and the old 398/399 adapter count risks producing shallow wrappers.

## Where We Want To Be

Map up to 10 narrowed components to one existing engine/output protocol with recorded evidence and no skill-specific process execution.

## Objective

Map several vendor components to one existing engine adapter and gate without building skill-specific wrapper scripts.

## Requirement Checklist

- [ ] Every component genuinely requires the named engine capability.
- [ ] The engine output field/taxonomy supporting the gate is recorded.
- [ ] Skill-specific interpretation is expressed as Rust policy over normalized output when necessary, not as arbitrary wrapper code.
- [ ] Recorded fixtures cover findings present/absent, severity boundary, malformed output, unavailable engine, and tool error.
- [ ] Mapping records engine/version/output constraints, source anchors, coverage, and `notProved`.
- [ ] Fetch-only SDK components send fetched JSON to a native predicate; they do not duplicate policy in the adapter.
- [ ] A single engine run may satisfy multiple mapped components, with deduplicated findings and retained provenance.

## Acceptance And Proof

Run the engine adapter/gate suite, mapping/disposition gate, harness crate tests, clippy/fmt, Enforcer checks, and optional live proof where available.

## Stop conditions

Stop if a skill needs different executable arguments outside the approved typed target, a second output protocol, credentials/network not approved by policy, or a native predicate instead.

## Parallel Ownership Notes

The boss replaces engine and batch placeholders. Different engine paths may be disjoint, but `tool-adapter-integrator` owns shared adapter registry changes and `cyberskills-ledger-integrator` serializes disposition updates.
