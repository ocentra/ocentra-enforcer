# Rust-build integration handoff

## Safety boundary

- `E:\ocentra-enforcer` is the frozen safety-scanner checkout. Do not modify,
  rewire, or merge it as part of this work. Its live checkout is currently on
  `codex/private-rust-test-allowlist`; `safety-main` remains a separate frozen
  branch and has not been changed by this integration work.
- `rust-build` is the only integration branch for Rust, Cyber, workpack, and
  documentation work. Commit and push every accepted change there.
- Do not merge `rust-build` into `main` until the frozen safety scanner is
  clean and the full Rust workspace validation has passed.
- Do not delete protected vendor state without an explicit user decision and
  complete mechanical parity evidence. The named
  `vendor/anthropic-cybersecurity-skills/skills/detecting-fileless-malware-techniques/SKILL.md`
  file and
  `vendor/anthropic-cybersecurity-skills/skills/hunting-for-anomalous-powershell-execution/references/api-reference.md`
  are both present and tracked. The corpus remains retained because Rust
  mechanical parity is incomplete.

## Verified branch state at handoff

The live remote branch set is `main`, `safety-main`, `rust-build`, and the
pre-existing `codex/private-rust-test-allowlist`. `main` and `safety-main`
are intentionally untouched; the codex branch is not an integration branch
and is pending separate cleanup/retention review.

The current pushed `rust-build` tip is the tree used for this handoff. Verify
the exact tip and remote alignment before relying on the evidence below:

```powershell
git rev-parse HEAD
git rev-parse origin/rust-build
```

The authoritative frozen scanner is stable against the current Rust tree:

```text
Ocentra Enforcer scan passed for 1,235 file(s).
Ocentra Enforcer scan passed for 1,235 file(s).
```

Both runs produced zero findings. The accepted packets harden the generic
Tree-sitter boundary against embedded-NUL and non-whitespace control input,
which previously caused a Linux `SIGSEGV` in `generic::parse_d`, and make graph
indexing deterministic by sorting normalized paths before symbol resolution.
The final parser-boundary packets also reject the supplementary-plane and
Unicode format/control classes that caused native `tree-sitter-just` and
`tree-sitter-odin` crashes. Hostile property inputs now execute each registered
native parser in an isolated child process with a fixed corpus, so any native
failure reports the exact parser instead of terminating an anonymous shared
batch. A bounded same-process composition test retains shared-lifecycle
coverage. The hostile-input regression is covered by
`property_parser_contracts` and direct language tests; reversed-input graph
ordering is covered by `parity_architecture`. The graph-impacted CI gate is
required for every pushed tip and must be green before merge.

The Rust-build MCP scan decoder now preserves an explicit language selection
such as `languages: ["rust"]`. Schema and end-to-end MCP regressions prove the
selection reaches the CLI report instead of silently falling back to every
profile language. The installed MCP remains the frozen MJS checkout until the
post-merge Rust cutover is explicitly performed.

Local Windows evidence is green for workspace format, full workspace clippy,
full workspace tests, and the exact `npm run ci:local` gate. The commit-bound
mutation-risk proof must be rerun after any subsequent commit; do not reuse a
proof from an older SHA.
The local gate now defaults Cargo to four build jobs so memory-constrained
Windows hosts do not launch an unbounded linker storm; an explicit
`CARGO_BUILD_JOBS` override remains supported. The latter reports advisory
documentation warnings but no hard findings.
The authoritative frozen scan is stable at zero findings across 1,235 files.
The prior CI run exposed one genuine Ubuntu native-parser `SIGSEGV`; the
regression was reproduced locally and fixed by narrowing the supplementary-
plane guard to the two affected external scanners. A full code-SHA CI run for
the final pushed tip remains a merge prerequisite.
Historical counts such as 8,310 or the earlier branch-native 309/281 baseline
must not be used as the current global result.

The memory-graph proof artifact `proof/memory/x06-kg-parity.json` compares 23
live tools against the installed baseline: 15 equal, 8 better, 0 worse, and 0
unrunnable. Candidate latency is lower in 21 of 23 rows, with a median
candidate/baseline ratio of approximately 1.16%. The deterministic
`x06_9_longitudinal` benchmark also passes for 10/50/100-file synthetic graphs,
including incremental-index speedups of 13.5x, 8.4x, and 7.8x and retrieval
p95 samples below 1 ms. These figures are evidence for the current Rust
implementation, not product guarantees for arbitrary repositories.

The current worktree has no unresolved vendor deletion. Both protected vendor
files named above are present and tracked. The wider vendor corpus is retained:
its advisory content is not eligible for removal until the Rust validators and
tests prove complete mechanical parity.

Recent validated `rust-build` packets after that scan include focused tests,
focused frozen-safety scanning, and a detached audit with zero introduced
findings. They cleared RR-5.1 clone findings in their owned files:

- config resolve/project tie
- validator diagnostic parity
- security money-critical and cyber prototype-pollution rules
- event dispatch reports, dead letter, enqueue, and subscriber flows
- plan orchestrator
- memory analysis
- proof read model
- mechanization scaffold

## Next execution slice

1. Verify `HEAD` equals `origin/rust-build` and the worktree is clean.
2. Record the final conclusion for the pushed code-SHA CI run; any failure
   must be fixed with a scoped regression and a new pushed run.
3. Run the authoritative frozen scanner twice on the final pushed tip:

   ```powershell
   node E:\ocentra-enforcer\scripts\rust-rules.mjs scan --root . --languages rust --workspace
   ```

4. Keep every accepted packet covered by scoped tests, focused frozen-safety
   proof, and a detached-parent audit with zero introduced findings before it is
   committed and pushed to `rust-build`. Never weaken a rule, add a waiver, or
   use a bypass merely to reduce the count.
5. Before any merge, update the public README and product documentation from
   current behavior only. Keep internal migration/research/planning history in
   `docs/plans/`; do not present it as product behavior.
6. Only after a full clean scanner result, final CI, full validation, and
   confirmation that the retained vendor corpus is unchanged may `rust-build`
   be merged into `main`.
