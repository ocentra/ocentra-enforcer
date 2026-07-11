# Native Window Interaction Audit - 2026-07-10

This audit is based on the running Tauri desktop at a 1536x972 native window,
using the controlled X06 fixture. It is an interaction audit, not a static
mock review. Every Project and Hub workspace was opened and inspected.

## Non-Negotiable Shell Rules

1. The application shell must not scroll horizontally or vertically. The
   sidebar, project context, workspace header, and workspace body are fixed
   regions inside the native window.
2. Only a workspace-owned region may scroll: a finding list, rule catalog,
   facet list, artifact list, editor, or detail inspector. Each region needs a
   stable height and `overflow: auto`.
3. A command appears once. The global Scan button is not a second Scan button
   on the Scan page. The Scan workspace owns its action and scope controls.
4. A visible control must have a real current effect. A disabled scope select
   presented as `Workspace` is not a scope feature.
5. Project context appears only in selected-project workspaces. Projects and
   Hub are separate roots and do not show the selected-project command bar.
6. Text in a header must name the current workspace or current scope. It must
   not describe an unavailable feature such as the existing top-bar search.

## Captured Surfaces

| Workspace | What the native window showed | Required correction |
|---|---|---|
| Projects | A compact inventory grouped by main repositories, worktrees, and external roots. No selected-project command bar. | Keep this model. Make card opening intent explicit and make health labels explain their source. |
| Engine | Capability map, metric strip, two segmented control rows, a long capability list, and a detail panel. The page itself scrolls. | Make the catalog panel scroll independently. Collapse metrics into a compact summary and retain one filter row. |
| Scan | Project bar contained an inert search and global Scan command. Scan repeated Run scan and showed a disabled `Workspace` scope select. Findings extended the page. | Remove the global Scan action. Show a target picker only when Rust discovers executable Cargo packages; otherwise use the supported workspace label. Bound category, history, findings, and detail panels independently. |
| Analysis | Global project bar repeats Scan. A workspace Run analysis command is correct, but the empty state consumes most of the page. | Keep a single Run analysis command. Use the empty region for recent analysis runs and compact report-kind descriptions. |
| Runs | Global project bar repeats Scan. The useful run table is below metrics and a long partial-capability notice. | Keep refresh as the workspace action. Make the run list and detail inspector fill the body, with the warning collapsed into an information affordance. |
| Rules | Facets and catalog have their own visual rails but the application frame still scrolls. The right detail is useful. | Preserve the three-panel shape. Set a fixed workspace grid so only the facet/catalog/detail areas scroll. Make a family selection filter the numbered list. |
| Policy | Effective project policy, detected stack, native tool ties, and rule changes are understandable. | Keep the page. Move long config path text into copyable metadata and retain a compact project header. |
| Settings | The settings rail is appropriate, but ignored paths are one long page list and the app frame scrolls. | The settings body and ignored-path list need local overflow. Keep the settings rail fixed. |
| Assurance | Activation form, test categories, and invariant lists appear together as a long document. | Split Profile, Test categories, and Invariants into tabs or a selected-list/detail layout. The activation form remains the only primary command. |
| Proofs | Compact proof ledger and proof-integration detail. The content fits but the shell still exposes page scrolling. | Retain the two-panel shape after shell overflow is fixed. |
| Memory | The graph workspace exposes both horizontal and vertical application scrollbars. Graph pan/zoom, facets, and evidence are otherwise correctly separate. | The graph canvas must fill a fixed body. Facets and evidence inspector get independent scroll. Never use browser/application scroll to traverse a graph. |
| Hub | Fixed harness context, ledger tabs, lane table, and health panel. It is the closest existing desktop layout. | Use Hub as the layout reference: fixed columns, tabbed local context, local scrolling, no project command bar. |

## Scan Decision

The Scan workspace is the first implementation slice because it demonstrates
all of the shell failures at once and is driven by the controlled fixture.

Target layout:

```
fixed app shell
  fixed project context (selected-project pages only)
  fixed Scan toolbar
    scope picker | target picker | filters | Run scan
  fixed three-panel body
    category and history rail (local scroll)
    findings table and priority tab (local scroll)
    selected finding inspector (local scroll)
```

The scope model must be honest:

- `Workspace` is always executable.
- A Rust Cargo workspace can expose `Crate` targets only after Rust discovers
  them with `cargo metadata`; the Tauri command accepts only a discovered target
  and invokes the packaged scanner with `--crate <package.name>`.
- `Files` accepts one or more validated project-relative file or directory
  paths. `Diff` accepts a validated `base` and `head` Git revision pair.
  Both share the same packaged scan command and desktop history as workspace
  and Cargo-package scans.
- Generic `Domain` remains absent until Rust supplies a target model and a
  matching executable scanner command.
- The fixture has no Cargo manifest. It must show Workspace only as a label,
  not an unusable dropdown.
- A Rust workspace should receive a target picker built from discovered Cargo
  packages and crates. The graph can enrich that picker with indexed module
  information, but graph data must not be required to run a scan.

## Delivery Order

1. Remove shell overflow and give every workspace a bounded body.
2. Remove duplicate Scan controls and the inert top-bar search from the
   selected-project command bar.
3. Rebuild Scan around one command point and an honest scope/target model.
4. Convert Engine, Rules, Settings, Assurance, and Memory to local scrolling
   panels using the Scan shell pattern.
5. Re-run native screenshot review at desktop and narrow-width sizes before
   declaring the desktop interaction model complete.

## First Corrective Pass - 2026-07-10

Implemented and verified in the native Tauri build at 1502x972:

- The application shell is now fixed. The sidebar and selected-project context
  remain in place; the browser-like document scrollbars are no longer the
  navigation model.
- The selected-project context bar now contains only a real project switcher
  and the desktop-shell state. The inert search and duplicate global Scan
  action were removed.
- Scan has exactly one executable command: `Run scan`. A project without a
  Cargo manifest shows the informational `Entire workspace` state, not a
  disabled dropdown. A Cargo workspace receives a Rust-discovered picker of
  executable packages; the selected package is verified against that catalog
  before the desktop command invokes `--crate`.
- Scan category/history, result table, priority view, and finding inspector
  are contained panels with their own scroll regions. The native command was
  clicked and produced a fresh desktop snapshot with 21 findings.
