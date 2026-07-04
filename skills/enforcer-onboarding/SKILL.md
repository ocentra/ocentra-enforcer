---
name: enforcer-onboarding
description: Onboard a brand-new project onto the enforcer end-to-end — install, inspect the real build system, configure, wire CI, and verify the wiring actually fires. Use once, the first time the enforcer touches an unfamiliar repo, before day-to-day usage (skills/enforcer/SKILL.md).
---

# Enforcer Onboarding

<!-- ai-dense -->
```yaml
seven_steps: "install (mechanical, per-harness adapter) -> inspect (read real manifest) -> configure (author enforcer-config.json) -> scaffold .enforce/ (f02) -> wire CI (new or integrate) -> verify (seed-fail + clean-pass, MANDATORY) -> report"
use_once: "first time the enforcer touches an unfamiliar repo; day-to-day usage after verification = skills/enforcer/SKILL.md"
never_report_done_without: "step 6 BOTH seeded-violation-fires AND clean-baseline-passes observed"
```
<!-- /ai-dense -->

**The real user of this procedure is an AI agent, not a human.** A human can trigger each step
manually, but that is incidental, not the design target. This skill is deliberately NOT a script —
which profile fits, what languages are present, whether the project already has CI to integrate with
rather than replace, are judgment calls only an agent (AI or human) can make. This skill's job is to
make that judgment RELIABLE and REPEATABLE across projects, not to eliminate it.

Use this skill exactly once per project, the first time Enforcer is introduced. Once onboarding is
verified (step 6), switch to `skills/enforcer/SKILL.md` for day-to-day route/scan/check/proof
usage — that skill assumes a repo is already wired and does not re-teach onboarding.

Do not report onboarding complete after any step short of a passing step 6. File existence is never
sufficient evidence on its own.

## The seven steps

### 1. Install
Run the mechanical harness install for whatever AI harness is present (Claude, Codex, Cursor, or any
of the other adapters) — MCP server registration, skill/agent-descriptor copy, managed config blocks.
This is entirely mechanical; do not hand-edit what the installer already writes.

Command surface (per-harness adapters, c03/c06/c07/c08/c09 — covering all 11 harnesses; c01/c02 are the
shared install core and harness autodetection they build on):

```bash
enforcer install --root <repo> --profile strict --dry-run
enforcer install --root <repo> --profile strict
enforcer doctor --root <repo>
enforcer init --root <repo> --profile strict --adapters codex,mcp,precommit,github-actions --dry-run
```

Prefer `--dry-run` first, review the planned write set, then apply without it. Global install
(MCP registration, user-level skill copy) happens once per machine per harness; `--root <repo>` adds
the target-repo-specific wiring on top.

### 2. Inspect — read the target's real build system
Before choosing anything, READ the project's actual manifest file(s):

- Rust: `Cargo.toml` (workspace shape, member crates, existing `[workspace.lints]`).
- TypeScript/JavaScript: `package.json` (workspaces, scripts, existing lint/test tooling).
- Python: `pyproject.toml` / `setup.cfg` / `requirements*.txt`.
- Any other manifest the languages present imply.

Determine: which language(s) are actually present, whether it's a single package or a workspace/
monorepo, and — critically — whether the project ALREADY has CI (`.github/workflows/*.yml`, or the
equivalent for GitLab CI / CircleCI / Bitbucket / Jenkins). Never skip this step and never apply a
default profile/config blind — the point of inspecting first is that step 3's choices are INFORMED by
what you find here, not assumed.

### 3. Configure — author a fitting `enforcer-config`
Author (or update) the project's `enforcer-config.json` using the 3-layer model (arc-03,
`enforcer-config` crate): a base profile (`profileName`: `strict`, `default`, `ocentra-enforcer`, or
`ocentra-parent`) plus project-local overrides layered on top. This is a judgment call informed by
step 2, never a blind copy of another project's config:

- `profileName` — pick the closest-fitting base profile for this project's posture.
- `languages` — exactly the language families step 2 actually found (e.g. `["rust"]`,
  `["typescript", "python"]`) — never all families by default.
- `supportedPlatforms` — declare this ONLY when the project genuinely targets a narrower platform set
  than win/mac/linux; omitting it defaults to the strict all-three-platforms behavior (no silent
  relaxation by omission).
