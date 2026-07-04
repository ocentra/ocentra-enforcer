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

- owns: `package.json (name/bin only), scripts/enforcer.mjs, mcp/enforcer-mcp.mjs, enforcer.config.json, mcp/rust-rules-mcp-fingerprint.mjs (server-name + path list)`
- deps: `none`
- tier: `P1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [ADBP_GAPS](../ADBP_GAPS.md).

## Where We Are
The product ships under the legacy name `ocentra-enforcer` across every workplace-visible surface: `package.json` `"name": "ocentra-enforcer"` and its `bin`; the CLI entry `scripts/ocentra-enforcer.mjs`; the MCP entry `mcp/ocentra-enforcer-mcp.mjs`; the config file `ocentra-enforcer.config.json`; the MCP **server name** `ocentra-enforcer` (exposing tools as `mcp__ocentra_enforcer__*`); the managed-block markers and global-instruction text; and the MCP fingerprint path list. The correct product name is **enforcer**.

## Where We Want To Be
Everything workplace-visible reads **enforcer**: the command is `enforcer ...`, the MCP server is `enforcer` (tools surface as `mcp__enforcer__*`), the package name/bin is `enforcer`, config is `enforcer.config.json`, and the two entry scripts are physically renamed. All internal `require`/`import` paths and the MCP fingerprint path list are updated to the new filenames. The local repo **folder** path (`C:/Projects/ocentra-enforcer`) is cosmetic and explicitly **out of scope**. Existing incidental file-path references elsewhere (other packs' prose citing `scripts/ocentra-enforcer.mjs` as a then-current name) are handled by those packs; this pack owns the physical rename + the shipped/config surfaces.

## Requirement Checklist
- [ ] `package.json`: `"name"` → `enforcer`; `bin` key/target → `enforcer` pointing at the renamed entry.
- [ ] Rename `scripts/ocentra-enforcer.mjs` → `scripts/enforcer.mjs`; update every internal require/import that resolved it.
- [ ] Rename `mcp/ocentra-enforcer-mcp.mjs` → `mcp/enforcer-mcp.mjs`; update every internal require/import.
- [ ] Rename `ocentra-enforcer.config.json` → `enforcer.config.json`; update the config loader's default lookup path.
- [ ] MCP server name `ocentra-enforcer` → `enforcer` (tool namespace becomes `mcp__enforcer__*`).
- [ ] Update managed-block markers and global-instruction text to say `enforcer`.
- [ ] Update the MCP fingerprint path list (`mcp/rust-rules-mcp-fingerprint.mjs`) to the renamed file paths.
- [ ] Do NOT rename the repo folder, and do NOT touch `Tools/ocentra-literal-scan/**` (that is a distinct tool dir; its own rename, if any, is not in scope here).

## Acceptance And Proof
Tier T1 (deterministic). Fail-fixture / pass-fixture expressed as a grep gate over shipped/config surfaces:
- **Pass condition:** after rename, `grep -riE "ocentra[-_]enforcer" package.json scripts/enforcer.mjs mcp/enforcer-mcp.mjs enforcer.config.json <managed-block+global-instruction surfaces>` returns **empty**.
- **Fail condition:** any remaining `ocentra-enforcer`/`ocentra_enforcer` token in those shipped/config surfaces (the grep must find zero — a match = fail). Note the grep is scoped to shipped/config surfaces, NOT to `Tools/ocentra-literal-scan/**` nor to plan-doc prose.
- **mcp:smoke still green:** the MCP smoke test passes end-to-end under the new server name `enforcer`, resolving tools as `mcp__enforcer__*` and loading the renamed entry.

Named proof rows in TEST_PROOF_EXPECTATIONS.md: `neutral-rename-grep-clean` (grep-empty over shipped/config surfaces) and `neutral-rename-mcp-smoke` (mcp:smoke green post-rename).

## Parallel Ownership Notes
`deps: none` — can run early. `owns:` is limited to the rename targets and the fingerprint path list; it does not edit sibling rule families, and it must not alter behavior beyond the name/path substitution. Sibling packs that cite `scripts/ocentra-enforcer.mjs` in prose keep their current text (they reference the pre-rename real filename) — reconciling those citations is not this pack's job. Coordinate ordering only insofar as any pack adding new bin/MCP-name references should target `enforcer` once this pack lands.
