# Ocentra Enforcer Product North Star

<!-- agent-capsule -->
> Product: `ocentra-enforcer`
> Category: portable mechanical software assurance and coordination runtime.
> Core promise: known, expressible, registered requirements are evaluated continuously and cannot quietly be skipped by a human, an AI agent, a local hook, or CI.
> Authority boundary: AI and humans may propose; deterministic tools, typed policy, executable fixtures, and exact-SHA proof decide mechanical acceptance.
<!-- /agent-capsule -->

## The Problem

Serious repositories repeatedly assemble compilers, language servers, linters, tests, architecture checks, security scanners, dependency auditors, pre-commit hooks, CI workflows, and project-specific scripts. The pieces are mature but fragmented. Their configuration drifts, outputs disagree, local and CI behavior diverge, checks run too late, and prose doctrine is mistaken for enforcement.

AI increases the volume and speed of authorship, but it did not create this problem. Humans have always forgotten rules and lost context. A large `AGENTS.md`, `SKILL.md`, or `rules.md` is specification and guidance; without an executable gate it is hope.

## The Product

Ocentra Enforcer composes existing analyzers and Enforcer-native mechanics into one incremental, non-bypassable contract:

```text
source / changed files / diff
            |
            v
language, role, ownership, and impact routing
            |
            +----------------------+----------------------+
            v                      v                      v
existing tool adapters     Enforcer-native rules   tests/proof commands
            |                      |                      |
            +----------------------+----------------------+
                                   |
                                   v
                    normalized diagnostics + evidence
                                   |
                    author-time -> commit -> CI -> release
```

The same rule identity and policy must mean the same thing at every stage. Scope widens with risk: exact files, crate/package, impacted graph, diff, workspace, release.

## Reuse-First Doctrine

1. Reuse a mature compiler, analyzer, linter, security engine, or package auditor when it already owns the semantics.
2. Wrap it with a typed, allowlisted adapter that pins availability/version policy, bounds execution, normalizes diagnostics, and records evidence.
3. Add Enforcer-native predicates for missing organization-specific, architectural, cross-tool, or normalized-fact requirements.
4. Keep AI/human judgment explicit for undecidable or genuinely contextual questions.
5. Convert recurring judgment into a mechanical rule only after its contract and positive/negative proof are stable.

Enforcer does not compete with rustc, Clippy, rust-analyzer, ESLint, TypeScript, Ruff, mypy, Semgrep, CodeQL, package auditors, or test frameworks. It makes them reliable proof providers inside one portable contract.

## Enforcement Layers

| Layer | Authority | Examples |
|---|---|---|
| Ecosystem mechanics | Existing mature tools | compiler/type/lint/test/security/package engines |
| Enforcer mechanics | Rust predicates over typed input, syntax facts, graph facts, or normalized tool results | architecture, ownership, doctrine, cross-tool consistency |
| Judgment | Human or AI review, visibly non-mechanical | product intent, ambiguous tradeoffs, threat hypotheses |

An AI reviewer may review AI output, but "looks good" is not a mechanical pass. Judgment becomes blocking only through an explicitly governed review requirement; it never fabricates compiler, rule, fixture, or CI evidence.

## Author-Time Contract

MCP places the contract inside the writing loop:

1. Route the exact intended files before editing.
2. Return compact relevant rule/tool capsules rather than loading the whole doctrine into model context.
3. Run file/crate/package checks after each cohesive change.
4. Return normalized machine-readable diagnostics and the smallest repair scope.
5. Refuse known violations at commit.
6. Reproduce the same semantics over impacted and workspace scopes in CI.

This makes mechanical review the reviewer of first resort. Human review begins with design and intent after repeatable mechanics are green.

## Knowledge Graph and Memory

The knowledge graph represents current repository structure: files, symbols, definitions, calls, imports, routes, ownership, dependencies, architecture, and supported cross-file relationships. It routes impact and enables fact-based rules; it is not itself a pass/fail authority.

Memory is durable temporal evidence: runs, failures, decisions, exact SHAs, tool versions, accepted artifacts, repeated mistakes, noisy checks, and why policy exists. It improves routing and explanation; it does not silently create policy.

Normalized syntax facts are shared infrastructure. Grammar-specific adapters produce them once. The graph and Enforcer both consume them. Rule crates never import memory persistence or raw parser nodes.

## Coordination

Parallel authors, human or AI, need transactional discipline:

- worktrees isolate physical state;
- exact claims declare intended writes;
- file and branch locks block conflicting edits;
- cross-branch overlap is retained as merge risk;
- mail records asynchronous handoffs and decisions;
- operation-aware guards protect edit, commit, push, merge, and PR-ready transitions;
- one integrator owns each shared registry or singleton surface.

Locks enforce declared ownership; they do not excuse overlapping workpack design.

## Proof and Honesty

- A parser, tool, or fixture that did not run is not a clean pass.
- Docs-only CI cannot validate a previous source failure.
- `supported` always names a capability level and evidence.
- A narrowed rule states `proves` and `doesNotProve`.
- Mechanical, external-engine, advisory, and manual components remain distinct.
- Terminal proof binds source SHA, tree SHA, tool/artifact versions, local run IDs, CI job SHA, and independent reproduction.
- Known mechanically expressible violations should be unable to land; this is not a claim that all bad software is decidable or preventable.

## Product Loop

```text
understand -> route -> coordinate -> write -> check -> prove -> remember -> improve
```

The CLI, MCP, CI integrations, and UI are views and control surfaces over that same loop. The UI must explain what happened, why it was blocked, the evidence, the owner, and the smallest safe next action.

## Commercial Value

Customers pay to stop rebuilding fragmented enforcement glue and to scale authorship without scaling review and failure cost at the same rate:

- portable versioned policy/tool packs across repositories;
- one local-to-CI diagnostic and evidence contract;
- fast impacted checks while code is being written;
- fewer conflicts, repeated failures, security escapes, and architecture regressions;
- model-independent mechanical authority;
- durable auditability and organizational knowledge;
- safer multi-agent parallelism.

The defensible asset is the proven behavior: normalized multi-language facts, tool adapters, rule/fixture corpus, exact-SHA proof chains, concurrency semantics, false-positive history, and self-dogfood—not a dashboard or a large collection of prose.

## Program Mapping

- `universal-language-enforcement-plan`: shared syntax facts, language/tool capability routing, shape-driven doctrine, and reusable mechanics.
- `cyberskills-parity-plan`: decomposes security knowledge into native predicates, shared capabilities, external engines, advisory knowledge, and manual procedures.
- `rust-mjs-parity-retirement-plan`: proves the Rust runtime equal-or-stricter, cuts over local/MCP/CI execution, and retires frozen MJS safely.

All three plans obey this document. A workpack that reinvents an established tool, relies on AI classification as proof, or overstates unavailable capability must stop and escalate.
