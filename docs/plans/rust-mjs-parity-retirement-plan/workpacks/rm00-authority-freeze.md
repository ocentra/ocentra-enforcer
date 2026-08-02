# RM00 — Authority Freeze and Pin Reconciliation

<!-- agent-capsule -->
```yaml
id: RM00
owns: "authority manifest and parity-plan status only"
deps: "none"
tier: P0
owner: "boss/Sol only"
```
<!-- /agent-capsule -->

> Plan: rust-mjs-parity-retirement-plan

## Where We Are

Public safety authority is immutable `origin/safety-main` SHA `267af94b701bd592e01a47649e3c18c26ee04239`, which dogfood correctly pins. `d7162b6173e2c664547fcb9715ba135c435d0b1e` is the common fork base only. Live `E:\ocentra-enforcer` SHA `9d21780f9a4f5a498fb16a6b1ae1c05ac2d83e36` is an exact private Rust-test allowlist overlay based on `d716`; it is never public authority/source/verdict input/merge.

## Where We Want To Be

One machine-readable authority record names the current public oracle, provenance base, candidate branch, private overlay prohibition, and split-runtime relationship.

## Acceptance And Proof

Verify all SHAs and current CI references; prove the overlay cannot participate in public verdict generation. The RM11 aggregate must prove the union/equal-or-stricter result of public `267af94` behavior plus the overlay's two exact allowlisted behaviors.

Accepted artifact: [`../authority/RM00_AUTHORITY.json`](../authority/RM00_AUTHORITY.json). It records the exact Git relationships, both dogfood pin anchors, both overlay behavior IDs and source commits, and the no-merge/no-public-pass prohibitions. RM01 must use this artifact verbatim.

## Stop Rules

Stop if split runtime authority is not modeled, any authority is mutable, the two exact overlay behaviors are not enumerated, or an overlay can affect a public pass. No Luna child may edit this surface.
