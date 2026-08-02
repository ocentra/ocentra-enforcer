# CyberSkills Parity Plan

<!-- agent-capsule -->
> Plan: `cyberskills-parity-plan`
> Purpose: turn the 817-skill vendor corpus into truthful, deterministic Enforcer coverage while reusing the Rust grammar and graph parity already present in `enforcer-memory`.
> Authority: this plan refines the broad `h11` and `h12` workpacks; it does not silently mark either one complete.
> Completion rule: all 817 tracked source identities are accounted for; every available-source enforcement claim has executable evidence; `sourceUnavailable` is explicit and never counted as covered; and the terminal dogfood gate is green on the exact source SHA.
<!-- /agent-capsule -->

## Outcome

The vendor corpus is knowledge for AI and humans. It is not executable policy by itself. Every skill is decomposed into one or more independently evidenced components:

| Component kind | Owner | Meaning |
|---|---|---|
| `native-predicate` | Rust Enforcer | Deterministic T1 or T2 behavior over typed input or syntax facts |
| `external-engine` | Third-party tool behind a Rust-owned adapter | The mature engine performs the specialist analysis; Enforcer controls execution and gates its typed output |
| `advisory` | Retained vendor knowledge | Useful guidance that does not decide pass/fail |
| `manual` | Explicit human procedure | Judgment or environment-dependent work that cannot honestly be mechanized |

One skill may contain all four. `covered` means every declared component from an available source has an honest disposition. It does not mean every component is a blocking Rust rule. A tracked identity with unavailable source is `sourceUnavailable`, not covered, retained, decomposed, implemented, or proved.

## Confirmed Baseline

These are the live values observed on `rust-build` at plan creation, not completion claims:

| Measure | Baseline |
|---|---:|
| Tracked vendor skill directories | 817 |
| Readable `SKILL.md` files | 816 |
| Source-unavailable identities | 1: `detecting-fileless-malware-techniques`, tracked blob `df48fa4149dd25956e730443d3582693a3f825a8` |
| Original triage | 145 T1, 137 T2, 399 adapter, 136 prose |
| Current disposition manifest | 6 native, 278 unported, 398 adapter-deferred, 135 advisory-prose |
| Native CyberSkills rule records | 41 |
| Formally source-linked native mappings | 6 |
| Generic adapter gates | 1 |
| Live third-party runners | 0 |

The original triage is an inventory aid, not an architecture. In particular, `399 adapter` must not produce 399 shallow wrappers.

## Read Order

1. [AGENTS.md](./AGENTS.md)
2. [PLAN_STATE.md](./PLAN_STATE.md)
3. [ARCHITECTURE.md](./ARCHITECTURE.md)
4. [WORKPACK_INDEX.md](./WORKPACK_INDEX.md)
5. [WORKER_CHECKLIST.md](./WORKER_CHECKLIST.md)
6. The one assigned workpack
7. The matching row in [TEST_PROOF_EXPECTATIONS.md](./TEST_PROOF_EXPECTATIONS.md)

Luna additionally reads [LUNA_RUNBOOK.md](./LUNA_RUNBOOK.md).

## Non-negotiable Safety

- The protected deletion at `vendor/anthropic-cybersecurity-skills/skills/detecting-fileless-malware-techniques/SKILL.md` is `sourceUnavailable` with tracked blob `df48fa4149dd25956e730443d3582693a3f825a8`; it is never restored, staged, committed, or discarded without an explicit owner decision.
- Existing compilers, analyzers, linters, security engines, and package auditors are reused first through the shared allowlisted tool-adapter contract. Native predicates exist only for a recorded semantic gap.
- `cyberskills-ledger-integrator` is the sole writer of the CyberSkills disposition ledger. `tool-adapter-integrator` is the sole writer of the shared adapter registry, runner, and normalized tool-result schema.
- The frozen MJS checkout remains comparison authority until Rust cutover proof is complete.
- Workers never merge to `rust-build` or `main`; the boss integrates accepted checkpoints.
- No AI classification is accepted as pass/fail evidence. AI proposes; schemas, fixtures, validators, and Enforcer gates decide.
- No parser, grammar, or external process is duplicated merely to make one security rule convenient.
