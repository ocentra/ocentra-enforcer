# Luna Manager Runbook

<!-- agent-capsule -->
```yaml
manager: "visible Luna manager"
mayDo: "bounded read-only audit rows; boss-approved disjoint repair packets"
mayNotDo: "change authority, shared singleton, CI/install selection, merge, cutover, retirement status"
mail: "START/BLOCKED/DONE with base, head, owns, gates, proves, doesNotProve"
```
<!-- /agent-capsule -->

## Ready Bundle

Every child receives one workpack ID, base SHA, exact owns/non-owns, one capability-row range, fixture location, command set, batch limit, and stop rule. A child must work in an isolated `codex/` branch/worktree and claim exact files before a repair.

## Acceptance

The manager records `legacy entrypoint`, `native entrypoint`, target SHA, fixture/input, observed verdict, diagnostics/evidence comparison, and `doesNotProve`. It may recommend completion; only the boss updates shared matrix state.

## Escalate Immediately

- MJS-only local/CI/install/MCP route;
- schema-only parity offered as behavioral parity;
- any result relying on `9d21780f9` for a public pass;
- a split public `267af94` and private-overlay `9d21780f9` runtime authority;
- a request touching a registry, workflow, installer default, or cutover record.
