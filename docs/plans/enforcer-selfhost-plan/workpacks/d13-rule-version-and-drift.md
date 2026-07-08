# d13 Rule Version And Drift

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Rule Version And Drift`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-rules/src/version_drift.rs, crates/enforcer-rules/rule-version-manifest.json, crates/enforcer-rules/tests/fixtures/version_drift/**`
- deps: `d01, arc-04, arc-05`
- tier: `P1 unit`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
When the enforcer binary is installed into a target repo, the structured rule registry (`enforcer-rules` records + any committed `rules.json`/RON the registry loads) can silently drift from the upstream release. `enforcer-domain` gives every `RuleId` a branded newtype and the registry is id-locked, but there is no version + content-hash drift check across the whole rule-record set.

## Where We Want To Be
A T1 `Validator` in `enforcer-rules` (arc-04) that records a version and a `Sha256` content hash for the loaded rule-record set and fails when the deployed registry drifts from the pinned manifest without a version bump. The manifest is a versioned `serde` record (a `schema_version` + branded-newtype struct in `enforcer-domain`, reusing `Sha256`), and the hash is computed with the `enforcer-core` hash primitive — never an ad-hoc JSON blob.

## Requirement Checklist
- [ ] Compute a stable `Sha256` content hash over the loaded rule-record set (the `enforcer-rules` registry + any committed `rules.json`/RON it loads) via the `enforcer-core` hash util.
- [ ] Record version + hash in `rule-version-manifest.json` as a versioned `serde` record parsed at boundary into an `enforcer-domain` newtype struct (`TryFrom`/`deserialize_with` + a `thiserror` typed error), never a bare `String`.
- [ ] On run, the `Validator` recomputes and compares: hash mismatch without a version bump emits a `Finding` and fails closed.
- [ ] A legitimate version bump requires both a new version and a new hash together (neither alone passes).
- [ ] The drift `Finding` names which rule record / source file changed, with a terse `Fix:` hint.

## Acceptance And Proof
Tier T1, P1 unit. Prove via `cargo test -p enforcer-rules` over `crates/enforcer-rules/tests/fixtures/version_drift/**`: unchanged registry passes; content change without version bump fails; matched version+hash bump passes; version bump without content change fails. Mechanism: deterministic `Sha256`-over-registry compared to the pinned manifest record, fail-closed on unexplained drift. Record the detection-test artifact path in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Depends on d01 (registry shape / parity) and lands inside the `enforcer-rules` crate whose skeleton arc-04 owns — this pack adds only `src/version_drift.rs` + the manifest + its fixtures and must not edit the registry loader or records arc-04 owns. Deps arc-05 (Validator trait). `owns:` is disjoint from d11 (CI parity, in `enforcer-harness`) and all siblings; concurrent once arc-04 and d01 land.
