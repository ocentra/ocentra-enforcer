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

- owns: `crates/enforcer-plan/src/cyberskills_packet.rs`, `crates/enforcer-plan/tests/cyberskills_packet.rs`, `crates/enforcer-plan/tests/fixtures/cyberskills_packet/**`
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

- [ ] Factory refuses an unapproved component or missing source hash.
- [ ] It never writes vendor files.
- [ ] It never invents predicate, severity, citations, or expected outcomes.
- [ ] Parity test fails when registry, validator, doc/evidence, or either fixture side is missing.
- [ ] Generated output routes through Enforcer before use.
- [ ] Demonstration uses one already proved pilot, not a new behavior claim.

## Acceptance And Proof

The factory contract, generated demonstration, rule registry, validator harness, and plan structure must all pass. Generated code is held to the same clippy/fmt and Enforcer rules as hand-written code.

## Parallel Ownership Notes

CP05 owns only the factory and its fixtures. It does not own generated production rule files; CP09 claims those per approved packet after the factory lands.
