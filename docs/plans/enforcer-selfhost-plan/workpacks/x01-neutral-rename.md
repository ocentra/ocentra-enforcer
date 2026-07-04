# x01 Neutral Rename

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Neutral Rename`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling completion, product status, PR readiness, or broad DONE.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `Cargo.toml (workspace.package name/metadata) + crates/*/Cargo.toml [package] name fields, crates/enforcer-cli/src/name.rs (binary name const), crates/enforcer-mcp/src/name.rs (MCP server-name const)`
- deps: `none`
- tier: `P1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md), [ADBP_GAPS](../ADBP_GAPS.md).

## Where We Are
The Cargo workspace ships under the legacy name `ocentra-enforcer` across every workplace-visible surface: the crate `[package]` names in `crates/*/Cargo.toml` (`ocentra-enforcer-cli`, `ocentra-enforcer-mcp`, ...), the produced binary name (`[[bin]] name` of `enforcer-cli`), and the MCP **server name** const the stdio server advertises — `ocentra-enforcer` — which surfaces tools to harnesses as `mcp__ocentra_enforcer__*` (plus the transitional `rust_rules_*` tool prefix). Legacy tool-name prefixes (`ocentra_enforcer_*`, `rust_rules_*`) are still baked into the MCP server-name/prefix consts. The correct product name is **enforcer**.

## Where We Want To Be
Everything workplace-visible reads **enforcer**: the produced binary is `enforcer` (`[[bin]] name = "enforcer"` on `enforcer-cli`), the MCP server advertises the name `enforcer` (tools surface as `mcp__enforcer__*`), and the workspace/crate `[package]` names are the neutral `enforcer-*` set (`enforcer-core`, `enforcer-domain`, `enforcer-cli`, `enforcer-mcp`, ...). The binary-name const (`crates/enforcer-cli/src/name.rs`) and the MCP server-name const (`crates/enforcer-mcp/src/name.rs`) are the single source of truth both surfaces read — no hardcoded `ocentra`/`rust_rules` literal anywhere in the shipped consts. The local repo **folder** path (`C:/Projects/ocentra-enforcer`) is cosmetic and explicitly **out of scope**. Legacy MCP registrations already written into harness configs are UPGRADED by x03 (rename-migration), not by this pack; x01 owns the shipped/config source-of-truth names so x03 has a stable target.

## Requirement Checklist
- [ ] Root `Cargo.toml` `[workspace.package]` metadata (and any workspace-level `name`/`description`/`repository` product strings) read `enforcer`, not `ocentra-enforcer`.
- [ ] Each member crate's `[package] name` in `crates/*/Cargo.toml` is the neutral `enforcer-*` form (e.g. `enforcer-cli`, `enforcer-mcp`); no `ocentra-enforcer-*` crate name remains. (Coordinate only the name field; a01 owns the workspace deny-wall/manifest structure — this pack changes name strings, not lint/dep structure.)
- [ ] `enforcer-cli`'s binary target is `[[bin]] name = "enforcer"`; the binary-name const in `crates/enforcer-cli/src/name.rs` is `"enforcer"`.
- [ ] The MCP server-name const in `crates/enforcer-mcp/src/name.rs` is `"enforcer"` (tool namespace becomes `mcp__enforcer__*`); drop the legacy `ocentra_enforcer` / `rust_rules` prefix literals from the shipped const.
- [ ] Both surfaces read the name from the const (single source of truth); no inline `ocentra`/`rust_rules` string literal in shipped Rust or `Cargo.toml` package/bin fields.
- [ ] Do NOT rename the repo folder, and do NOT rename the `enforcer-literal-scan` crate contents beyond dropping any `ocentra` product literal — the T2 scanner crate keeps its role; its internal crate name follows the neutral `enforcer-*` convention only.
- [ ] Do NOT touch shared index/state/proof docs, other crates' logic, or harness-side already-installed configs (x03 owns the migration of live registrations).

## Acceptance And Proof
Tier T1 (deterministic). 5-way parity is Rust-native: a fail-fixture / pass-fixture pair plus a `cargo test` grep-gate over the shipped/config name surfaces.
- **Pass condition:** after rename, a grep gate over the owned surfaces (`Cargo.toml`, `crates/*/Cargo.toml` `[package]`/`[[bin]]` name fields, `crates/enforcer-cli/src/name.rs`, `crates/enforcer-mcp/src/name.rs`) for `ocentra[-_]enforcer` and `rust_rules` returns **empty**.
- **Fail condition:** any remaining `ocentra-enforcer`/`ocentra_enforcer`/`rust_rules` token in those shipped/config name surfaces (a match = fail). The grep is scoped to the owned name surfaces, NOT to the `enforcer-literal-scan` crate internals nor to plan-doc prose.
- **MCP smoke still green:** `cargo test` / the `enforcer-mcp` integration smoke starts the server under the new name `enforcer`, resolves tools as `mcp__enforcer__*`, and the built `enforcer` binary responds — end-to-end under the renamed server-name const. Native-tool invocations remain routed via `enforcer-harness`.
- Clean `cargo clippy` / `cargo fmt --check` (obey `[workspace.lints]`; no `pub use` barrels; the name const is a plain `pub const`, not a barrel re-export).

Named proof rows in TEST_PROOF_EXPECTATIONS.md: `neutral-rename-grep-clean` (grep-empty over shipped/config name surfaces via `cargo test`) and `neutral-rename-mcp-smoke` (`cargo test -p enforcer-mcp` smoke green post-rename under server name `enforcer`).

## Parallel Ownership Notes
`deps: none` — can run early. `owns:` is limited to the workspace/crate `[package]`/`[[bin]]` name fields and the two name consts (`crates/enforcer-cli/src/name.rs`, `crates/enforcer-mcp/src/name.rs`); it does not edit sibling rule families, validator logic, or crate skeletons beyond the name string. It must not alter behavior beyond the name/const substitution. a01 owns the workspace-root manifest STRUCTURE (deny-wall, member glob, centralized deps) — x01 touches only the product NAME strings within it; sequence so the name edit does not fight a01's structural edit (x01 deps none but any concurrent a01 run should be reconciled by the later index pass). Sibling adapter packs (c03..c09) CONSUME the server-name const rather than hardcoding it; x03 (rename-migration) targets the `enforcer` name this pack establishes. owns disjoint? = Y.
