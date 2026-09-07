# CP02 - Extract the Shared Syntax Module

<!-- agent-capsule -->
> Agent Capsule
> Plan: `cyberskills-parity-plan`
> Doc: `CP02 Extract the Shared Syntax Module`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `proof/cyberskills/cp02/**` and approved CyberSkills consumer-adoption tests only
- deps: `UL02`, `UL03`
- tier: `P3 T1`

> Owner class: Sol/architect-only.
> Batch limit: one consumer-adoption contract.
> Depends on: accepted Universal UL02 grammar ownership and UL03 shared syntax extraction; integrate before CP03.

## Owns

- CyberSkills demand/adoption evidence and approved consumer tests
- minimal security-consumer wiring after UL03 is accepted

No parser, grammar, syntax-core, memory, workspace-manifest, or CyberSkills rule behavior change occurs in CP02.

## Where We Are

Universal UL02/UL03 own the grammar registry, Tree-sitter bindings, language quirks, parsers, and shared syntax extraction.

## Where We Want To Be

Adopt the accepted Universal shared syntax interface from CyberSkills without creating a second parser owner or grammar dependency.

## Objective

Record the CyberSkills consumer requirements and prove security callers can consume the UL03 interface without linking to memory persistence, raw Tree-sitter nodes, or a duplicate grammar runtime.

## Adoption manifest

Before editing, attach accepted UL02/UL03 evidence and enumerate each CyberSkills consumer requirement. Classify each as:

- accepted shared syntax capability: consume;
- missing capability: return to Universal UL04;
- graph/index concern: return to Universal UL13 provider work;
- unrelated memory concern: stay untouched.

## Requirement Checklist

- [x] Attach accepted UL02 ownership and UL03 extraction proof to the packet.
- [x] CyberSkills consumes the shared interface directly with no grammar/runtime duplication.
- [x] Security consumers have no raw Tree-sitter, parser, persistence, retrieval, or UI dependency.
- [x] Missing facts are recorded as requirements for UL04, never implemented locally.
- [x] `cargo tree` proves the consumer did not introduce a second grammar/runtime owner.

## Acceptance And Proof

Run consumer adoption tests and affected security-crate checks. Final proof includes UL02/UL03 dependency evidence, dependency-policy, clippy/fmt where Rust changed, and a diff scan. The accepted packet is recorded in `proof/cyberskills/cp02/shared-syntax-adoption.json` and exercises `crates/enforcer-scan/tests/cyberskills_syntax_adoption.rs`; it proves consumer adoption only and does not promote native CyberSkills implementation, executable proof, live execution, or overall parity.

## Stop conditions

Stop if UL02/UL03 are not accepted, the consumer needs a missing syntax fact, a raw grammar dependency, or a duplicate owner. Route missing facts to UL04; CP02 does not add facts.

## Parallel Ownership Notes

This is an architect-only consumer contract. CP02 never claims grammar, parser, memory, or workspace dependency paths; Universal retains those singleton surfaces.
