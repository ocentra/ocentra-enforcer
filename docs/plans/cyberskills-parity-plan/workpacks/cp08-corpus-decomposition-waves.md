# CP08 - Corpus Decomposition Waves

<!-- agent-capsule -->
> Agent Capsule
> Plan: `cyberskills-parity-plan`
> Doc: `CP08 Corpus Decomposition Waves`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `proof/cyberskills/cp08/<batch-id>/**` and immutable decomposition proposals; `cyberskills-ledger-integrator` owns ledger application
- deps: `cp00`, `cp01`
- tier: `P1 T1`

> Owner class: Luna-safe with boss approval between audit and write.
> Batch limit: exactly 10 skills, except the final remainder.
> Depends on: CP00 and one accepted CP01 batch.

## Where We Are

The 817-row triage was generated from broad corpus signals and a small deep-read sample, so whole-skill T1/T2/adapter/prose labels are not accepted design truth.

## Where We Want To Be

Deep-read coherent 10-skill batches and write only boss-approved component decompositions into the mechanically validated ledger.

## Objective

Deep-read each skill and propose its actual components. This is AI analysis constrained by a mechanical schema; it is not mechanical conversion and it does not itself prove enforcement.

## Batch manifest

Before work, commit or mail an ordered list of exactly 10 catalog IDs selected by one coherent capability/input family. Do not mix unrelated skills merely to fill the batch.

## Requirement Checklist

- [ ] Verify canonical path, source hash, license/attribution, frontmatter, references, and scripts.
- [ ] Identify each distinct claimed outcome or procedure.
- [ ] Classify each component using the architecture decision matrix.
- [ ] For native candidates, state input kind, exact deterministic predicate, likely typed/syntax/text mechanism, positive/negative boundary, and limitations.
- [ ] For external candidates, name the actual engine capability/output needed; do not assume one adapter per skill.
- [ ] For advisory/manual components, retain exact anchors and explain why no mechanical verdict is honest.
- [ ] Identify third-party dependencies used by the vendor scripts and whether they are engine, fetch-only, format conversion, or incidental glue.
- [ ] Mark uncertainty `proposed`/`blocked`; never guess.
- [ ] Do not change implementation status without executable evidence.

## Acceptance And Proof

Luna first mails the proposed 10-skill table. The boss may approve, revise, or reject individual components. Only `cyberskills-ledger-integrator` writes approved decomposition to the ledger. The disposition gate must show exactly the expected increase in decomposed components and zero source loss.

## Stop conditions

Stop on ambiguous source, licensing uncertainty, a component needing a new architecture category, or pressure to classify by filename/script imports alone.

## Parallel Ownership Notes

Multiple workers may audit disjoint batch manifests read-only, but `cyberskills-ledger-integrator` serializes every approved ledger application and gate.
