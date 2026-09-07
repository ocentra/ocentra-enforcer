# Capability Model

## Canonical row

Each language row is mechanically derived and carries:

- stable `languageId`, extensions/basenames, aliases, and source registry;
- discovery, lexical, structural, graph, ecosystem, and rule capability states;
- parser provider/version, parse-quality behavior, fact completeness, and proof references;
- applicable native tools and external semantic providers;
- supported and unsupported rule families;
- exact positive, negative, malformed, and fallback fixtures;
- `notProved` text for every absent capability.

Allowed capability states are `proved`, `partial`, `unsupported`, `blocked`, and `not-applicable`. `supported` without a level is invalid.

## Rule requirements

Every fact-backed rule declares a closed minimum set, for example:

```text
language-identity
parse-quality
imports
calls
definitions
inheritance
annotations
boundary-shape
cross-file-resolution
```

Dispatch executes the rule only with all required facts. Missing facts produce a typed coverage diagnostic. A text fallback is legal only when the rule record declares the fallback and separately proves its narrower claim.

## Shape doctrine

The requirement and the framework are separate data:

| Requirement | Example framework families |
|---|---|
| boundary decoding | Effect Schema, Zod, Valibot, Pydantic, attrs validators, serde/newtypes |
| stable domain identity | branded/refined types, validated newtypes, value objects |
| serialization contract | typed encoders/decoders and schema-derived serializers |
| configuration boundary | typed config parser with closed unknown-field behavior |

The active profile maps `(language, requirement, framework family)` to a verdict. The default may prefer Effect for the owner's projects, but rule code asks the resolver and never hard-codes “Zod is universally wrong.”

## Evidence levels

- T1: deterministic exact verdict over typed inputs/facts.
- T2: deterministic scored/heuristic verdict with thresholds and false-positive proof.
- External: typed result from one allowlisted third-party engine through the shared runner.
- Advisory/manual: never represented as a mechanical pass/fail rule.
