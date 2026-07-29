# p01 Choosable Doctrine Profiles

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Choosable Doctrine Profiles`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling completion, product status, PR readiness, or broad DONE.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-config/src/doctrine_profile.rs`, `crates/enforcer-config/profiles/*.ron`, `crates/enforcer-config/tests/doctrine_profile.rs`, `crates/enforcer-config/tests/fixtures/doctrine_profile/**`
- deps: `arc-03`, `arc-04`, `g05`
- tier: `P1` (P0 contract for the profile schema as a secondary row)

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
The doctrine layer — parse-at-boundary, schemas required, no raw strings crossing a boundary, brand domain values — is universal and correct. But the *library* that satisfies it is currently hard-coded. The `.mjs` rules (and the Rust rules that inherit their intent, e.g. `e-pack-frontend-react`'s `FE-EFFECT-1.1` banning Zod and mandating Effect Schema) treat **Effect** as the one true schema library and flag every alternative. That is the OWNER'S personal default, not a universal law: parse-at-boundary is equally satisfied by `zod`/`valibot` in TS, `pydantic`/`attrs+validators` in Python, and `serde`+newtypes in Rust. Nothing in `enforcer-config` models "which library family satisfies which doctrine requirement", so a project that legitimately standardises on zod cannot pass the boundary-doctrine rules without hand-waivers. The arc-03 loader gives typed parse-at-boundary but has no `doctrine_profile` concept, and rule evaluation has no profile parameter to consult.

## Where We Want To Be
A DOCTRINE-PROFILE model in `crates/enforcer-config/src/doctrine_profile.rs` (`serde` structs on `enforcer-domain` newtypes, parsed at boundary by the arc-03 loader) that separates the universal REQUIREMENT (`parse-at-boundary`, `schema-required`, `no-raw-boundary-strings`, `brand-domain-values`) from the LIBRARY FAMILY that satisfies it. A profile declares, per requirement, which library families are accepted (e.g. effect-profile accepts only `Effect`; zod-profile accepts `Zod`; a permissive profile accepts `Effect|Zod|Valibot`) and carries per-rule and per-family TOGGLES (enable/disable a requirement, accept/reject a family, severity override). Rule evaluation is PARAMETERIZED by the active profile via a resolver the rule packs consult — instead of a rule hard-coding `bans Zod`, it asks the resolver "is family `Zod` accepted for requirement `schema-required` under the active profile?" and only flags when the answer is no. The owner's Effect-only stance is PRESERVED verbatim as the shipped DEFAULT profile (`profiles/effect-default.ron`), so nothing changes for the owner out-of-the-box. The g05 settings UI (which owns the config CONTROL surface) exposes the per-rule/per-family toggles; this pack owns the MODEL + resolver + shipped profiles that g05 renders and round-trips.

## Requirement Checklist
- [ ] A `serde` `DoctrineProfile` type in `src/doctrine_profile.rs` built on `enforcer-domain` newtypes: universal `Requirement` enum, `LibraryFamily` enum (per language: TS `Effect|Zod|Valibot`, Python `Pydantic|AttrsValidators`, Rust `SerdeNewtype`), and a `requirement -> {accepted families, enabled, severity}` map; loaded/validated through the arc-03 parse-at-boundary loader (a malformed profile -> typed `thiserror` boundary error, never a silent default).
- [ ] The shipped DEFAULT profile `profiles/effect-default.ron` encodes the owner's current stance exactly (Effect-only for TS `schema-required`; no behaviour change when no profile is selected) — the owner default is data, not code.
- [ ] A `resolve(requirement, family) -> FamilyVerdict { Accepted | Rejected | RequirementDisabled }` resolver API consumed by the library-family rules (e.g. `e-pack-frontend-react` `FE-EFFECT-1.1`, `e-pack-python` schema rules, `d17` Rust boundary rules) instead of hard-coded library checks; the resolver, not the rule, decides which family is legal under the active profile.
- [ ] Per-rule / per-family toggles round-trip losslessly through serde (re-serialising a toggled profile yields byte-identical config; no duplicated entries) so the g05 settings UI can read/toggle/persist without drift.
- [ ] Disabling a requirement or rejecting the owner-default family is an EXPLICIT profile choice recorded in the profile (with owner + reason where it weakens the default), never a silent inline suppression — honesty doctrine preserved; the resolver returns `RequirementDisabled` visibly rather than pretending a check ran.
- [ ] No hard-coded library string survives in the resolver path: swapping the active profile from effect to zod flips the family verdict without touching rule code.

## Acceptance And Proof
Tier P1 (with a P0 contract/schema row for the profile decode). Proof row `doctrine-profile-parameterization` in TEST_PROOF_EXPECTATIONS.md asserts `cargo test -p enforcer-config --test doctrine_profile` exits 0:
- fail-fixture: malformed profile (unknown `Requirement`/`LibraryFamily`, or a family not valid for the declared language) in `tests/fixtures/doctrine_profile/**` -> typed boundary decode error naming the field, no silent default.
- pass-fixture: the SAME requirement (`schema-required` with family `Zod`) resolves to `Rejected` under `effect-default.ron` and `Accepted` under a `zod-profile.ron` fixture — proving a fixed codebase's findings flip with the profile, not with rule edits; a per-rule severity override and a per-family toggle round-trip through serde byte-identically.
- detection test: absence of any profile -> the resolver returns the shipped `effect-default` verdicts (owner stance preserved); a requirement disabled in a profile resolves to a visible `RequirementDisabled`, never a fabricated pass.
Clean `cargo clippy` / `cargo fmt --check` (obey `[workspace.lints]`).

## Parallel Ownership Notes
Owns ONLY the new `doctrine_profile.rs` module, the shipped `profiles/*.ron` assets, and its `tests/doctrine_profile.rs` + `tests/fixtures/doctrine_profile/**` inside the arc-03 crate — disjoint by file from arc-03 (which owns the crate skeleton + base loader), from `a07` (`load.rs`/`env.rs`/`schema.rs`), and from `f03` (`project_tie.rs`/`policy.rs`). Deps arc-03 (loader), arc-04 (rule records carry the requirement/family tag the resolver keys on), and g05 (the settings UI seam that exposes + round-trips the toggles this pack models). It is the profile CONTRACT the library-family rule packs (`e-pack-frontend-react`, `e-pack-python`, `d17`) consume; those packs adopt the resolver rather than this pack editing their rule files. Orthogonal to `p02` (scan-ignore) and `p03` (AST matching). `owns disjoint? = Y`.
