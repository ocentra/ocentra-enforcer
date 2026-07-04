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

- owns: `src/ui/actions/*`, `.enforce` waiver/override writer
- deps: `g02`, `a08`
- tier: `P1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
The g02 report shows violations but is read-only. A human reviewing a violation has no in-UI way to act on it, and the enforcer has no honest, gated path for a human to defer or override an individual rule hit.

## Where We Want To Be
Per-violation actions on each report row: `fix` | `ignore` | `later` | `add-comment` | `write temp/override profile`. Honesty per doctrine: `ignore`, `later`, and any override MUST write an EXPLICIT, gated WAIVER into `.enforce/` — reusing a08's waiver shape (`owner` + `reason` non-empty + `ruleId` + optional `expires`) — never a silent mute. `fix` and `add-comment` mutate code/annotations, not suppression. Every waiver-writing action requires owner and reason before it commits; a missing reason is refused, not defaulted.

## Requirement Checklist
- [ ] `src/ui/actions/*` exposes fix | ignore | later | add-comment | override per violation row.
- [ ] `ignore` / `later` / `override` write a structured waiver to `.enforce/` (a08 shape), never a silent suppression.
- [ ] Each waiver-writing action REQUIRES `owner` + non-empty `reason` + `ruleId`; empty reason is refused.
- [ ] `override`/temp profile is expiry-bearing and auditable, not a bare numeric limit bump.
- [ ] Written waivers are decodable and re-read by g02 so a waived violation shows AS waived, not hidden.

## Acceptance And Proof
Tier P1. Fail-fixture: `ignore-without-reason-refused` (ignore action, empty reason) -> action rejected, nothing written. Pass-fixture: `ignore-writes-named-waiver` -> a named `.enforce/` waiver row appears with owner+reason+ruleId (NOT a hidden mute). Detection test: `waiver-honesty-actions` asserts every ignore/later/override produces a visible waiver, no code path performs silent suppression, and empty-reason writes are refused. Rows in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Owns `src/ui/actions/*` and the `.enforce` waiver/override WRITER exclusively; reuses a08's waiver SCHEMA (does not redefine it) and does not touch `ocentra-enforcer.config.json`. Attaches to g02's row surface (read-only on report). Depends on g02 (rows) and a08 (waiver honesty).
