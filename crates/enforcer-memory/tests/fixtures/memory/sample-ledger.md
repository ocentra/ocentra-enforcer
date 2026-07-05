# Fixture ledger

| id | date | observed | lesson | landed-at | ships-via |
|---|---|---|---|---|---|
| L1 | 2026-07-04 | init threw raw EEXIST | init must be idempotent (return existing identity, not a filesystem error) | arc-16 finding | fixed MCP tool behavior (arc-16) |
| L2 | 2026-07-04 | context blocks reported wrong worktreeRoot | hub context must record caller identity, not server-side resolution | arc-16 finding | fixed MCP tool behavior (arc-16) |
