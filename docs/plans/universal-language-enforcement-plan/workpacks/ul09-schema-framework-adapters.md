# UL09 - Schema-Framework Adapters

<!-- agent-capsule -->
> Agent Capsule
> Plan: `universal-language-enforcement-plan`
> Doc: `UL09 Schema-Framework Adapters`
> Kind: bounded framework-recognition workpack.
> Read when: UL01, UL04, and UL08 are accepted for the selected language.
> Stop rule: recognize one framework for one requirement; the doctrine resolver owns the verdict.
> Proves: framework evidence and policy acceptance are separate.
> Does not prove: all uses of the framework are valid or all boundary shapes are covered.
> Proof rule: one recognized shape receives different profile verdicts without adapter changes.
<!-- /agent-capsule -->

- owns: one `(language, requirement, framework-family)` adapter directory, its fixtures, immutable proposal/evidence under `proof/universal-language/ul09/**`; framework registry row is integrator-only
- deps: `UL01, UL04, UL08`
- tier: `P1 T1/T2 recognizer`

> Owner class: Luna-safe leaf packet; `framework-registry-integrator` applies shared rows.
> Batch limit: one language, one requirement, one framework family.

## Where We Are

Effect, Zod, and Pydantic are detected through library-specific text markers and mixed directly with policy verdicts. That cannot express profile choice or equivalent shape providers reliably.

## Where We Want To Be

Adapters emit normalized evidence such as boundary decoder, validated model, branded/newtype identity, schema-derived serializer, and configuration parser. UL01 resolves whether the recognized family is accepted.

## Owns

- one leaf recognizer/adapter and disjoint positive/negative/malformed/alias/unavailable fixtures;
- immutable evidence and a proposed shared registry row;
- no doctrine resolver/profile, syntax fact contract, language registry, shared framework registry, or unrelated rule.

## Objective

Replace hard-coded library doctrine with reusable recognition plus data-driven policy, choosing mature semantic tooling over source inference where it exists.

## Requirement Checklist

- [ ] Record reuse decision: existing analyzer/plugin output versus native normalized-fact recognition.
- [ ] Recognizer emits evidence only; it cannot return `accepted/rejected` itself.
- [ ] Alias/import/decorator/macro/generic forms and misleading comments/strings are tested where applicable.
- [ ] Required facts/tool results are declared and unavailable behavior is visible.
- [ ] Same evidence is accepted/rejected under two profiles as configured.
- [ ] False-positive/negative limits and `doesNotProve` are explicit.
- [ ] Child submits a registry proposal; only integrator writes shared data.

## Acceptance And Proof

Run adapter fixtures, doctrine resolver composition test, selected tool/fact provider gate, registry proposal validator, and Enforcer checks. Manager reproduces the decisive profile flip on packet SHA.

## Stop conditions

Stop if recognition requires raw parser nodes, a new fact/tool contract, a hard-coded policy verdict, or a mature semantic engine that has not been evaluated for reuse.

## Parallel Ownership Notes

Up to three disjoint framework packets may run only after the contract freezes. They cannot share language adapter files or write the framework registry.
