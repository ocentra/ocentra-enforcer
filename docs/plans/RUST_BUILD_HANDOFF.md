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
- Do not restore, delete, or commit protected vendor state without an explicit
  user decision. The named
  `vendor/anthropic-cybersecurity-skills/skills/detecting-fileless-malware-techniques/SKILL.md`
  file is present and tracked. The current dirty deletion is instead
  `vendor/anthropic-cybersecurity-skills/skills/hunting-for-anomalous-powershell-execution/references/api-reference.md`;
  it remains untouched.

## Verified branch state at handoff

The live remote branch set is `main`, `safety-main`, `rust-build`, and the
pre-existing `codex/private-rust-test-allowlist`. `main` and `safety-main`
are intentionally untouched; the codex branch is not an integration branch
and is pending separate cleanup/retention review.

The latest stable full frozen-safety scan on the Rust-source commit
`b15c85f4b` reported zero violations across 1,235 Rust files. The subsequent
`c079bcb6b` packet changes only CI cache policy and its regression test; it
does not change Rust source. The subsequent `f66b2ab61` packet fixes
cross-platform expectations for Windows model-proof fixtures and is covered
by focused model-runtime contract tests. In the current CI dogfood, the branch-native
scan reports 309 total findings, of which 281 are baselined and zero are new.
The next source packet hardens the generic Tree-sitter boundary against
embedded-NUL input, which previously caused a Linux SIGSEGV in
`generic::parse_d`; the exact hostile input and a minimal NUL case are now
covered by `unit_languages_d`, and the full property-parser contract passes on
Linux and Windows under default features.
The earlier 8,310 count is historical only and must not be used as the current
baseline. Do not infer a new global count from file-level scans; rerun the full
scanner against the accepted clean commit when the integration state changes.

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
