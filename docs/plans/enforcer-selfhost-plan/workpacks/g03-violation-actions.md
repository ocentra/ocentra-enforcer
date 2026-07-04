# g03 Violation Actions

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Violation Actions`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling completion, product status, PR readiness, or broad DONE.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-ui/src/actions/`, `.enforce` waiver/override writer
- deps: `g02`, `arc-03`
- tier: `P1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
The g02 report module shows violations but is read-only. A human reviewing a violation has no in-UI way to act on it, and the enforcer's honest, gated path for a human to defer or override an individual rule hit is the declarative committed policy in `enforcer-config` (arc-03) — nothing in the UI writes it.

## Where We Want To Be
Per-violation actions on each report row: `fix` | `ignore` | `later` | `add-comment` | `write temp/override profile`, exposed as Tauri commands + served-fallback endpoints in `crates/enforcer-ui/src/actions/`. Honesty per doctrine: `ignore`, `later`, and any override MUST write an EXPLICIT, gated WAIVER through the `enforcer-config` (arc-03) declarative control-plane into `.enforce/` — a serde waiver newtype (`owner` + non-empty `reason` + `RuleId` + optional `expires`) with parse-at-boundary validation — never a silent mute and never an inline-disable (the `enforcer-security` no-bypass meta-check bans inline suppressions). `fix` and `add-comment` mutate code/annotations, not suppression. Every waiver-writing action requires owner and reason before it commits; a missing reason is refused at the boundary (typed error), not defaulted.

## Requirement Checklist
- [ ] `crates/enforcer-ui/src/actions/` exposes fix | ignore | later | add-comment | override per violation row (Tauri commands + served-fallback endpoints).
- [ ] `ignore` / `later` / `override` write a structured serde waiver through the `enforcer-config` (arc-03) control-plane into `.enforce/`, never a silent suppression or inline-disable.
- [ ] Each waiver-writing action REQUIRES `owner` + non-empty `reason` + `RuleId`; empty reason is refused at the boundary (typed `thiserror`, not a default).
- [ ] `override`/temp profile is expiry-bearing and auditable, not a bare numeric limit bump.
- [ ] Written waivers are decodable and re-read by g02 so a waived violation shows AS waived, not hidden.

## Acceptance And Proof
Tier P1. Fail-fixture: `ignore-without-reason-refused` (ignore action, empty reason) -> action rejected at the boundary, nothing written. Pass-fixture: `ignore-writes-named-waiver` -> a named `.enforce/` waiver record appears with owner+reason+`RuleId` (NOT a hidden mute). Detection test: `waiver-honesty-actions` (`cargo test -p enforcer-ui`) asserts every ignore/later/override produces a visible waiver, no code path performs silent suppression, and empty-reason writes are refused. Clean `cargo clippy` / `cargo fmt --check` (obey `[workspace.lints]`). Rows in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Owns `crates/enforcer-ui/src/actions/` and the `.enforce` waiver/override WRITER exclusively; reuses the `enforcer-config` (arc-03) waiver SCHEMA + control-plane (does not redefine it) and does not touch the committed config policy files directly (writes route through arc-03). Attaches to g02's row surface (read-only on report). Depends on g02 (rows) and arc-03 (config waiver honesty / declarative control-plane). Deps arc-24 skeleton (via g02); owns stay DISJOINT BY FILE from sibling g0x modules.
