# Research Grounding

This document records the bounded research and design sources behind the
native Enforcer. It is documentation only: it defines no validator, rule,
waiver, or runtime behavior. Executable rules, fixtures, tests, and retained
proof remain the authority for what is enforced.

## Borrowed ideas

1. **Context budgets are measured and ratcheted.** Always-on instructions and
   descriptions should have an observable budget and a reviewed ceiling rather
   than silently expanding with every feature [S1][S2].
2. **AST-backed checks beat prose-only assertions.** Parse source into bounded,
   typed facts and evaluate explicit predicates; prose explains the policy but
   is not the acceptance authority [S3][S4].
3. **Grandfather ratchets preserve an honest starting point.** Existing debt
   can be baselined, but growth beyond the baseline must remain visible and
   fail the gate [S1][S5].
4. **Deferred work is a visible state, not a silent skip.** A capability that
   is unavailable or dependency-blocked stays explicit, linked to its next
   gate, and cannot be promoted by a checklist alone [S6].
5. **Rules are structured data.** Stable identifiers, tiers, validators,
   fixtures, documentation anchors, and proof expectations are records that
   can be checked for parity and completeness [S4][S7].

## Sources

- **[S1]** [Enforcer ADBP gap inventory](plans/enforcer-selfhost-plan/ADBP_GAPS.md)
  — records the context-budget, baseline-ratchet, and deferred-work gaps that
  the Rust plan turns into explicit workpacks.
- **[S2]** [Lost in the Middle: How Language Models Use Long Contexts](https://arxiv.org/abs/2307.03172)
  — empirical motivation for treating context length as a measured resource,
  not an unbounded quality substitute.
- **[S3]** [Tree-sitter documentation](https://tree-sitter.github.io/tree-sitter/)
  — primary reference for incremental concrete-syntax parsing used to obtain
  reusable structural facts.
- **[S4]** [Rust architecture and rules-as-data contract](plans/enforcer-selfhost-plan/RUST_ARCHITECTURE.md)
  — defines typed facts, structured rule records, fixture parity, and the
  boundary between documentation and executable validation.
- **[S5]** [Product thesis and ratchet-first onboarding](PRODUCT_THESIS.md)
  — states the baseline-ratchet adoption model for brownfield repositories.
- **[S6]** [Plan execution blueprint](plans/enforcer-selfhost-plan/PLAN_EXECUTION_BLUEPRINT.md)
  — defines dependency-ordered work, explicit blocked/ready states, and the
  rule that a proof gate—not a checkbox—advances a pack.
- **[S7]** [Proof-harness migration authority chain](PROOF_HARNESS_MIGRATION.md)
  — makes Rust source/generated artifacts authoritative and requires mirrors
  and claims to be checked rather than trusted by assertion.

## Scope guardrail

The numbered sources ground design choices only. This file does not ship a
`Validator`, alter a rule registry, or authorize a workpack status change.
