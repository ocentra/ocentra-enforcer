# CP00 - Truth Ledger and Retention Gate

<!-- agent-capsule -->
> Agent Capsule
> Plan: `cyberskills-parity-plan`
> Doc: `CP00 Truth Ledger and Retention Gate`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: immutable CP00 audit/proposal packet and CP00-only fixtures; `cyberskills-ledger-integrator` owns the ledger, loader, and tests
- deps: `none`
- tier: `P1 T1`

> Owner class: Luna performs the read-only audit; Sol/boss approves the schema and implementation.
> Batch limit: one schema migration and its tests.
> Depends on: none.

## Where We Are

The current ledger retains all 817 catalog rows but forces each skill into one whole-skill bucket and formally links only six rows to native evidence.

## Where We Want To Be

Use a typed per-skill component ledger with derived totals, exact source identity, narrowed claims, and hard failures for malformed or contradictory coverage. The 817 tracked identities reconcile as 816 readable sources plus one `sourceUnavailable` source identity.

## Owns

- `crates/enforcer-rules/dispositions/cyberskills-disposition.json`
- `crates/enforcer-rules/src/cyberskills_disposition.rs` if a typed loader is required
- `crates/enforcer-rules/tests/cyberskills_disposition.rs`
- new CP00-only fixtures under `crates/enforcer-rules/tests/fixtures/cyberskills_disposition/**`
- the CP00 proof row; no vendor file

## Objective

Replace the mutually exclusive, hand-totaled disposition with a typed per-skill component ledger while retaining all 817 source identities. The migration must preserve the six already proved mappings and mark everything else no stronger than current evidence permits.

## Required schema behavior

- Stable `catalogId`, canonical `sourcePath`, lowercase SHA-256, attribution, and source anchors.
- `sourceAvailability` is exactly `available` or `sourceUnavailable`. The unavailable identity is `detecting-fileless-malware-techniques`, with tracked blob `df48fa4149dd25956e730443d3582693a3f825a8`, observed absence, and owner-decision reference; it has no reviewed-source components and never contributes to covered/proved/retained totals.
- Non-empty `components[]` with stable component IDs.
- Component kind is exactly `native-predicate`, `external-engine`, `advisory`, or `manual`.
- Mechanical components name T1/T2, predicate, implementation status/reference, evidence, and `notProved`.
- Advisory/manual components name retained source sections and a mechanization reason.
- Status vocabulary is closed and mechanically validated.
- Optional fields, including conversion difficulty, use a closed enum; arbitrary predicate text in an enum field fails.
- Totals and progress are derived at test time and are not authoritative JSON fields.
- Duplicate skill, source path, component ID, rule target, or contradictory component fails.

## Requirement Checklist

- [x] Read-only report compares the current 817 rows, six evidence files, 41 native records, and adapter registry.
- [x] Boss approves a before/after schema example before edits.
- [x] Migration is deterministic and reviewable; no AI-generated classification is silently accepted.
- [x] All 817 identities survive exactly once: 816 readable and one source-unavailable identity.
- [x] Existing six fingerprints and evidence paths still validate.
- [x] Negative fixtures cover missing source, duplicate ID, empty components, invalid kind/status/tier, malformed hash, missing `notProved`, hand-total drift, and treating `sourceUnavailable` as covered.
- [x] Derived summary distinguishes source-retained, decomposed, implemented, proved, advisory-retained, manual-retained, and unexplained.

## Acceptance And Proof

Run after every schema/test edit:

```text
cargo test -p enforcer-rules --test cyberskills_disposition
```

Then run the CP00 row in `TEST_PROOF_EXPECTATIONS.md` and the worker checklist. Do not classify new skills in this pack.

## Stop conditions

Stop if migration would strengthen an unproved row, mutate vendor content, or make the protected deletion part of a generated diff.

## Parallel Ownership Notes

`cyberskills-ledger-integrator` is the sole writer for the ledger schema and retention test. CP00 audit is read-only until the boss approves an immutable proposal packet; no vendor file is owned.
