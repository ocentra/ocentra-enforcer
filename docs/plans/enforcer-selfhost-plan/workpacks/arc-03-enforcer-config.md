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
Config and env handling is scattered across `.mjs` (ad hoc `process.env` reads, JSON parsing without validation, per-script option parsing). There is no typed, parse-at-boundary config layer, so invalid config surfaces as runtime failures deep in the engine.

## Where We Want To Be
`enforcer-config` provides typed config loading with parse-at-boundary: it reads config files + env, validates them into `enforcer-domain` types once at the edge, and hands the rest of the workspace already-valid config. Downstream crates never touch raw env/JSON.

## Requirement Checklist
- [ ] Implement typed config load per RUST_ARCHITECTURE.md (`enforcer-config`): parse config file(s) + env into validated structs built on `enforcer-domain` newtypes.
- [ ] Parse-at-boundary: all validation happens at load; the returned config is total (no `Option` soup downstream for required fields), with clear errors on malformed input.
- [ ] Port the `.mjs` env/JSON/option-parsing conventions (the config-shaped reads currently in `scripts/*.mjs` and `mcp/*.mjs`) into this crate.
- [ ] `cargo test -p enforcer-config` passes with fail/pass fixtures (valid config loads; missing/malformed config produces a typed error) + env-override tests.
- [ ] Clean `cargo clippy` / `cargo fmt --check`.

## Acceptance And Proof
Tier P1. Proof row in TEST_PROOF_EXPECTATIONS.md asserts `cargo test -p enforcer-config` exits 0 with fail/pass config fixtures. Record the artifact path.

## Parallel Ownership Notes
Owns only `crates/enforcer-config/**`. Deps arc-01 + arc-02. Runs in parallel with arc-04 (rules) once the foundation is in; both consume domain types but own disjoint crate trees.