- Rules is a fixed three-panel catalog: local facet rail, local numbered-rule
  table, and local inspector. The native window showed all 436 scoped rules.
- Engine catalog/detail and Memory graph facets/detail are bounded panels.
  The graph canvas owns the remaining viewport; it does not use page scroll.
  The native X06 fixture view showed 9 native nodes, 10 projected edges, node
  and edge facets, selected-node source evidence, and zoom controls.

Still required before the interaction model can be called complete:

- Add generic domain targets only when Rust supplies a target model and a
  matching executable scanner command. Improve Files with index-backed path
  suggestions only after the suggestion list is read from the project Store.
- Re-run the screenshots at a narrow desktop width. The current responsive
  breakpoint still stacks selected inspectors; it needs a drawer or selected
  detail mode rather than a second vertical page section.

## Second Corrective Pass - 2026-07-10

- Scan now has four executable, validated scope forms: workspace, discovered
  Cargo package, explicit project-relative files, and a Git revision diff.
  The UI does not display a generic domain target because the Rust command has
  no matching target model.
- Harness Runs keeps its heading and refresh command fixed. Run metrics,
  notices, history, diagnostics, and artifact detail now occupy one bounded
  local content region rather than adding implicit rows to the native page.
- Proofs already uses independent ledger and inspector scroll regions.
  Assurance uses Profile, Test categories, Invariants, and Rules content tabs.
  Settings and Analysis still need a final compact list/detail audit with
  realistic large payloads.
- Rules defaults to the selected project's policy-covered stack and only
  exposes the complete numbered registry through an explicit `All rules` view.
  The current registry is not a 158-language catalog: it contains `common`,
  Rust, TypeScript, Python, and IaC rules. Project inspection separately uses
  the public literal-scanner registry in a bounded filesystem walk, skips
  generated/dependency trees and symlinks, and presents extra observed
  languages in a visible "outside policy registry" boundary rather than as
  fake rule families.
- The native shell has been rebuilt after these changes. The remaining visual
  proof is a narrow-desktop screenshot and interaction pass; the current
  responsive CSS still stacks selected inspectors below their list at the
  narrow breakpoint, so it needs a selected-detail drawer rather than another
  page-length section.

## Native Click-Through Evidence - 2026-07-10

- The native application now opens at the Project inventory. Its cards are
  grouped as main repositories, linked worktrees, and external roots; no
  selected-project command bar appears on this inventory surface.
- Clicking the controlled X06 fixture card opened its selected-project
  Overview. The overview showed only registered root facts plus the current
  packaged scan count, and its Scan route opened the existing 21-finding scan
  workspace. This establishes the intended flow: inventory -> selected root
  -> distinct project workspace.
- Memory was inspected as separate modes. Code graph showed the controlled
  Store projection (9 native nodes, 5 native edges, 10 rendered edges) with
  facets and source evidence. Learning was visibly separate and, with no
  `x06-learning-curve.json` evidence for the fixture, showed its three zero
  counters and a centered unavailable-evidence state rather than an empty
  document.
- Hub was inspected through the `Hub` side-mode tab. It did not show project
  context; it showed its ledger root, folded event count, deduplication state,
  lane table, and cross-project health panel. Lanes, inbox, claims, tasks, and
  workers stayed inside the Hub workspace.
- Native controls were clicked for Project -> Overview -> Scan, Project ->
  Memory, Memory -> Learning, and Project -> Hub. The remaining interaction
  proof is a constrained-width pass across the selected-finding and rule
  detail drawers.

## Constrained-Width Evidence - 2026-07-10

- At a 1100x850 native window, the compact sidebar now preserves separate
  icon controls for Project and Hub. The earlier responsive defect that hid
  the only mode switcher has been removed.
- The Projects command header and discovery notice remain fixed at that width.
  Main repositories, worktrees, and external roots use a dedicated local
  project-list scrollbar rather than making the whole workspace scroll.
- The compact Scan icon opened the selected project's real cached report. Its
  selected finding rendered as a closeable overlay drawer, while category and
  result panes remained behind it as bounded local surfaces. It did not create
  a third page-length grid row.
- The compact Rules icon opened the 436-rule selected-project catalog. Clicking
  `AI-1.1` opened a closeable rule inspector drawer with its numbered identity,
  source, validator, applicability, triggers, fixture contract, and immutable
  registry state. The catalog remains visible behind the drawer.

## Capability-Boundary Evidence - 2026-07-10

- Policy displayed the selected root's `enforce.config.json` binding as
  read-only live data, its detected language stack, resolved native-tool ties,
  and a distinct project-wide override section. It stated that rule disabling
  requires a named waiver owner and reason; it did not claim an individual
  finding had been waived.
- Proofs displayed a verified hash-chained journal, one accepted required proof
  claim, the configured `proofs.json` registry, one recorded run, and
  unavailable Git freshness. Its integration panel explicitly states that a
  scan result never becomes proof evidence.
- Assurance displayed the Rust profile source, 13 backed rules, 20 required
  test categories, and 10 declared invariants. Its caveat states that profile
  availability and activation intent do not prove selected-project coverage,
  scan execution, or CI gating; the activation form writes intent only after a
  source specification, owner, and reason are provided.
- Engine displayed 17 Rust-provided capability rows with explicit state totals,
  separate Capabilities and Workpacks modes, All/Usable/Partial/Planned/Evidence
  filters, a bounded catalog, and a bounded detail panel. Planned features stay
  discoverable through this map rather than being represented by unavailable
  project-side commands.
- Engine Workpacks now has independent local filters for track and declared
  Markdown status. Its inspector resolves each selected workpack through the
  Rust-owned capability map: mapped project/UI surfaces are actionable routes,
  while unmapped workpacks state that no current desktop placement is known.
  Selecting `f02` showed its `Connected projects and worktrees` placement and
  opened the Projects inventory. This placement is kept distinct from declared
  plan, execution, and proof status.
- Projects now derives repository-family presentation from the registered
  `mainRoot` relationship. Each registered main root is a family header with
  its registered linked worktrees alongside it. A linked worktree whose primary
  root is not registered is shown in a separate, explicitly named section;
  the app does not invent the missing primary project. The fixed command area
  and local inventory scroll remain unchanged.
