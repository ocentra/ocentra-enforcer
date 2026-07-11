# Desktop Capability Coverage - 2026-07-10

## Purpose

This is the current product-placement matrix for the Rust desktop. It answers
three different questions without collapsing them:

1. What product capability is visible to a user?
2. Which Rust command or typed read model drives that surface today?
3. What remains missing, partial, or intentionally unavailable?

The matrix is not a release checklist and does not infer workpack completion
from a route. Engine -> Workpacks remains the authoritative desktop view of the
authored workpack index; proof and test artifacts remain the authority for
execution claims.

## Project Control Plane

| Product concern | Desktop placement | Current Rust boundary | Current truth and missing boundary |
|---|---|---|---|
| Connected repositories, primary roots, and linked worktrees | `Project -> Projects` | `load_desktop_projects`, `preview_desktop_project_registration`, `register_desktop_project`, `discover_desktop_project_worktrees` | Inspect-first, desktop-local inventory and Git topology are real. Discovery is user-confirmed. Engine-owned project family lifecycle, selection/diff before discovery, relink, cleanup, and persisted index updates are absent. |
| Selected-project identity | Project header outside project-local workspaces | Desktop project registry plus selected React state | The header switches a registered project and shows kind, branch, worktree, observed languages, and index state. It is intentionally absent from Projects, Engine, and Hub. |
| Project setup | `Project -> Setup` and Overview destination | Existing project registration, `load_project_settings`, `load_scan_scope_settings`, proof read model, and index state | A readiness map, not a score. Registration, scope, policy, index, proof state, and legacy CI posture are distinct cards. Global harness adapters route to Hub rather than becoming selected-project setup state. f02 onboarding/baseline and C11 CI lifecycle are explicitly not implemented in Rust. |
| Scanner scope and ignored paths | `Project -> Settings -> Scan scope` | `load_scan_scope_settings`, `write_scan_scope_settings` | Typed scanner configuration and ignore paths can be written. It is separate from rule policy and does not create an onboarding baseline. |
| Rule policy and native tool ties | `Project -> Policy` and `Project -> Rules` | `load_desktop_rule_catalog`, `load_project_settings`, `write_rule_override` | The canonical `rules/rules.json` registry is read through Rust; the frontend renders its typed payload, then applies project-wide overrides, severity changes, and owner/reason waivers. A resolved f03 enforcement tie, finding-level waiver/defer, expiry history, and impact preview are absent. |
| Code-memory index | `Project -> Settings -> Index` and `Project -> Memory` | `create_memory_index`, `load_graph` | Explicit index creation and persisted Store reads are real. Incremental refresh is deliberately unavailable. Index state alone is not a scan baseline. |

## Scan, Findings, and Change Control

| Product concern | Desktop placement | Current Rust boundary | Current truth and missing boundary |
|---|---|---|---|
| Run a scan | `Project -> Scan` | Packaged Enforcer scan command and typed workspace/package/path/diff target validation | One Run scan command accepts bounded scopes. Results are a desktop-cached packaged report; canonical Rust Report history remains absent. |
| Browse and prioritize violations | `Project -> Scan` | Loaded report payload and desktop scan history read model | Categories, severity/status filters, file/rule priority groups, source snippet, and rule guide are local-scroll surfaces. A scan result is not proof. |
| Decide a finding action | Selected Scan finding inspector | Hub handoff plus project-wide rule policy | A user can create an exact-path Hub claim after choosing a lane, or inspect project policy. FixIntent, agent pickup, code edits, finding-level waiver/defer, verification, proof recording, and report closeout are absent and visibly labeled. |
| Evidence and proof | `Project -> Proofs` | `load_project_proof_snapshot`, `list_proof_artifacts` | Journal replay, artifact presence, Git freshness, and PR-ready claim state are real read models. Recording/routing, full digest verification, and profile selection remain incomplete. |
| Focused analysis | `Project -> Analysis` | `run_legacy_analysis` | Test Doctrine and UI-logic coupling reports can run explicitly. They are legacy-bridge evidence, not scan findings, Rust-native history, or CI execution proof. |
| Harness run history | `Project -> Runs` | `load_harness_runs`, `load_harness_run_detail` | Read-only history, diagnostics, and bounded artifacts are real. Desktop run execution, pin/prune/reset, and CI ingestion are unavailable. |

