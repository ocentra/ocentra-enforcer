# Rust-build integration handoff

## Safety boundary

- `E:\ocentra-enforcer` is the frozen, currently wired MCP checkout on
  `safety-main`. Do not modify, rewire, or merge it as part of this work.
- `rust-build` is the only integration branch for Rust, Cyber, workpack, and
  documentation work. Commit and push every accepted change there.
- Do not merge `rust-build` into `main` until the frozen safety scanner is
  clean and the full Rust workspace validation has passed.
- Do not commit or restore the pre-existing deletion at
  `vendor/anthropic-cybersecurity-skills/skills/detecting-fileless-malware-techniques/SKILL.md`
  without an explicit user decision.

## Verified branch state at handoff

Remote branches are limited to `main`, `safety-main`, and `rust-build`.
`main` and `safety-main` are intentionally untouched.

The latest stable full frozen-safety scan on `rust-build` commit `57c52b5e1`
reported zero violations across 1,235 Rust files. The branch-native scan also
reported zero hard violations (with 1,292 advisory DOC-1.1 warnings across
1,846 files). The earlier 8,310 count is historical only and must not be used
as the current baseline. Do not infer a new global count from file-level
scans; rerun the full scanner against the accepted clean commit when the
integration state changes.

The current worktree has one unresolved vendor deletion that is intentionally
preserved and not part of the pushed commit:
`vendor/anthropic-cybersecurity-skills/skills/hunting-for-anomalous-powershell-execution/references/api-reference.md`.
The separately named `detecting-fileless-malware-techniques/SKILL.md` file is
present and tracked; it is not currently deleted.

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

1. Verify `HEAD` equals `origin/rust-build`; resolve no local residue other
   than the explicitly preserved vendor deletion.
2. Run the authoritative frozen scanner:

   ```powershell
   node E:\ocentra-enforcer\scripts\rust-rules.mjs scan --root . --languages rust --workspace
   ```

3. Group findings by rule and crate. RR-5.1 ownership work is no longer the
   primary lane. Prioritize coherent RR-6.x type-boundary refactors (public
   raw DTO fields, primitive domain signatures, and raw private invariants)
   in isolated worktrees, with all affected callers and external tests.
4. Each packet must have scoped tests, focused frozen-safety proof, and a
   detached-parent audit with zero introduced findings before it is committed
   and pushed to `rust-build`. Never weaken a rule, add a waiver, or use a
   bypass merely to reduce the count.
5. Before any merge, update the public README and product documentation from
   current behavior only. Keep internal migration/research/planning history in
   `docs/plans/`; do not present it as product behavior.
6. Only after a full clean scanner result and full validation may
   `rust-build` be merged into `main`.
