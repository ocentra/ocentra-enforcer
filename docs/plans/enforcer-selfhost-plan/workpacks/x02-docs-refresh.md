# x02 Docs Refresh

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Docs Refresh`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling completion, product status, PR readiness, or broad DONE.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `README.md, docs/**.md (CODEX_SETUP, COORDINATION, INSTALL, TARGET_REPO_WIRING, ENFORCED_CHECKS, SKILL_MCP_SYSTEM, etc.), skills/*/SKILL.md, AGENTS.md/CLAUDE templates`
- deps: `x01-neutral-rename`
- tier: `P1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
The enforcer's own product docs still describe the legacy `ocentra` product name and the Codex-only install path (`codex install`, `CODEX_SETUP` framed as the single setup doc). They also predate the new capabilities: the detect-and-route router (f05), scan modes (f01), the UI layer (g-track), multi-harness install across all 11 harnesses (c-track), onboarding/autoindex (f02), and the new language packs (dart/cfml/frontend/python). x01 renames the shipped/config surfaces; the docs describing them are stale.

## Where We Want To Be
Product docs read **enforcer** everywhere and describe current capability. `CODEX_SETUP` becomes a per-harness / neutral setup doc (Codex is one of 11). Every new top-level capability has a doc section:
- detect-and-route router (f05), scan modes (f01), UI layer (g-track), multi-harness install / all 11 harnesses (c-track incl. c09), onboarding+autoindex (f02), silent-vs-human mode (f04), new languages dart/cfml/frontend/python.

## Requirement Checklist
- [ ] Rename product/command/MCP-server/skill references from `ocentra`/`codex install` to `enforcer` across README.md, docs/**.md, skills/*/SKILL.md, AGENTS.md/CLAUDE templates.
- [ ] Reframe `CODEX_SETUP` as neutral per-harness setup (Codex = one adapter of eleven).
- [ ] Add/refresh a doc section for each new top-level capability listed above.
- [ ] Do not edit real file-path references that x01 owns (physical rename/config paths) nor Tools/ocentra-literal-scan/**.
- [ ] Product name is **enforcer**, never **ocentra**, in all prose.

## Acceptance And Proof
Tier T1 (deterministic) link-and-name gate (`docs-refresh-grep-clean` + `docs-refresh-sections-present` in TEST_PROOF_EXPECTATIONS.md):
- **fail fixture**: a docs surface containing a stale `ocentra`/`codex install` *product* reference, OR a missing capability section -> gate fails naming the offending file/section.
- **pass fixture**: `grep -riE "ocentra|codex install"` over the owned docs product surfaces (excluding real file-path refs x01 owns and `Tools/ocentra-literal-scan/**`) returns empty, AND every new top-level capability (router f05, scan modes f01, UI g-track, multi-harness c-track, onboarding f02, silent-vs-human f04, dart/cfml/frontend/python) has a present, non-empty doc section.
- **detection test** (`docs-refresh-check`): asserts grep-clean over product surfaces and presence of each required capability heading.

## Parallel Ownership Notes
Depends on x01 (rename must land first so docs describe the real names). Owns product/doc prose only; disjoint from x01's shipped/config file surfaces. The grep gate is deliberately scoped to exclude x01-owned path references and the distinct `Tools/ocentra-literal-scan/**` dir to avoid double-ownership.
