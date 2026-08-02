# UL13 - Graph and Semantic Providers

<!-- agent-capsule -->
> Agent Capsule
> Plan: `universal-language-enforcement-plan`
> Doc: `UL13 Graph and Semantic Providers`
> Kind: architect-owned one-provider/predicate workpack.
> Read when: UL04-UL08 foundations are accepted.
> Stop rule: choose one graph predicate or one external semantic provider mapping, not both.
> Proves: one cross-file/semantic requirement through a typed bounded interface.
> Does not prove: graph completeness or general semantic analysis.
> Proof rule: absent, stale, incomplete, and unavailable providers cannot yield clean.
<!-- /agent-capsule -->

- owns: one typed read-only graph/provider interface slice, exactly one predicate/provider adapter, fixtures, and `proof/universal-language/ul13/**`; no memory persistence/database ownership
- deps: `UL04, UL05, UL06, UL07, UL08`
- tier: `P0 provider boundary, P1 one predicate`

> Owner class: Sol/architect; Luna may prepare isolated fixtures after interface freeze.
> Batch limit: exactly one graph predicate or one semantic tool provider.

## Where We Are

`enforcer-memory` has graph/impact/data-flow capabilities, but validators cannot consume a lightweight repository-scoped fact interface. Deep semantics may already belong to compilers/analyzers exposed through UL07.

## Where We Want To Be

Rules consume bounded, typed, read-only graph or normalized semantic results. They do not access SQLite, memory stores, raw parser nodes, mutable indexing, or arbitrary processes.

## Owns

- one provider interface slice and one proof predicate/provider mapping;
- fixtures for complete, absent, stale, partial, cyclic, and unavailable states;
- no graph database/persistence, general indexer rewrite, shared tool runner, or broad rule wave.

## Objective

Prove cross-file/semantic enforcement reuses graph/tool infrastructure without coupling rule crates to storage or process details.

## Requirement Checklist

- [ ] Reuse decision selects graph facts or mature semantic tool output.
- [ ] Provider input/output is bounded, deterministic, versioned, and provenance-bearing.
- [ ] Graph freshness/tree SHA or tool version/config digest is checked.
- [ ] Missing/stale/partial/cyclic/unavailable behavior is explicit and policy-controlled.
- [ ] One predicate has fail/pass and no-claim fixtures.
- [ ] Rule crate depends only on lightweight interfaces.
- [ ] Query/process limits prevent unbounded traversal or output.

## Acceptance And Proof

Run provider/predicate fixtures, stale/exact-SHA checks, dependency/import boundaries, graph/tool focused tests, cargo gates, and scoped Enforcer validation. Independent reproduction uses the same repository tree.

## Stop conditions

Stop if storage/database access leaks into validators, graph mutation is needed during rule evaluation, arbitrary process execution appears, or one packet expands beyond one predicate/provider.

## Parallel Ownership Notes

Provider interface and predicate are serialized. Read-only graph corpus/fixture audits may run in parallel.
