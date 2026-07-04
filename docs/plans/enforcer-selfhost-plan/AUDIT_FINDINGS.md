# AUDIT_FINDINGS — proactive code-vs-plan gap audit (2026-07-04)

Durable record of the gap audit (6 parallel agents, code vs the 109-workpack plan) so nothing is lost to a
cutoff. This is the fix-backlog + resume pointer. 5 of 6 audits completed; docs/skills/install was re-run
(first attempt misfired). Owner directive that triggered this: "AI should scan and tell me and fix, not wait
to be told." Root cause of the gaps: the plan was built by RE-FRAMING existing workpacks to Rust — it
preserved what existed, never DISCOVERED what was absent, and several arc-* packs are monolithic "port the
.mjs" catch-alls that name subsystems without decomposing their load-bearing semantics into provable rows.

## FINAL STATUS (2026-07-04): ALL WAVES 1-6 DONE — plan reconciled to **111 workpacks**, verification verdict **CLEAN** (7/7 categories: counts consistent, 111 files=111 rows, all deps resolve incl arc-15/16/17->arc-25, disjoint-owns holds, zero TS-engine residue, zero stale framing, x04/b06 integrated). Pushed to `plan/enforcer-selfhost` (commit 9ebf5df). WAVE 1 counts+framing; WAVE 2 monolith re-scope (arc-16/17/18/03+a08); WAVE 3 all 569 rules enumerated (arc-06..10); WAVE 4 orphaned MCP/CLI/install surfaces homed; WAVE 5 new packs x04+b06 + resume-state/capability-detection/dual-audience; WAVE 6 docs/skills folded in.
- ONE optional execution-time tightening (verifier-cleared, NOT a defect): `arc-02` owns `crates/enforcer-domain/**` (whole glob) while `a03/a04/a05/a06` own specific files inside it (`src/{rule_id,path,sha256,coordination_ids}.rs`) and dep only `a01`. Sequencing IS documented in the A.1 prose (arc crate lands skeleton, then a0x brand). To make it strictly machine-honest, the arc-02 worker should either carve arc-02's owns to a skeleton list OR add `arc-02` to a03-a06 deps (both frontmatter + WORKPACK_INDEX rows). Left for execution — no runtime risk (hub claims are file-level).

## STATUS of fixes: [ ] = todo, [x] = done
- [x] arc-15/16/17 frontmatter now really declare `deps: arc-25` (index claimed it; frontmatter lacked it — execution-critical for frontier computation).
- [x] arc-15 owns narrowed to skeleton (removed `modes.rs`/f01, `router/**`/f05, `rules/baseline_ratchet.rs`/d02).
- [x] arc-25 note de-futured ("wired by reconciliation pass" -> "declared 2026-07-04").

## WAVE 1 — mechanical consistency (cheap, do first)
- [x] Propagate **109 total / Track C = 11** into the 10 stale docs (all say 107/C=9): PLAN_STATE, PLAN_EXECUTION_BLUEPRINT, NEXT_ACTIONS, PLAN_HEALTH, README, AGENTS, DOC_INDEX, ARCHIVE_INDEX, ROUTE_INDEX. Root cause: c10/c11 were added to WORKPACK_INDEX only (satellite reconciliation ran BEFORE c10/c11 existed).
- [x] Kill stale framing: PLAN_EXECUTION_BLUEPRINT:17 "B/C/D/E/F/G/H re-framed in a follow-up pass" (DONE already), :31 "sharing the ONE ../enforcer-rust worktree" (contradicts §2b total-isolation), :66/:69 pre-Rust `src/**`/`.ts`/`eslint-rules` globs (now `crates/**`); ARCHIVE_INDEX:22 "when Track G is re-framed" (done).
- CLEAN (verified): no dangling deps; no disjoint-owns violation (every owns-overlap has a dep edge); "MCP secondary" never asserted.

