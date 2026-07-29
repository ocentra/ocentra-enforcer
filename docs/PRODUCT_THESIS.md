# Product thesis — the trust layer for AI-written code

Status: living doc. Owner-endorsed direction, 2026-07-12. Informs workpack priorities; does not
override plan mechanics.

## The bet

As AI agents write an increasing share of code, the scarce resource flips from *writing* to
*trusting*. Human review does not scale to agent throughput; linters check style, not
architecture; LLM-reviews-LLM adds opinion, not certainty. The Enforcer is the missing layer:

> Humans set doctrine once. A deterministic harness enforces it at machine speed, at write-time.
> Agent fleets work inside it without colliding. Every "done" carries proof.

No competitor covers the full loop: **doctrine → write-time gate → fleet coordination → proof**.
Linters/SAST are CI-time and advisory; AI review tools are probabilistic; none coordinate agents
or produce an audit chain.

## Pillars (and the crates that back them)

1. **Doctrine, not lint** — architectural rules (parse-at-boundary, branded domains, no raw
   strings across boundaries, justified alloc/clone/index, error honesty) enforced as hard gates.
   (`enforcer-rules`, `enforcer-lang-*`, `enforcer-validator`)
2. **Write-time, not PR-time** — MCP tools + PreToolUse/pre-commit hooks gate agents while they
   type, in every harness (Codex, Claude, Gemini, Cursor, Zed…). (`enforcer-mcp`,
   `enforcer-install`, c03–c09)
3. **One engine, every language** — tree-sitter grammars (dozens of languages) + code graph +
   data flow give AST-accurate rules and polyglot doctrine from a single engine.
   (`enforcer-memory`: grammars, `code_graph`, `data_flow`, `impact`, `complexity`)
4. **Impact-scoped trust** — a change gates its blast radius, not just its diff. Machine-speed
   "what could this break". (`enforcer-memory::impact` + `enforcer-scan` router)
5. **Local-first privacy** — embeddings and semantic features run on-prem via bundled local model
   runtimes; code never leaves the building. The air-gapped enterprise wedge.
   (`enforcer-memory`: `llama_cpp`, `ort_runtime`, `embed`, `hf_cache`)
6. **Fleet coordination** — lanes/claims/ledger let many agents edit one repo without collision;
   proven in production with Codex. (`enforcer-coordination`)
7. **Proof, not vibes** — hash-chained run records and proof claims turn "the gate passed" into
   an auditable artifact. Compliance buyers (AI-oversight regimes) pay for this.
   (`enforcer-proof`, d04 telemetry, arc-25 events)
8. **Adoptable on brownfield** — baseline ratchet: any repo starts green, only NEW violations
   fail, thresholds tighten over time. Solves the "27k-wall" that kills adoption of strict tools.
   (d02; lived experience: our own repo, 2026-07-12)
9. **Choosable doctrine** — the doctrine layer is universal; the satisfying library (Effect vs
   zod vs valibot; pydantic vs attrs; serde+newtypes) is a per-project profile with UI toggles.
   Owner defaults ship as the default profile, never as law. (new doctrine-profile packs, g05 UI)

## Packaging shape (design feature flags for this now, price later)

- **Free CLI**: gates + ratchet + rules, single repo. The adoption engine.
- **Team**: desktop app, coordination hub, fix campaigns (cluster findings → agent swarm burns
  down debt under gates → proof per fix).
- **Enterprise**: org-wide doctrine profiles, audit/proof export, air-gapped local-model RAG,
  cross-repo federation (`cross_repo`).

## Composite features to build from existing parts (reuse map)

- **Fix campaigns** = embeddings-clustered findings → coordination lanes → d07 fix loop → proof
  claims → live in Runs/Hub UI. Dogfood target: our own 27.7k debt.
- **Shadow-parity MCP** = run Rust MCP beside .mjs, mirror calls, diff outputs (arc-05 harness
  pattern) until the deprecation checklist empties itself.
- **Honest UI** = Tauri workspaces driven by real run/ledger/proof records, never mock constants.
- **Cross-harness worklog (x08)** = the ledger+telemetry+proof records already capture every
  session, scan, claim, and proof across ALL installed harnesses; a read-model + `enforcer worklog`
  surface answers "what did I work on, where, with which AI tool" — the unified trail multi-tool
  practitioners are publicly asking for ("the tooling multiplied, the tracking did not"). Adoption
  wedge: install for the worklog, stay for the gates.
- **Ratchet-first onboarding (f02)** = ten minutes from install to a green gate on any repo.
- **Explainable findings** = data_flow-backed "why this is a violation" traces.

## Evidence to produce (marketing needs numbers)

- Violations caught that eslint/clippy/ruff/Semgrep miss, on fixture corpora (parity harness).
- False-positive reduction from AST-backed vs regex rules (the AST migration pack measures this).
- Debt burn-down rate of a coordinated fix campaign vs single-agent (our own repo is the case
  study).