- Any other override the inspected build shape requires (roots, ignore globs, etc.).

Minimal example for a Rust-only project:

```json
{
  "schemaVersion": 2,
  "profileName": "strict",
  "languages": ["rust"]
}
```

A malformed config (missing `schemaVersion`/`profileName`, or an unknown `profileName`) fails to LOAD
with a typed error — this is intentional fail-closed behavior, not a bug to work around.

### 4. Scaffold `.enforce/`
Run the project onboarding/scaffold step (f02) to create the project's `.enforce/` working directory
(baseline, run registration). Where this step's own mechanism has not yet landed in your Enforcer
build, treat this as a named, explicit gap in the sequence — report it rather than silently skipping
straight to step 5, and note in your final report that step 4 was unavailable and why.

### 5. Wire CI — tailored to what step 2 found
Author the actual CI wiring for this specific project's real CI provider, using the languages/
platforms detected in step 2:

- If the project has NO existing CI: create a fresh workflow. For GitHub Actions targets, the
  `github-actions` install adapter's consumer-CI emitter writes the bundled `.github/workflows/*.yml`
  set (`enforcer`, `codeql`, `dependency-policy`, `secret-scan`, `sbom`) scoped to this
  project; other CI providers get their own native equivalent, hand-authored to match.
- If the project ALREADY has CI: INTEGRATE with it — add the enforcer step(s) into the existing
  pipeline rather than blindly overwriting what is already there. Never force-overwrite a
  pre-existing, unrelated CI config; a targeted `--force` re-apply of just the enforcer-owned files is
  fine, clobbering everything else is not.
- The zero-Rust-toolchain consumer-CI bootstrap (binary install script / composite GitHub Action /
  optional npm wrapper, c10) is the piece that lets the wired CI run `enforcer` without installing a
  Rust toolchain — reference it by name here; if it has not yet landed in your Enforcer build, the
  wired CI step may need to build from source or call an already-installed binary instead, and that
  substitution must be reported, not silently assumed away.

### 6. Verify — MANDATORY, never optional advice
This is the step most often skipped and the most important. Onboarding is not complete — and must
never be reported as complete — until BOTH of the following are true:

1. **Seed a known-bad case and confirm the gate actually fires.** Locate or introduce one concrete,
   real violation appropriate to this project (a rule the configured profile/languages actually cover)
   and run the wired check (the CI-equivalent local command, e.g. `enforcer scan --root <repo>
   --workspace` or the project's own CI job locally) against it. Confirm it exits non-zero and names
   the violation. If it does not fire, onboarding has FAILED — go back and fix the configuration, do
   not report done.
2. **Confirm a clean baseline passes.** Remove or fix the seeded violation and re-run the same check.
   Confirm a clean exit (zero violations, exit 0).

Only after BOTH the seeded-fail and the clean-pass are observed may the agent report onboarding
complete. A run that skips this step — or that only checks file existence — is an incomplete,
unverified onboarding and must be reported as such, never as done.

### 7. Report
Report back exactly what was set up (installed harness(es), the languages/profile chosen and why,
what CI wiring was added or integrated, and the concrete verification result from step 6 — the seeded
violation, its exit code, and the clean-pass exit code). A report with no attached verification result
is not a completion report.

## Cross-references

This skill stitches together, without duplicating, the mechanics owned elsewhere:

- Install (step 1): `c01`/`c02` (install core + harness autodetection) and the per-harness adapters
  `c03`/`c06`/`c07`/`c08`/`c09`.
- Configure (step 3): `arc-03` (`enforcer-config` crate, 3-layer resolution).
- Scaffold (step 4): `f02` (`.enforce/` onboarding scaffold).
- Wire CI (step 5): the `github-actions` install adapter / consumer-CI emitter (`c07`) for the
  per-project workflow files, and `c10` (release/binary-bootstrap pipeline) for the zero-toolchain
  consumer-CI install path.
- Day-to-day usage once onboarding is verified: `skills/enforcer/SKILL.md`.

This procedure is explicitly NOT eliminable by more automation. Steps 2, 3, and 5 require judgment;
step 6 is the one part that IS mechanically gated — never trust prose over its result.
