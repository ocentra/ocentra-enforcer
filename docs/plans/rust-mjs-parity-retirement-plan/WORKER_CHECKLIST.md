# Worker Checklist

## Before starting

- [ ] Read the plan router and only the assigned RM workpack/proof row.
- [ ] Record assigned packet, exact candidate base SHA, public oracle SHA, and whether an overlay behavior applies.
- [ ] Verify dependencies are accepted artifacts, not merely checked boxes.
- [ ] Record exact owns/non-owns, batch limit, inner gate, outer reproduction, and stop conditions.
- [ ] Use an isolated branch/worktree for implementation; claim and guard exact writable paths.
- [ ] For a read-only audit, confirm the packet has no production owns and do not create a repair opportunistically.

## While working

- [ ] Run the public and native entrypoints with the same fixture/input/config and retain exact commands, exit/status, diagnostics, side effects, and artifact hashes.
- [ ] Invoke the private overlay only for one of its two enumerated behaviors; never use it to make a public row pass.
- [ ] Classify unavailable, skipped, timed-out, malformed, or SHA-mismatched evidence as open/blocked, never clean.
- [ ] Keep schema parity, fixture parity, behavioral parity, wiring parity, and production selection as separate claims.
- [ ] Stop at the first shared registry/schema/workflow/installer/manifest/status/cutover path or architecture decision outside the packet.
- [ ] Run Enforcer route before edits and the smallest exact-file/crate gate after each cohesive repair.

## Before DONE

- [ ] Diff contains only exact owns and no frozen-authority, vendor, proof-residue, or unrelated changes.
- [ ] Positive, negative, malformed, unavailable, and failure-exit cases are recorded where applicable.
- [ ] Evidence binds base/head/oracle/tree SHAs, tool versions, commands, config/fixture digests, run IDs, and artifact hashes.
- [ ] Manager independently reproduces the decisive result on the packet head.
- [ ] Report `proves` and `doesNotProve`; do not self-promote a matrix row or plan state.
- [ ] Release/close exact claims and send the boss the immutable evidence or repair commit.

## Immediate stop

Stop and send `BLOCKED` for ambiguous authority, missing comparable input, a required MJS mutation, any public pass that depends on the private overlay, a request for Node fallback, a rule downgrade/waiver, shared singleton ownership, or a cutover/merge/delete request.
