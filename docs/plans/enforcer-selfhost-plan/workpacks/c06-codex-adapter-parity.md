# c06 Codex Adapter Parity

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Codex Adapter Parity`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-install/src/adapters/codex.rs, crates/enforcer-install/tests/fixtures/codex/**`
- deps: `c01-install-core-and-cli-contract, arc-23`
- tier: `P5 install-proof`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
The Codex integration (TOML `mcp_servers` upsert, skill copy, `AGENTS.md` managed block, doctor with 15+ checks, backups) is the reference behavior we must reproduce natively. Once c01/arc-23 stand up the harness-neutral `enforcer-install` core and its `Adapter` trait, Codex must be re-expressed as a Rust adapter module behind that interface without regressing any existing on-disk result. The registered MCP server is the `enforcer` binary itself speaking MCP on stdio — no `node`/`.mjs` shim.

## Where We Want To Be
A `crates/enforcer-install/src/adapters/codex.rs` module implementing the arc-23 `Adapter` trait (`plan`/`apply`/`verify`) that produces the same on-disk result as the reference Codex integration, driven through the c01 install orchestrators. TOML is edited via `toml_edit` (format-preserving); the `AGENTS.md` managed block and backups are emitted by shared arc-23 core helpers.

## Requirement Checklist
- [ ] Re-express TOML `mcp_servers.<name>` upsert, ledger env, and skill copy as `plan`/`apply` on the `Adapter` trait (`toml_edit` for format-preserving upsert; command points at the `enforcer` binary path).
- [ ] Preserve the `AGENTS.md` managed-block start/end markers and content byte-for-byte via the c01/arc-23 marker helpers.
- [ ] Preserve timestamped backup filenames and the doctor check set (including `warning` severities), emitted through the shared core.
- [ ] `verify` returns typed checks mirroring the reference doctor set (binary resolves, server file, TOML section present, cwd, enabled) as structured `Check`/`Finding` records — no ad-hoc `print_*`, no `unwrap`/`expect`.
- [ ] Adapter registers in the arc-23 registry under id `codex` so the `enforcer install`/`enforcer doctor` CLI (arc-22) and c02 autodetect route to it; native tool checks run via `enforcer-harness` (arc-18), not ad-hoc shell-outs.

## Acceptance And Proof
P5 parity (`codex-adapter-parity` in TEST_PROOF_EXPECTATIONS.md), proved by `cargo test -p enforcer-install`: a golden-file `#[test]` asserts the adapter's generated TOML block and `AGENTS.md` block equal pinned snapshots under `crates/enforcer-install/tests/fixtures/codex/`; doctor check names/severities match a pinned snapshot. Any diff fails the build.

## Parallel Ownership Notes
Owns only `crates/enforcer-install/src/adapters/codex.rs` (+ its `tests/fixtures/codex/**`); the crate skeleton, `Adapter` trait, registry, and shared core/doctor/marker helpers belong to arc-23. Disjoint by file from claude (c03), generic (c07), and stub (c08) adapters, so it runs concurrently. Depends on c01 and arc-23. owns disjoint? = Y
