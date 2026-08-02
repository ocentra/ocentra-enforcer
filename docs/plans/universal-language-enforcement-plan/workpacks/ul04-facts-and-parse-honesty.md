# UL04 - Facts and Parse Honesty

<!-- agent-capsule -->
> Agent Capsule
> Plan: `universal-language-enforcement-plan`
> Doc: `UL04 Facts and Parse Honesty`
> Kind: architect-owned capability contract.
> Read when: UL03 behavior-preserving extraction is accepted.
> Stop rule: add one normalized capability slice, not a universal AST model.
> Proves: parser quality and named facts are distinguishable from empty/default output.
> Does not prove: completeness for all languages or cross-file resolution.
> Proof rule: every fact and parse outcome carries identity, span/provenance, and negative behavior.
<!-- /agent-capsule -->

- owns: `crates/enforcer-domain/src/syntax_types.rs`, `crates/enforcer-syntax/src/facts/**`, `crates/enforcer-syntax/tests/fact_contract.rs`, `crates/enforcer-syntax/tests/fixtures/facts/**`
- deps: `UL03`
- tier: `P0 fact contract, P1 parser honesty`

> Owner class: Sol/architect; later language fixture packets may be delegated.
> Batch limit: one fact capability slice plus parse-quality metadata.

## Where We Are

`ParsedFile` has nine useful collections but no explicit language/file identity, provider/version, spans beyond lines, parse errors, missing nodes, unsafe-input outcome, completeness, or provenance. Parser failure often collapses to an empty default value.

## Where We Want To Be

A typed analysis result distinguishes parsed-clean, parsed-with-errors, unsafe-input-refused, provider-unavailable, unsupported, and internal failure. Normalized facts state exactly what was observed and what remains unavailable.

## Owns

- lightweight domain fact and capability types with no parser dependency;
- syntax implementation for one additive fact slice;
- dedicated positive, negative, malformed, partial-parse, unsafe-input, and provider-failure fixtures;
- no validator or rule migration.

## Objective

Make syntax output usable as mechanical evidence without allowing empty vectors or missing providers to masquerade as clean code.

## Requirement Checklist

- [ ] Analysis result includes language/file, provider/version, outcome, error/missing counts, capabilities, and provenance.
- [ ] Facts have stable kinds and byte/line spans with checked conversions.
- [ ] Existing nine collections map losslessly or are versioned explicitly.
- [ ] Unsupported versus unavailable versus malformed are distinct.
- [ ] Unsafe/binary/control input fails/refuses before native parser entry.
- [ ] One selected fact slice has cross-language fixtures and `notProved` coverage rows.
- [ ] Raw Tree-sitter nodes/strings do not cross the public fact boundary.

## Acceptance And Proof

Run fact-contract tests, all syntax regression tests, property/negative tests for spans/outcomes, cargo check/clippy, and Enforcer scope gates. Include golden serialized fact/output fixtures if a wire form is exposed.

## Stop conditions

Stop if the design claims semantic meaning unavailable from syntax, adds unbounded source excerpts, exposes raw parser nodes, or requires all languages to implement a new fact in one packet.

## Parallel Ownership Notes

The domain contract has one writer. After it freezes, disjoint language fixture children may prove the same capability without touching shared types.
