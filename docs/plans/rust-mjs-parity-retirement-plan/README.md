# Rust/MJS Parity Retirement Plan

<!-- agent-capsule -->
```yaml
planId: rust-mjs-parity-retirement-plan
goal: "Replace live MJS enforcement only after native Rust is proven equal-or-stricter at one exact SHA."
authority: "safety-main 267af94b is immutable public oracle; d7162b617 is common-fork provenance only; private overlay 9d21780f9 is allowlist-only; rust-build is candidate integration."
closure: "exact-SHA aggregate -> native rollback rehearsal -> observed production cutover -> delete-not-merge retirement"
```
<!-- /agent-capsule -->

This plan makes the harness—not an AI review—the acceptance authority.  MJS remains a read-only oracle while parity is measured. It is never a production fallback after native cutover.

## Authority

| Role | Exact authority | Constraint |
|---|---|---|
| Public frozen oracle | `origin/safety-main`, `267af94b701bd592e01a47649e3c18c26ee04239` | Immutable and read-only current public authority. It is never updated from candidate work. |
| Provenance base | `d7162b6173e2c664547fcb9715ba135c435d0b1e` | Common fork base only; it is not the current public oracle. |
| Private overlay | `E:\ocentra-enforcer`, `9d21780f9a4f5a498fb16a6b1ae1c05ac2d83e36` | Live exact private Rust-test allowlist overlay based on the provenance base. It is never a public oracle, public source, public verdict input, or merge source. |
| Candidate | `E:\ocentra-enforcer-rust-build`, `rust-build` | Sole integration branch for accepted native replacements. |

`dogfood.yml` correctly pins the current public frozen oracle `267af94b701bd592e01a47649e3c18c26ee04239`. RM00's blocker is instead split runtime authority: the live private overlay is based on `d7162b617` and lacks the public `267af94` safety fix. Public closure requires an aggregate union/equal-or-stricter proof of the public behavior at `267af94` plus the overlay's two exact allowlisted behaviors.

RM00 is accepted by the machine-readable [authority manifest](authority/RM00_AUTHORITY.json). Later audits consume its exact SHAs and behavior IDs; they do not reinterpret branch names.

## Acceptance model

A capability is closed only when the same exact target SHA and fixture input yield a native result that is equal-or-stricter than the public frozen oracle, with documented scope, normalized diagnostics, exit semantics, and retained evidence. A missing tool, timeout, private-overlay-dependent pass, or unexplained delta is a failure to close—not a pass.

The manager may schedule bounded read-only inventory rows and boss-approved disjoint repairs. The manager and children may not modify singleton registries, authority records, CI selection, installation defaults, cutover state, or retirement status.

Read [AGENTS.md](AGENTS.md) and [ARCHITECTURE.md](ARCHITECTURE.md) first, then [WORKPACK_INDEX.md](WORKPACK_INDEX.md), [MANAGER_RUNBOOK.md](MANAGER_RUNBOOK.md), [WORKER_CHECKLIST.md](WORKER_CHECKLIST.md), and [TEST_PROOF_EXPECTATIONS.md](TEST_PROOF_EXPECTATIONS.md).
