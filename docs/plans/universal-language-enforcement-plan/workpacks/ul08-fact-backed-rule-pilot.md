# UL08 - Fact-Backed Rule Pilot

<!-- agent-capsule -->
> Agent Capsule
> Plan: `universal-language-enforcement-plan`
> Doc: `UL08 Fact-Backed Rule Pilot`
> Kind: one-rule implementation workpack.
> Read when: UL05 and UL06 are accepted and the boss selects one existing noisy rule.
> Stop rule: migrate exactly one rule using already-landed facts.
> Proves: the shared analysis seam can preserve true positives and improve or equal precision.
> Does not prove: other rules or languages are fact-backed.
> Proof rule: old/new behavior is compared on the same fixtures and unavailable analysis is explicit.
<!-- /agent-capsule -->

- owns: one boss-selected existing validator file or new fact-backed module, its exact rule record, disjoint fixtures, `proof/universal-language/ul08/**`
- deps: `UL05, UL06`
- tier: `P1 T1 pilot`

> Owner class: Luna-safe implementation after boss selection; manager independently reproduces.
> Batch limit: exactly one rule and one language.

## Where We Are

All current validators consume text/path/scope. High-false-positive rules use line or marker approximations even when normalized facts could express the predicate.

## Where We Want To Be

One existing rule consumes declared facts through the new bridge, retains its registry identity, preserves true positives, demonstrates fewer false positives or equivalent behavior, and reports missing analysis honestly.

## Owns

- only the selected rule implementation/record/fixtures and immutable comparison proof;
- no fact type, parser, grammar, validator trait, scan dispatch, doctrine resolver, or shared registry.

## Objective

Prove the end-to-end architecture before language and rule waves begin.

## Requirement Checklist

- [ ] Boss records why the rule is a suitable pilot and which existing fact set is sufficient.
- [ ] Rule declares required capability IDs.
- [ ] Same fail/pass fixtures run against old and new behavior.
- [ ] Add comment/string/alias/malformed and fact-unavailable fixtures.
- [ ] Findings preserve rule ID, severity, file/span, deterministic ordering, and narrow claim.
- [ ] Old implementation is removed or retained only as a declared, separately proved fallback.
- [ ] No new parser/fact/tool/framework requirement is smuggled into the packet.

## Acceptance And Proof

Run selected language rule tests, registry/fixture parity, scan integration, old/new comparison report, unavailable-capability test, cargo check/clippy, and exact-file/crate Enforcer gates on packet HEAD and integrated SHA.

## Stop conditions

Stop if the pilot needs a new fact, graph edge, framework resolver, external tool, shared contract edit, or more than one rule/language.

## Parallel Ownership Notes

This packet is serial. Read-only fixture discovery may run in parallel, but one child owns the selected implementation.
