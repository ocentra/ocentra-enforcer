# CP03 - Syntax Facts and Parse Honesty Contract

<!-- agent-capsule -->
> Agent Capsule
> Plan: `cyberskills-parity-plan`
> Doc: `CP03 Syntax Facts and Parse Honesty Contract`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `proof/cyberskills/cp03/**` and approved CyberSkills fact-demand/consumer tests only
- deps: `CP02`, `UL04`
- tier: `P2 T1`

> Owner class: Sol/architect-only.
> Batch limit: one approved consumer requirement slice.
> Depends on: CP02 and accepted Universal UL04 fact/parse-honesty contract.

## Owns

CyberSkills fact-demand records, consumer tests, and minimal approved consumer changes. Universal UL04 owns the `enforcer-syntax` interface, fact modules, fixtures, and parse-honesty contract.

## Where We Are

CyberSkills needs accepted facts and honest outcomes, but it does not own their schema or grammar adapter.

## Where We Want To Be

Declare the smallest security-consumer requirement against UL04 and prove its use without extending the shared fact contract locally.

## Objective

Consume the UL04 language-neutral interface without leaking raw grammar node vocabulary into CyberSkills.

## Initial contract

- The requirement names the accepted `enforcer_syntax::facts::function_facts::analyze` seam and its typed `SyntaxAnalysisResult` capability required by one approved consumer.
- Missing assignments, literals, annotations, control-flow, or bounded data-flow return to UL04; CP03 cannot add them.

## Requirement Checklist

- [x] Attach UL04 capability/parse-honesty evidence before consumer implementation.
- [x] Unsupported, malformed, unavailable, and unsafe outcomes are visible and never clean; oversized/resource-limited outcomes remain explicitly routed to UL04 because the accepted interface does not expose them.
- [x] Security callers never import Tree-sitter or grammar crates.
- [x] Consumer tests assert typed behavior, checked spans, and provider outcomes rather than internal parse-tree shape.

## Acceptance And Proof

The accepted UL04 inventory/provenance plus the CP03 consumer regression proves this bounded slice: clean function facts, checked spans, and explicit malformed/unsupported/unavailable/unsafe outcomes. CP03 records oversized/resource-limited support as an UL04 follow-up and never claims language-neutrality, native rule implementation, executable proof, live execution, or overall parity.

## Stop conditions

Stop if UL04 has not accepted the capability, the consumer needs a raw AST facade, or invalid syntax would become an empty successful fact set.

## Parallel Ownership Notes

UL04 serializes syntax interface files. CyberSkills workers consume the landed contract read-only and never edit Universal syntax paths.
