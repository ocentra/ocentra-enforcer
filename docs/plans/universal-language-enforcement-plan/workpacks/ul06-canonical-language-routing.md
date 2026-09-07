# UL06 - Canonical Language Routing

<!-- agent-capsule -->
> Agent Capsule
> Plan: `universal-language-enforcement-plan`
> Doc: `UL06 Canonical Language Routing`
> Kind: architect and integrator migration workpack.
> Read when: UL03 and UL05 contracts are accepted.
> Stop rule: migrate identity/routing data without inventing language capability.
> Proves: discovery, lexical, syntax, tool, rule, CLI/MCP, and UI routes derive from one identity registry.
> Does not prove: the routed provider or rule is correct.
> Proof rule: every prior identity is preserved or receives an explicit typed disposition.
<!-- /agent-capsule -->

- owns: `crates/enforcer-domain/src/language_types.rs`, `crates/enforcer-syntax/src/registry.rs`, `crates/enforcer-syntax/registry/languages.json`, generated adapters/tests in `crates/enforcer-literal-scan`, `crates/enforcer-scan`, `crates/enforcer-mcp`, and `crates/enforcer-cli` named by an approved migration manifest
- deps: `UL03, UL05`
- tier: `P0 identity contract, P1 generated routing`

> Owner class: Sol/architect plus one cross-crate registry integrator.
> Batch limit: one canonical registry migration; no new provider/rule behavior.

## Where We Are

Parser `Language`, literal registry, coarse `LanguageFamily`, `DetectedLanguage`, `NativeScanLanguage`, CLI/MCP language enums, and native-tool routes disagree in size and meaning. Dart/CFML crates exist while router comments say no pack; unrecognized parsed languages fall to `Other` or `Unknown`.

## Where We Want To Be

One stable language identity registry generates or validates every projection. Coarse families and aliases are derived. A language can be discovered without falsely claiming a parser, tool, or rule capability.

## Owns

- canonical identity types/data and a migration manifest listing every touched generated/validated projection;
- projection tests and generated artifacts only through the registry integrator;
- no language grammar, validator semantics, doctrine, or tool runner.

## Objective

Eliminate competing language truth and make all interfaces report the same honest capability row.

## Requirement Checklist

- [ ] Preserve all 160 parser identities, 65 literal names plus fallback, current aliases, and filename-only cases.
- [ ] Detect extension/basename collisions and require explicit precedence.
- [ ] Derive coarse scan families, route identities, CLI/MCP schema values, UI values, and tool ties.
- [ ] `Other`/`Unknown` cannot hide a recognized canonical identity.
- [ ] A projection may omit a capability only with typed `unsupported/not-applicable` evidence.
- [ ] Stable serialization and backwards-compatible alias decoding are proved.
- [ ] Generated files have drift checks; no hand-edited duplicate lists remain authoritative.

## Acceptance And Proof

Run registry generation in check/dry-run mode, all literal/syntax/router/CLI/MCP serialization tests, old-to-new golden crosswalk, dependency checks, and scoped Enforcer gates.

## Stop conditions

Stop on an unresolved alias/extension collision, wire-format break without migration, or any attempt to label routed-but-unproved rules as supported.

## Parallel Ownership Notes

Read-only per-registry inventories are parallel-safe. Canonical data and every shared projection have one integrator and are applied serially.