- The Projects inventory now has a local text filter, leaving the fixed command
  and filter rows in place while only the family list scrolls. Native inspection
  confirmed that filtering `games` displayed one matching project without
  growing the window. Registering a nested folder showed its entered path,
  resolved linked-worktree root, primary root, branch, index state, and
  observed languages as distinct facts; the Windows `\\?\\` canonical-path
  prefix is intentionally hidden from presentation. The worktree command now
  opens a confirmation with the exact target and explicit desktop-local side
  effect before it can register anything. Registration and confirmation were
  dismissed during this audit, so no desktop inventory mutation was made.
- The selected-project `Setup` workspace now consolidates lifecycle placement
  without collapsing meanings: project registration, scan scope, rule policy,
  code index, harness observations, and proof readiness use the existing Rust
  reads; legacy Test Doctrine CI posture has its own observation card; baseline
  onboarding and CI lifecycle are non-actionable `not implemented in Rust`
  cards. The native window showed the cards in a bounded
  two-column grid with only that grid scrolling. `Open settings` was exercised
  and routed to the existing typed scan-scope editor; no setting was written.
- Engine's Rust capability map now has a separate partial `Project setup and
  lifecycle` row. In native review, Workpacks filtered to track `F`, selected
  `f02 Onboard And Autoindex`, and showed both its existing Projects placement
  and the new Setup placement. Activating the Setup link opened the lifecycle
  surface. `f02` remained declared `TODO`; the route does not alter plan or
  proof state.
- Engine Workpacks now has a fixed local search above track, status, and
  placement filters. It searches ID, title, track, status, ownership,
  dependencies, and parallel-frontier metadata before the catalog is rendered.
  Native review searched `f02`, which returned `f02` itself plus `arc-22` and
  `c11` because both declare the onboarding dependency. The local catalog count
  changed to three while the app window and plan status remained unchanged.
- The native Scan inspector was exercised with a real `RR-12.17` fixture
  finding. Its explicit controls remained `Assign in Hub` and `Inspect rule
  policy`; the first says it creates an exact-path claim only and the second
  says it is project-wide. A new local boundary callout identifies the absent
  typed FixIntent, agent pickup, edit, finding-level waiver/defer, verification,
  proof, and report-closeout lifecycle. No claim or policy write was made.
- Hub now keeps a local filter beside its Lanes, Inbox, Claims, Tasks, and
  Workers tabs. It filters the typed ledger rows in the selected coordination
  view only; it does not introduce a selected-project control, change ledger
  state, or fabricate a project association for lane activity.
- Graph now distinguishes its normal bounded projection from a focused Rust
  projection. On a capped native graph, a user can load matching indexed files
  by path, symbol, or call name. The canvas identifies the focused result (or
  an honest no-match state) and provides a return to the normal bounded
  projection; it does not claim GPU/LOD or full-graph rendering.
- Scan scope controls were reviewed again in the running native window with a
  selected Enforcer worktree. The report title and view mode remain on the
  heading row; scope, package/path selection, and `Run scan` occupy a separate
  wrapping local toolbar. The `Paths` control exposes a bounded Rust-discovered
  directory picker, and choosing a directory fills the validated relative-path
  input. The fixture still correctly has no directory choices because it has
  no eligible top-level project directories.
- Workpacks now includes a local desktop-placement filter. It distinguishes
  workpacks referenced by a Rust capability row from those with no current
  capability placement. The native review showed `49` capability-mapped and
  `65` without placement; the `No map` filter showed only the latter. This is
  a UI coverage statement, not a claim about declared workpack completion.
- Search graph was exercised with `config` against the persisted X06 fixture
  index. Its result rows now keep a long symbol name and its kind/path/rank in
  separate readable lines. Selecting a result opens the Code graph with a
  Rust-focused projection for that symbol; the focused graph still states that
  only matching indexed files and symbols are displayed and does not claim
  answer synthesis or full-repository rendering.
- The Code graph labels now distinguish the native Store summary from the
  rendered navigation projection. For the X06 fixture, the native header showed
  `9 indexed nodes / 5 stored call, import, or route edges`, while the canvas
  showed `10 projection links` because it also renders file-to-symbol `defines`
  links. This makes the relationship counts explainable rather than appearing
  to contradict each other.
- A native operational-screen pass confirmed the remaining desktop boundaries:
  `Project analysis` runs its focused legacy bridge and renders heuristic
  evidence separately from Scan; `Runs` reads typed harness records and opens
  bounded artifacts without execution controls; `Proofs` keeps journal,
  artifact, and PR-claim state separate from findings; and `Hub` has no
  selected-project control. The Index settings tab now renders `Index ready`
  as state instead of a disabled action. Assurance, Rules, and Policy keep
  their long catalogs in local panels and label unavailable coverage,
  activation, and policy boundaries explicitly.
- The Test Doctrine report was rerun against the controlled X06 fixture after
  its inspection-layout repair. The category table and coverage-gap detail now
  retain a 300-420px local viewport: five category rows and several gap cards
  are visible at once, each side scrolls independently, and the surrounding
  report continues to scroll only inside the Analysis workspace rather than
  the Tauri application frame.
- Scan was also rerun from the same fixture. It has one `Run scan` command,
  local category/results/inspector panes, and `Workspace`, `Paths`, and `Diff`
  scopes. The `Package` scope is already data-driven and appears only when Rust
  supplies a discovered crate target; this fixture has no Cargo package target,
  so its omission here is intentional. A small Cargo-workspace fixture remains
  the needed native proof for package selection and crate-limited execution.
- The package-selector presentation was then exercised against the registered
  Enforcer Cargo worktree without launching a scan. The `Package` tab appeared,
  the selector was populated from Rust `cargo metadata` targets (starting at
  `enforcer-cli`), and its description identified the exact packaged-scanner
  `--crate` argument. The selected project was restored to the controlled X06
  fixture afterwards. A small-workspace *execution* proof is still separate
  work; this check proves discovery and selection placement only.
- The native Rules screen was rechecked after the registry-data boundary moved
  behind `load_desktop_rule_catalog`. It rendered all `436` canonical rows for
  the X06 selection, including numbered IDs, language/family facets, inspector
  provenance, and local scroll panes. The frontend no longer imports
  `rules/rules.json`; Rust reads and returns the typed canonical registry.
- A controlled two-package Cargo workspace now lives at
  `crates/enforcer-ui/frontend/src-tauri/tests/fixtures/desktop/cargo-workspace`.
  It is an isolated Git root because registration deliberately resolves nested
  folders to their enclosing Git worktree. Native registration identified its
  `toml / rust` stack and exposed both packages through the Package scan scope.
  Selecting `desktop-scan-alpha` displayed the Rust-provided description
  `Rust package at crates\\scan-alpha; passes --crate desktop-scan-alpha to the
  packaged scanner.`
- The native Package scan was executed for `desktop-scan-alpha`. It rendered a
  persisted desktop-cache snapshot with eight findings, categorized them into
  Enforcer policy, Architecture, Rust domain, and Documentation, and showed
  only `crates/scan-alpha` package findings. The inspector retained the current
  boundaries: project-wide policy is not a finding waiver, Hub assignment is
  ownership only, and canonical Rust Report persistence plus the typed fix
  lifecycle remain unimplemented.
- The command-level paired proof ran the same JSON scanner two ways. The
  alpha run reported `scope.mode=crate`,
  `scope.crateName=desktop-scan-alpha`, and only
  `crates/scan-alpha/Cargo.toml, crates/scan-alpha/src/lib.rs`; it had zero beta
  findings. The workspace run reported four findings under `crates/scan-beta`.
  This establishes package exclusion with a known detectable control rather
  than relying on the absence of beta rows alone.
- Setup's `Open index` action previously routed only to the Settings workspace,
  leaving its Scan scope tab active. It now carries an explicit presentation
  deep link: `Open index` selects Index, while `Open settings` selects Scan
  scope. The selected project is preserved and index creation remains Rust
  owned. The Memory empty-state index action uses the same Index deep link.
- The controlled workspace was indexed through the native Index tab. The
  screen changed from `missing` to `ready`, after which the Memory Code graph
  rendered `14 indexed nodes / 0 stored call, import, or route edges`, five
  indexed files, and nine projected `defines` links. Facets, graph controls,
  selected-node source excerpt, and object inspector stayed within their local
  panels.
- Search graph was exercised against `alpha_marker` after indexing. Native
  deterministic BM25 returned two source rows: `alpha_marker` at
  `crates/scan-alpha/src/lib.rs` with rank `11.00`, followed by the beta
  control at `crates/scan-beta/src/lib.rs` with rank `10.33`. The screen
  explicitly labels this as source-evidence retrieval, with no LLM answer
  synthesis or retrieval-QA artifact claimed.
- Hub was audited in its separate cross-project mode. It has no selected
  project header, keeps its lane/inbox/claim/task/worker data in local panes,
  and labels the append-only message, acknowledgement, and exact-claim boundary
  apart from unavailable lane lifecycle and automated code-fix dispatch.
- The native Claims tab originally allowed long writer and reason values to
  overlap the next column. Claim rows now hold path/lane and writer/reason in
  two bounded wrapping columns with full-value tooltips. The live ledger sample
  with `codex-review-finish` claims remained readable without changing ledger
  state; Workers retains its separate four-field layout.
- Engine's planned `Fix dispatch` capability now opens the Scan finding
  surface instead of being left without a product route. The capability panel
  identifies its live inputs as the existing exact-path Hub handoff and the
  inspector's unavailable lifecycle boundary. It remains `not implemented`:
  no typed FixIntent, disposition, agent pickup, verification, proof, token
  gate, or report-row closeout exists in Rust.
- Scan was re-audited at the native desktop window size with the controlled
  Cargo workspace selected. The shell does not scroll: broken categories and
  run history scroll locally at left, findings scroll locally in the center,
  and the finding inspector scrolls locally at right. There is one `Run scan`
  action in the scan surface; the project header is selection and live-status
  context only. The Package tab exposes the Rust-provided
  `desktop-scan-alpha` selector. The UI now distinguishes `Current report`
  from `Next scan` in a compact strip beneath the stable scope/action row, so
  changing scope cannot be mistaken for re-scoping already loaded findings.
- Rules was re-audited at the same native desktop size. The canonical
  436-rule catalog, rule-family list, and inspector all stay in bounded local
  panes. The catalog previously overflowed horizontally because its four
  metadata columns had a combined fixed minimum wider than the central pane.
  The rule title is now the flexible column, while effective severity, source,
  and state are bounded with hover access to their complete values; the
  horizontal scrollbar is gone and the inspector remains the detail surface.
- Analysis was exercised on the controlled Cargo workspace rather than only
  reviewing its empty state. `Test doctrine` produced the Node-bridge report:
  local category and coverage-gap panes stay bounded and the result labels its
  evidence as heuristic rather than a certification. The screen continues to
  state the missing Rust-native analysis persistence, run history, and CI-grade
  execution envelope. The analysis-root row now constrains long project roots
  and retains the complete value in a tooltip.
- Runs was checked against the same controlled project. The Rust read model
  returned zero harness records and the native empty state clearly separates
  this history from Scan and Proofs. It exposes only refresh: Rust has no
  desktop-backed execute, pin, prune, reset, or repair workflow for harness
  runs. The raw storage-root explanation remains a presentation follow-up, not
  a reason to fabricate those lifecycle controls.
- Proofs was checked with no journal, claim registry, or recorded proof run.
  It separately renders the missing journal, unconfigured PR-ready claim, and
  zero run-record state, while the inspector states that Rust owns journal and
  artifact verification. `Open scan` was clicked and correctly navigated to
  the project Scan report; this is navigation only and the proof screen never
  claims that the scan result is evidence.
- Policy was reworked after native review showed that its empty override
  inspector consumed a permanent column and pushed scan exclusions below the
  primary viewport. The no-override state now uses the full policy width and
  places one Rules route beside rule-policy coverage. The detailed override
  inspector still appears only when typed Rust project overrides exist. The
  controlled fixture verified the no-override layout, observed-language warning,
  native-tool ties, and scan-exclusion route.
- Assurance was checked against the controlled project. It exposes the backed
  money-critical profile, its rule/category/invariant counts, and an explicit
  Rust-backed activation request requiring source specification, owner, and
  reason. The screen says activation records intent only; project coverage,
  scanning, and CI gating remain unavailable and are not claimed.
- Memory's Code graph and Search graph were exercised after indexing the
  controlled workspace. The Code graph rendered 14 indexed nodes, zero stored
  call/import/route edges, and nine projected links in bounded facets, canvas,
  and source-inspector panes. Search graph searched `alpha_marker` and returned
  only ranked source evidence: alpha at 11.00 and beta at 10.33. The result
  explicitly states that answer synthesis and a retrieval-QA artifact are not
  available.
- Parity was then found to have the wrong data boundary: the selected Cargo
  project showed zeroes because `load_memory_summary` looked for X06 artifacts
  beneath that project root. Rust now reads the canonical Enforcer engine
  proof directory instead. Native verification rendered 23 tools, 16 equal,
  seven better, zero worse, and zero incomparable rows while the UI explicitly
  states that parity is engine proof rather than the selected project's graph
  size. `memory_summary_uses_engine_x06_proofs_for_a_selected_project` guards
  the same boundary in Rust.
- Learning uses the same engine-artifact summary and was verified to render
  58 lessons, zero blockers, and four follow-ups for the selected fixture.
  The screen now says that detailed lesson entries and their t0/t1/t2 timelines
  are not loaded by Rust yet, instead of implying that a per-lesson view exists.
- Model capability also reads the engine artifact. Native review showed a
  `plan` runtime, disabled network, and zero observations; the UI now states
  that installation, selection, execution, and observation capture have no
  Rust desktop workflow instead of suggesting a passive viewer is a model
  control surface.

## Scroll Ownership Follow-Up

- The native audit found a remaining web-page failure in the common
  `main-surface` wrapper: it could become the scroll owner for whole project
  screens even when the screen had panes intended to be independently
  navigable. The desktop shell now always clips its content region; a workspace
  must explicitly name the pane that owns overflow.
- Scan already follows this model: categories/history, findings, and the
  inspector are separate local scroll regions. It has one `Run scan` command,
  while Workspace, Package, Paths, and Diff are next-run scope selectors.
  `Current report` and `Next scan` remain visibly separate so a scope change
  cannot imply that the loaded report was retroactively changed.
- Scan target discovery now makes Cargo-package scope explicit instead of
  silently omitting it. The desktop reports whether target discovery is in
  progress, unavailable, or found no Cargo packages; when packages exist it
  exposes a `Cargo packages (N)` selector and labels the resulting field
  `Cargo package`. Native verification on the controlled Cargo workspace found
  two packages, selected `desktop-scan-alpha`, and created a new scan snapshot
  whose current-report and next-scan scope both remained `desktop-scan-alpha`.
  This proves the selected package is passed through the Rust scan path rather
  than being a presentation-only filter.
- Policy and Hub were the two remaining screens relying on the shared wrapper.
  Policy now gives local scroll ownership to its policy and override panes;
  Hub gives it to the harness context, active lane view, and detail inspector.
  Native verification after the rebuild confirmed Policy's policy-binding,
  rule-coverage, native-tool-tie, and scan-exclusion sections, and Hub's
  Lanes, Inbox, Claims, Tasks, and Workers controls, without restoring a
  whole-app scrollbar.
- Follow-up native review showed that Policy still read as a web document when
  no overrides existed: its one full-width local pane put its scrollbar at the
  far application edge. Policy is now a fixed `Policy binding`, `Rule
  coverage`, `Native tools`, `Scan scope`, and conditional `Overrides` rail
  beside one focused local content pane. The controlled Cargo workspace was
  clicked through Binding, Native tools, Scan scope, and Rule coverage; each
  retained the same shell geometry and exposed only its relevant content.
- Hub had the analogous structural issue: a permanent context column displayed
  only the ledger root, folded-event count, and deduplication count. These are
  now a compact strip above the active Hub view, leaving a wide local-scroll
  operational list and the inspector. Native verification rendered the live
  eight-lane table, then selected Inbox and confirmed its empty-ledger state
  still exposes the Rust-backed recipient and message composer. Hub remains
  explicitly separate from a selected project and does not claim unavailable
  lane creation or automated fix dispatch.
- Setup was also restructured after native review found nine lifecycle cards
  behaving as a long readiness document. Its fixed phase rail now separates
  Foundation, Integrations, Evidence, and Delivery, while the local pane shows
  every backed, legacy, or missing Rust boundary for the selected phase.
  Native verification opened Evidence (`Baseline onboarding` is not implemented
  in Rust; `Proof readiness` is unconfigured) and Delivery (`CI posture` is a
  legacy observation; `CI lifecycle` is not implemented in Rust). Settings was
  left structurally unchanged because its existing Scan scope, Index, and
  Connections rail already provides focused, local-scroll
  control surfaces rather than a document-length page.
- Proofs and Assurance were reviewed without a layout change. Proofs already
  keeps hash-chained journal state, PR-ready claim state, recorded runs, and
  external files separate in a local ledger/inspector view; native verification
  showed the controlled project has a missing journal, unconfigured claim, and
  no Rust proof-run records. Assurance already uses focused Profile, Test
  categories, Invariants, and Rules tabs. Its native Profile view made the
  backed-rule/category/invariant counts and the Rust-backed activation-intent
  form visible while explicitly stating that activation is neither project
  coverage, scanning, nor CI gating.

## Workpack Placement Evidence

- Engine now separates declared plan work from desktop placement. The Rust read
  model reports 114 declared workpacks: 49 are intentionally placed in a
  desktop capability and 65 are not placed in desktop. `Unplaced` is a filter
  over that explicit capability map, not an inferred implementation state.
- The native view labels every unplaced row `not placed in desktop`. Its
  inspector explains that this says nothing about code, proof, or completion;
  it means only that no current desktop capability is mapped to the workpack.
  The plan-index warning remains visible: declared Markdown status is not
  execution, proof, or repository-completion truth.

## Project Directory Follow-Up

- Connected projects no longer present a repository family as a set of equal,
  180px cards. Primary checkouts and registered linked worktrees now use
  compact, clickable rows with aligned checkout role, branch, root, index
  state, and observed language fields. Rust already provides all of these
  fields through the desktop project registry and Git worktree discovery; the
  change adds no frontend-owned project state. Native verification opened the
  `cargo-workspace` primary row and reached its selected-project Overview.
- Unregistered-root worktrees and external roots remain separate groups. They
  intentionally retain their card treatment because there is no registered
  family context to compare; their labels explain that boundary rather than
  implying a hidden parent relationship.

## Typed Backend Follow-Up

- Implemented: Rust now emits engine-proof scope, selected project root,
  artifact root, and latest artifact modification time. Search graph identifies
  its own graph as selected-project data; Learning, Models, Parity, and QA
  metrics identify their separate engine-proof artifact and generation time.
  `memory_summary_uses_engine_x06_proofs_for_a_selected_project` guards the
  contract.
- Implemented: search now carries the Rust graph node ID through the Tauri
  payload. A result click loads its owning-file projection and selects that
  exact node in the graph inspector; the focused-projection test and native
  `read_config` click-through cover the path.
- Implemented first aggregate level: Rust emits full-index folder counts for
  files, symbols, and calls. The desktop shows the local-scroll Folder map only
  for a bounded projection; clicking a folder loads a focused projection.
  Native proof used an independently registered 600-file fixture (1,206 nodes)
  and drilled into `src/generated`. A future second level is still required
  when one focused folder itself remains too large to inspect comfortably.
- Implemented: Project Rules now receives Rust-provided detected languages,
  catalog-backed languages, observed languages without catalog support, scope,
  effective severity, and override state. Once this payload is loaded,
  TypeScript no longer decides which rules are in the selected-project scope.
- Implemented: Rust evaluates each rule's declared path globs against the
  selected project's non-ignored file inventory and returns `matched`,
  `no-match`, `invalid-pattern`, or `unscoped`, plus the number of matching
  paths. Native verification on the controlled Cargo fixture showed
  `RR-12.17` matching two Rust paths and a common documentation rule correctly
  reporting no matching paths. This is catalog applicability, not evidence
  that the rule validator has executed.
- Implemented: the same payload reports an explicit disabled state and
  effective severity for project overrides. Rules owns the edit interaction;
  Policy remains a project-policy snapshot.
- Implemented: the persistent shell now renders Rust's `desktop_status`
  binding mode. Native verification showed `Desktop live / mixed bindings` on
  the Projects directory and `Tauri / mixed bindings` in the selected-project
  command bar. This prevents a native shell label from implying that every
  desktop surface is a fully live Rust workflow.

## Runs And Analysis Review

- Runs remains a narrow, local execution-history workspace rather than a
  duplicate Scan or Proof page. Native verification against the controlled
  Cargo fixture showed zero readable harness records, the separate
  Scan/Proof boundary, explicit read-only storage discovery, and the exact
  unavailable operations: execute, pin, prune, reset, and repair. The empty
  state explains the absence of records without implying a clean scan or a
  valid proof journal.
- Analysis remains deliberately separate from generic enforcement Scan and
  from native harness history. Native verification switched between `Test
  doctrine` and `UI boundaries`; each retained a compact segmented control,
  a specific question, and its own honest no-run state. The UI states that the
  action uses the legacy Node bridge and that Rust-native persistence, run
  history, and CI-grade execution envelopes are not implemented. No action
  was run during this visual review because it would create report output;
  the review did not need invented or mutated execution data.

## Final Project-Shell Review

- The selected-project Overview remains a route directory, not a second
  navigation system: its cards open the distinct project workspaces and carry
  the current Rust-provided boundary state. The Engine workspace is a separate
  capability/workpack matrix with a fixed list and inspector; it is the place
  to understand what exists, while Overview is the place to enter a workflow.
- A final native Scan click-through confirmed the original duplicate-control
  and whole-window-scroll concerns remain addressed. The shell owns project
  selection only; Scan owns one contextual `Run scan` action. `Workspace`,
  `Cargo packages`, `Paths`, and `Diff` are target modes, and the Cargo mode
  rendered its actual Rust-discovered package selector (`desktop-scan-alpha`).
  Category history, the findings table, and the finding inspector use their
  own bounded panes, so a large report does not turn the desktop window into a
  vertically scrolling web page.

## Finding Actions Gap

- The Scan inspector now renders a compact lifecycle boundary directly beside
  each finding: inspect evidence, assign an exact-path Hub claim, understand
  the project-wide policy boundary, and see the unavailable defer/waive/fix/
  verify/close path. Native verification rendered all four states inside the
  inspector's own scroll area. This is presentation of Rust capability truth,
  not a frontend-owned workflow state.
- `crates/enforcer-rules` already has a strict `WaiverRegistry` with exact
  normalized `(path, ruleId)` matching, non-empty owner/reason, optional
  inclusive expiry, duplicate rejection, and fail-closed parsing. It is not
  a per-diagnostic waiver and it has no production caller in the desktop scan
  path. Its current repository-wide registry file is test-only.
- The existing Tauri `write_rule_override` command writes the selected
  project's `enforce.config.json` through the typed settings control plane.
  It is intentionally a project-wide rule toggle and has no path or expiry;
  using it as a row-level waiver would silently widen scope. The UI therefore
  continues to open Rule policy only as an inspection surface.
- The missing implementation remains workpack `g03`: a dedicated
  `.enforce/` waiver/action writer, a typed Tauri boundary, production scan
  integration that re-renders an applicable waiver as visible, and a stable
  finding fingerprint before any true per-diagnostic waiver can exist. Until
  then the desktop must not expose a clickable Ignore, Later, or Close action.
- Foundation implemented: `enforcer-ui::actions::file_rule_waiver` now writes
  `<project-root>/.enforce/waivers.json` through the strict existing waiver
  registry. It rejects invalid paths, unknown rules, missing owner/reason,
  expiry failures, malformed existing state, and duplicate ambiguity before a
  write; it preserves other rows and is idempotent for one `(path, ruleId)`
  record. Its focused Rust tests cover no-write rejection, strict round trip,
  and byte-identical repeat writes.
- Packaged scan integration is now implemented. The project-local
  `.enforce/waivers.json` registry is read after legacy policy waivers and the
  Enforcer-owned packaged registry, then applies only to the remaining exact
  `(path, ruleId)` findings before severity splitting. It uses a distinct
  `PROJECT-WAIVER:<ruleId>:<path>` ID and `project-registry` source, so report
  evidence cannot misattribute a project exception to Enforcer's packaged
  registry. The focused `scan --json` proof shows the exact matching finding
  moved to `waived`, removed from violations, and a same-rule finding in a
  different file remaining active.
- Desktop action integration deliberately remains incomplete. The Tauri
  surface still lacks a canonical `RuleRegistry` loader, a guarded waiver
  command/form, report-cache refresh, waiver history, and an explicit finding
  fingerprint. Therefore the desktop still does not expose a clickable Ignore,
  Later, or Close action even though a manually created project-local registry
  affects a fresh packaged scan.
- The Rust-owned Engine capability map now reflects this distinction as
  `Finding actions: partially wired`, rather than planned. Native verification
  of the rebuilt desktop rendered `16 partially wired / 1 not implemented`
  and the Finding actions inspector named the implemented writer/Hub claim
  foundation alongside the canonical-registry, packaged-scan, and FixIntent
  gaps. A focused desktop test guards against promoting this capability beyond
  those missing boundaries.
- Engine now also differentiates the `rules/rules.json` display catalog from a
  production `enforcer-rules::RuleRegistry`. Native inspection of the Rules
  capability rendered that distinction and its direct consequence: the
  packaged scanner, typed waiver writer, and desktop catalog do not yet share
  one runtime rule contract. The metadata test prevents the presentation
  catalog from being described as the missing production registry.
- Workpack inspector placement now appears directly below a selected row's
  declared plan state, before ownership/dependency metadata. Native g03
  verification showed `declared TODO` alongside `Finding actions / partial`.
  This makes the authored Markdown route and current Rust capability legible
  together without treating either one as evidence of the other.
- Capability detail workpack IDs are now active navigation links rather than
  inert labels. Native verification selected `g03` from the Finding actions
  capability and opened the filtered Workpacks view at `g03 Violation
  Actions`, retaining its declared `TODO` and immediate partial-capability
  placement. This gives users a direct path from a current engine boundary to
  its plan route without a separate search step.

## Active And Waived Scan States

- The Scan presentation now treats a scanner-returned waiver as a distinct
  audit state rather than a remediation state. The Rust-owned report payload
  already carries `violations`, `warnings`, and `waived` rows separately; the
  TypeScript adapter only renders those states and no longer labels a warning
  as `fixing`.
- Category totals distinguish active, warning, and waived rows. Waived rows
  stay searchable through the `Waived` state filter and remain visible in the
  inspector, but they do not inflate the active count or the repeated-file and
  repeated-rule remediation priority tables.
- The inspector changes its boundary copy for a waived row: it no longer says
  the finding remains active or offers a Hub assignment. It states that the
  scanner waived the row and that the result remains visible for audit; rule
  policy remains inspectable. The desktop still lacks a Tauri waiver form,
  waiver owner/reason/detail payload, report-cache refresh after a mutation,
  and a closeout lifecycle.
- Native verification against the X06 controlled fixture rendered the revised
  `Finding categories` panel (`21 active / 0 waived`) and an active inspector
  with the corrected active-state text. The project-local waiver scanner has
  separate end-to-end Node coverage; a persistent controlled fixture with a
  real waiver has not been added because it would alter the X06 parity corpus.

## Planning Placement

- Planning is an Engine concern, not a selected-project Rules concern. Rules
  remains the place for the selected project's applicable numbered rules and
  policy overrides; the authored plan/workpack index crosses those boundaries
  and belongs in Engine.
- The Rust-owned Planning capability now routes its `Open planning workpacks`
  control to Engine's Workpacks view. It focuses exactly the capability-linked
  `b01`, `b02`, `b03`, `b05`, `d01`, and `d08` rows rather than opening an
  unrelated Rules catalog or making the user find one selected row in the
  entire 114-workpack index. The focus strip explicitly states this is a
  capability placement filter and provides `Show all workpacks` to return to
  the complete declared plan index.
- Native click-through verified the capability inspector, the route into
  Workpacks, six visible rows, per-row declared `TODO`/`DONE` state, and the
  unchanged distinction between declared Markdown status and engine/proof
  completion truth. The Planning metadata test guards the Engine placement.

## Scan Target Clarity

- The selected-project header is context only: it switches the project and
  reports its kind, branch/worktree, languages, index state, and desktop
  binding state. It does not start a scan. `Run scan` remains the only scan
  mutation, inside the Scan workspace.
- Scan target selection is now an explicit fixed-height sequence: `Project`,
  `Rust crates`, `Folder or path`, and `Git diff`. Choosing a target type
  reveals only its necessary control: the Rust package selector, an explicit
  project-relative path plus discovered-folder picker, or verified Git base
  and head fields. A project without a `Cargo.toml` keeps `Rust crates`
  visible but disabled with an explanation instead of hiding the capability.
- The redundant current-report/next-scan summary was replaced by one compact
  target note. It explains the selected target before the action and retains
  the loaded report scope as provenance. Category lists, history, results,
  and the finding inspector remain separate bounded scroll panes; the Tauri
  window root remains non-scrolling.

## Memory Scope And Graph Scale

- Memory is now deliberately split into two data scopes. `Code graph` and
  `Search graph` operate on the selected project's persisted code-memory
  Store. `Learning evidence`, `Models`, and `Parity` render checked-in
  Enforcer Engine/X06 proof artifacts. The header and tab groups state this
  boundary, so X06 parity is not misrepresented as the selected project's
  graph or learning state.
- The graph canvas reports its rendered node count beside the full persisted
  project count. When the Rust projection is bounded, the notice gives the
  exact `rendered of indexed` count and directs the user to the Rust-derived
  Folder map or focused projection. It does not imply that a visually bounded
  canvas is the whole indexed graph.
- The Memory root no longer owns a generic scroll area. Graph panes already
  use bounded local scroll regions; Engine-evidence content now scrolls inside
  its own surface. This preserves the fixed Tauri frame rather than making the
  entire application behave like a document page.
- Browser-preview click-through at `1536x960` exercised the updated Memory
  empty state and the Scan empty/unavailable state. Both retained a fixed
  frame and local panes. The native desktop window remained running, but its
  automation handle changed between observation and click, so this is layout
  proof only, not a replacement for a future native click-through capture.

## Workpack Density

- The Workpacks view retains its authored-plan filters but no longer spends
  three full-height rows on segmented controls. Search, track, declared plan
  state, and desktop placement are one compact control strip with native
  selects. The summary still reports declared, placed, and unplaced counts.
- `Unplaced` remains a routing fact only: it means no current desktop
  capability deliberately points at that workpack. It does not claim the
  workpack's code or proof is missing, incomplete, or complete.

## Graph Conversation Boundary

- `Ask graph` is now a first-class Memory surface rather than silently
  omitted. It deliberately contains no prompt field or simulated response:
  Rust has deterministic graph retrieval but no context-pack contract, answer
  session, model invocation, citation persistence, or observation capture.
- The surface identifies the selected-project scope, names the exact missing
  Rust boundary and its `g09` plan placement, and routes to the already-live
  `Search graph` evidence workflow. Preview click-through confirmed the
  unavailable state fits inside the fixed application frame.

## Hub Adapter Placement

- Harness adapter discovery is a global Hub concern, not selected-project
  configuration. It can observe user-level installations that serve multiple
  projects and lanes, so Project Settings no longer renders discovery data;
  Project Setup provides one explicit `Open Hub` route instead.
- The Hub `Adapters` tab invokes the Rust/Tauri
  `enforcer-install::detect::detect_harnesses` path when the Hub opens and on
  refresh. It shows only presence, source-path, and capability evidence. A
  detected home directory is not presented as an installed, registered, or
  verified Enforcer adapter.
- The adapter tab deliberately omits ledger-root and event counters because
  those describe coordination data, not adapter discovery. Its list and
  inspector remain bounded inside the fixed desktop frame.
- Browser-preview inspection at `1536x960` confirmed the document scroll
  height equals the viewport height (`960px`), while the Hub content and
  inspector panes each retain their own `473px` bounded surface. Preview mode
  has no Tauri `invoke`, so unavailable adapter data was shown honestly; this
  is frame-layout evidence rather than native discovery proof.
- Missing native actions remain explicit: adapter installation, repair,
  registration verification, and removal require dedicated Rust/Tauri
  commands before the desktop can offer controls for them.

## Bounded Desktop Fixture

- `E:\ocentra-enforcer-ui-fixture` is now an independent Git repository with
  a minimal Cargo package, TypeScript source, baseline toolchain files, and
  no Enforcer memory store. It is deliberately separate from this repository
  so project registration, worktree classification, scan target discovery,
  scan persistence, and first-time memory indexing can be tested without
  treating Enforcer itself as the user project.
- Its checked scanner result has four blocking findings: `RR-6.1`, `RR-6.2`,
  `TS-6.2`, and `TS-6.24`, plus three `DOC-1.1` advisories. This gives the
  Scan workspace both language families, rule categories, severity filtering,
  and a practical small report to inspect.
- The packaged-report normalizer now removes exact duplicates by a full
  finding fingerprint and recomputes active severity totals after
  normalization. Focused tests keep two findings at one source location when
  their evidence differs, while a real fixture scan verified `4` errors and
  `3` warnings with one `TS-6.24` row.
- Native registration, first scan, and first index are the next live desktop
  workflow against this fixture. Browser preview cannot invoke Tauri and is
  not evidence for those mutations.
- A mocked-Tauri rendering of the real fixture report shape exercised Scan's
  project target, one Run scan action, category list, local report list, and
  selected-finding inspector. It rendered `4` blocking and `3` advisory rows
  at `1536x960` with document height equal to the viewport height. This is UI
  layout and data-contract evidence; the fixture's actual packaged command
  scan is verified separately.

## Capability Destination Contract

- The Rust-owned Engine capability map now emits a typed destination instead
  of a scalar workspace name: `mode` (`project` or `hub`), `workspace`, an
  optional `subview`, and `projectContext` (`required` or `none`). TypeScript
  only selects that target; it does not infer scope from a capability domain.
- This keeps the requested two-mode shell intact while distinguishing a
  selected-project destination such as `Project -> Scan` from a global route
  such as `Project -> Engine` or `Hub -> Adapters`. `Harness adapters` now
  opens the Adapters tab, rather than the default Hub Lanes tab.
- Workpack filters and detail copy now say `capability mapped` rather than
  `placed in desktop`. A mapping identifies a declared product target; it
  never establishes workpack completion, action availability, or proof.
- A mocked-Tauri browser click-through supplied the Rust-shaped Harness
  adapter target and discovery payload, opened Engine's `Open Hub -> Adapters`
  control, verified the Adapter tab and global-context inspector, and
  confirmed no ledger status leaked into that subview. The document remained
  exactly `960px` at a `1536x960` viewport. This validates the TypeScript
  routing contract, not native command execution.

## Waived Finding Audit

- Packaged scanner finding DTOs now retain `waiverId`, `waiverOwner`,
  `waiverReason`, `waiverExpires`, and `waiverSource` while the desktop caches
  and reloads a report. The selected waived finding shows those values in a
  bounded inspector section. This is read-only audit evidence, not a desktop
  waiver mutation action.
- The independent fixture's live packaged scan returned `4` active errors,
  `2` active warnings, and `1` waived `DOC-1.1`. Its waiver came from
  `project-registry`, with owner `fixture-maintainer` and a recorded reason;
  the fixture has no expiry, which the inspector renders as `No expiry`.
- A focused Rust JSON-boundary test covers the scanner waiver fields, and a
  mocked-Tauri screenshot at `1536x960` confirmed the audit inspector is
  visible while the document height remains exactly the viewport height.
  The visible latest desktop build is
  `C:\Users\sujan\AppData\Local\Temp\ocentra-enforcer-ui-live-waiver\debug\enforcer-ui-desktop.exe`.
- Direct creation, expiry, revocation, and closeout of a finding waiver remain
  unavailable. They need a Rust/Tauri command backed by the packaged
  `rules/rules.json` policy catalog, not the smaller structured Rust registry.

## Finding Waiver Action

- Scan now exposes canonical `waivable` eligibility from `rules/rules.json`.
  Immutable findings do not expose a waiver action. Eligible active findings
  collect an accountable owner and reason for one exact project-relative path.
- Rust/Tauri `waive_packaged_finding` calls the packaged writer, which validates
  the same registry used by the scanner, writes `.enforce/waivers.json`, and
  reruns the packaged scan before the desktop receives an updated report.
- The fixture accepted `DOC-1.1` and rejected immutable `TS-6.24`. Expiry,
  revocation, and finding closeout remain future actions; no control claims
  those capabilities today.
