# UL01 - Shape-Driven Doctrine

<!-- agent-capsule -->
> Agent Capsule
> Plan: `universal-language-enforcement-plan`
> Doc: `UL01 Shape-Driven Doctrine`
> Kind: architect-owned contract workpack.
> Read when: UL00 truth is accepted and the boss assigns this contract.
> Stop rule: do not bulk-edit language rules or change the shipped default posture in this packet.
> Proves: requirement/framework/profile separation and typed resolution.
> Does not prove: framework detection accuracy or universal language coverage.
> Proof rule: the same recognized shape must receive different verdicts only through selected profile data.
<!-- /agent-capsule -->

- owns: `crates/enforcer-domain/src/doctrine_profile_types.rs`, `crates/enforcer-config/src/doctrine_profile.rs`, `crates/enforcer-config/profiles/doctrine/**`, `crates/enforcer-config/tests/doctrine_profile.rs`, `crates/enforcer-config/tests/fixtures/doctrine_profile/**`
- deps: `UL00`
- tier: `P0 contract, P1 resolver`

> Owner class: Sol/architect; Luna may add fixtures after the contract lands.
> Batch limit: one typed requirement/framework resolver and shipped profiles.

## Where We Are

The live `FE-EFFECT-1.1` path bans Zod through hard-coded source markers. Planned `p01` correctly identifies the distinction but has not landed. Existing JSON profiles do not model accepted framework families per requirement.

## Where We Want To Be

Universal doctrine requires validated boundary shapes and explicit domain identities. Framework family recognition is separate. The selected project profile decides whether Effect, Zod, Valibot, Pydantic, attrs validators, serde/newtypes, or another registered family satisfies a requirement.

## Owns

- new typed requirement, framework-family, verdict, severity, owner/reason, and profile values;
- parse-at-boundary profile loader/resolver and dedicated profile fixtures;
- shipped doctrine profiles under a new exact directory;
- no existing language validator in this packet.

## Objective

Land a closed, language-aware resolver returning `accepted`, `rejected`, or visible `requirement-disabled`. Preserve the owner's Effect-preferred default as data while making library neutrality a valid explicit profile choice.

## Requirement Checklist

- [ ] Malformed requirement/family/language combinations fail at decode with field-specific typed errors.
- [ ] Disabling or weakening a requirement requires owner and reason.
- [ ] Rule code cannot construct a framework verdict without the resolver.
- [ ] The same Zod shape is rejected by an Effect-only profile and accepted by a Zod/permissive profile without source changes.
- [ ] Effect remains the configured default where current project behavior requires compatibility.
- [ ] Profiles round-trip without losing toggles, severity, owner, or reason.
- [ ] No blanket waiver or inline bypass is introduced.

## Acceptance And Proof

Run `cargo test -p enforcer-config --test doctrine_profile`, config crate checks/clippy, profile negative fixtures, and scoped Enforcer gates. Record the exact profile and source SHA in proof.

## Stop conditions

Stop if the contract embeds library strings in rule logic, silently changes default findings, or requires editing existing validators before the resolver is independently proved.

## Parallel Ownership Notes

Only the doctrine integrator edits types/resolver/shipped profiles. Fixture-only children may work on disjoint directories after the schema is frozen.
