# Enforcer Agent Router

<!-- ai-dense -->
```yaml
doctrine: "the harness is the reviewer of first resort; AI and humans may write code, the harness decides if it is structurally acceptable"
route_first: "enforcer route / mcp__enforcer__route before opening detailed rule docs; smallest scope: files -> crate/package -> diff -> workspace"
pre_done_gate: "enforcer check mutation-risk --root . --base origin/main --head HEAD; enforcer verify --root . --profile strict"
banned: "bypass comments, skipped tests, broad waivers, re-export shims, rule downgrades to force a gate to pass"
```
<!-- /ai-dense -->

The harness is the reviewer of first resort.

AI and humans may write code. The harness decides whether code is structurally
acceptable. Human review begins after mechanical policy, compiler/type/lint
gates, architecture gates, tests/proofs, dependency/security gates, and local/CI
parity pass.

Call `enforcer route` / MCP `mcp__enforcer__route` before opening detailed
rule docs. Use the smallest route: exact files, crate/package, diff, then
workspace only when needed.

Before claiming work complete, run the scoped enforcer gate for the changed
surface. If policy-critical files changed, run:

```bash
enforcer check mutation-risk --root . --base origin/main --head HEAD
enforcer verify --root . --profile strict
```

Do not add bypass comments, skipped tests, broad waivers, re-export shims, or
rule downgrades to make a gate pass.
