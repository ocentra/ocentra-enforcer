# CP05 - Native Rule Packet Factory

<!-- agent-capsule -->
> Agent Capsule
> Plan: `cyberskills-parity-plan`
> Doc: `CP05 Native Rule Packet Factory`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-plan/src/lib.rs` (minimal module wiring), `crates/enforcer-plan/src/cyberskills_packet.rs`, `crates/enforcer-plan/tests/cyberskills_packet.rs`, `crates/enforcer-plan/tests/fixtures/cyberskills_packet/**`
- deps: `cp00`, `cp03`, `cp04`
- tier: `P2 T1`

> Owner class: Sol creates the factory; Luna consumes it later.
> Batch limit: one scaffold and one demonstration packet.
> Depends on: CP00, CP03, CP04.

## Where We Are

Native rule packets require repetitive registry, validator, fixture, and evidence wiring that can drift or be incompletely copied.

## Where We Want To Be

Generate only the clerical skeleton from an approved component and make every missing parity edge a mechanical failure.

## Objective

Make a native CyberSkills packet mechanically incomplete unless all source, implementation, fixture, and evidence links are present. The factory should remove clerical work without generating security meaning.

## Factory input

- approved component ID and exact source fingerprint/anchors;
- precise T1/T2 predicate and `notProved`;
- target input mechanism: typed structured parser, syntax facts, or textual matcher;
- rule ID, severity/confidence policy, and threat citations;
- assigned fail/pass/malformed/boundary fixture names.

## Factory output

- rule record skeleton;
- validator/test skeleton in the approved crate;
- fixture directories;
- component evidence skeleton;
- focused gate command manifest.

## Requirement Checklist

- [x] Factory refuses an unapproved component or missing source hash.
- [x] It never writes vendor files.
- [x] It never invents predicate, severity, citations, or expected outcomes.
- [x] The packet contract rejects missing source, protected source, duplicate fixtures, and incomplete evidence paths.
- [x] The generated skeleton is held by the exact Enforcer route/strict/source-shape/secrets gates before use.
- [x] Demonstration uses one already proved pilot, not a new behavior claim.

## Acceptance And Proof

The factory contract, generated demonstration, typed boundary, and plan structure pass at implementation commit `84745c7c8`. The exact factory suite is 4/4; package check, library tests, parity-plan tests, full enforcer-plan tests, fmt, diff, source-shape, secrets, and strict Enforcer gates pass. Dependency-inclusive Clippy remains an existing graph.rs issue outside this packet; no waiver or unrelated edit is part of CP05.

The packet proves clerical completeness and supplied-input preservation only. It does not generate security meaning, write vendor files, implement the demonstrated rule, prove live execution, or promote native/executable-proof/overall parity.

## Parallel Ownership Notes

CP05 owns only the factory and its fixtures. It does not own generated production rule files; CP09 claims those per approved packet after the factory lands.
