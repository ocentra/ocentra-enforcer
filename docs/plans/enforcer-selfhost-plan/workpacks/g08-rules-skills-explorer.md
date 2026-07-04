# g08 Rules Skills Explorer

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Rules Skills Explorer`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling completion, product status, PR readiness, or broad DONE.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-ui/src/explorer/`
- deps: `g01`
- tier: `P3`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
Rules are STRUCTURED DATA per doctrine: `enforcer-rules` (arc-04) holds a typed rule record per rule carrying `RuleId <-> validator <-> {fail+pass fixtures} <-> doc-anchor <-> tier`, and skills live as prose corpora (e.g. `skills/**`, `docs/agents/**`). The AI consumes the structured rule, never the prose. But a HUMAN has no way to browse what a rule means, how it behaves, why it matters, or what passes vs fails — the `.md` human-canonical text is scattered and unreachable from the UI. The g01 serve surface exposes a view-mount registry and the arc-24 backend derives frontend types from `enforcer-domain` via `ts_rs`; nothing yet renders the rule/skill catalog for people.

## Where We Want To Be
A rules-&-skills EXPLORER module — `crates/enforcer-ui/src/explorer/` — that renders EVERY rule and skill as browsable UI mounted into the g01 view registry (Tauri command + served HTML fallback). For each rule it reads the typed `enforcer-rules` (arc-04) record and presents: meaning/behavior, why-it-matters, fail vs pass examples (from the fixtures), tier (P0-P5 proof tier and T1/T2/T3 rule tier), and framework/language mapping. For each skill it renders the prose corpus. THIS is where the human-canonical `.md` lives — humans browse it via the UI — while the AI still reads the STRUCTURED rule record, never the prose. The explorer is a read-only presentation surface: it never mutates rules, fixtures, or config. Frontend (TS under `crates/enforcer-ui/frontend/`) only presents; the browse payload is built in Rust from types derived from `enforcer-domain` / `enforcer-rules`. Strictly HUMAN-invoked; inline agent runs stay silent (`enforcer-core` run-context).

## Requirement Checklist
- [ ] `crates/enforcer-ui/src/explorer/` reads the typed `enforcer-rules` (arc-04) records + skill prose corpora; it never re-derives rules from `.md` and never mutates rules/fixtures/config.
- [ ] Every rule renders as a browsable entry exposing meaning/behavior, why-it-matters, fail vs pass examples (from the rule's fixtures), tier (proof P0-P5 + rule T1/T2/T3), and framework/language mapping.
- [ ] Every skill renders its human-canonical prose; the `.md` is presented HERE for humans while the AI consumes the structured record.
- [ ] Output mounts into g01's view registry (Tauri command + self-contained served HTML fallback, no external assets); frontend types derived from `enforcer-domain`/`enforcer-rules` via `ts_rs`.
- [ ] Every rule record surfaces a complete entry: a rule missing a doc-anchor or fail/pass fixtures is rendered as INCOMPLETE (flagged), not silently blank, so the explorer doubles as a rules-completeness view.
- [ ] Honors silent mode (`enforcer-core` run-context): no explorer render, no UI, during inline agent runs.

## Acceptance And Proof
Tier P3. Fail-fixture: `explorer-incomplete-rule-flagged` (a rule record missing its doc-anchor/fixtures) -> the entry renders as INCOMPLETE (flagged), not silently blank. Pass-fixture: `explorer-catalog-render` (fixture rule set + one skill) -> every rule entry exposes meaning + why + fail/pass examples + tier + mapping, and the skill prose renders. Detection test: `explorer-view-contract` (`cargo test -p enforcer-ui`) asserts each rendered rule entry is complete or explicitly flagged, the payload is built from the typed `enforcer-rules` record (not re-parsed from prose), the view mounts into g01's registry, silent-mode suppression holds, and no external asset is fetched. Clean `cargo clippy` / `cargo fmt --check` (obey `[workspace.lints]`; no `pub use` barrels). Rows in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Owns `crates/enforcer-ui/src/explorer/` exclusively. Mounts into g01's view registry (read-only on serve); consumes the `enforcer-rules` (arc-04) records + skill corpora read-only and never mutates them. Depends on g01 for the surface (and, via g01, the arc-24 crate skeleton); reads arc-04 rule records. Disjoint from g02/g03/g04/g05/g06/g07 — owns stay DISJOINT BY FILE from sibling g0x modules.
