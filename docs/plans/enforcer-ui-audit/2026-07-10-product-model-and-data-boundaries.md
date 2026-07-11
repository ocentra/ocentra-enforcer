# Enforcer Desktop Product Model And Data Boundaries

## Decision

Enforcer is a Rust enforcement engine with an optional Tauri human control plane.
The desktop application is not a generic dashboard and it is not a single graph
application. It has two explicit areas: `Project` and `Hub`.

`Project` is selected once. All project-scoped screens use that selection; they
must not duplicate a project inventory sidebar or force a second project choice.
`Hub` is harness coordination across projects and therefore has no project
selection in its primary navigation.

## Project Area

| Surface | User question | Rust source of truth | UI responsibility |
|---|---|---|---|
| Projects | Which repository roots and worktrees are connected? | registered roots plus project discovery/index status | inventory, root/worktree identity, explicit index lifecycle |
| Project Overview | What can I do with the selected root and what is already loaded? | selected project registration plus the current cached scan summary | one selected-project control map into Scan, Rules, Policy, Proofs, Memory, Analysis, Assurance, and Runs; each route states its real boundary |
| Scan | What failed in this selected project? | `.enforce/` typed `Report` from `enforcer-scan` | report history, category/rule/file grouping, occurrence inspection |
| Project Analysis | What test posture or presentation-boundary signals deserve review? | typed Rust desktop boundary over named legacy analysis reports | explicit, focused report runs; category/gap and boundary-evidence inspection without creating scan violations |
| Harness Runs | Which native commands ran, failed, or produced diagnostics? | `enforcer-harness` NDJSON query/read model | read-only run history, last failure, diagnostics, redacted bounded artifacts, and retention provenance |
| Rule Actions | What can I do with this finding? | `enforcer-config` waiver records and `enforcer-coordination` fix intent | fix, defer, comment, or named waiver with owner/reason/expiry |
| Rules and Skills | What is this rule and why does it exist? | structured `enforcer-rules` registry plus skill corpus | language/family filters, rule id, tier, description, fail/pass examples, framework mapping |
| Doctrine and Settings | Which policies apply here? | typed `enforcer-config` control plane | project enablement, language applicability, profile, rule severity, explicit waiver authoring |
| Proofs | What evidence supports this result or claim? | `enforcer-proof` journal and report artifacts | freshness, command, scope, source location, result, linked report/intent |
| Memory Explorer | What does x06 know and why did it answer this way? | `enforcer-memory` Store projections and `proof/memory/**` | read-only code intelligence, retrieval explanation, learning evidence, model state, parity evidence |

The Scan screen is the only place that presents violations as the primary object.
Rules and Doctrine do not render as a graph. They link to a finding only after a
user selects that finding.

Project Analysis is separate from Scan. `test-doctrine` is a heuristic test
posture and CI-gate report; `ui-logic-coupling` is an ARCH-1.16 import/call
signal report. Neither result creates a rule violation, waiver, Hub claim, or
proof. Their current Node implementations cross a versioned Rust desktop
boundary and are explicitly marked partial until native analyzers, persisted
analysis runs, and CI execution envelopes exist.

Harness Runs is separate from both Scan and Proofs. A run is evidence that a
native tool command executed and recorded an outcome; it does not establish a
proof claim and it does not create a scan violation. The current desktop reads
the typed run store, including compatibility discovery of legacy storage roots,
and exposes no execution, pin, prune, reset, or repair write action.

## Finding Resolution Roadmap

The Scan screen may currently route a finding to project-wide rule policy or
to a manual Hub claim. Those are different actions: a global rule toggle is
not a single-finding waiver, and a Hub claim is not proof that a finding was
fixed. The desktop must retain those labels until these engine records land.

1. `a08`: Rust-owned, path-scoped waiver registry with stable record ID,
   owner, issue, reason, expiry, CI visibility, and narrow file scope.
2. `g03`: selected-finding `Waive` and `Defer` records keyed by stable finding
   identity. `Fix` and `Add comment` remain unavailable until their mutation
   contracts exist.
3. `g04`: validated/deduplicated `FixIntent` tied to a resolution record and
   then to a claimable Hub task; claim and closeout state returns to the row.
4. `g07`: loopback/token/origin authorization gates before any Tauri mutation
   or dispatch control is offered.

The existing coordination fix loop is a future execution guard, not a desktop
action: it has no human intent, agent identity, token gate, or finding lifecycle
record today.

