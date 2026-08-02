# UL00 - Capability Truth Inventory

<!-- agent-capsule -->
> Agent Capsule
> Plan: `universal-language-enforcement-plan`
> Doc: `UL00 Capability Truth Inventory`
> Kind: assigned workpack; read only when selected by the boss or WORKPACK_INDEX.
> Read when: this exact audit/schema packet is assigned.
> Stop rule: do not edit parser, language, rule, or routing implementation during the audit phase.
> Proves: live registry counts and a proposed typed capability row.
> Does not prove: any language capability, parser completeness, or rule coverage.
> Proof rule: derive every count from source and record command/run evidence.
<!-- /agent-capsule -->

- owns: `proof/universal-language/ul00/**`, then boss-approved `crates/enforcer-domain/src/language_capability_types.rs`, `crates/enforcer-rules/capabilities/language-capabilities.json`, and `crates/enforcer-rules/tests/language_capability_inventory.rs`
- deps: `none`
- tier: `P0 inventory, P1 typed contract`

> Owner class: Luna-safe read-only audit; Sol/boss approves and integrates the schema.
> Batch limit: one inventory snapshot and one schema proposal.

## Where We Are

Parser identities, literal-language rows, native scan families, route identities, tool ties, validator crates, and fact fields are independent sources. Hand-counted “language support” can drift without a failing gate.

## Where We Want To Be

One machine-readable inventory derives every row and distinguishes discovery, lexical, structural, graph, ecosystem/tool, and rule capabilities with evidence and `notProved` fields.

## Owns

- immutable audit evidence under `proof/universal-language/ul00/`;
- after boss approval only, the new capability domain types, manifest, and inventory test;
- no existing registry or implementation source.

## Objective

Produce a lossless crosswalk of all live language/tool registries and a closed capability schema. Preserve every identity and expose collisions, aliases, unreachable parsers, unwired validator crates, unsupported routes, and hand-maintained counts.

## Requirement Checklist

- [ ] Derive parser variant/dispatch, grammar binding/vendor, literal registry, native scan, detected route, validator registry, native tool, MCP/CLI enum, and fixture counts from live source.
- [ ] Record exact base/tree SHA and inherited protected residue without modifying it.
- [ ] A row carries stable identity, aliases/extensions/basenames, capability states, providers/versions, evidence, and `notProved`.
- [ ] Allowed states are closed: `proved`, `partial`, `unsupported`, `blocked`, `not-applicable`.
- [ ] `supported` without a named level is rejected.
- [ ] Duplicate identity, extension conflict, missing evidence, unreachable provider, and hand-count mismatch have negative fixtures.
- [ ] Generated totals are derived, never authoritative manifest fields.

## Acceptance And Proof

The audit first runs read-only source queries and mails its report. After schema approval, run the dedicated inventory test plus Enforcer scans over only the new manifest/types/tests. The test must fail when any source registry changes without capability reconciliation.

## Stop conditions

Stop if a count cannot be derived, if graph index and live source disagree, or if the audit would classify capability through AI judgment rather than executable evidence.

## Parallel Ownership Notes

Read-only inventory slices may run in parallel. Only the named capability integrator writes the manifest/types/test.
