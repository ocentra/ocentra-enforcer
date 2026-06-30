# Agent Rule Index Rules

## Covered Rules

- `AI-1.1`: Agent rule docs must be indexed from `AGENTS.md` through a small rule index, and rule files must stay under the configured line limit.

## Enforcement

Run:

```bash
ocentra-enforcer check ai-rule-index --root <repo>
```

This keeps agent instructions routed and prevents future context-heavy rulebook drift.