## WAVE 2 — re-scope the monolith packs (they name subsystems but don't decompose semantics)
### arc-16 enforcer-coordination — DECOMPOSE (biggest). Its "Where We Are" lists only `vendor/*.js` and OMITS `runner.mjs` + `api.mjs` (~1000 lines of orchestration-critical logic). Missing semantics, each HIGH unless noted:
- [ ] lock-kind taxonomy (writeLock/globalWriteLock/branchLease/workReservation) + 6 conflict classes (write-lock/branch-write/global-write/branch-lease/merge-risk/work-reservation-overlap).
- [ ] protected-singleton auto-escalation to `globalWriteLock` (lockfiles/release/migrations/generated/`.github/workflows`) — the ONE intentional CROSS-worktree lock; EXECUTION_MODEL §2d is INCOMPLETE without it.
- [ ] editIntent / onConflict=intent / release->notify wait-notify overlap protocol (b04 consumes it; arc-16 never specs it).
- [ ] session-lease + thread-mode + delegate-grant "org chart" engine (`runner.mjs:365-537`: session.claim/release TTL, thread-mode-state, hub:delegate:grant/revoke, read-only vs coordinated-delegate).
- [ ] lifecycle-report structured-field validation (`runner.mjs:327-363`: STARTED/BLOCKED/PR_READY/DONE require structured --details) — the mechanical due-diligence gate behind §2d pr_ready.
- [ ] guard operation modes (inspect/edit/commit/push/rebase/merge) + focused-vs-global findings + truncation budget (only pr_ready+primary captured today).
- [ ] wire-hash vs extension-hash canonicalization + `EXPECTED_HASH_COMPATIBILITY_WIRE_HASH` golden sentinel (`events.js:97-150`) — a Rust port MUST reproduce exactly or every ledger fails hash checks / cross-impl sync breaks. MED-HIGH.
- [ ] repair = 3 distinct engines (legacy-hash reconcile, sequence-break rehash, stale-claim/claim-conflict resolve). MED.
- [ ] closeout scoped release + stale-claim repair filters (lane/writer/node/thread/session/project/worktree). MED.
- [ ] retention/compact -> `archived/<stream>/<stamp>.ndjson` + `streamSegments` reads archive+live as one stream. MED.
- [ ] read-index `db/coordination-index.json` + OPT-IN `node:sqlite` hot index (OCENTRA_COORDINATION_SQLITE=1) — port decision: SQLite opt-in or JSON-only. MED.
- [ ] peer server (`server.js`) HTTP surface + daemon.js (ensure/spawn/health/pidfile) — RUST_ARCHITECTURE says "tokio only if serve daemon appears"; DECIDE serve-daemon in/out (arc-22 lite/full says CI never needs hub). MED.
- [ ] notify wake-requests + primary handoff detection + seen-dedupe (distinct from mail). MED.
- FIX SHAPE: split arc-16 into sub-packs (e.g. coord-core/locking, coord-session-lease, coord-sync-peer, coord-repair-retention) OR keep one crate but add per-semantic provable rows; and correct scope to include `runner.mjs`+`api.mjs`.

### arc-17 enforcer-proof — reduces real working mechanics to single words:
- [ ] in-toto attestation (`attestation.json` in-toto Statement v1, predicateType ocentra proof-run/v1, subject digest = gitCommit [non-standard — preserve intentionally]). [G7]
- [ ] git-provenance claim gates: stale-commit / dirty-worktree / missing-artifact / deleted-required-path + `--allow-dirty` escape hatch. [G8]
- [ ] legacy proof = 3 ops: migrateLegacyProofs (generate per-profile proofs.json + copy to legacy-scripts/), importLegacyProof (hash+in-toto), proofParity (coverage equivalent|weaker|not-comparable + deletionReady — the point of migration). [G9]
- [ ] proofs.json registry schema (appliesTo/triggers/capabilities/collector/ciSupport/deviceSupport/requiredArtifacts/requiredForPrReady/claimsProved) + base+profile deep-merge. [G10]
- [ ] manual-required / unavailable proof states + capability model + PROOF-MANUAL diagnostic. [G11]
- [ ] retention constant-vs-config divergence (proof uses hardcoded DEFAULT_PROOF_RETENTION, ignores harness config) + dead `pinPrReadyDays:30`. [G6]
- [ ] proofExport redacted manifest-only bundle (CI-contract: "upload artifacts separately, don't commit proof outputs"). [G14]

### arc-18 enforcer-harness — scoped to PARSING only; the whole run-STORAGE half is unhomed:
- [ ] run storage layout `.enforce/runs/<id>/{raw,diagnostics.ndjson,events.ndjson,summary.json}` + manifest `.enforce/db/ingest-manifest.json`. [G1]
- [ ] retention/prune engine (maxRuns/maxRunsPerTool/maxFailedRuns/pruneAfterDays/pin-keep) + resetRuns/listRuns/runSummary/runDiagnostics/lastFailure/readArtifact (these back 6 MCP tools). [G1]
- [ ] `ndjson-duckdb` store + `duckdb-status.json` optional-ingestion seam (every profile sets store=ndjson-duckdb). [G2]
- [ ] `.ocentra-enforcer` -> `.enforce` legacy dual-read migration (drop it and installs lose run history). [G3]