## Hub Area

The Hub is a separate, cross-project harness surface. It renders the live typed
coordination ledger: lanes, claims, leases, tasks, workers, mail, and sync state.
It can append lane-addressed messages, acknowledgements, and exact-path claims
through the Rust hash-chained coordination API; new-lane lifecycle and
finding-to-fix dispatch remain unavailable. It never pretends that lane activity is project
inventory or scan output.

## Memory Explorer (g09)

The Memory Explorer is one read-only x06 console with separate tools. A graph is
only one tool inside it.

| Tool | Answers | Does not contain |
|---|---|---|
| Code graph | How are files, modules, symbols, types, calls, imports, routes, traces, and cross-repo links connected? | rule controls, doctrine controls, scan rows |
| Ask graph | Which indexed source symbols and paths match this question, and what focused code-graph context supports them? | a fabricated chat answer, hand-written TS ranking, or false citations; current behavior is source retrieval plus Rust-backed focused graph inspection, not answer synthesis |
| Retrieval explanation | Why did BM25/vector/RRF/reranking choose this context pack? | a generic chat answer without candidates, scores, source refs, and capability state |
| Learning evidence | What observation, landed lesson/artifact, clean scan, recurrence, or procedural outcome proves learning? | a claim that the system learned without t0/t1/t2 evidence |
| Model health | Which local embedding/reranker/provider capability is available, degraded, or missing? | passive model downloads or model process startup |
| Parity | Which codebase-memory tool rows are equal, better, worse, or incomparable, and why? | an overall "better" badge detached from rows and fixtures |

The code graph may link a selected symbol to a supporting lesson or proof in a
detail/context view when the Store has a real edge. It must not combine rules or
doctrine into the topology just to make the graph look busy.

## Data Boundary Rules

1. Rust produces every data payload. TypeScript stores only local presentation
   state such as the active tab, selected node, open group, and typed query text.
2. The x06 canonical source is Store append logs plus manifest. CodeGraph,
   MemoryGraph, SQLite operational graph, retrieval documents, and learning
   curves are derived projections.
3. The Memory Explorer opens an existing Store projection from
   `.enforce/memory`. A missing Store is a stable, explicit unindexed state. It
   never silently creates a store, rebuilds an index during passive render, or
   starts model downloads.
4. Scan UI reads a persisted report. It never manufactures a report from the
   screen model and it does not re-run a scan during passive render. Until the
   canonical typed Rust `Report` lands, the desktop may read its clearly named
   packaged-scanner cache only.
5. Every configuration write routes through `enforcer-config`; every fix
   dispatch routes through `enforcer-coordination`; every proof is read from
   `enforcer-proof`.
6. Specialized analysis reports use a discriminated `AnalysisRun` payload.
   They must not be flattened into `UiReportPayload` or presented as ordinary
   scan findings.

## Current Truth (2026-07-10)

The desktop Engine page mirrors this implementation map so product-level
capability state is visible without conflating it with a selected project's
findings, settings, or the cross-project Hub ledger.

