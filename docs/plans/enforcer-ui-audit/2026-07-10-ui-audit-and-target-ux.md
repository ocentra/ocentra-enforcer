# Enforcer UI Audit And Target UX - 2026-07-10

This is an implementation handoff for the current first-pass Tauri/React UI.
It records what is on screen now, why parts are confusing, what Enforcer
features exist, what should drive the UI data, and what a follow-up agent should
change next.

## Implementation Update - 2026-07-10

The initial layout audit below is retained as a record of the original gaps.
The following UI work is now implemented and verified in the live Tauri/Vite
surface:

The current source of truth for implementation status is
`2026-07-10-product-model-and-data-boundaries.md`. The original audit entries
below deliberately remain as historical evidence of the first mock pass; do
not use their old fixture references to decide whether a control is live.

| Surface | Current UI status | Current driver | Native work still required |
|---|---|---|---|
| Project directory | Main/worktree identity, detected stack, project selection, and a staged Add Project flow are present. | React project fixtures | Discover roots, worktrees, branch, config, and index store from the Rust project registry. Persist newly added projects. |
| Scan | Scope selector, category drilldown, finding evidence, rule-policy jump, and run state are present. | `reportAdapter.ts`, typed as generated `UiReportPayload` | Invoke the scan/check command and replace fixture reports with returned payloads and run history. |
| Rules | Catalog parses canonical `rules/rules.json` (578 rules), filters Universal/Project stack/Overrides/All, and has a detail drawer plus waiver-required disable flow. | `ruleCatalog.ts` plus local override state | Read/write `SettingsViewPayload` and `ToggleRuleRequest`; add descriptions/fixtures from native metadata. |
| Policy | Project config source, detected stack, effective override summary, and rule-catalog handoff are present. | Project fixture plus local overrides | Read `.enforce/config`, show native ties and exemption sources, persist policy writes. |
| Settings | Project-scoped Scan scope, ignored paths, assistant/generated source flags, worktree-index setting, index status, and source connections are present. | Local session state | Persist exclusions and settings; invoke native index refresh and use real connection health. |
| Proofs | Existing proof workspace remains fixture-backed. | `appData.proofs` | Connect proof inventory, run/finding/rule relations, detail preview, and artifact opening. |
| Graph | Project-scoped graph with node-kind filters, selected-node snippet/links, related-object navigation, and no Hub lane nodes. | `graphAdapter.ts` staged `ProjectGraph` shape | Query the selected `enforcer-memory`/codebase-memory index; add real file navigation and graph refresh. |
| Graph Chat | Dedicated page returns cited graph-object cards and returns to the graph. | `graphAdapter.ts` staged answer | Invoke graph/RAG query service, retain query history, and map citations to native node ids. |
| Hub | Lanes, Inbox, Claims, and Workers remain separate from Project state; the typed ledger fold is live. Message dispatch and acknowledgement append Rust hash-chained events and refresh the fold. | `enforcer-ui` hub fold + `enforcer-coordination` message/ack API | Add claim, lane lifecycle, and finding-to-fix dispatch without falling back to legacy JavaScript writes. |

The frontend still must not claim a native action completed when it has only
changed local React state. The adapters above intentionally make each future
Rust command boundary explicit.

## Current Truth

- App shell lives in `crates/enforcer-ui/frontend`.
- Tauri shell lives in `crates/enforcer-ui/frontend/src-tauri`.
- Project, Hub, and proof fixtures remain in `crates/enforcer-ui/frontend/src/data/enforcerAppData.ts`.
- Rules are now read from the canonical `rules/rules.json` through `src/data/ruleCatalog.ts`.
- Scan fixtures are shaped as generated `UiReportPayload` through `src/data/reportAdapter.ts`.
- Graph and graph-chat fixtures use the staged `ProjectGraph` contract in `src/data/graphAdapter.ts`.
- The Tauri command surface includes live scan/cache, Hub read/message/ack,
  project settings, rule writes, root inspection/registration, Store graph,
  retrieval, and memory index commands. The product-model document records
  which remain staged or partial.
