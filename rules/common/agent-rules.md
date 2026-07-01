# Agent Rule Index Rules

## Covered Rules

- `AI-1.1`: Agent rule docs must be indexed from `AGENTS.md` through a small rule index, and rule files must stay under the configured line limit.

## Enforcement

Run:

```bash
ocentra-enforcer check ai-rule-index --root <repo>
```

This keeps agent instructions routed and prevents future context-heavy rulebook drift.

## Fails

- A root or skill instruction tells agents to read broad rulebooks by default.
- A rule document grows without an index route or exceeds configured size limits.

## Passes

- Agents start from a small index, classify the task, and load only the matching rule family.
- Rule docs stay small enough to be used as targeted repair instructions.

## Fix Recipe

1. Add or update the index route before adding detailed instructions.
2. Move broad prose into small routed family docs.
3. Re-run the agent-rule index validator.

## Validator

- scanner: `common/ai-rule-index`
- command: `ocentra-enforcer check ai-rule-index --root <repo>`