| Surface | Current state | Required correction |
|---|---|---|
| Native desktop shell | Tauri has Store-backed graph commands, a packaged Enforcer scan command, typed Hub/settings readers, and read-only Rust harness discovery. | Package the scanner command as a desktop resource and replace the temporary workspace-relative lookup. |
| Engine capability map | The desktop loads a Rust-owned catalog covering Project, Intelligence, Harness, Assurance, and Planning. Every row names its current source, exposed desktop control, missing Rust boundary, workpack IDs, and relevant workspace when one exists. | Keep this catalog synchronized with engine delivery; planned rows must not gain a desktop action until their Rust boundary exists. |
| Workpack routing | The Engine workspace reads the authoritative `WORKPACK_INDEX.md` through a Rust parser and exposes declared status, ownership, dependencies, tier, parallel frontier, and source document. Its local placement filter separates declared workpacks referenced by the Rust-owned capability map from workpacks with no current capability placement; the latter means only that the desktop has no mapped product surface. | Keep document-declared status distinct from repository execution and proof truth; add an engine-owned workpack/proof read model when that boundary lands. |
| Graph | A real Store-backed CodeGraph projection is rendered after explicit indexing. The graph is bounded and uses an SVG layout. On a capped graph, a user may request a focused Rust projection by indexed file path, symbol name, or call name; it includes matching source files and their existing indexed symbols without claiming a full-repository render. | Add GPU/LOD rendering, neighbour expansion, semantic/ranked graph focus, and trace queries before treating it as large-repository ready. |
| Scan | The fixture adapter is removed. The desktop invokes the current packaged Enforcer command with typed workspace, discovered Cargo package, bounded Rust-discovered top-level project directories or validated project-relative files, or verified Git diff scope; directory choices become ordinary `--files` inputs and exclude hidden, dependency, build, and Enforcer-state directories. It persists a current per-project `desktop-scan-report.json` cache plus individually readable desktop-cache snapshots under `.enforce/ui/scan-runs/`, and renders its real category -> rule -> occurrence output. Its Findings view has local text/severity/state filters and a transparent priority view grouping the loaded report by file and rule. The desktop history is explicitly labeled as packaged-command evidence, not canonical Rust Report history. | Persist the canonical typed Rust `Report`, add generic domain and named-check targets, support engine-owned report history, then retire the packaged command bridge and desktop cache. |
| Project analysis | A selected-project Analysis workspace explicitly runs `test-doctrine` and `ui-logic-coupling` through `scripts/desktop-analysis.mjs`. Rust validates a versioned, discriminated payload and the view renders test gaps/CI evidence or ARCH-1.16 boundary evidence without relabeling it as Scan output. The bridge is exercised against the controlled fixture. | Replace Node-backed analyzers with Rust-owned contracts; persist immutable analysis-run history with source revision/provenance, and add CI-grade execution envelopes before using it for gating. |
| Harness runs | A selected-project Runs workspace reads the `enforcer-harness` run store through typed Rust query APIs. It shows newest-first run records, latest failure data when present, diagnostics, and bounded redacted stdout/stderr only after a run is selected. | Add desktop command execution, pin/prune/reset confirmation flows, CI-run ingestion, and run-to-proof/linkage only after their typed write and provenance contracts exist. |
| Rules | The frontend imports the canonical `rules/rules.json` catalog and applies explicit typed project toggles from `enforce.config.json`; it shows numbered rules, registry lock level, validators, fixture contracts, and doc anchors. Rust project inspection now uses the public `enforcer-literal-scan` registry over a bounded, ignored-tree-aware filesystem walk plus manifest hints. The UI distinguishes that broader observed stack from the policy-backed `common`, Rust, TypeScript, Python, and IaC catalog families. | Add broader rule applicability, fixture/example payloads, and waiver history through engine-owned contracts. Observing a language is not policy coverage for that language. |
| Doctrine/settings | Rule toggles use typed `enforce.config.json` settings. Scan scope reads and writes the separate canonical `ocentra-enforcer.config.json`, validates it through `enforcer-config`, and preserves unrelated project fields. | Add named waiver expiry and richer exemption records without session-only fallback. |
| Harness adapters | `Hub -> Adapters` renders `enforcer-install`'s user-level detection of all known harness homes, evidence, and declared capability manifests. It is explicitly not selected-project configuration or an adapter-health assertion. | Wire adapter-level verification, hook installation, repair, and desktop onboarding only after their write/verification contracts are available. |
| Assurance | `enforcer-security` contains Rust money-critical and threat-map validators, while `profiles/money-critical-security.json` defines a neutral profile with backed rules, test categories, and invariants. The selected-project Assurance surface reads that profile through Rust and provides an explicit, typed activation-intent form; it clearly shows that activation is not coverage. | Add selected-project profile ingestion, runtime security report/proof payloads, threat-to-invariant-to-test evidence, and CI execution visibility before offering remediation controls or a coverage score. |

### Required Assurance Activation Bridge

The existing `enforcer-config::resolve` contract is intentionally not the
activation mechanism for `money-critical-security`: it resolves scanner and
harness configuration, while the neutral security profile is a distinct model
of rules, required test categories, and invariants. The desktop must not merge
those JSON shapes or write a profile name into `enforce.config.json` and claim
the scanner now enforces it.

The missing Rust boundary is a typed project-security control plane with:

1. A project-local, versioned activation record naming a security profile and
   its source specification, owned by `enforcer-security` rather than the UI.
2. A resolver that returns profile availability, activation state, backed and
   unbacked asserted rules, required categories, invariants, and source refs.
3. An explicit write request that validates the profile name and source path,
   records ownership/reason, and rejects unknown or unbacked claims.
4. Scan/proof integration that emits runtime findings and evidence references
   for the active profile. Only this step can move the Assurance screen from
   `available` to `covered` or `CI-gated`.

