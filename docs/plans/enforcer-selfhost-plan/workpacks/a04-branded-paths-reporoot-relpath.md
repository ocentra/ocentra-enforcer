# a04 Branded Paths RepoRoot RelPath (enforcer-domain)

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Branded Paths RepoRoot RelPath (enforcer-domain)`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-domain/src/path.rs`
- deps: `a01`
- tier: `P0`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
The legacy path utilities returned and accepted raw `string` paths with no type-level distinction between an absolute repo root and a repo-relative path. In Rust, using bare `PathBuf`/`String` everywhere reintroduces the same hazard: a relative joined onto another relative, or an absolute leaked into a report field expected to be relative, with no compiler signal. This is a **domain-modeling** workpack that populates the `enforcer-domain` crate — it FOLDS INTO arc-02-enforcer-domain.

## Where We Want To Be
`RepoRoot` (validated absolute base) and `RelPath` (validated repo-relative) branded **newtypes** in `enforcer-domain`, each with a private inner field, minted only via a `parse`-at-boundary constructor (and `serde`), so mixing them is a compile error and cross-platform normalization happens once at the boundary.

## Requirement Checklist
- [ ] Define `RepoRoot` and `RelPath` newtypes in `crates/enforcer-domain/src/path.rs` with private inner fields and parse-at-boundary constructors; derive `Debug, Clone, PartialEq, Eq, Hash` + `serde` via the validator.
- [ ] Only this module mints the newtypes; consumer crates import the types, never construct the inner field.
- [ ] `RepoRoot::relativize(&self, abs)` and `RepoRoot::resolve(&self, rel: &RelPath)` typed so root+rel is checked and rel+rel is a type error (no method takes two `RelPath`s as a base).
- [ ] Windows/POSIX separator normalization applied at mint time; `RelPath` stores a normalized forward-slash form and never contains `\\` or a drive prefix.
- [ ] Parse fails-closed (`Err`) on absolute-passed-as-relative and on `..` escapes above root.

## Acceptance And Proof
Tier P0. `cargo test` in `enforcer-domain`: valid mint, rejection of absolute-as-relative and `..` escape, separator normalization asserted on both Windows and POSIX (via `#[cfg]` or normalized-input tests). Fail/pass fixtures per RUST_ARCHITECTURE 5-way parity. The private field + distinct types make `RelPath` substituting for `RepoRoot` a compile error. Rows in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Depends on a01. Populates the `enforcer-domain` crate (folds into arc-02-enforcer-domain); owns `crates/enforcer-domain/src/path.rs` exclusively; disjoint from a03/a05/a06 newtype modules. Downstream consumer crates adopt the types in their own workpacks — this pack ships the types + boundary only. Coordinate `mod`/`pub use` in `enforcer-domain/src/lib.rs`.