- Generated Rust-to-TypeScript bindings exist in `crates/enforcer-ui/frontend/src/bindings`, but the app is not yet driven by those bindings.
- Real UI payload code already exists in Rust:
  - scan report payload: `crates/enforcer-ui/src/payload.rs`
  - settings read payload: `crates/enforcer-ui/src/settings/read.rs`
  - settings write/toggle route model: `crates/enforcer-ui/src/settings/write.rs`
- Rule registry data exists and must drive the rule UI:
  - canonical registry: `rules/rules.json`
  - source policy descriptions: `src/source-policy-rules.json`
  - Rust common registry: `crates/enforcer-lang-common/src/registry.rs`
- Current `rules/rules.json` snapshot has 578 rules:
  - `common`: 270
  - `rust`: 166
  - `typescript`: 73
  - `python`: 61
  - `iac`: 8
- The UI must scale beyond this snapshot. A project should not show every language family by default. It should show universal/common rules plus rules for detected project languages and enabled profiles.

## Snapshot Index

- [Projects](snapshots/project-projects.png)
- [Scan](snapshots/project-scan.png)
- [Rules](snapshots/project-rules.png)
- [Doctrine](snapshots/project-doctrine.png)
- [Proofs](snapshots/project-proofs.png)
- [Graph](snapshots/project-graph.png)
- [Graph Chat](snapshots/project-graph-chat.png)
- [Hub](snapshots/hub-lane-hub.png)

## Feature Inventory

| Feature | Real source or likely driver | Current UI location | Current driver | Current problem | Target UI |
|---|---|---|---|---|---|
| Connected projects and worktrees | Needs project discovery/store. Current mock only. Should distinguish repo identity, main root, worktree root, branch, graph store, index id. | Projects | `appData.projects` mock | Shows roots, but no real discovery, no detected stack, no index store id, no last scan. | Project directory grouped by main repos and worktrees. Card opens selected project workspace. |
| Active project context | Project selection state in `App.tsx`; later real project store. | Project-specific header | React state only | Header appears on project pages, but not yet data-backed. | Dropdown plus chips: kind, branch, worktree, detected stack, index status, last scan. No header on Projects or Hub. |
| Scan/check | MCP/CLI `ocentra_enforcer_scan`, `ocentra_enforcer_check`; UI payload `UiReportPayload`. | Scan | mock findings | No real run, scope, command, stdout, status, or report source. | Run controls by scope, named check, files/diff/workspace, report rows from `UiReportPayload`. |
| Findings by category | `UiReportPayload` plus rule metadata and category mapping. | Scan | mock category reduce | Category cards exist but not tied to real rules/report data. | Category drilldown: severity, language, family, file, owner, waiver state, proof state. |
| Rule catalog | `rules/rules.json`, `src/source-policy-rules.json`, Rust registries/parity tests. | Rules | mock families and six mock named rules | Shows Rust/Python/IaC even if selected project may not use them. Named rules are not loaded from registry. | Data-driven catalog filtered by selected project stack: Universal/Common + project languages + enabled profiles. |
| Rule detail and explanation | Rule registry doc/doc link, snippet, lockLevel, appliesTo, validator, triggers, doctrine metadata. | Rules right side is generic selected policy | no real driver | No info drawer, no hover/click explanation, no distinction between doctrine vs industry/safety rule. | Rule detail drawer: why, examples, appliesTo, validator, source doc, doctrine owner, industry/default label, fixtures, current project override. |
| Rule toggle/waiver | `SettingsViewPayload`, `ToggleRuleRequest`, `.enforce/config` policy.ruleToggles. | Rules toggle icon, Doctrine page | visual mock | Toggle has no waiver flow. No owner/reason/scope/expiry/proof requirement. | Toggle flow opens waiver/config drawer. Disable requires owner, reason, scope, optional expiry, proof link. Severity override visible. |
| Doctrine/profile | `enforcer-config` policy plus project profile config. | Doctrine, Rules right panel | mock dropdowns | Duplicated and underspecified. Does not show what changes when doctrine changes. | Project policy/Profile page or drawer attached to Rules. Show effect preview: errors -> warnings/allowed for affected rules. |
| Ignored roots/exemptions | `Policy.exempt_globs`, owner globs, allow regex, test path globs. | Doctrine mock | mock | `.claude ignored` is arbitrary and not tied to config. | Settings/Profile section: scan scope/exemptions with explicit glob list, owner, reason, source path. |
| Proof ledger | MCP proof tools, proof artifacts under `proof/**`, proof inventory/status. | Proofs, Scan detail artifacts | mock proof rows | Thin list. No run/finding/project/claim relation. | Proof ledger with filters by project, run, rule, lane, status. Click artifact opens details. |
| Graph explorer | `crates/enforcer-memory`, codebase-memory graph DB/API, graph UI pattern from `E:/codebase-memory-mcp/graph-ui`. | Graph | mock SVG nodes | Not real graph, mock counts, no code snippets, no edge filters from graph DB. | Real graph scene or dense 2D graph: nodes/edges from selected project index, filters, selected node detail, code snippet, rule/finding/proof links. |
| Graph chat | Same graph/RAG backend. | Graph Chat and Graph right panel | mock canned answers | Duplicate of Graph side panel and no citations. | One graph chat experience with cited node cards, jump-to-node, proof links, and query history. |
| Hub/lane coordination | `crates/enforcer-coordination`, MCP coordination tools, ledger root. | Hub tab | mock hub lanes/inbox | Correctly separated from Project now, but sparse and not live. | Hub command center: all projects, lanes, claims, inbox, threads, workers, worktrees, mail composer, health, stale locks. |
| MCP tool surface | `crates/enforcer-mcp/src/registry.rs` canonical tools. | Not explicit, only top scan button and hub mock | none | UI does not show available tool/action surface. | Contextual actions and command palette: scan, check, proof, route, run, last failure, coordination actions. |
| Native tool ties | `SettingsViewPayload.native_ties` for cargo, tsc, ruff, dart, cflint. | Not shown | none | Missing. | Project Settings/Profile: native tool mode/scope per selected project. |
| Installer/onboarding | `crates/enforcer-install`, adapters for Codex/Claude/etc. | Add Project button mock | none | Missing flow. | Add Project wizard: choose root, detect repo/worktrees/languages/config, install hooks/MCP if requested. |

