# arc-23 Crate enforcer-install

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Crate enforcer-install`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-install/**`
- deps: `arc-01`, `arc-02`, `arc-03`, `arc-22`
- tier: `P1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
Harness registration (writing MCP server config for claude/codex/gemini/cursor/...) belonged to the retired Node engine (per-harness install + asset-validation logic). That engine is dropped; there is no Rust installer, no binary-distribution download path, and no emitter for the non-MCP surfaces (cargo-alias, pre-commit hook, tool-neutral doctrine ref) that make the CLI equally first-class.

## Where We Want To Be
`enforcer-install` is the Track C crate per RUST_ARCHITECTURE.md: a multi-harness installer that stands up the crate SKELETON — install/uninstall/update/doctor core + adapter interface + report types + platform binary distribution/download (the codebase-memory model) — over which per-harness adapters register the `enforcer` binary as each harness's MCP server. It treats BOTH surfaces as first-class: the installer emits whatever a target wants — a harness MCP auto-registration, a cargo-alias, a pre-commit hook, and/or a tool-neutral doctrine reference — from Rust emitter modules (never a `.ts`/Node hook). It hosts the Track C per-harness adapters (c01-c09) and the x03 legacy-name migration.

## Requirement Checklist
- [ ] Stand up the `enforcer-install` crate skeleton per RUST_ARCHITECTURE.md: the harness-neutral `install/uninstall/update/doctor` core, the adapter interface (`plan(ctx)->report`, `apply(report)->result`, `verify(ctx)->checks`), report types, managed-block/backup helpers, and the CLI contract seam (`--scope user|project`, **default `user`/global**, `--dry-run`, non-TTY JSON) consumed by arc-22.
- [ ] Emit the MCP registration: each per-harness adapter writes/updates that harness's **user-level (global) registry** — for Claude the top-level `mcpServers` in `~/.claude.json` (codebase-memory-mcp's file), for Codex `~/.codex/config.toml`, etc. — pointing at the **absolute** path of the `enforcer` binary (the binary itself speaks MCP on stdio); never a per-repo project file — the install-once, zero-per-repo-config agent UX.
- [ ] Emit the CLI-first-class artifacts from Rust emitter modules: a cargo-alias (so `cargo enforce`/direct CLI use is first-class for CI/precommit), a pre-commit hook that runs the `enforcer` binary, and a tool-neutral doctrine reference — each optional per target, all Rust-emitted (Claude-specific artifacts like a `CLAUDE.md` managed block or PreToolUse/SessionStart hooks are emitted by the claude adapter/hook modules, distinct from a shared `AGENTS.md`/tool-neutral doctrine ref).
- [ ] Implement the platform binary distribution/download path: resolve the correct released binary (win/mac/linux incl. musl + apple-silicon) and install it; support the `enforcer install` entrypoint (via arc-22 CLI); no runtime toolchain required by consumers.
- [ ] `cargo test -p enforcer-install` passes with fail/pass fixtures per adapter: install into a temp harness-config fixture yields the expected registration (pass), `--dry-run` writes zero files, idempotent re-install / malformed-existing-config is handled (fail fixture asserts detection), and the cargo-alias/pre-commit/doctrine-ref emitters produce the expected artifacts.
- [ ] Clean `cargo clippy` / `cargo fmt --check` (obey `[workspace.lints]`; no `pub use` barrels).

## Acceptance And Proof
Tier P1. Proof row asserts `cargo test -p enforcer-install` exits 0 — per-harness registration proven against temp-config fixtures (fail/pass), the cargo-alias/pre-commit/doctrine-ref emitters covered, binary-resolution logic covered, and `--dry-run` filesystem-diff empty. Record the artifact path in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Owns the `enforcer-install` crate SKELETON: `crates/enforcer-install/Cargo.toml`, `src/lib.rs`, `src/core.rs`, `src/cli_contract.rs`, `src/report.rs`, and the binary-distribution/download module. Deps arc-01/02/03/22 (needs the CLI/binary it installs).

Parallel Ownership Note (disjoint feature packs): the Track C adapter/hook packs each own SPECIFIC files under this crate — c02 `src/detect.rs`; c03 `src/adapters/claude.rs`; c04 `src/hooks/pretooluse.rs`; c05 `src/hooks/sessionstart.rs`; c06 `src/adapters/codex.rs`; c07 `src/adapters/generic.rs` + `src/doctor.rs`; c08 `src/adapters/{gemini,cursor,zed}.rs`; c09 `src/adapters/{antigravity,windsurf,opencode,aider,kilocode,kiro}.rs` — plus x03 `src/migrate_legacy_name.rs`, each with its own `tests/` fixtures. They own their files, NOT the whole crate; they `deps:` arc-23 (c-packs also keep their intra-track deps on c01/c02) and are sequenced after this skeleton. owns stay DISJOINT BY FILE. Parallel-safe with arc-24 (ui) — disjoint crate trees.
