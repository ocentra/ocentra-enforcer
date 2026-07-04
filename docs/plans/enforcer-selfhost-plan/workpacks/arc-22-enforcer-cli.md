# arc-22 Crate enforcer-cli

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Crate enforcer-cli`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-cli/**`
- deps: `arc-01`, `arc-02`, `arc-03`, `arc-15`, `arc-17`, `arc-20`, `arc-21`
- tier: `P1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
CLI entrypoints belonged to the retired Node engine (script entrypoints + output family). That engine is dropped; there is no single Rust binary that is both the CLI and the MCP stdio server.

## Where We Want To Be
`enforcer-cli` is the clap CLI per RUST_ARCHITECTURE.md that compiles to the `enforcer` binary (one binary IS the engine) and is a FIRST-CLASS product surface — equally first-class with the MCP, not secondary: `enforcer scan|check|install|serve|plan|proof|...`, and when invoked as an MCP server it serves `enforcer-mcp` on stdio. This is the single distributed artifact. It stands up the crate SKELETON with the tri-modal scope grammar, exit-code-driven verdicts, terse `Fix:` hints, NO override flag, and Windows-first argv handling; the `print_stdout`/`print_stderr` deny-wall lints are allowed in exactly ONE output-sink module. It hosts the d06 lifecycle-command family (`src/lifecycle.rs`).

## Requirement Checklist
- [ ] Implement the clap CLI per RUST_ARCHITECTURE.md with the subcommands (`scan`, `check`, `install`, `serve`, `plan`, `proof`, ...), each wired to the corresponding engine crate.
- [ ] Define the tri-modal scope grammar in clap: `enforcer check <paths...>` | `--base <sha> --head <sha>` | `--all` (mutually-exclusive scope selection), exit-code-driven (non-zero on findings), Windows-first (argv-quoting + backslash path normalization).
- [ ] Emit terse `Fix:` hints per finding when rendering a `Report`; there is NO override/bypass flag — the enforcer cannot be silenced from the command line; the ONLY exemption is a declarative, committed, gated waiver read from `enforcer-config`.
- [ ] Confine all stdout/stderr writes to ONE output-sink module carrying a scoped, documented `#![allow(clippy::print_stdout, clippy::print_stderr)]`; every other module obeys the `[workspace.lints]` deny wall. This is the ONLY sanctioned print site in the crate.
- [ ] `enforcer serve` (or stdio-detected) starts the `enforcer-mcp` server on stdio — one binary = MCP server + CLI.
- [ ] Produce the `enforcer` binary target; wire config via `enforcer-config` (the single declarative control-plane both surfaces read), output via `enforcer-domain` `Report` rendering.
- [ ] `cargo test -p enforcer-cli` passes with fail/pass fixtures: `enforcer check` on a fixture tree exits non-zero with findings (fail fixture) and zero on a clean tree (pass fixture); each of the three scope modes parses and routes; a `--base/--head` + `<paths...>` collision is a clap error; there is no flag that suppresses a finding; `serve` starts the MCP loop.
- [ ] Clean `cargo clippy` / `cargo fmt --check` (deny wall honored everywhere except the single output-sink module; no `pub use` barrels).

## Acceptance And Proof
Tier P1. Proof row asserts `cargo test -p enforcer-cli` exits 0 and an integration test runs the built `enforcer` binary on fail/pass fixture trees across all three scope modes with the expected exit codes, and asserts no override flag exists. Record the artifact path in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Owns the `enforcer-cli` crate SKELETON (the binary crate): `crates/enforcer-cli/Cargo.toml`, `src/main.rs`/`src/lib.rs`, the clap grammar (tri-modal scope), the `Report` rendering + `Fix:` hint path, and the single output-sink module (the sanctioned print site). Deps the engine + mcp + plan surfaces.

Parallel Ownership Note (disjoint feature packs): d06 lifecycle-commands owns `crates/enforcer-cli/src/lifecycle.rs` (the `plan|implement|check|fix|review` phase family) + its fixtures — NOT the whole crate; d06 `deps:` arc-22 and is sequenced after this skeleton. f02 onboard also lands a CLI-side seam here per the F mapping; owns stay DISJOINT BY FILE. It is the top integration point of the workspace; arc-23 (install) distributes this binary and registers it as each harness's MCP server, arc-24 (ui) may embed/launch it. CLI and MCP (arc-21) are both first-class, neither secondary.
