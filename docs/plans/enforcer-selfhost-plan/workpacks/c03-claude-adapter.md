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

- owns: `src/install/adapters/claude.*`
- deps: `c01-install-core-and-cli-contract, c02-harness-autodetect`
- tier: `P5 install-proof`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
Only Codex has an adapter. Claude Code reads a different config surface: `~/.claude/.mcp.json` (JSON, not TOML), a skills dir, a `CLAUDE.md` managed block, and hook wiring. The Codex TOML upsert in `codex-install.mjs` cannot be reused verbatim for Claude's JSON `mcpServers` map.

## Where We Want To Be
A Claude adapter that installs the enforcer MCP server into `~/.claude/.mcp.json`, drops the enforcer skill, upserts a `CLAUDE.md` managed block, and sets the ledger-root env — all via the c01 report/apply/verify interface.

## Requirement Checklist
- [ ] JSON upsert of `mcpServers["ocentra-enforcer"]` = `{command:"node", args:[serverPath], env:{OCENTRA_LEDGER_HOME}}`, preserving unrelated keys.
- [ ] Install enforcer skill under `~/.claude/skills/ocentra-enforcer`.
- [ ] Upsert a `CLAUDE.md` managed block (reuse c01 marker helpers) pointing at the MCP tools.
- [ ] Set ledger env consistently with Codex (`OCENTRA_LEDGER_HOME`), backup-on-change with timestamp.
- [ ] `verify` re-reads `.mcp.json` and confirms the server path resolves to the installed `mcp/ocentra-enforcer-mcp.mjs`.

## Acceptance And Proof
P5 install-proof (`claude-adapter-install` in TEST_PROOF_EXPECTATIONS.md): against a temp `~/.claude` fixture, `install` then `verify` returns all-green checks; a hand-edited/corrupt `.mcp.json` makes `verify` fail-closed. Round-trip `install`->`uninstall` restores the pre-state file.

## Parallel Ownership Notes
Owns `src/install/adapters/claude.*` only. Hooks live under `src/install/hooks/**` (c04/c05), so this runs concurrently with them; it depends on c01/c02 for the interface and detected home path.