## Intelligence, Assurance, and Harness

| Product concern | Desktop placement | Current Rust boundary | Current truth and missing boundary |
|---|---|---|---|
| Code graph | `Project -> Memory -> Code graph` | X06 Store `load_graph`, `load_graph_source_snippet` | Facets, relationship filters, selected-node source excerpts, focused projections, and bounded pan/zoom are real. Stored call/import/route edges are labeled separately from canvas projection links, which also include `defines`. GPU/LOD, expansion, and large-repository readiness are absent. |
| Graph retrieval | `Project -> Memory -> Ask graph` | deterministic persisted-graph search | BM25 retrieval returns source evidence and opens a focused graph. Semantic fusion, reranking, context packs, and model answer synthesis are not implemented. |
| Learning, models, and parity | `Project -> Memory -> Learning / Models / Parity` | X06 proof artifact summaries | These are read-only evidence surfaces. Passive model viewing does not download a model. Model execution, learning lifecycle navigation, and non-degraded runtime proof remain separate work. |
| Security policy | `Project -> Assurance` | `load_security_profile`, `activate_security_profile` | Profile, rules, categories, invariants, and activation intent are visible. Intent does not establish selected-project coverage, runtime findings, or CI gating. |
| Cross-project harness coordination | `Hub -> Lane Hub` | typed coordination ledger, message/ack/claim Rust APIs | Lanes, inbox, claims, tasks, workers, messages, acknowledgements, and exact-path claims are real. Lane lifecycle, leases, and safe code-fix dispatch are absent. Hub intentionally has no selected-project control. |
| Harness discovery | `Hub -> Adapters` | `load_harness_discovery` | User-level harness homes, source-path evidence, and declared capabilities are observed across projects. Installation, repair, adapter verification, and C11 onboarding are unavailable. |

## Engine and Plan Visibility

| Product concern | Desktop placement | Current Rust boundary | Current truth and missing boundary |
|---|---|---|---|
| Product capability map | `Project -> Engine -> Capabilities` | `load_engine_capabilities` | Rust owns domain/title/source/controls/missing/workpack metadata. The frontend may filter and route but cannot promote a partial/planned capability. |
| Workpack index | `Project -> Engine -> Workpacks` | `load_workpack_index` | All authored workpacks are visible with local text search, track, status, and placement filters. Search covers ID, title, ownership, dependencies, and frontier metadata. Declared plan status is not execution or proof truth. |
| Workpack capability mapping | Workpack detail panel | Rust capability map workpack references plus typed destination | A mapped workpack shows its capability's exact Project/Hub destination, optional subview, and project-context requirement. `f02`, `f03`, and `c11` additionally expose the partial Setup lifecycle route. `No map` means no current capability mapping; it does not prove missing or complete implementation. |

Current map audit: `49` of `114` authored workpacks have a capability mapping;
the remaining `65` are deliberately visible through `Engine -> Workpacks -> No
map` rather than silently omitted. Planned `Fix dispatch` now routes to the
Scan finding inspector, which exposes ownership handoff and the unavailable
lifecycle. Its typed Rust `FixIntent`, disposition, verification, proof, and
closeout contracts are still absent.

## Explicit Coverage Limits

- The UI does not claim 158 policy-covered languages. Current named policy
  families are limited to common, Rust, TypeScript, Python, and IaC; other
  observed languages appear as outside the policy registry.
- A desktop-local project record is not equivalent to a project being onboarded
  by the Enforcer engine.
- A Workpack `DONE` label is plan routing information until the associated
  proof/test source confirms the required behavior.
- No UI route may create a synthetic health or completion score from partial
  data sources.