The first record boundary now exists as
`enforcer-security::activation`: it validates and persists a versioned
`.enforce/security-profile.json` record for `money-critical-security` with a
source specification, owner, and reason. The desktop exposes an explicit form
that writes this record only after the user supplies those fields. It must
provide no synthetic coverage score or remediation action until scan/proof
integration exists.
| Proofs | `enforcer-proof` now owns the `.enforce/proofs` project layout and read model. The desktop reads verify-on-open/replay journal state, parsed proof runs, declared-artifact presence, per-run commit freshness, and a real PR-ready claim only when the project supplies `proofs.json`. Files merely found under legacy `proof/` remain explicitly external/unverified. | Wire proof recording/routing into the CLI and desktop, add richer artifact digest/size verification in the reader, and support intentional profile-registry selection rather than guessing one. |
| Projects | Bundled root cards plus user-added roots persisted in `%APPDATA%/OcentraEnforcer/desktop-projects.json` expose live memory-index status, availability, Git branch, and compact observed-stack summaries. Registration is inspect-first: Rust resolves a requested root, derives primary versus linked-worktree topology from Git porcelain, branch, canonical root, primary root, bounded literal-registry language observation, and memory-index state before the user may write the desktop-local record. The walker skips symlinks and generated/dependency trees and stops at explicit directory/file bounds. An explicit Rust Git porcelain action discovers the selected repository's primary checkout and linked worktrees, then persists those registrations locally; no claim/finding totals are fabricated. The linked-worktree derivation and mixed-language ignored-tree behavior are covered by temporary fixture tests. | Replace desktop-local registration with an engine-owned connected-project/worktree-family discovery model. |
| Project Overview | Project inventory opens first. Selecting a registered root opens a bounded selected-project overview containing only registration facts and a loaded packaged-scan count when one exists; every workspace action is a real route, and state labels preserve read-only/partial/planned boundaries. | Add engine-owned project health aggregation only after it has a typed source; do not turn this route into a synthetic dashboard score. |
| Hub | The native desktop reads the typed `enforcer-ui` ledger fold for lanes, exact claims, latest task states, workers, messages, duplicate count, and parse warnings. It sends lane-addressed messages, acknowledgements, and user-confirmed exact-path claims through the Rust coordination API, then reloads the typed fold. | Add lane lifecycle, lease projection, and safe finding-to-fix execution dispatch; do not reintroduce static lane rows or a legacy JavaScript write path. |
| Memory/RAG/learning/model/parity | Memory Explorer reads the persisted X06 code graph with its typed symbol facets, a bounded pan/zoom SVG topology view, proof summaries, and deterministic BM25 retrieval. | Add GPU/LOD rendering for very large graphs, capability-gated semantic fusion, reranking, context-pack explanation, and model-backed synthesis without passive downloads. |

## Evidence Behind The Correction

The current `proof/memory/x06-kg-parity.json` assesses a focused set of backend
tool contracts. Its `equal`/`better` verdicts do not prove the Tauri frontend is
wired to those tools, visualizes an actual Store projection, or renders a scan
report. The previous UI implicitly claimed all three and was therefore
misleading.

The legacy HTML reports demonstrate the required report density: category and
rule totals, rule definitions, why/fix guidance, per-occurrence file/line/source
evidence, severity filters, and manual-verification status. That information
belongs in the Scan report payload and surface, not in the code graph.

## Controlled Fixture Evidence

The native desktop application starts against
`crates/enforcer-memory/tests/fixtures/memory/feature_parity/repo`, a two-file
Rust fixture with a caller/callee relationship. Its explicit memory index
created a Store projection with 2 files, 9 code nodes, and 5 edges. Its
packaged Enforcer scan produced 21 findings: 17 errors and 4 warnings. The
desktop persisted that result at
`.enforce/ui/desktop-scan-report.json` and can reload it without another scan.
The same controlled project carries one Rust proof run at
`.enforce/proofs/runs/fixture-proof-001/proof-run.json`, a declared local
artifact, a one-record SHA-256 journal at `.enforce/proofs/journal.ndjson`, and
a project-local `proofs.json` requiring that proof. Native desktop inspection
verified the journal, found the run and its `1/1` artifact, and reported the
configured claim as `ready`. The fixture is intentionally outside Git, so the
run's commit freshness is correctly shown as unavailable rather than current.
The same fixture also executes both specialized analysis contracts through the
desktop bridge: test doctrine returns typed coverage/CI posture categories and
UI-logic coupling returns the typed `ARCH-1.16` boundary report. The result is
useful report evidence, not a certification or a generic scan finding.
It also contains an actual harmless `node --version` harness record; the
desktop Run workspace reads this through `enforcer-harness`, displays the typed
summary and the bounded redacted stdout artifact, and keeps it distinct from
the scan snapshot and proof journal.
The worktree-discovery command is separately proven with a temporary Git fixture
that initializes a primary repository, commits a file, creates one linked
worktree, and asserts that Rust reports both roots and the linked branch. This
is the required development loop for desktop changes; do not begin with the full
Enforcer installation until the same interaction is proven here.

