<!-- ocentra-enforcer:start -->
# Ocentra Enforcer

Use Ocentra Enforcer for project-independent enforcement, coordination, and compact diagnostics.
MCP server name: `enforcer`.

Before relying on raw terminal output, prefer:
- `mcp__enforcer__route` for indexed rule routing.
- `mcp__enforcer__check` / `mcp__enforcer__scan` for hard validation.
- `mcp__enforcer__run` plus `mcp__enforcer__last_failure` for compact harness diagnostics.
- `mcp__enforcer__coordination_health` / `claim` / `guard` for lane/mail/exact-file coordination.

Coordination is a harness concern, not a product-repo concern. Live state belongs under the Enforcer install ledger root by default.
<!-- ocentra-enforcer:end -->
