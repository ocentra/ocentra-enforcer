# Worker Checklist

Copy this checklist into every packet report. Every box requires evidence.

## A. Before editing

- [ ] I read the root and plan `AGENTS.md`, plan state, architecture, index, my one workpack, and its one proof row.
- [ ] I recorded worktree, branch, base SHA, head SHA, and `git status --short`.
- [ ] I identified inherited residue and confirmed the protected vendor deletion is outside my diff.
- [ ] I recorded the `sourceUnavailable` identity, tracked blob `df48fa4149dd25956e730443d3582693a3f825a8`, and confirmed it cannot become a clean or covered result.
- [ ] I recorded the named singleton integrator; I will submit an immutable packet instead of editing its shared surface.
- [ ] I mailed `<lane> started` with workpack ID and exact `owns` paths.
- [ ] `enforcer route` was run on the exact files/crate/diff.
- [ ] I claimed and guarded every file I may edit; no wildcard exceeds my assigned scope.
- [ ] I recorded baseline counts and ran the smallest existing regression test.

## B. For each cohesive edit

- [ ] The batch is within its skill/rule/engine limit.
- [ ] A failing or missing-contract fixture exists before the behavior claim.
- [ ] I changed one cohesive module, mapping, or fixture pair.
- [ ] I ran the file/module inner gate immediately.
- [ ] Parse/tool absence and errors remain distinct from clean results.
- [ ] Source fingerprints, anchors, predicate, and `notProved` are explicit.
- [ ] Unavailable source, parser, graph, and tool outcomes remain explicit and are never converted into clean results.
- [ ] I mailed progress after a meaningful green checkpoint or unexpected decision.

## C. Required local proof

- [ ] Positive fixture produces the expected finding/retained disposition.
- [ ] Negative fixture remains clean.
- [ ] Malformed/unsupported input has the declared outcome.
- [ ] Boundary fixture defeats the obvious false positive or false negative.
- [ ] Focused test target passes through `ocentra_enforcer_run`.
- [ ] `cargo fmt --all -- --check` passes for Rust changes.
- [ ] `cargo clippy -p <changed-crate> --all-targets -- -D warnings` passes for Rust changes.
- [ ] `cargo test -p <changed-crate> <focused target>` and the required crate test pass.
- [ ] `git diff --check` passes.

## D. Enforcer debt-prevention ladder

- [ ] Exact changed files: routed and scanned.
- [ ] Changed crate: relevant Enforcer checks and crate gate pass.
- [ ] Diff: base-to-head scan/check reports no introduced finding.
- [ ] Policy-critical change: mutation-risk and strict verify pass, or an exact external blocker is reported.
- [ ] Detached-parent comparison shows no new failure hidden by the dirty worktree.
- [ ] No substantive CI job was skipped because the commit was docs-only or path-filtered.

If a file-level or crate-level gate fails, stop and repair that batch. Do not accumulate violations for terminal cleanup.

## E. Closeout

- [ ] Only claimed paths changed.
- [ ] Proof row contains commands, run IDs, exit codes, artifact paths, exact SHA, and required dependency proof.
- [ ] Counts are derived by the validator, not hand-edited; ledger changes were applied only by `cyberskills-ledger-integrator`.
- [ ] Authorized implementation is committed and pushed to the task branch.
- [ ] I mailed `<lane> done` or `<lane> blocked` with branch, SHA, files, results, remaining unknowns, and recommended next packet.
- [ ] Coordination closeout released all claims.
- [ ] I did not merge, declare PR-ready, or declare plan completion.