## Page Audit

### Projects

Snapshot: [Projects](snapshots/project-projects.png)

Purpose:
- Show all connected roots Enforcer knows about.
- Distinguish main repositories from worktrees.
- Let user select/open a project workspace.
- Show enough health to know whether scan/index/graph data is current.

Current good:
- Left rail has `Project | Hub` tabs.
- Projects page no longer repeats the active project header.
- Page groups `Main repositories` and `Worktrees`.
- Worktree card shows its main root.

Current fuckups:
- Cards are still mock data.
- `READY`, `STALE`, `MISSING` bars have no explanation or source.
- No detected language/stack chips. This is essential because all downstream Rules/Scan views depend on stack.
- No last scan, last index, graph store, config source, or proof health.
- `Add project` is present but no flow.
- Card click jumps to Scan, but the card does not say "Open project" or show what will happen.

Target:
- Card fields: project name, root, kind (`main`/`worktree`), main root if worktree, branch/worktree, detected languages, config source, index status, last scan, finding count, proof status.
- Card actions: Open, Refresh index, Scan, Configure.
- Do not show universal settings here. Projects is a directory, not the settings page.

### Scan

Snapshot: [Scan](snapshots/project-scan.png)

Purpose:
- Run a scan/check for the selected project.
- Show violations grouped by broken category.
- Drill from category -> rule -> finding -> file/snippet -> action/proof.

Current good:
- Broken category panel is the right shape.
- Finding detail panel exists.
- Selected path graph hints at project -> rule -> finding -> proof.
- The live packaged scan report has text, severity, and state filters plus a transparent by-file/by-rule prioritization view derived from the loaded findings.

