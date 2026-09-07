# Test and Proof Expectations

<!-- agent-capsule -->
```yaml
oracle: "same target SHA plus same fixture/input"
required: "exit code, normalized diagnostics, scope, side effects, exact source/tree SHA, tool versions"
rejection: "unavailable, timeout, flaky, overlay-dependent, or unexplained results do not close parity"
```
<!-- /agent-capsule -->

| Stage | Evidence |
|---|---|
| Oracle row | Frozen and native commands, fixture hash, normalized result, versions and SHA. |
| Repair packet | Positive and negative fixture, scoped test/lint, diff check, claimed-file proof. |
| Aggregate | Candidate source/tree SHA, independent reproduction, CI job SHA, unclassified-delta count of zero. |
| Cutover | Clean-profile install/MCP/CLI/CI proof and rollback to prior native release. |
| Retirement | Deny-list scan shows no executable MJS enforcement call; clean native install succeeds without Node. |
