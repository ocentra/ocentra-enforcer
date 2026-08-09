# Enforcer program graph

The repository now has a small, read-only control plane for the whole Enforcer
system. It coordinates the existing plans; it does not replace their Markdown,
workpack indexes, proof artifacts, AGENTS rules, or frozen authorities.

## Authority boundaries

- The graph owns identity, hierarchy, dependency edges, derived readiness, and
  blocker explanations.
- Plan and workpack documents own intent, scope, expected tests, and proof
  requirements.
- Tests and retained proof own technical evidence.
- AGENTS.md and skills own execution behavior.
- The user/boss owns unresolved scope, authority, and destructive decisions.

`docs/program-engineering-graph.json` is the graph configuration. The CLI
imports every directory under `docs/plans`, parses each existing
`WORKPACK_INDEX.md`, and reports plans without an index instead of inventing
workpacks. CyberSkills is a first-class program node and its graph-first
manager instruction is represented in the program metadata. The Universal
Language and Rust/MJS programs are represented alongside it, as is the
Enforcer self-host plan.

The graph also records the boss-owned bootstrap workpack that validates this
control plane before Cyber corpus work is considered ready. The two Cyber rows
already accepted by the committed program dashboard are explicit lifecycle
overrides with their source recorded; ordinary index labels are never treated
as completion proof by themselves. Plans without an index (currently the UI
audit plan) remain visible as plans with zero imported workpacks and an
`indexExists: false` status.

## Commands

Run from the repository root:

```text
node scripts/program-graph.mjs validate
node scripts/program-graph.mjs status
node scripts/program-graph.mjs ready
node scripts/program-graph.mjs blocked
node scripts/program-graph.mjs inspect WP/cyberskills-parity-plan/CP08
node scripts/program-graph.mjs why WP/rust-mjs-parity-retirement-plan/RM08
node scripts/program-graph.mjs deps WP/cyberskills-parity-plan/CP12
node scripts/program-graph.mjs dependents WP/universal-language-enforcement-plan/UL07
node scripts/program-graph.mjs next --plan cyberskills-parity-plan
```

Every command emits deterministic JSON. `ready` is derived from hard
`depends-on` edges and verified completion contracts; a manually written
`READY` label cannot bypass an unsatisfied dependency. `blocked` includes the
exact dependency or missing-path reason. `validate` rejects duplicate or
missing IDs, unsafe/missing paths, cycles, invalid lifecycle values, and DONE
nodes whose required paths are absent.

The imported graph is evidence-oriented rather than a duplicate of every
Markdown sentence. Stable workpack IDs, declared dependencies, the plan's
existing README/state/instruction/index/checklist/proof artifacts, and explicit
boss evidence are linked; ambiguous prose remains in its source document
instead of becoming a fabricated hard edge.

The v1 CLI does not mutate graph state. That is deliberate: no agent can make
progress look complete by editing a status field. Acquisition, execution,
proof, and completion transitions remain governed by coordination, the
workpack, tests, and the boss acceptance gate.

## Adding a plan or workpack

Add the plan's normal documents and `WORKPACK_INDEX.md` under `docs/plans`.
Use the existing workpack IDs and dependency text. The next `status` or
`validate` command imports the index automatically. If a dependency cannot be
resolved or a plan has no index, the graph reports that fact; it does not turn
ambiguous prose into a hard edge.