Current fuckups:
- Current scan data is a packaged-command desktop cache, not a canonical typed Rust `Report` history.
- The desktop runs only the packaged workspace scan; diff/files/named-check/profile/base-head execution boundaries are not implemented here.
- Test-doctrine and UI/business-logic coupling reports have richer, distinct evidence models and need a discriminated typed analysis-run contract rather than being flattened into ordinary findings.
- `Tune rules` belongs to rule policy/selected rule context, not a generic scan table button unless it opens exactly that rule.
- Lane pill in top bar is harness state mixed into project scan state. It may be useful, but should be visually secondary or a small status chip, not a core scan parameter.
- Proof artifact list is mock and not linked to actual proof files.

Target:
- Top scan controls: scope, named check/profile, include/exclude, base/head if diff.
- Category panel populated from real report rows plus rule metadata.
- Finding table rows from `UiReportPayload`.
- Detail panel includes code snippet, rule info, waiver/action buttons, proof requirements, and exact command/run id.

### Rules

Snapshot: [Rules](snapshots/project-rules.png)

Purpose:
- Show the rule catalog that applies to the selected project.
- Let user understand a rule, why it exists, where it comes from, and whether it is configurable.
- Let user set project-specific rule overrides with strict waiver requirements.

Current good:
- Named numbered rule rows now exist.
- Family summary and language cards exist.
- Rule toggle concept is visible.

Current fuckups:
- It shows language families regardless of selected project. A project without Rust should not see Rust source discipline in its primary project rule list.
- Language cards are static. They should come from detected project stack plus universal/common rules.
- Family table and named rules are disconnected. Clicking a family should filter named rules.
- Toggle icon has no flow and no distinction between locked, profiled, advisory, disabled, severity override, or waived.
- Right panel says "Doctrine turns errors into decisions" but is not tied to selected rule or selected family.
- No info badge/drawer for "why is this a rule", "is this Sujan doctrine", "industry default", "security required", "harness invariant", "project profile".

Target:
- Rule page layout should be faceted, not only table:
  - Tabs or segmented filters: Universal, Detected languages, All, Changed by project.
  - Facets: language, family, severity, status, source, doctrine, configurable, fixture coverage.
  - Family list controls the numbered rule table.
- Each rule row should have:
  - Rule id, title, short definition.
  - Severity and effective severity.
  - Source label: universal, language, security, doctrine, project override.
  - Info button/drawer.
  - Toggle/override button only when allowed.
- Rule detail drawer:
  - Why this rule exists.
  - What it catches.
  - Examples/pass/fail fixture links.
  - AppliesTo and triggers from `rules/rules.json`.
  - Validator and doc link.
  - Whether this is doctrine vs industry/security/harness.
  - Current project override and waiver history.

Data model:
- Load rule catalog from `rules/rules.json`.
- Join descriptions from `src/source-policy-rules.json` when available.
- Join project toggles from `SettingsViewPayload.rule_toggles`.
- Join detected stack from project scan/index metadata.
- Common/universal rules should always be visible. Language rules should be visible only for detected languages by default, with an "All languages" escape hatch.

### Doctrine

Snapshot: [Doctrine](snapshots/project-doctrine.png)

Purpose:
- Manage project policy/profile decisions that change rule classification.

Current good:
- Shows schema style, ignored roots, waivers.
- Shows category policy idea.

Current fuckups:
- Page is too generic and duplicates the Rules right panel.
- It does not show the actual config source or changed rules.
- It does not explain impact. "Zod allowed" should show which rules become warning/allowed.
- "Ignored roots" is arbitrary mock data and not linked to `exempt_globs`.
- Waivers are policy state, not a dropdown label.

Target:
- Either merge Doctrine into Project Settings/Rules policy, or rename to `Policy`.
- Show source config file, effective profile, changed rules, exemptions, owner globs, allow regex, native ties.
- Every doctrine toggle must preview impact and write through typed config.

### Proofs

