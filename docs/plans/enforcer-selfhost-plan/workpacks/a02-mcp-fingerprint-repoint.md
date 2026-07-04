# a02 MCP Fingerprint Repoint

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `MCP Fingerprint Repoint`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-mcp/src/fingerprint.rs`
- deps: `a01`, `a05`
- tier: `P3`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
The legacy MCP fingerprint hashed a hardcoded list of ~10 source `.mjs` paths (`MCP_FINGERPRINT_FILES`) to power restart/freshness detection. In the Rust engine the MCP server is the compiled `enforcer` binary itself (`enforcer-mcp` on stdio), so a frozen list of `.mjs` source paths is meaningless — it describes neither what is executing nor what changed. The freshness/restart signal must be derived from the actually-running binary.

## Where We Want To Be
The MCP fingerprint is a `Sha256` (a05's brand) computed over the running crate artifact(s) — the built `enforcer` binary and/or the workspace source tree it was compiled from — so the freshness/restart signal is truthful for the Rust engine and changes iff the shipped binary changes.

## Requirement Checklist
- [ ] Replace the hardcoded `.mjs` path list with a fingerprint over the running binary artifact (e.g. `current_exe()`) and/or a build-time content hash of the compiled crates, not source `.mjs` paths.
- [ ] `build_mcp_fingerprint` returns a `Sha256` (a05 brand); the digest changes iff the built artifact changes; unrelated source-adjacent files do not perturb it.
- [ ] A missing/unresolvable artifact yields an explicit `exists: false` (or a typed error) surfaced by `mcp_status`, never a silent pass.
- [ ] No path is hardcoded to a location that does not correspond to the loaded binary/crate.
- [ ] Optionally fold in the build fingerprint (`CARGO_PKG_VERSION` + git hash / build id) so a rebuilt-but-unmoved binary is still detectable.

## Acceptance And Proof
Tier P3 (live MCP-tool). A live test invokes the MCP `mcp_status`/freshness path and asserts the fingerprint `Sha256` over the running artifact; rebuild/replace the binary and reobserve a changed digest; a missing artifact yields `exists: false`. A `cargo test` in `enforcer-mcp` asserts the digest is a valid `Sha256`. Named in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Depends on a01 (workspace/binary) and a05 (consumes the `Sha256` brand from `enforcer-domain`). Owns only `crates/enforcer-mcp/src/fingerprint.rs`; disjoint from a05's domain newtype file. Sequence a05 before a02 (a02 consumes `Sha256`).
