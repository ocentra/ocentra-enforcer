# z01 Dogfood Proof Gate

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Dogfood Proof Gate`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling completion, product status, PR readiness, or broad DONE.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `xtask/src/dogfood_gate.rs, crates/enforcer-cli/tests/dogfood_gate.rs, proof/dogfood-manifest.json`
- deps: `ALL tracks (A, B, C, D, E, F, G, H) — this is the LAST gate`
- tier: `P4`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md), [ADBP_GAPS](../ADBP_GAPS.md).

## Where We Are
The enforcer's central doctrine is "did we follow our own advice?" — but there is no terminal gate that actually **runs the finished `enforcer` binary against its own Rust, now-multi-language self** and refuses plan-DONE on any self-violation. The self-validation *code* is authored elsewhere: a10 stands up the native dogfood loop (`xtask dogfood` = the `enforcer` binary + `cargo clippy`/`fmt`/`deny`/`audit` on `crates/**`) and its `crates/enforcer-cli/tests/self_enforce.rs`; the source-policy/coverage self-checks are the Rust rules from a09 + the `enforcer-rules`/`enforcer-lang-rust` validator crates; e01 is the universal literal-scan floor in `enforcer-literal-scan`; b02 is the `enforcer-plan` PLAN-* structure self-validation. Nothing composes these into ONE terminal run+prove gate that emits a durable manifest and blocks plan-DONE.

## Where We Want To Be
A single terminal gate that, **after everything else is written and validated**, RUNS the built `enforcer` on its own repository (all shipped `crates/**` source, `enforcer-config`, the `enforcer-rules` structured rule set, and the plan surfaces) and produces a durable proof artifact. It COMPOSES the existing entrypoints (it does not reimplement them): a10's `xtask dogfood` native loop, the a09/`enforcer-rules`/`enforcer-lang-*` Rust validators, e01's `enforcer-literal-scan` floor, and b02's `enforcer-plan` structure check. The gate lives as `xtask/src/dogfood_gate.rs` (the terminal composing gate, distinct from a10's `xtask/src/dogfood.rs` native-loop command) and is asserted by `crates/enforcer-cli/tests/dogfood_gate.rs`. Plan-DONE is gated on **zero self-violations**: any violation the enforcer would flag in someone else's project must also be zero in ours (or below its committed T2 ceiling), or the gate fails and DONE cannot move. It emits `proof/dogfood-manifest.json` and, per the OcentraParent proof borrow, records a tamper-evident entry via the `enforcer-proof` hash-chained NDJSON journal.

## Requirement Checklist
- [ ] Runs the built `enforcer` binary end-to-end against its own repo (`enforcer scan crates/` — source policy + the `enforcer-lang-*` rule families + the `enforcer-literal-scan` floor + the `enforcer-plan` PLAN-* check), not a mocked subset.
- [ ] Composes the self-validation entrypoints from a10 (`xtask dogfood` native loop: `cargo clippy`/`fmt`/`deny`/`audit` + the `enforcer` binary), a09 + the `enforcer-rules`/`enforcer-lang-rust` validators, e01 (`enforcer-literal-scan`), and b02 (`enforcer-plan` structure) rather than reimplementing them.
- [ ] Emits a proof artifact `proof/dogfood-manifest.json`: timestamp, ruleset fingerprint (from `enforcer-rules`), per-family finding counts, and the terminal PASS/FAIL verdict; append a tamper-evident record to the `enforcer-proof` hash-chained journal (verify-on-open).
- [ ] Gate is fail-closed: any self-violation (or any advisory above its committed T2 ceiling) blocks plan-DONE; honors a08 declarative waivers as the only sanctioned exceptions; with a09 honest coverage a hollow (zero-ran) self-scan hard-fails rather than passing.
- [ ] Runs LAST: executes only after all writing/validating packs are complete; it is a run+prove gate, not an authoring pack. Obey `[workspace.lints]` (no `unwrap`/`expect`/`panic`/`print_*` outside the sanctioned sink); no `pub use` barrels.

## Acceptance And Proof
Tier T1 terminal gate (blocking on plan-DONE). 5-way parity is Rust-native: `cargo test -p enforcer-cli` runs `crates/enforcer-cli/tests/dogfood_gate.rs`, which invokes `xtask dogfood_gate` against the live workspace and asserts the run completes with a nonzero ran-count (a09 coverage) and the manifest records a zero-self-violation PASS verdict.
- **Fail-fixture** (proving the gate bites): a deliberately-planted self-violation — a fixture workspace state seeded with a known T1 breach (a `clippy` deny-wall hit such as an `unwrap()`, a banned literal, or a PLAN-* structure break) — makes the gate exit non-zero and refuse the DONE verdict.
- **Pass-fixture**: the clean workspace produces a PASS `proof/dogfood-manifest.json` with a valid ruleset fingerprint and per-family counts, and a verified hash-chain journal entry.
The gate itself IS the proof artifact for plan-DONE; TEST_PROOF_EXPECTATIONS.md row `dogfood-self-zero-violations` is the terminal green that authorizes moving product status.

## Parallel Ownership Notes
`owns:` is the terminal composing gate `xtask/src/dogfood_gate.rs` + its CLI integration test `crates/enforcer-cli/tests/dogfood_gate.rs` + the manifest artifact — disjoint from every authoring pack AND from a10, which owns `xtask/src/dogfood.rs` (the native-loop command this gate CALLS) and `crates/enforcer-cli/tests/self_enforce.rs` (a distinct test file). z01 does NOT own `xtask/src/dogfood.rs`; it composes a10's command — this keeps owns DISJOINT BY FILE (see Issues: the group mapping literally lists `xtask/src/dogfood.rs` for z01, which collides with a10's owns; resolved by giving z01 the distinct `dogfood_gate.rs` and depending on a10). `deps` is intentionally the whole plan (Tracks A, B, C, D, E, F, G, H): this pack must not start its RUN until siblings are DONE, because it validates their output. It does not edit sibling source; it only reads their shipped crate artifacts and composes the self-validation entrypoints they expose (a10 `xtask dogfood`, a09/`enforcer-rules`/`enforcer-lang-*`, e01 `enforcer-literal-scan`, b02 `enforcer-plan`). owns disjoint? = Y.
