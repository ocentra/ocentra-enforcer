# AGENTS.md - Universal language operating contract

This contract binds the visible manager and every child task.

## One-workpack rule

The boss assigns one workpack instance. A manager may split a wave into at most three disjoint child packets only when the workpack explicitly permits it. Every child uses its own worktree and branch, owns exact paths, and reports to the manager; only the named integrator edits shared registries or capability matrices.

## Required lifecycle

1. Record branch, worktree, base SHA, inherited dirty state, and active claims.
2. Route exact files through Enforcer.
3. Register a lane and mail `<lane> started` to the manager and boss.
4. Claim and guard exact paths before editing.
5. Make one smallest compilable capability change.
6. Run the inner gate immediately.
7. Run the workpack proof row and Enforcer file/crate gate.
8. Guard changed paths for commit and push.
9. Commit/push only the task branch; mail evidence and close claims.

Read-only audits stop after mailing the report.

## Integrator rule

Shared files have exactly one lane owner. Child tasks write immutable packets under their workpack proof root and mail a proposed patch summary. The integrator alone claims, edits, gates, commits, and releases the shared file. Locks are mandatory but do not replace exact ownership.

## Mechanical doctrine

- Universal requirement: validated shape at a declared boundary.
- Framework family: one implementation capable of satisfying a requirement.
- Profile verdict: `accepted`, `rejected`, or visibly `requirement-disabled` with owner/reason for weakening.
- Rule capability: the minimum fact set needed to make a verdict.
- Unavailable capability: a typed unsupported/indeterminate outcome, never fabricated clean.
- Text scan, syntax fact, graph fact, external tool result, and human advice are distinct evidence classes.

## Stop and mail the boss when

- a claimed file overlaps another lane or branch;
- a shared registry has no named integrator;
- UL02 has not authorized a grammar/parser move;
- a rule requests raw Tree-sitter node kinds instead of normalized facts;
- a validator would import `enforcer-memory` or a grammar crate;
- a new language is called supported without positive, negative, malformed, and parse-quality fixtures;
- a framework is accepted by hard-coded library string instead of the active doctrine profile;
- a parser/tool failure would be reported as clean;
- a packet exceeds its batch limit or requires architecture outside its assigned workpack;
- inherited residue, including the protected vendor deletion, enters the diff.

## DONE

A child packet is DONE only when its named proof, exact ownership, Enforcer gates, commit, push, mail, and closeout are complete. Only the boss changes plan status or declares a capability level complete.