### arc-03 enforcer-config — enumerate the real field set, not "port config-shaped reads":
- [ ] ~30 Rust-policy fields: rawTypeBoundaryGlobs/facadeFileGlobs/rawStringOwnerGlobs/domainPrimitiveOwnerGlobs/serializedDomainOwnerGlobs/runtimeStringOwnerGlobs, enforceRuntimeStringLiterals/runtimeStringLineAllowPatterns/enforceSerializedPublicDomainPrimitives/enforceWorkspaceFiles, blockedProtocolDependencies/runtimeCrates/testOnlyCrates/allowedGitDependencies/allowGitDependencies/allowPathDependencies/allowBuildRs/allowUnsafeCode/publicReexportPolicy, rustRoots/crateRootGlobs/testFileGlobs/cargoOnFileScope/cargoOnDiffScope/cargoTestThreads/requireCargoDeny/requireCargoAudit/runCargoDoc/failFast. Per-field fixtures. [G5]
- [ ] sourceShapePolicies base shape (distinct from a08's overrides) has no home. [G4]

### a08 waiver-honesty — mis-frames the mechanic:
- [ ] a08 treats all 51 sourceShapeOverrides as dishonesty-to-waive; some are legit per-file policy tuning. Reconcile: distinguish honest per-file policy from dishonest silent bumps; the base sourceShapePolicies shape belongs in arc-03/arc-04, not a08. [G4]

## WAVE 3 — rule enumeration (rules.json = 569 rules; plan enumerates ~45)
- [ ] arc-09: add `checks-contracts.mjs`, `check-metadata.mjs`, `checks.mjs`, `check-docs.mjs`, `check-policy.mjs` to its port list; add per-prefix rows for the common contract families: PROOF-1(15), MCP-1(12), SCAN-1/2(30), HAR-1/2(16), ENF-1(15), DOCENF-1(10), CFG-1(12), CI-1(21), REPO-1(15), NPM-1(15), ARCH-1/BOUND-1(25) — ~200 rules currently unhomed.
- [ ] arc-06: enumerate the ~130 Rust rules under RR-3/5/8/9/10/11/12/14 (only RR-6/7/4/2 in the matrix today).
- [ ] arc-07 (~65 ts: TS-6/7/8) + arc-08 (~55 py: PY-4/5/6): enumerate under-the-generic-bullet rules.
- [ ] MIS-MAP: `generic-scanner` (81 rules) spans common+py+ts — specify how the single engine partitions across arc-07/08/09 (avoid double-own / dropped slice).
- [ ] MIS-MAP: SEC-2 (20 rules) uses validator `generic-scanner` language=common, NOT `source-policy-common-security*` — arc-10 will MISS them; arc-09 may inherit them. Assign explicitly.
- [ ] NAMING COLLISION: PROOF-1/MCP-1/HAR-2 are RULE families (validate a target repo) vs the runtime crates enforcer-proof/mcp/harness — home the rule families in a validator crate (arc-09), don't assume the runtime crate covers them.
- CLEAN: literal-scan ~70-language registry preserved (arc-13 folds wholesale); LIT-1 T2 thresholds covered; e-pack languages are net-new (not gaps).

## WAVE 4 — orphaned MCP/CLI surfaces (no crate owner)
- [ ] legacy `rust_rules_*` MCP alias surface + deprecation window -> arc-21 / x03.
- [ ] stale-server write-gate (`shouldBlockStaleMcpTool`/`mcpStaleError`/COORDINATION_WRITE_TOOLS) + `ocentra_enforcer_run` CLI fallback -> arc-21 + arc-16 (hash-compat). (This is the exact behavior we hit live in smoke tests.)
- [ ] coordination `repair` write/dry-run gating at the MCP boundary -> arc-21 + arc-16.
- [ ] harness run-store MCP tools (run_status/diagnostics/last_failure/artifact/prune_runs/reset_runs) + `runs` CLI -> arc-18 (with WAVE 2 G1).
- [ ] CLI subcommands absent from the clap grammar: `verify` (+ modes fast/local/ci/parent), `advise`, `architecture`, `ledger` (alias of coordination) -> arc-22 (+ reconcile with d06's plan|implement|check|fix|review vocabulary).
- [ ] `check` named-check enum (20 checks incl sbom/ai-rule-index/single-source-contracts) — add an explicit parity checklist so none silently drop -> arc-15 / d01.

## WAVE 5 — new owner requirements (bake into plan AND Track B plan-skill; see memory project-plan-resumability-and-open-requirements)
- [ ] Plan RESUME-STATE discipline: every plan carries Where-We-Are + checklist + tasklist + progress + prev/next records; b02 PLAN-* validator ENFORCES their presence. (This AUDIT_FINDINGS.md + the memory file are the current stand-in.)
- [ ] Main-branch protection + CI-must-pass-before-merge — NEW pack; MODEL ON OcentraParent's `.github/workflows` (read that repo). This repo does not protect main today.
- [ ] AGENTS.md decision-forest — NEW pack: global agent.md -> project agent.md -> per-plan agent.md -> decision tree, read-FIRST-on-resume to minimize tokens.
- [ ] Dual-audience authoring: every rule/skill/instruction ships human-verbose + AI-ultra-dense forms (x02 / Track B / g08).
- [ ] .md is TRANSITIONAL: design so rules/skills/AGENTS.md can be dropped for a typed system/db/schema; humans use the Tauri desktop.
- [ ] Orchestration depth note in EXECUTION_MODEL: Workflow nesting = 1 level only (a workflow child cannot launch a workflow); background sub-agents CAN spawn leaf sub-agents. Three-tier shape (orchestrator -> workpack worker -> intra-workpack leaf) is supported.
- [ ] **EXECUTION LOCUS CORRECTION (owner 2026-07-04):** execution runs in THIS MCP-wired tree (`C:/Projects/ocentra-enforcer`), NOT the pre-made `../enforcer-rust` worktree. Flow: finish plan -> push -> owner switches THIS tree's branch -> pull/rebase -> restart as Fable 5 -> orchestrator runs HERE; LANES cut their own worktrees for isolated parallel work. The pre-made `enforcer-rust` worktree is SUPERSEDED (remove or ignore). Update EXECUTION_MODEL §1 (drop "cut ../enforcer-rust as THE build tree"; the MCP-wired main tree is the orchestrator home + integration branch; per-lane worktrees per §2b still apply for workers).
- [ ] **HARNESS CAPABILITY-DETECTION + ADAPTIVE DEGRADATION (owner req 2026-07-04).** The enforcer defines a TARGET agentic system (orchestrator/primary lane + worker sub-agents + lanes + claims + mail/inbox + heartbeat/scheduled mail-check + pr_ready handoff + registration). Harnesses differ in which primitives they actually have: max concurrent agents, sub-agent nesting depth, background-task support, scheduled-task/cron/automation support, cross-session/direct messaging (Codex strong; others weaker/none), implicit-invocation. REQUIREMENT: each harness must DETECT/DECLARE its own capabilities + limits (a per-harness capability manifest produced at install/doctor time) and the orchestration must ADAPT / gracefully degrade to fit — throttle to the concurrency cap, flatten when no nesting, POLL when no scheduled mail-check, fall back to manual/human-relayed handoff when no cross-session messaging, etc. The enforcer never assumes a primitive exists; it maps the harness's real capability set onto the target model and degrades honestly (labeled, not silent). HOMES: EXECUTION_MODEL (the adaptation doctrine) + the c-track adapters c03/c06/c08/c09 (each declares its capability matrix, consumed by the orchestrator) + arc-16/EXECUTION_MODEL taxonomy (the target model it maps onto). Extends the reference-multiharness-install-matrix.

## WAVE 6 — docs/skills/install audit (COMPLETE)
### Skills / agent descriptors
- [ ] `skills/rust-rules-hard-gate/SKILL.md` (legacy alias skill) has no explicit RETIRE/re-home workpack — only swept by x02's `skills/*/SKILL.md` glob (which refreshes, not retires). Own the retirement in x03 (rename-migration).
- [ ] `skills/rust-rules-hard-gate/agents/openai.yaml` (the ONLY agent descriptor asset) has NO workpack home at all. Decide: retire with the legacy skill (x03) OR re-emit natively (c06 Codex adapter).
- [ ] `.codex-plugin/plugin.json` `"skills": "./skills/"` BULK publish drags the legacy skill + openai.yaml along — collapse this in c06.
### AGENT/SUB-AGENT/SWARM TAXONOMY (owner ask 2026-07-04 — "define what is agent/sub-agent/swarm, in the hub plan")
- [ ] DEFINE the taxonomy in EXECUTION_MODEL.md orchestration section + reference from arc-16 (the hub): orchestrator (primary lane) / worker sub-agent (one per workpack) / leaf sub-agent (intra-workpack) / swarm (a fan-out set) / lane / worktree. Include the nesting rule (Workflow=1 level; background sub-agents can spawn leaf sub-agents; 3-tier max).
- [ ] PER-HARNESS AGENT DESCRIPTOR: Codex works today via `agents/openai.yaml` (descriptor: display_name/default_prompt/allow_implicit_invocation). CLAUDE has NO equivalent — c03 (Claude adapter) must EMIT the Claude form (e.g. `.claude/agents/*.md` subagent definitions) mirroring how c06 emits the Codex descriptor. This is the "Claude setup for the agent/sub-agent/swarm" the owner asked for. Own in c03 (+ the taxonomy it references in EXECUTION_MODEL/arc-16).
### Docs capabilities with no/partial home
- [ ] Neutral client-identity shim `clientThreadId`/`clientKind` (the CLAUDE-drivable identity; docs CLAUDE_DRIVE_FINDINGS/HANDOFF flag it as a design change) — arc-16 ports `context`/`identity`/`presence` but never names the neutral fields; add to arc-16 so Claude sessions register like Codex threads. MEDIUM.
- [ ] Release policy (RELEASE_POLICY.md): signed tags + `package.json` files-allowlist gate + release-cut-from-green-`main` + no-release-on-uncommitted-proof — c10 builds the release pipeline but NOT these gates. Extend c10 (ties to the WAVE 5 main-protection pack). MEDIUM.
- [ ] Branch-protection required-status-check CONFIG (BRANCH_PROTECTION.md names 3 GitHub required contexts + "no bypass") — d11/d28 validate CI PARITY, not the protected-branch settings; the WAVE 5 main-protection pack owns configuring/validating them (and the named contexts are stale pre-rename). MED-LOW.
- [ ] DuckDB analytics store + SQLite hot index (COORDINATION.md 508-514, both "optional") — appear NOWHERE in the plan; arc-16 read-index decision must state port-or-drop (consistent with config-audit G-note + coordination-audit #11). MED-LOW.
- [ ] CodeQL init-generated `codeql.yml` adapter output — low; c-track/c10 (parent parity already marks it "workflow template only").
### Hardcoded-path GAP (real gate hole)
- [ ] `TARGET_REPO_WIRING.md:105` `E:/ocentra-enforcer/rules/INDEX.md` is well-owned (x02 prose + c10 mechanism). BUT ~25 more `E:/ocentra-enforcer/...` example paths across CODEX_SETUP.md / COORDINATION.md / BOOTSTRAP_PROMPT.md / OCENTRA_PARENT_PARITY.md are only IMPLICITLY owned by x02 — and **x02's proof gate greps only `ocentra`/`codex install`/TS-residue, NOT a general absolute-path grep**, so residual `E:/` paths pass the gate. FIX: add a general absolute-path grep to x02's proof gate (or widen c10's grep beyond generated output). MEDIUM.
- [ ] Untracked handoff docs `docs/HANDOFF_CLAUDE_CODE.md` + `docs/CLAUDE_DRIVE_FINDINGS.md` carry `C:\Projects…`/machine `1K08FH4` paths — keep uncommitted (transient) or give them an owner. LOW.
### Install-EMITTER gaps (init writes these today; no Rust c-track owner)
- [ ] Consumer CI-workflow emitters — `init`/`buildInitWrites` writes 5 files from `adapters/github-actions/{ocentra-enforcer,codeql,dependency-policy,secret-scan,sbom}.yml`; TARGET_REPO_WIRING documents all 5. c10 owns the enforcer's OWN release + the `enforcer-scan` action, explicitly NOT the per-consumer `codeql/sbom/secret-scan/dependency-policy` generation. Home in c07 (generic writer) or a dedicated emitter pack. HIGH.
- [ ] Pre-commit emitters — plain git-hook (`adapters/git-hooks/pre-commit.sh`), husky, AND lefthook (`adapters/lefthook/lefthook.yml`). No c-track pack owns the consumer git-hook/husky/lefthook emitters (c04/c05 are Claude PreToolUse/SessionStart — a different mechanism). Home in c07 or a hooks-emitter pack. HIGH.
- [ ] `scripts/validate-codex-assets.mjs` skill-asset validator (validates both skills + `.codex-plugin` `plugin.skills`) not re-homed → fold into c01 install crate. MED.
- [ ] Global `AGENTS.md` managed-block (`<!-- ocentra-enforcer:start/end -->`) + user-skill copy + global `OCENTRA_LEDGER_HOME` config — Codex-only in code; confirm c06 scope explicitly covers the harness-neutral global-AGENTS block + user-skill-copy behavior. MED.
- [ ] `ocentra-parent` profile port — its specific Rust posture (pub-use forbidden repo-wide, runtime-literal ban, domain-typed serialized fields, protocol-crate dependency bans) must be named as a rules-as-data port target in arc-03/arc-04, not left implicit. MED.

## THEN: re-verify + commit + push.
