# UL11 - Language Capability Waves

<!-- agent-capsule -->
> Agent Capsule
> Plan: `universal-language-enforcement-plan`
> Doc: `UL11 Language Capability Waves`
> Kind: repeated parallel-safe language packet.
> Read when: UL06/UL08 are accepted and the selected language's required facts/tools already exist.
> Stop rule: one language per child and no shared-file edit.
> Proves: named capability levels for selected language rows.
> Does not prove: all languages, rules, or semantic correctness.
> Proof rule: four fixture classes and immutable evidence accompany every claimed level.
<!-- /agent-capsule -->

- owns: one language adapter directory, its disjoint fixtures, and `proof/universal-language/ul11/<language-id>/**`; canonical registry/matrix are integrator-only
- deps: `UL06, UL08`; selected `UL07/UL09` capabilities as required
- tier: `P1 capability waves`

> Owner class: visible Luna manager with at most three disjoint Luna children.
> Batch limit: one language per child, at most three languages per wave.

## Where We Are

Many languages structurally parse, but capability completeness, parse quality, tool availability, and applicable rules are not uniformly proved. Bulk “support” claims would hide uneven facts.

## Where We Want To Be

Every wave advances only evidenced capability levels. Shared fact/tool/rule contracts are reused; language-specific work maps syntax or ecosystem output into them.

## Owns

- child: `crates/enforcer-syntax/src/languages/<language-id>/**` only when already transferred/assigned, disjoint fixtures, immutable evidence/proposed capability row;
- integrator: canonical matrix/registry after child acceptance;
- no Cargo/workspace manifest, domain/validator/scan/MCP/CI/plan state, or shared registry by a child.

## Objective

Scale the language substrate without copying scanners, inventing framework doctrine, or accumulating a terminal enforcement debt.

## Requirement Checklist

- [ ] Child starts from one accepted integration SHA and one capability slice.
- [ ] Positive, negative, malformed/partial, and unavailable/unsupported fixtures exist.
- [ ] Existing grammar/provider is reused; new dependency stops the packet.
- [ ] Required external tool uses an already-accepted UL07 adapter.
- [ ] Evidence proposes exact L0-L5 states, providers, versions, fixtures, rules, and `notProved`.
- [ ] Inner Enforcer gate runs after every cohesive edit.
- [ ] Manager independently reproduces the decisive gate before integrator application.

## Acceptance And Proof

Run selected language fact/tool fixtures, capability packet validator, unchanged parser regression suite for that language, cargo check/clippy, exact file/crate Enforcer scan, and manager reproduction. Integrator reruns registry drift and impacted gates.

## Stop conditions

Stop at the first shared file, missing fact/tool contract, new dependency, architecture decision, false semantic claim, or second language in one child.

## Parallel Ownership Notes

Exactly one active UL11 wave may have up to three disjoint children. The capability matrix integrator works only after all children finish and never concurrently with their shared proposal application.
