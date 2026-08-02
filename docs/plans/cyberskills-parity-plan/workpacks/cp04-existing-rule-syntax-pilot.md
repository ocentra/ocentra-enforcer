# CP04 - Existing Rule Syntax Pilot

<!-- agent-capsule -->
> Agent Capsule
> Plan: `cyberskills-parity-plan`
> Doc: `CP04 Existing Rule Syntax Pilot`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-lang-security/src/rules/cyberskills/<approved-pilot>.rs`, `crates/enforcer-lang-security/tests/**/<approved-pilot>/**`, `proof/cyberskills/cp04/**`
- deps: `CP01`, `CP03`, `UL04`
- tier: `P2 T1`

> Owner class: Luna implementation with boss review.
> Batch limit: exactly one existing rule.
> Depends on: CP01, CP03, and accepted Universal UL04 capability evidence.

## Where We Are

Several source-pattern rules use text heuristics even though the shared parser port can distinguish calls, arguments, imports, comments, and literals.

## Where We Want To Be

Use exactly one existing rule to prove the normalized syntax interface improves behavior before any family migration.

## Objective

Prove that shared syntax facts improve a real CyberSkills rule before migrating a family. Select the highest-value `syntax-candidate` whose false positives or negatives are demonstrated by fixtures.

## Selection requirements

- Existing validator and vendor mapping are proved in CP01.
- Text matching is structurally insufficient.
- Required facts already exist in CP03; Luna may not add grammar facts.
- The pilot does not require cross-file graph traversal.

Likely candidates include command injection, insecure deserialization, mass assignment, NoSQL/SQL construction, SSTI, or prototype pollution. The boss selects one after the CP01 report.

## Requirement Checklist

- [ ] Freeze the current validator result over its complete fixture corpus.
- [ ] Add comment, string-literal, nested-call, alias/import, multiline, malformed, and language-variant cases relevant to the predicate.
- [ ] State expected old-versus-new result for every case before implementation.
- [ ] Implement the rule over normalized facts, retaining text matching only for genuinely textual sub-predicates.
- [ ] Prove no previously accepted true positive is lost without an explicit correction rationale.
- [ ] Prove at least one demonstrated structural false positive or false negative is fixed.
- [ ] Parse failure is explicit and does not become clean.
- [ ] Update only this rule's component evidence and `notProved`.

## Acceptance And Proof

Run syntax tests, the focused rule test, all CyberSkills tests in the changed crate, clippy/fmt, Enforcer exact-file/crate/diff checks, and detached-parent introduced-findings comparison.

## Stop conditions

Stop if new syntax capabilities, validator routing, or cross-file facts are required. Return them to UL04, UL05/UL06, or UL13 instead of bypassing the interface.

## Parallel Ownership Notes

The boss replaces `<approved-pilot>` with one exact rule and fixture path before claim. CP04 consumes `enforcer-syntax` read-only and cannot overlap another rule worker on the same files.
