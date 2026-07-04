# e01 Literal-Scan Universal T2 Layer

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Literal-Scan Universal T2 Layer`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling completion, product status, PR readiness, or broad DONE.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `src/validators/literal-scan-bridge.ts, src/validators/literal-scan-bridge.*, tests/fixtures/literal-scan/**, tests/literal-scan-bridge.test.mjs`
- deps: `d01`
- tier: `P1 / T2`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [ADBP_GAPS](../ADBP_GAPS.md).

## Where We Are
`Tools/ocentra-literal-scan` (the Rust scored literal-risk scanner covering ~65 languages, emitting `score`+`confidence`) exists but is not wired into the enforcer engine as a standing layer. Language-specific rule families (Python/Rust/TS, and the new Dart/CFML/frontend packs) leave every other language with **zero mechanical coverage**. ADBP_GAPS treats the Rust literal-scan model as the reference for the T2 rung (scored, non-blocking). The scanner's language registry currently **lacks Dart and CFML** entries.

## Where We Want To Be
`Tools/ocentra-literal-scan` is wired into the engine as the **always-on universal T2 advisory layer**: it runs on every scan target regardless of language, emits per-finding `score`+`confidence`, and is **non-blocking** (advisory — it never fails a gate on its own; it feeds the report and the scored proof). This is what gives every one of the ~65 languages a baseline mechanical floor even where no bespoke rule family exists. The bridge **graceful-skips** when `cargo` is absent (emits a clearly-labeled "literal-scan skipped: cargo not found" advisory rather than erroring), so the enforcer stays green on toolchain-less hosts. This pack also **adds Dart and CFML** to the literal-scan language registry so the two new-language packs share the universal floor.

## Requirement Checklist
- [ ] Bridge invokes `Tools/ocentra-literal-scan` and parses its `score`+`confidence` output into engine findings tagged T2/advisory (non-blocking).
- [ ] Layer is always-on: it runs for every scan target, independent of which bespoke rule family (if any) matched the file's language.
- [ ] Non-blocking: a literal-scan finding raises the report score but never sets a fatal/exit-nonzero gate on its own.
- [ ] Graceful-skip: when `cargo` (or the built scanner binary) is absent, the bridge emits a labeled skip advisory and the run still succeeds; it does not throw.
- [ ] Dart and CFML are added to the literal-scan language registry (`.dart`, `.cfc`, `.cfm` recognized and scored).
- [ ] Scaffolded via d01 so the bridge rule id carries doc + fixtures + detection test in 5-way parity.

## Acceptance And Proof
Tier T2 (scored/advisory) per doctrine — fixtures test the **score threshold**, not a hard block (the Rust literal-scan model). Fail-fixture: a high-literal-risk source file (e.g. a Dart or CFML file dense with hardcoded literals/secrets-shaped strings) whose literal-scan `score` must **cross** the configured threshold and be reported. Pass-fixture: a clean equivalent whose `score` must **stay under** threshold. Detection test `tests/literal-scan-bridge.test.mjs` asserts: (1) fail fixture crosses threshold, (2) pass fixture stays under, (3) a run with `cargo` stubbed-absent skips gracefully and still exits 0 with the skip advisory present, (4) `.dart`/`.cfc`/`.cfm` targets are recognized by the registry. Named proof rows in TEST_PROOF_EXPECTATIONS.md: `literal-scan-universal-threshold` and `literal-scan-graceful-skip`.

## Parallel Ownership Notes
`owns:` is the new bridge validator + its fixtures/tests only — disjoint from all sibling packs. It does NOT own `Tools/ocentra-literal-scan` internals except the additive language-registry rows for Dart/CFML (coordinate the registry addition so e-pack-dart / e-pack-cfml can rely on the universal floor). z01 consumes this layer during the terminal dogfood run; e-pack-dart and e-pack-cfml assume this floor exists but do not depend on this file's completion to author their bespoke rules.
