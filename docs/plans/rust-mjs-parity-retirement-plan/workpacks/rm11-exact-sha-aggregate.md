# RM11 — Exact-SHA Aggregate Parity

<!-- agent-capsule -->
```yaml
id: RM11
owns: "shared aggregate manifest and closure verdict"
deps: "RM09-RM10"
tier: P0
owner: "boss/Sol plus independent reproducer"
```
<!-- /agent-capsule -->

> Plan: rust-mjs-parity-retirement-plan

## Where We Are

Individual rows can be green while integration, versioning, or cross-surface behavior remains unknown.

## Where We Want To Be

One `rust-build` candidate source/tree SHA has a complete public matrix of equal-or-stricter native results.

## Acceptance And Proof

Run the public `267af94` oracle and native candidate against the same fixtures/target SHA, then include the private overlay's two exact allowlisted behaviors in a union/equal-or-stricter aggregate; retain source/tree SHA, tool versions, CI job SHA, artifacts, normalized deltas, and independent reproduction.

## Stop Rules

Stop on unavailable, timeout, flaky, unclassified, or overlay-dependent row. MJS remains oracle-only, never candidate runtime fallback.
