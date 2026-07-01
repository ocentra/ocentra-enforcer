# Ocentra Enforcer Agent Router

The harness is the reviewer of first resort.

AI and humans may write code. The harness decides whether code is structurally
acceptable. Human review begins after mechanical policy, compiler/type/lint
gates, architecture gates, tests/proofs, dependency/security gates, and local/CI
parity pass.

Read `rules/INDEX.md` before opening detailed rule docs. Use the smallest
route: exact files, crate/package, diff, then workspace only when needed.

Before claiming work complete, run the scoped Enforcer gate for the changed
surface. If policy-critical files changed, run:

```bash
ocentra-enforcer check mutation-risk --root . --base origin/main --head HEAD
ocentra-enforcer verify --root . --profile strict
```

Do not add bypass comments, skipped tests, broad waivers, re-export shims, or
rule downgrades to make a gate pass.
