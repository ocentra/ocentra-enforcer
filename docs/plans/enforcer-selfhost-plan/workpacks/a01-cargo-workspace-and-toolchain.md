# a01 Cargo Workspace And Toolchain

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Cargo Workspace And Toolchain`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `Cargo.toml`, `rust-toolchain.toml`, `.cargo/config.toml`, `deny.toml`, `package.json`, `.github/workflows/ci.yml`
- deps: `none`
- tier: `P1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
The repo is `.mjs` with `"type": "module"` and `engines.node` in `package.json`. There is no `Cargo.toml`, no `rust-toolchain.toml`, and no `crates/` tree. The enforcer is being rebuilt as a **Cargo workspace of Rust crates** (see RUST_ARCHITECTURE.md), so there is no compiler contract, pinned toolchain, or lint/deny gate for any crate to build against. Every crate-build workpack needs the workspace + toolchain to exist first.

## Where We Want To Be
A committed root `Cargo.toml` declaring the workspace and its member crates, a pinned `rust-toolchain.toml` (channel + `clippy`/`rustfmt` components), the **`[workspace.lints]` deny-wall** (the OcentraParent borrow), and a `deny.toml`, so the whole workspace builds under one contract and CI runs `cargo fmt --check` + `cargo clippy -D warnings` + `cargo test` + `cargo deny check` + `cargo audit` as hard gates the whole plan depends on. The deny-wall is `[workspace.lints.rust] unsafe_code = "forbid"` plus a `[workspace.lints.clippy]` DENY set (`unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented`, `dbg_macro`, `print_stdout`, `print_stderr`, `await_holding_lock`, `future_not_send`, `clone_on_ref_ptr`, `redundant_clone`, `needless_pass_by_value`, `map_err_ignore`, `large_enum_variant`); every crate opts in via `[lints] workspace = true`. `print_stdout`/`print_stderr` are `allow`ed via a scoped `#[allow(...)]` in exactly ONE output-sink module of `enforcer-cli` and ONE in `enforcer-mcp` — nowhere else. `package.json` retains only what the Tauri UI frontend needs (no `.mjs` bin / no Node engine as the runtime); the engine ships as a native binary.

## Requirement Checklist
- [ ] Root `Cargo.toml` declares `[workspace]` with `members = [ "crates/*" ]` (the crate map from RUST_ARCHITECTURE.md: `enforcer-core`, `enforcer-domain`, `enforcer-config`, `enforcer-rules`, `enforcer-validator`, `enforcer-scan`, `enforcer-cli`, ...) and `resolver = "2"`, plus `[workspace.package]`/`[workspace.dependencies]` for centralized versions.
- [ ] `rust-toolchain.toml` pins a specific stable `channel` and lists `components = ["clippy", "rustfmt"]`, so `cargo clippy` and `cargo fmt` are reproducible.
- [ ] Root `Cargo.toml` declares the `[workspace.lints]` deny-wall: `[workspace.lints.rust] unsafe_code = "forbid"` + a `[workspace.lints.clippy]` DENY set (`unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented`, `dbg_macro`, `print_stdout`, `print_stderr`, `await_holding_lock`, `future_not_send`, `clone_on_ref_ptr`, `redundant_clone`, `needless_pass_by_value`, `map_err_ignore`, `large_enum_variant`). Each member crate opts in with `[lints] workspace = true`. The ONLY `print_stdout`/`print_stderr` escape hatch is a scoped `#[allow(...)]` reserved for a single output-sink module in `enforcer-cli` and one in `enforcer-mcp`.
- [ ] `deny.toml` configured for `cargo deny check` (advisories + licenses + bans + sources); `cargo audit` wired in CI.
- [ ] `package.json` drops the `.mjs` bin and Node-runtime `engines`; it keeps nothing beyond what the Tauri UI frontend build needs (or is reduced to a UI-only manifest).
- [ ] `.github/workflows/ci.yml` (the base gate a01 owns) runs the full gate on push/PR across the target matrix: `cargo fmt --all --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --workspace`, `cargo deny check`, `cargo audit`. (The separate native self-enforcement workflow `.github/workflows/dogfood.yml` is owned by a10, not a01.)
- [ ] `cargo build --workspace` succeeds on the skeleton tree; an injected `clippy`/`fmt` violation makes the corresponding CI step exit non-zero.

## Acceptance And Proof
Tier P1. A CI/test row in TEST_PROOF_EXPECTATIONS.md asserts each gate's exit code: `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test --workspace`, `cargo deny check`, and `cargo audit` all exit 0 on the clean skeleton and non-zero against a seeded violation fixture (unformatted file / clippy lint / vulnerable dep pin). A seeded `unwrap()` (or an unscoped `println!`) in a member crate makes `cargo clippy` exit non-zero, proving the deny-wall is live and workspace-wide. `Cargo.toml`, `rust-toolchain.toml`, and `deny.toml` presence + required keys (including the `[workspace.lints]` deny set) asserted by a config test.

## Parallel Ownership Notes
a01 is the SOLE owner of the workspace-root manifests (`Cargo.toml`, `rust-toolchain.toml`, `.cargo/config.toml`, `deny.toml`, `package.json`) and the base CI `.github/workflows/ci.yml`, INCLUDING the `[workspace.lints]` deny-wall block in the root `Cargo.toml`. It blocks every other Track A workpack (they need a compiling workspace + pinned toolchain): every `arc-NN` crate deps a01 (via arc-01) for the workspace to exist. `arc-01` (`enforcer-core`) is the first member crate and now owns ONLY `crates/enforcer-core/**` — it no longer owns the workspace root. Sibling packs own disjoint crate source trees under `crates/<name>/**` (auto-included by a01's `members = ["crates/*"]` glob), so no overlap; each crate pack adds its own `[lints] workspace = true` opt-in inside its own `Cargo.toml`. a01 owns the ENGINEERING enforcement (the `Cargo.toml` deny-wall that governs THIS workspace); `arc-04` (`enforcer-rules`) separately ships the SAME lint set as a typed T1 rule RECORD so the enforcer governs CONSUMER repos too — disjoint (a01 owns the manifest keys, arc-04 owns the rule data). a10 owns the disjoint `.github/workflows/dogfood.yml` (the native self-enforcement workflow) — a separate file from a01's `ci.yml`, and a10 deps a01 so they never run concurrently. Sequence a01 FIRST.