## Hub Compatibility Evidence

The live ledger at `E:/ocentra-enforcer/.ledger` uses camelCase event and
identity fields such as `nodeId`, `prevEventId`, and `defaultLane`. The Rust
coordination model now decodes and re-serializes that wire form, while its
legacy event fixture proves the recorded hash stays valid. In a native desktop
verification after that correction, the Hub folded 85 real events and rendered
the observed lanes. The composer was inspected with live lane recipients but
was not used to create a live coordination message; message/ack mutation is
proven by temporary-ledger Rust tests rather than writing test traffic to the
user's harness.

## Project Lifecycle Boundary

Desktop project registration is an inventory operation, not Enforcer onboarding.
Its current Rust-backed preview validates a directory, resolves the Git primary
or linked-worktree topology, detects the branch, observes bounded languages,
and reports index availability before a user can create a desktop-local record.
The UI now shows the entered path separately when it resolves to a different
canonical worktree root. It does not create `.enforce`, select a profile, write
a baseline, configure adapters, wire CI, or claim that any of those steps are
complete.

Git worktree discovery is also an explicit desktop-local registry mutation.
The Projects workspace identifies the selected discovery target and requires a
confirmation that says exactly what can be registered and what cannot change
(repository files, Git state, scans, and indexes). This is an interim safety
boundary, not a substitute for the missing `f02` onboarding flow. That flow
needs an engine-owned, idempotent Rust command with separately evidenced states
for install, inspect, configure, baseline, CI wiring, failing-case proof, and
clean-baseline proof. Discovery still lacks a Rust dry-run diff, per-worktree
selection, stale-root cleanup, relinking, and a persisted index-state update.

The desktop now places this lifecycle in one selected-project `Setup` surface.
It composes only existing facts: project registration/topology, typed scan-scope
configuration, typed rule policy settings, memory-index state, discovery-only
harness observations, the Rust proof read model, and the legacy Test Doctrine
CI-posture observation. A card may route to its existing surface only where
that surface already exists. The baseline and CI
cards are deliberately non-actionable and say `not implemented in Rust`; they
must not be changed to a progress score or a generic `connected` result until
the f02 and C11 commands persist their own lifecycle records.

The Rust-owned Engine capability map also exposes `Project setup and lifecycle`
as `partial`, linked to `f02`, `f03`, and `c11`. The Engine workpack inspector
therefore directs those workpacks to Setup in addition to any narrower existing
surface. Its declared workpack status remains independent from that placement;
a Setup route is not evidence that an authored workpack is complete.

The Engine workpack index is a large, read-only plan surface, so its text search
is local to the catalog and operates on the authored routing metadata (ID,
title, track, status, owns, dependencies, and parallel frontier). It neither
rewrites plan data nor upgrades a declared status; its purpose is to find a
workpack or the workpacks that depend on it without forcing a window-length
scroll through the plan.

## Graph Count Boundary

The Memory graph header reports the native Store's indexed node count and its
stored call/import/route edge count. The canvas reports separate projection
links because it also emits file-to-symbol `defines` relationships needed for
navigation. Both are real Rust-derived values, but they describe different
relationship sets and are therefore labeled separately in the desktop.

## Finding Action Boundary

The selected Scan finding is the only legitimate place to decide its immediate
next step. The desktop can create an explicit exact-path Hub claim after the
user chooses a live lane, and can open the project-wide rule-policy surface.
It cannot edit code, create a typed FixIntent, apply a finding-level waiver or
defer, invoke an agent, verify a change, record proof, or close a report row.
Those missing operations are shown beside the selected finding as a Rust-missing
fix lifecycle, rather than being hidden behind a fake `Fix` button or inferred
from a claim.