Snapshot: [Proofs](snapshots/project-proofs.png)

Purpose:
- Show proof artifacts and proof gaps for selected project/run/rule/lane.

Current good:
- Simple status list exists.

Current fuckups:
- It is a thin table with no relationship to selected scan, selected finding, rule, lane, run, or artifact content.
- No filters.
- No evidence preview.
- No "why proof is required" or "what blocks DONE".

Target:
- Proof ledger with filters: project, run, rule, lane, status.
- Rows: proof id/name, status, path, created time, producing tool, related finding/rule, lane/thread.
- Detail drawer: artifact content preview, validation status, stale/missing reason, open file.

### Graph

Snapshot: [Graph](snapshots/project-graph.png)

Purpose:
- Explore selected project's code graph, rules, findings, proofs, and RAG context.

Current good:
- The selected project supplies a real persisted `enforcer-memory` graph projection, including indexed node kinds and `defines`/`calls`/`imports` edges.
- The graph has node and edge facets, selected-node links, a project-relative source excerpt, and a separate deterministic retrieval tab.
- A small controlled index shows labels by default; labels still suppress automatically when the rendered projection is large.

Current fuckups:
- The SVG view is intentionally bounded and needs GPU/LOD rendering plus neighbor expansion before it can serve a very large repository in the style of the original codebase-memory graph UI.
- Source excerpts are read-only context, not an editor or file jump contract.
- Cross-project graphing is not implemented and must stay opt-in so Hub coordination is not mixed into a project code graph.
- Retrieval is deterministic BM25, not graph chat with cited synthesis; semantic fusion, reranking, and context-pack explanations remain X06 delivery work.

Target:
- Use real graph data from `enforcer-memory`/codebase-memory index.
- Graph facets from actual node/edge labels.
- Selected node detail: file path, code snippet, inbound/outbound grouped edges, related rules/findings/proofs.
- Cross-project graph only when user enables it.
- Graph Chat should be integrated with citations/jump cards.

### Graph Chat

Snapshot: [Graph Chat](snapshots/project-graph-chat.png)

Purpose:
- Ask questions against selected project graph/proofs/rules.

Current good:
- Full-screen chat surface exists.

Current fuckups:
- Canned answers only.
- Duplicates Graph side panel.
- No citations, source nodes, or jump targets.
- No indication of selected graph index freshness.

Target:
- Keep either:
  - Graph page with embedded chat drawer, or
  - Full Graph Chat page with cited answer cards and jump targets.
- Answers must cite graph nodes, file snippets, proof artifacts, and rule ids.

### Hub

Snapshot: [Hub](snapshots/hub-lane-hub.png)

Purpose:
- Harness coordination across Codex/Claude/workers/projects.
- This is not project inventory and not project scan.

Current good:
- Correctly separated under `Project | Hub`.
- Project header is hidden.
- Shows all-project context, lanes, inbox.

Current fuckups:
- Mock data only.
- Lane rows are too thin: no worktree root, branch, claims, locks, stale age, thread link, unread count.
- Inbox rows are not actionable.
- No claim map, worker health, tasks, message composer, closeout/release/report actions.

Target:
- Hub subviews or internal tabs: Lanes, Inbox, Claims, Workers, Threads/Worktrees.
- Drive from `enforcer-coordination` API and MCP coordination tools.
- Let user filter by project, lane, worker, status.
- Show exact harness state under Enforcer ledger root, not product repo state.

## Missing Pages Or Surfaces

- Project Home / Overview after selecting a project:
  - detected stack
  - main/worktree identity
  - index health
  - last scan summary
  - config source
  - open blockers
  - proof gaps
- Project Settings / Policy:
  - native ties
  - exemptions
  - owner globs
  - rule toggles
  - waiver records
  - profile/doctrine decisions
- Rule detail drawer.
- Finding detail drawer with real code snippet and rule explanation.
- Run history / scan history.
- Add Project wizard.
- Index manager / graph store manager.
- MCP/command palette for available tools and actions.
- Hub Inbox detail / message composer.
- Hub Claims map.

