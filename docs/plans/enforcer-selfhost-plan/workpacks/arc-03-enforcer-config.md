# arc-03 Crate enforcer-config

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Crate enforcer-config`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-config/**`
- deps: `arc-01`, `arc-02`
- tier: `P1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
Config and env handling is scattered across `.mjs` (ad hoc `process.env` reads, JSON parsing without validation, per-script option parsing). There is no typed, parse-at-boundary config layer, so invalid config surfaces as runtime failures deep in the engine. HOWEVER a real, already-working THREE-LAYER config model exists today and must be PRESERVED, not flattened: (1) global/shared **profiles** — `profiles/{strict,ocentra-enforcer,ocentra-parent}.json`, each a COMPLETE config shape, the "any project, zero config" fallback; (2) the **per-project config** (`ocentra-enforcer.config.json`) which declares `profileName` (which global profile it layers on top of) + local overrides — self-mechanically-enforced today by `CFG-1.10`/`CFG-1.11` (must declare `schemaVersion`+`profileName`; `profileName` must be one of `knownProfiles = {strict, default, ocentra-enforcer, ocentra-parent}`); (3) **per-run OUTPUT** under `.enforce/runs/<id>/**` (diagnostics/verdicts) — this is NOT config, it is the harness's job (arc-18/arc-17), config only resolves the settings (incl. the `harness` sub-object: storageDir/maxRuns/retention) those crates consume.

## Where We Want To Be
`enforcer-config` provides typed config loading with parse-at-boundary AND preserves the 3-layer resolution: EMBED the canonical profiles into the binary at compile time (`include_str!` or a build-script, so the engine is self-contained per the "one binary IS the engine" doctrine — no external `profiles/` dir required for baseline operation, though a project may still supply extra custom profiles as external files); load the project's `ocentra-enforcer.config.json` (or its `.enforce/config` successor per f03) if present; resolve `profileName` -> the matching embedded (or custom) profile as the DEFAULTS layer; deep-merge the project's local overrides on top to produce ONE typed, total `EffectiveConfig` struct built on `enforcer-domain` newtypes. If no project config exists at all, the "default" profile alone IS the effective config — zero-config projects work out of the box. Downstream crates (scan/harness/proof/lang-*) consume only the resolved `EffectiveConfig`, never raw files/env.

## Requirement Checklist
- [ ] Implement typed config load per RUST_ARCHITECTURE.md (`enforcer-config`): parse config file(s) + env into validated structs built on `enforcer-domain` newtypes.
- [ ] Embed the canonical profiles (`strict`, `ocentra-enforcer`, `ocentra-parent`, `default`) into the binary at compile time; support additional custom profiles supplied as external files by a project.
- [ ] Implement 3-layer resolution: profile (defaults) -> project config (local overrides, deep-merged) -> `EffectiveConfig` (total, no downstream `Option` soup for required fields). Zero project config => the `default` profile alone is the effective config.
- [ ] Preserve the mechanical self-check semantics of `CFG-1.10`/`CFG-1.11` as TYPED LOAD ERRORS (not runtime findings): a project config missing `schemaVersion` or `profileName` fails to load with a typed error; a `profileName` that isn't in the known-profiles set fails to load with a typed error naming the unknown value.
- [ ] Parse-at-boundary: all validation happens at load; the returned config is total, with clear typed errors on malformed input.
- [ ] Port the `.mjs` env/JSON/option-parsing conventions (the config-shaped reads currently in `scripts/*.mjs` and `mcp/*.mjs`) into this crate.
- [ ] Add a `supportedPlatforms: ["windows"|"macos"|"linux", ...]` field to the config/profile shape (defaults to all three if absent — no silent relaxation by omission); this is what `PORT-1.1` in `enforcer-lang-common` (arc-09) reads to scope its platform-specific-script check to the project's DECLARED CI platforms rather than blanket-failing.
- [ ] `cargo test -p enforcer-config` passes with fail/pass fixtures: valid config with a known profile loads and correctly merges overrides over profile defaults (pass); zero config falls back to the `default` profile alone (pass); missing `schemaVersion`/`profileName` fails typed (fail); unknown `profileName` fails typed, naming it (fail); env-override tests; `supportedPlatforms` present vs. absent-defaults-to-all-three.
- [ ] Clean `cargo clippy` / `cargo fmt --check`.

## Acceptance And Proof
Tier P1. Proof row in TEST_PROOF_EXPECTATIONS.md asserts `cargo test -p enforcer-config` exits 0 with fail/pass config fixtures, INCLUDING the 3-layer resolution cases (profile-only, profile+override merge, unknown-profile rejection, missing-field rejection). Record the artifact path.

## Parallel Ownership Notes
Owns only `crates/enforcer-config/**` (incl. the embedded profile data). Deps arc-01 + arc-02. Runs in parallel with arc-04 (rules) once the foundation is in; both consume domain types but own disjoint crate trees. Produces the `EffectiveConfig` (incl. the `harness` sub-config) that arc-15 (scan)/arc-17 (proof)/arc-18 (harness) consume read-only to decide `.enforce/` output location/retention — `enforcer-config` never writes to `.enforce/` itself.
