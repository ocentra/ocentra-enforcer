# c03 Claude Adapter

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Claude Adapter`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-install/src/adapters/claude.rs, crates/enforcer-install/tests/fixtures/claude_adapter/**`
- deps: `arc-23`, `c01-install-core-and-cli-contract`, `c02-harness-autodetect`
- tier: `P5 install-proof`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
Only Codex had legacy `.mjs` install logic. Claude Code reads a different config surface: the top-level `mcpServers` map in **`~/.claude.json`** — the SAME user/global file `codebase-memory-mcp` is registered in (JSON, not TOML), NOT `~/.claude/.mcp.json` and NOT a per-repo `.mcp.json` — plus a skills dir, a `CLAUDE.md` managed block, and hook wiring. The Codex TOML upsert cannot be reused verbatim for Claude's JSON `mcpServers` map, and there is no Rust Claude adapter yet — the arc-23 crate skeleton registers the adapter module-root but leaves `src/adapters/claude.rs` empty.

## Where We Want To Be
A Claude adapter type in `src/adapters/claude.rs` that implements the c01 `HarnessAdapter` trait (`plan`/`apply`/`verify`). It registers the `enforcer` binary (by ABSOLUTE path) as Claude's MCP stdio server in the top-level `mcpServers` map of **`~/.claude.json`** (USER/GLOBAL scope — the same file `codebase-memory-mcp` registers into; never a per-repo `.mcp.json`), drops the enforcer skill, emits the Claude agent/subagent descriptor (`.claude/agents/<server-name>.md` — the Claude-native analog of Codex's `agents/openai.yaml`) so Claude gets the same implicit-invocation/agent setup Codex has, upserts a `CLAUDE.md` managed block (harness-neutral doctrine ref that names the enforcer subagent), and sets the ledger-root env — all as structured c01 `Report`/`ApplyResult`/`Checks` records.

## Requirement Checklist
- [ ] Implement `struct ClaudeAdapter` + `impl HarnessAdapter for ClaudeAdapter` (from c01), returning c01 report/result records — not a standalone binary.
- [ ] JSON upsert into the top-level `mcpServers` map of `~/.claude.json` (USER/GLOBAL scope; the same file `codebase-memory-mcp` lives in — never a per-repo `.mcp.json`): `mcpServers[<server-name>]` = `{ command: <ABSOLUTE enforcer-binary-path>, args: [], env: { OCENTRA_LEDGER_HOME } }` (the binary speaks MCP on stdio — no `node`/`.mjs`; the absolute path is required so it resolves from any repo cwd), preserving unrelated keys via `serde_json` value-merge. The server-name const is owned by x01 (neutral-rename) — the key resolves to `"enforcer"`; this adapter consumes the const, never a hardcoded literal.
- [ ] Install the enforcer skill under `~/.claude/skills/<server-name>` (skill assets emitted by the installer).
- [ ] Upsert a `CLAUDE.md` managed block (reuse c01 marker/managed-block helpers) pointing at the MCP tools.
- [ ] EMIT the Claude agent/subagent descriptor — the equivalent of Codex's `skills/rust-rules-hard-gate/agents/openai.yaml` (`display_name`/`default_prompt`/`allow_implicit_invocation`), which Claude has NO equivalent of today. Write `.claude/agents/<server-name>.md` subagent definition(s) (YAML-frontmatter `name`/`description`/tool-scope + body prompt) mirroring the Codex descriptor's implicit-invocation/agent setup, plus the `CLAUDE.md` managed-block that references the enforcer subagent + MCP tools. This is the "Claude setup for the agent/sub-agent/swarm" the owner flagged; it mirrors how c06 emits the Codex `openai.yaml` descriptor, and references the agent/sub-agent/swarm taxonomy defined in EXECUTION_MODEL.md + arc-16.
- [ ] Set ledger env consistently with the Codex adapter (`OCENTRA_LEDGER_HOME`), backup-on-change with a timestamped copy.
- [ ] `verify` re-reads `~/.claude.json` (top-level `mcpServers`) and confirms the registered `command` resolves to the installed `enforcer` binary, AND confirms the `.claude/agents/<server-name>.md` descriptor exists and parses (frontmatter present, implicit-invocation intent wired). Fail-closed on malformed JSON or a missing/corrupt descriptor. Obey `[workspace.lints]`; no `pub use` barrels.

## Acceptance And Proof
Tier P5 install-proof (`claude-adapter-install` in TEST_PROOF_EXPECTATIONS.md): `cargo test -p enforcer-install` against a temp `~/.claude` fixture (`tests/fixtures/claude_adapter/**`) runs `install` then `verify` and asserts all-green checks (pass fixture) — including that `install` wrote `.claude/agents/<server-name>.md` with valid frontmatter and `verify` confirms the descriptor. A run where the descriptor is absent or hand-corrupted makes `verify` return a typed error, not skip (descriptor fail fixture); likewise a hand-edited/corrupt `~/.claude.json` fails closed (fail fixture). Round-trip `install`->`uninstall` restores the pre-state file byte-for-byte (descriptor removed). Clean `cargo clippy` / `cargo fmt --check`.

## Parallel Ownership Notes
Owns only `crates/enforcer-install/src/adapters/claude.rs` (+ its fixtures). Hooks live under `crates/enforcer-install/src/hooks/**` (c04/c05) and are emitted BY this adapter but owned as separate files, so this runs concurrently with them; it depends on arc-23 (skeleton) + c01 (trait) + c02 (detected home path). owns disjoint? = Y.
