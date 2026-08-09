# AGENTS.md - CyberSkills parity operating contract

This contract binds every task working in this plan.

## Executable graph control plane

Before selecting work, query the repo-owned Cyber Plan graph from the
repository root:

```text
enforcer graph validate
enforcer graph status
enforcer graph ready
enforcer graph inspect <stable-id>
enforcer graph blocked
```

The graph controls dependency order and reports readiness; it does not replace
this Markdown contract or grant authority to edit a singleton path. Do not
start a workpack whose graph state is `BLOCKED`. Readiness is not completion:
the graph only derives `DONE` after the workpack's explicit paths, tests,
proof, ADR, and checklist contract is mechanically satisfied. CP08
decomposition evidence never promotes native implementation or executable
proof. Use the existing coordination claim/guard lifecycle for authority and
keep all `sourceUnavailable`, external, advisory, and manual boundaries
explicit.

## One-workpack rule

Read only the plan read-order documents and the one workpack assigned by the boss. Do not edit sibling workpacks, infer product completion, or broaden the `owns` set. A repeated wave is still one workpack instance with a named batch manifest.

## Authority order

1. Repository root `AGENTS.md` and Enforcer doctrine.
2. This contract.
3. [ARCHITECTURE.md](./ARCHITECTURE.md).
4. The assigned workpack.
5. Worker interpretation.

## Singleton integrators

- `cyberskills-ledger-integrator` alone claims, edits, gates, commits, and releases `crates/enforcer-rules/dispositions/cyberskills-disposition.json` and its ledger validator/tests. Workers submit immutable proposal/evidence packets only.
- `tool-adapter-integrator` alone claims, edits, gates, commits, and releases the shared `enforcer-harness` process runner, adapter registry, and normalized tool-result schema under Universal UL07. CyberSkills consumes that landed contract and never creates a second generic runner.

## Required lifecycle

1. Establish branch, worktree, base SHA, inherited dirty state, and protected residue.
2. Route exact files through Enforcer.
3. Register lane and mail `<lane> started` to `primary`.
4. Claim and guard exact files before editing.
5. Work in the smallest compilable or schema-valid batch.
6. Run the inner gate after every cohesive edit, not only at the end.
7. Run the workpack crate/diff gates and update only its proof row.
8. Guard the exact changed paths for commit/push.
9. Commit and push the task branch only when implementation was authorized.
10. Mail `<lane> done` or `<lane> blocked` with exact evidence and close out claims.

Read-only audit assignments stop after the report and do not commit.

## Mechanical doctrine

- `native-predicate`: T1 deterministic or T2 scored mechanical behavior.
- `external-engine`: typed Rust-owned invocation and ingestion of a named, allowlisted engine.
- `advisory`: retained knowledge with no enforcement claim.
- `manual`: retained human procedure with a reason mechanization is not honest.
- A narrowed predicate must say what it proves and what it does not prove.
- An absent or failed parser/tool is never a clean pass.
- A docs-only CI success is never source validation.
- The protected absent source is `sourceUnavailable`, tracked blob `df48fa4149dd25956e730443d3582693a3f825a8`; it is never a clean, retained, decomposed, covered, or proved result.
- No bypass comments, skipped tests, broad waivers, re-export shims, arbitrary commands, or downgraded rules.

## Stop and mail the boss when

- the protected vendor deletion enters the diff;
- another task owns an exact path;
- a source mapping lacks a stable source fingerprint or pass/fail evidence;
- a grammar change would be made outside the single grammar owner;
- a security rule needs raw Tree-sitter node kinds not represented by the syntax-facts interface;
- an external adapter needs an unallowlisted executable, unbounded output, network access, or credentials not declared by policy;
- an inherited failure prevents the named gate from giving an honest result;
- a worker is asked to edit a ledger or shared-adapter singleton without its named integrator;
- a workpack would exceed its stated batch limit.

## DONE

A workpack is DONE only when its checklist, named proof row, exact-file ownership, local gates, Enforcer gates, commit, push, and mail closeout are all complete. A worker cannot declare the plan, branch, PR, or migration complete.
