# CP12 - Cross-File Graph Rules

<!-- agent-capsule -->
> Agent Capsule
> Plan: `cyberskills-parity-plan`
> Doc: `CP12 Cross-File Graph Rules`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-memory/src/security_view/**`, `crates/enforcer-security/src/cyberskills/<approved-predicate>.rs`, `crates/enforcer-security/tests/fixtures/cyberskills_graph/<approved-predicate>/**`
- deps: `CP03`, `CP08`, `UL13`
- tier: `P3 T1`

> Owner class: Sol/architect-only.
> Batch limit: exactly one repository-scoped predicate.
> Depends on: CP03, an approved CP08 graph candidate, and the accepted Universal UL13 graph/provider contract.

## Where We Are

The Rust knowledge graph can resolve cross-file relationships, but ordinary per-file validators cannot honestly claim repository-level flow or reachability.

## Where We Want To Be

Prove exactly one bounded repository predicate through a small read-only graph interface, with explicit unavailable/stale/limited outcomes.

## Objective

Reuse the accepted Universal UL13 graph/provider contract only where a file-local syntax predicate cannot prove the security contract. Do not turn ordinary rules into graph queries by default.

## Entry criteria

- The component states a deterministic repository-level predicate.
- CP08 and a failing fixture demonstrate why file-local syntax is insufficient.
- Required graph relations already exist in UL13 or a bounded addition is approved by Universal.
- The rule declares maximum files, nodes, edges, depth, time, and ambiguity behavior.

## Requirement Checklist

- [ ] Define a small read-only graph interface for the predicate; rules do not access storage internals or Cypher text.
- [ ] Build fixtures for positive, negative, unresolved import, ambiguous symbol, cycle, generated/vendor exclusion, unsupported language, stale index, and resource limit.
- [ ] Graph unavailable/stale/limited is explicit and policy-controlled, not clean.
- [ ] Same repository snapshot produces the same result without embeddings or AI.
- [ ] Provenance traces the finding through source spans and graph edges.
- [ ] Compare against the C/C++ baseline where equivalent behavior exists.
- [ ] Keep indexing, persistence, retrieval, and UI outside the security predicate.

## Acceptance And Proof

Run syntax, memory graph, repository validator, scan integration, resource-limit, clippy/fmt, mutation-risk, and strict verification gates.

## Stop conditions

Stop if the predicate relies on probabilistic retrieval, unbounded traversal, incomplete graph state interpreted as clean, or direct database queries from a rule.

## Parallel Ownership Notes

The boss replaces `<approved-predicate>` before claim. Universal UL13 owns shared graph/provider interfaces; the rule consumes approved interfaces only.
