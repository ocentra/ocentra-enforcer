# c11 Onboarding Skill: Install, Inspect, Configure, Wire CI, Verify

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Onboarding Skill: Install, Inspect, Configure, Wire CI, Verify`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `skills/enforcer-onboarding/SKILL.md`, `crates/enforcer-install/tests/fixtures/onboarding_skill/**` (a dogfood fixture project + a distinct catfood fixture project)
- deps: `c01`, `c02`, `f02-onboard-and-autoindex`, `arc-03-enforcer-config`, `c10`
- tier: `P1/P5 (T3-labeled procedure + T1 verification gate)`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
**The real user of this engine is an AI agent, not a human** (owner directive, 2026-07-04) — a human can trigger the CLI manually from an IDE/shell, but that is incidental, not the primary design target. Today's mechanical primitives (`c01`-`c02` harness MCP registration, `f02` `.enforce/` scaffolding, `arc-03`'s 3-layer config resolution, `c10`'s CI bootstrap) each do ONE STEP correctly, but nothing STITCHES them into the sequence an agent actually needs when dropped into a brand-new, unfamiliar project: install -> inspect the project's real build system -> author a config that fits it -> wire CI for it -> verify the wiring actually works. The existing `skills/ocentra-enforcer/SKILL.md` teaches DAY-TO-DAY USAGE once a repo is already wired (route/scan/check/proof/coordination) — it does not teach onboarding a fresh project at all. This end-to-end onboarding sequence is NOT fully mechanically automatable: which profile fits, what languages are present, whether the project already has CI to integrate with rather than replace — these are judgment calls only an agent (AI or human) can make. The gap is that this judgment-requiring procedure is undocumented and unverified, so it happens ad hoc and inconsistently today.

## Where We Want To Be
A new skill, `skills/enforcer-onboarding/SKILL.md` (harness-neutral prose, shipped by the `c01` install payload to whichever harness reads it — Claude/Codex/Cursor/etc., same as the total-isolation doctrine snippet), that an AI agent follows step by step to onboard ANY project:
1. Run the mechanical install for the current harness (`c01`-`c09`: MCP registration).
2. **Inspect** the target project's real build system — read `Cargo.toml`/`package.json`/`pyproject.toml`/etc., detect languages, workspace shape, and any pre-existing CI to integrate with rather than blindly replace.
3. **Configure** — author the project's `enforcer-config` using `arc-03`'s 3-layer model: pick/declare a `profileName`, set `languages`, `supportedPlatforms`, and any overrides — a judgment call informed by step 2, never a blind default copy.
4. Run `enforcer onboard` (`f02`) to scaffold `.enforce/` + baseline.
5. **Wire CI** for this specific project using `c10`'s installer/GH-Action/npm-wrapper — author the actual `.github/workflows/*.yml` (or the target's real CI provider's equivalent) tailored to the languages/platforms detected in step 2.
6. **Verify** — the step most often skipped and most important: seed or locate a real violation and CONFIRM the wiring actually fires (the authored CI would genuinely fail on it), and confirm a clean baseline passes. Never report done on file-existence alone.
7. Report back what was set up, with the verification result attached.
This procedure is explicitly NOT eliminable-by-more-automation — the skill's job is to make agent judgment RELIABLE and REPEATABLE, not to remove it.

## Requirement Checklist
- [ ] Write `skills/enforcer-onboarding/SKILL.md` covering all 7 steps above, cross-referencing (not duplicating) `c01`-`c02` (install), `f02` (onboard), `arc-03` (config/profile resolution), and `c10` (CI bootstrap) by name/command, matching the existing `skills/ocentra-enforcer/SKILL.md`'s frontmatter + prose conventions.
- [ ] The skill explicitly instructs the agent to READ the target's actual manifest file(s) before choosing a profile/config — no blind default-profile application without inspection.
- [ ] The skill explicitly instructs the agent to author CI wiring appropriate to what it finds (integrate with existing CI where present; create fresh only when none exists) rather than always overwriting.
- [ ] The skill makes step 6 (verify) MANDATORY, not optional advice: it must name a concrete verification action (seed a known-bad fixture line, run the wired CI/local equivalent, confirm non-zero exit; then confirm a clean pass) before the agent may report onboarding complete.
- [ ] **Dogfood + catfood proof, not self-validation alone:** build TWO fixture projects under `tests/fixtures/onboarding_skill/` — a DOGFOOD fixture (this repo's own shape) and a CATFOOD fixture genuinely different in language/build system (e.g. a plain TypeScript-only or Python-only project with no prior enforcer awareness). An integration test drives the skill's steps (or a scripted equivalent of them) against BOTH fixtures and asserts each ends with a working, verified CI gate — proving the skill generalizes, not that it was secretly tailored to this repo's own shape.
- [ ] `cargo test -p enforcer-install --test onboarding_skill` (or equivalent) passes: dogfood fixture reaches a verified-working gate; catfood fixture reaches a verified-working gate; a fixture where step 6 is skipped is asserted to be an incomplete/failing onboarding (the verify step cannot be silently bypassed).

## Acceptance And Proof
Tier P1/P5, mixed T3 (the procedure itself is agent-executed prose, labeled per doctrine) + T1 (the verification gate in step 6 is mechanically asserted, not advisory). Proof row in TEST_PROOF_EXPECTATIONS.md requires BOTH the dogfood and catfood fixture runs to independently reach a verified-working CI gate; a run that skips the verify step fails closed. Record the artifact path.

## Parallel Ownership Notes
Owns only `skills/enforcer-onboarding/SKILL.md` and its own fixture/test tree — disjoint from `skills/ocentra-enforcer/SKILL.md` (day-to-day usage, unaffected) and from every c0x/`f02`/`arc-03`/`c10` file (this pack references, does not modify, their mechanics). Sequenced after `c01`, `c02`, `f02`, `arc-03`, and `c10` land (it stitches their already-built primitives together), but does not block any of them.
