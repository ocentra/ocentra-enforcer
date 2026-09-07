# Program Execution Architecture

## Control hierarchy

```text
Primary boss (architecture, integration, status authority)
  |
  +-- CyberSkills visible Luna manager
  |     +-- bounded Luna audit/implementation children
  |
  +-- Universal-language visible Luna manager
  |     +-- bounded Luna language/tool packets
  |
  +-- Rust/MJS parity visible Luna manager
        +-- bounded Luna inventory/oracle packets
```

Visible managers are user-owned side tasks. They coordinate implementation but do not redefine architecture, edit another program's plan, merge to `rust-build`/`main`, or promote terminal status. The boss assigns one ready bundle, resolves cross-program decisions, independently verifies decisive proof, and integrates accepted commits.

## Boss heartbeat

While any program is active, an hourly heartbeat wakes the primary boss. Each heartbeat:

1. reads compact cursors for all three visible manager tasks;
2. reads Enforcer coordination health, inbox, active claims, and branch/worktree state;
3. reconciles manager reports with [PROGRAM_STATUS.md](./PROGRAM_STATUS.md) and the committed workpack dependency graph;
4. nudges a manager only when it has a legal next action and has produced no progress during the preceding interval;
5. independently reproduces decisive `DONE` evidence before accepting a packet;
6. answers or escalates `BLOCKED` packets without widening a child's ownership;
7. dispatches the next ready bundle only after its predecessor has an accepted artifact on the integration SHA.

Silence is not progress and repeated messages are not proof. After one quiet interval the boss requests a compact status record. After two quiet intervals it inspects the task and coordination ledger directly. It never asks a manager to bypass a gate merely to keep work moving. The heartbeat stops when all three programs are closed or explicitly paused.

## One ready bundle

A manager receives exactly one bundle containing:

- plan and workpack ID;
- base SHA and dependency proof;
- exact owns and explicit non-owns;
- batch and concurrency limits;
- inner and outer gates;
- shared-file integrator identity;
- mail schema and stop conditions;
- expected commit/push/handoff shape.

A manager may spawn children only where the workpack says `parallel-safe`. Each child owns one disjoint packet in its own worktree/branch. Architecture workpacks remain boss/Sol-only.

## Branch and lock discipline

- Every implementation child uses a branch with `codex/` prefix unless explicitly assigned otherwise.
- Physical worktree separation is mandatory for concurrent implementation.
- Claims include project, worktree, branch, exact paths, lane, and operation.
- Same project + same branch + overlapping path + different worktree is a hard branch-write conflict.
- Different branches + overlapping path is a merge risk: edit may proceed only when the workpack intentionally permits it, and `pr_ready` must block until reconciled.
- Protected singleton and shared registry paths have one integrator and may not be child-owned.
- Stale claims are repaired through auditable closeout/repair, never by deleting ledger state.

## Gate ladder

| Moment | Required scope |
|---|---|
| Before edit | route + claim + edit guard for exact files |
| After cohesive edit | formatter/parser/schema plus exact-file Enforcer scan |
| Packet checkpoint | crate/package tests, lints, negative fixtures, diff check |
| Manager acceptance | reproduce decisive gate on packet SHA; verify claimed diff |
| Boss integration | merge/cherry-pick onto current integration branch; rerun impacted graph gates |
| Program closure | mutation-risk, strict verify, workspace/CI/dogfood, exact-SHA independent reproduction |

An inherited failure is recorded with its prior run/SHA and cannot be hidden by a narrower green check.

## Mail protocol

Start:

```text
START workpack=<id> packet=<id> base=<sha> branch=<branch>
owns=<exact paths>
innerGate=<command/profile>
```

Decision/blocker:

```text
BLOCKED workpack=<id> packet=<id>
decision=<one concrete question>
evidence=<paths/run ids>
smallestSafeNext=<action>
```

Completion:

```text
DONE workpack=<id> packet=<id> base=<sha> head=<sha>
changed=<exact paths>
gates=<commands and run ids>
proves=<narrow claim>
doesNotProve=<remaining scope>
claims=<released/closeout status>
```

## Shared integration surfaces

The following are singleton by default and require a named integrator:

- workspace manifests and lockfiles;
- canonical language/tool registries;
- validator/fact domain contracts;
- scan/router/MCP schemas;
- CyberSkills disposition ledger;
- parity capability matrix;
- CI workflow and installation/runtime configuration;
- program status and closure artifacts.

Children submit immutable evidence or patch packets for these surfaces. The integrator applies them serially.

## Cross-program dependency order

1. Product north star and truth inventories.
2. Grammar ownership decision and shared syntax/fact contract.
3. Validator/tool-adapter and external-engine contracts.
4. One proved pilot per contract.
5. Bounded parallel coverage waves.
6. Rust/MJS behavioral closure and integration CI.
7. Runtime cutover, observation, rollback proof, then legacy retirement.

CyberSkills may audit/decompose in parallel with syntax architecture. It may not implement syntax- or graph-dependent predicates before the shared foundation is accepted. Rust/MJS inventory/oracle work may proceed read-only while feature work continues; terminal parity waits for the candidate integration SHA.

## Boss acceptance checklist

- packet is dependency-legal and within exact owns;
- source and tests demonstrate the narrow claim, including negative/unavailable behavior;
- existing mature tool was reused or a native implementation justification exists;
- local proof comes from the packet head SHA;
- manager independently reproduced the decisive gate;
- claims are closed and no protected residue entered the diff;
- integration rerun is green on the new `rust-build` SHA;
- plan status changes only after committed proof.

The boss dashboard is a projection, not an authority. Workpack files, exact-SHA artifacts, Enforcer runs, and CI remain the evidence sources.