## Target Navigation

Left side remains domain switch:

- `Project`
  - Projects
  - Scan
  - Rules
  - Policy or Doctrine
  - Proofs
  - Graph
  - Graph Chat, unless merged into Graph
- `Hub`
  - Lane Hub
  - later: Inbox, Claims, Workers, Threads

Project-specific pages should show the top project context header:

- project dropdown
- kind: main/worktree/external
- root or worktree root
- branch/worktree
- detected languages
- index status
- last scan status

Projects and Hub should not show that header.

## Data-Driven Design Requirements

Do not hand-code rule/family cards in React.

Required adapters:

- `ProjectSummary[]`
  - source TBD: project registry/store, config discovery, worktree detection.
  - includes detected stack, main/worktree relation, index health, graph store id.
- `RuleCatalog`
  - source: `rules/rules.json` plus metadata joins.
  - filter by selected project detected stack.
- `SettingsViewPayload`
  - source: `crates/enforcer-ui/src/settings/read.rs`.
  - drives native ties and explicit rule toggles.
- `ToggleRuleRequest`
  - source: `crates/enforcer-ui/src/settings/write.rs`.
  - drives rule enable/disable/severity/waiver writes.
- `UiReportPayload`
  - source: `crates/enforcer-ui/src/payload.rs`.
  - drives scan/finding/category UI.
- `AnalysisRun`
  - source: `crates/enforcer-ui/frontend/src-tauri/src/main.rs` via the
    versioned `scripts/desktop-analysis.mjs` bridge.
  - drives only the selected-project Test doctrine and UI boundaries views.
    It is not a `UiReportPayload`, and its heuristic evidence must not be
    turned into a scan violation.
- `HarnessRunPayload` and `HarnessRunDetailPayload`
  - source: `enforcer-harness` query APIs through
    `crates/enforcer-ui/frontend/src-tauri/src/main.rs`.
  - drives the selected-project Runs view: native execution records,
    diagnostics, and bounded redacted artifacts. It is not Scan history or
    Proof evidence.
- `ProofInventory`
  - source: MCP proof tools and artifact files.
  - drives Proofs page and finding detail proof strip.
- `HubState`
  - source: `crates/enforcer-coordination` and MCP coordination tools.
  - drives Hub.
- `GraphOverview`
  - source: `crates/enforcer-memory`/codebase-memory graph store.
  - drives Graph and Graph Chat.

## Implementation Notes For Next Agent

1. Keep `Project | Hub` tabs.
2. Add a typed UI adapter layer under `crates/enforcer-ui/frontend/src/data` or `src/adapters`.
3. Replace `appData.ruleFamilies` and `appData.ruleRows` with parsed data from a generated/static rule catalog fixture first.
4. Add selected-project `detectedLanguages` mock field before real detection. Use it immediately to filter Rules. This will prevent Rust/Python/IaC showing on a TypeScript-only project.
5. Split rule catalog into:
   - Universal/Common
   - Detected language tabs
   - All rules
   - Project overrides
6. Add rule detail drawer with info button.
7. Turn rule toggle into a real staged flow:
   - enable/disable
   - severity override
   - waiver owner/reason/scope/expiry/proof link when disabling
8. Keep Scan rows on `UiReportPayload`; specialized test-doctrine and
   UI/business-logic coupling reports use their own discriminated
   `AnalysisRun` contract and explicit run action.
9. Replace Doctrine page with Policy/Settings or make it clearly a project policy editor.
10. Keep Hub data separate from Project data. Hub is harness/codex/claude/worker coordination across projects.

## Short Verdict

The current UI is now better structured than the first pass, but it is still
mostly mock layout. The highest-priority fix is not more decoration. It is
data-driven product modeling:

- project identity and detected stack
- rule catalog filtered by project stack and common/universal rules
- typed rule/settings toggles with waiver flow
- real scan report payload
- real hub state
- real graph/proof sources

Once those exist, the UI can become intuitive because each screen will answer a
real user question instead of displaying arbitrary cards.
