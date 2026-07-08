# arc-09 Crate enforcer-lang-common

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Crate enforcer-lang-common`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-lang-common/**`
- deps: `arc-01`, `arc-02`, `arc-05`
- tier: `P1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
Language-agnostic rule detection (governance/manifest, docs/policy, shared cross-language checks) lives in `src/source-policy-common*.mjs` and `src/check-governance-*.mjs` as ad hoc JS. No crate implements the common family against the `Validator` trait.

The prior draft of this pack named only `source-policy-common*.mjs` + `check-governance-*` and therefore **omitted the contract-family dispatch/registry surface** that actually homes ~200 common rules. The audit (AUDIT_FINDINGS.md WAVE 3) flagged arc-09 as the single biggest rule-enumeration gap: `rules.json` carries **269 rules with `"language": "common"`**, of which this pack previously enumerated only a handful. The load-bearing sources for the contract families are:
- `src/checks.mjs` — the common-validator **registry/dispatch**; references every `common/*` contract validator id (proof/mcp/scanner/harness/repo/config/ci/docs/package/architecture/rule-coverage/…). This is the map from `RuleId` → validator and is the primary port target for the per-prefix inventory below.
- `src/checks-contracts.mjs` — contract-shape helpers consumed by the `common/*-contracts` validators (PROOF-1 / MCP-1 / SCAN-1/2 / HAR-1/2).
- `src/check-metadata.mjs` — governance/metadata checks backing REPO-1, SBOM-1, DEP-1, TEST-1/2, AI-1, SRC-shape, single-source-contracts.
- `src/check-docs.mjs` — documentation-completeness checks backing DOC-1 / DOCENF-1.
- `src/check-policy.mjs` — policy/architecture checks backing ARCH-1 / BOUND-1 / CFG-1.

These five files are ADDED to the port list (Requirement Checklist below) — they were previously unhomed.

### Naming-collision guard (do NOT drop these as "covered by the runtime crate")
`PROOF-1`, `MCP-1`, `HAR-1/HAR-2` are **RULE families that VALIDATE a target repo** (e.g. "does this repo's proof registry / MCP tool surface / harness run-capture wiring satisfy the contract?"). They are homed **HERE in arc-09** as `common/*-contracts` validators. They are DISTINCT from the runtime crates `enforcer-proof` (arc-17), `enforcer-mcp` (arc-21), `enforcer-harness` (arc-18), which IMPLEMENT those subsystems. A Rust port must not assume the runtime crate "covers" the rule family — the rule family is a validator over an arbitrary target repo and lives in this crate. Same-name, different-thing.

### Generic-scanner partition (no double-ownership, no dropped slice)
arc-09 owns the **shared `generic-scanner` engine** (`src/generic-common-scanner.mjs` / `generic-scanner-shared.mjs` / `generic-scanners.mjs`): the language-neutral line/AST scan machinery. The rules that RUN on that engine partition by `language`:
- `language: common` slice → **owned HERE (arc-09)**: the common `generic-scanner` rows enumerated below (GEN-2, SRC-2 tail, and — see SEC decision — the SEC-2 engine surface).
- `language: ts` slice → **arc-07** (enforcer-lang-ts) owns the *semantics*; consumes this engine.
- `language: py` slice → **arc-08** (enforcer-lang-py) owns the *semantics*; consumes this engine.

Rule: **the engine is single-owned by arc-09; the rule ROWS are owned by the per-language pack keyed on `language`.** arc-07/arc-08 depend on arc-09 for the engine and MUST NOT re-implement it; arc-09 MUST NOT enumerate the ts/py rows. This prevents both double-ownership of the engine and a dropped language slice.

### SEC-2 decision (explicit — arc-10 must not miss these)
`SEC-2` (**20 rules**, ids `SEC-2.1`..`SEC-2.20`) carry `language: common` and validator `generic-scanner` (16) / `generic-scanner-redaction` (1) / with the remaining SEC-2 rows in the security source policy. Because they are `language: common` they would silently fall to arc-09; because they are *security* semantics arc-10 (enforcer-security) is the natural semantic owner — the audit warns arc-10 will MISS them (they are NOT under `source-policy-common-security*` by validator name).
**DECISION (recommended, make it unambiguous):**
- **arc-10 (enforcer-security) OWNS the SEC-2 rule SEMANTICS** (the 20 SEC-2.x detection bodies + their fail/pass fixtures + count-parity). arc-10's checklist MUST explicitly claim `SEC-2.1..SEC-2.20`.
- **arc-09 OWNS the shared `generic-scanner` engine** that SEC-2 runs on (as above), and nothing else about SEC-2.
- arc-10 `deps: arc-09` for the engine. arc-09 does NOT enumerate SEC-2 rows in its count-parity set; arc-10 does. This split is called out here AND must be mirrored in arc-10 so SEC-2 is never dropped by either side.
  (Note: `SEC-1` = `common/secret-scan` (2) — a distinct family; not part of this SEC-2 handoff.)

## Where We Want To Be
`enforcer-lang-common` is the per-family validator crate for cross-language / common rules: `Validator` impls (built on `enforcer-validator`) covering the shared rule family (governance, manifest, docs/policy, common shape), each with fail/pass fixtures and a `cargo test` detection test.

## Requirement Checklist
- [ ] Implement the common-family `Validator` impls per RUST_ARCHITECTURE.md, keyed to their `RuleId`s in `enforcer-rules`.
- [ ] Port the corresponding `.mjs` detection logic (`src/source-policy-common*.mjs`, `check-governance-*`, docs/policy checks) to Rust validators.
- [ ] **Port the contract-family dispatch/registry sources (previously omitted):** `src/checks.mjs` (validator registry / RuleId→validator map), `src/checks-contracts.mjs` (contract-shape helpers), `src/check-metadata.mjs` (governance/metadata), `src/check-docs.mjs` (docs completeness), `src/check-policy.mjs` (policy/architecture). These back the per-prefix families enumerated in **Rule inventory (per-prefix)** below.
- [ ] Home every common-family prefix from the inventory table below as a `Validator` impl with fail/pass fixtures. Respect the generic-scanner partition and the SEC-2 handoff to arc-10 (do NOT enumerate ts/py rows or SEC-2 rows here).
- [ ] **`PORT-1.1` (platform-specific script commands must be guarded) gets a declared-scope RELAXATION it does not have today** (owner-identified gap, 2026-07-04): today this rule blanket-fails any platform-specific script regardless of what platforms the project actually targets. Read a `supportedPlatforms: ["windows"|"macos"|"linux", ...]` field from `enforcer-config`'s `EffectiveConfig` (arc-03 adds it) — a project that declares e.g. `["linux"]` only is NOT hard-failed for a Linux-only script; the rule fires ONLY on platform-specific code that falls outside the project's declared scope. Missing/absent `supportedPlatforms` defaults to ALL THREE (current strict behavior, no silent relaxation by omission). Fail/pass fixtures: declared `["linux"]` + a bash-only script passes; declared `["linux"]` + a Windows-only `.ps1`/`cmd` invocation with no cross-platform guard still fails; no declaration + any platform-specific script fails (unchanged default).
- [ ] Provide fail/pass fixtures per rule; wire them through the `enforcer-validator` parity harness.
- [ ] `cargo test -p enforcer-lang-common` passes: every validator fires on its fail fixture and is silent on its pass fixture.
- [ ] Clean `cargo clippy` / `cargo fmt --check`.

## Rule inventory (per-prefix)

Every `"language": "common"` family in `rules.json` (269 rules total), with its RuleId prefix, the validator id it dispatches through (`src/checks.mjs`), the count, and the backing `.mjs` source. Each row is provable: a fail fixture that trips the validator and a pass fixture that is silent. Prefixes marked **arc-10** / **arc-07/08** are called out for ownership but are NOT part of this crate's count-parity set (see notes under the table).

| Prefix (family) | Count | Validator id | Backing source | Owner | Fail fixture / Pass fixture |
|---|---|---|---|---|---|
| PROOF-1 | 15 | `common/proof-contracts` | `checks-contracts.mjs`, `checks.mjs` | **arc-09** | target repo missing/mismatched proof-registry contract fields / conformant `proofs.json` contract |
| MCP-1 | 12 | `common/mcp-contracts` | `checks-contracts.mjs`, `checks.mjs` | **arc-09** | MCP tool surface violates contract (missing tool/shape) / conformant MCP surface |
| SCAN-1 | 20 | `common/scanner-contracts` | `checks-contracts.mjs`, `checks.mjs` | **arc-09** | scanner-contract violation (SCAN-1.x) / conformant scanner wiring |
| SCAN-2 | 10 | `common/scanner-contracts` | `checks-contracts.mjs`, `checks.mjs` | **arc-09** | scanner-contract violation (SCAN-2.x) / conformant scanner wiring |
| HAR-1 | 1 | `harness/run-capture` | `checks.mjs` | **arc-09** | run-capture wiring absent / present (rule validates a target repo — NOT the arc-18 runtime crate) |
| HAR-2 | 15 | `common/harness-contracts` | `checks-contracts.mjs`, `checks.mjs` | **arc-09** | harness-contract violation / conformant harness contract (validates target repo — NOT arc-18) |
| ENF-1 | 15 | `common/rule-coverage` (13), `common/report-shape` (2) | `checks.mjs`, `check-metadata.mjs` | **arc-09** | rule-index coverage gap / full coverage; malformed report shape / valid shape |
| ENF-2 | 1 | `common/mutation-risk` | `checks.mjs` | **arc-09** | mutation-risk detected / none |
| DOCENF-1 | 10 | `common/docs-completeness` | `check-docs.mjs` | **arc-09** | required docs section missing / present |
| DOC-1 | 1 | `common/documentation` | `check-docs.mjs` | **arc-09** | documentation rule tripped / clean |
| CFG-1 | 12 | `common/config-lockdown` | `check-policy.mjs` | **arc-09** | config-lockdown violation (unpinned/mutable config) / locked-down config |
| CI-1 | 21 | `common/ci-integrity` (15), `ci-integrity` (6) | `check-policy.mjs`, `checks.mjs` | **arc-09** | CI-integrity violation (unpinned action, missing gate) / conformant CI |
| REPO-1 | 15 | `common/repo-governance` (14), `repo-governance` (1) | `check-metadata.mjs` | **arc-09** | repo-governance file missing/malformed / conformant repo |
| NPM-1 | 15 | `common/package-determinism` (7), `package-determinism` (5), `dependency-policy` (2), `sbom` (1) | `check-metadata.mjs`, `check-governance-*` | **arc-09** | non-deterministic/undeclared package deps / deterministic lockfile |
| ARCH-1 | 15 | `common/architecture` | `check-policy.mjs` | **arc-09** | architecture-boundary violation / conformant layering |
| BOUND-1 | 10 | `common/architecture` | `check-policy.mjs` | **arc-09** | module-boundary violation / conformant boundary |
| GEN-1 | 2 | `common/generated-artifacts` | `check-metadata.mjs` | **arc-09** | stale/committed generated artifact / clean |
| GEN-2 | 10 | `common/generated-artifacts` (4), `generic-scanner` (6) | `generic-common-scanner.mjs`, `checks.mjs` | **arc-09** (engine + common rows) | generated-artifact scan trips / clean |
| SRC-1 | 2 | `common/source-shape` | `source-policy-common.mjs` | **arc-09** | source-shape violation / conformant shape |
| SRC-2 | 15 | `source-shape-check` (7), `common/source-shape` (4), `generic-scanner` (3), `common/source-scan` (1) | `source-policy-common.mjs`, `generic-common-source-ownership.mjs` | **arc-09** | source-ownership/shape violation / conformant |
| LIT-1 | 9 | `common/literal-risk` | `generic-common-line-rules.mjs` | **arc-09** | risky literal detected (T2 threshold) / clean |
| WAIVER-1 | 10 | `common/waiver-policy` | `source-policy-common-policy.mjs` | **arc-09** (see a08) | dishonest/expired waiver / honest waiver |
| TEST-1 | 3 | `common/test-doubles`, `common/weak-assertions`, `common/skipped-focused-tests` | `source-policy-common-security-test-doubles.mjs`, `check-metadata.mjs` | **arc-09** | test double / weak assertion / focused-skip present / clean tests |
| TEST-2 | 2 | `common/required-tests` | `check-metadata.mjs` | **arc-09** | required test absent / present |
| DEP-1 | 2 | `common/dependency-policy` | `check-governance-dependency.mjs` | **arc-09** | disallowed dependency / allowed |
| CONTRACT-1 | 1 | `common/single-source-contracts` | `check-metadata.mjs` | **arc-09** | duplicated contract source / single source |
| SBOM-1 | 1 | `common/sbom` | `check-governance-sbom.mjs` | **arc-09** | SBOM missing/stale / present |
| AI-1 | 1 | `common/ai-rule-index` | `check-metadata.mjs` | **arc-09** | AI rule-index out of sync / in sync |
| PORT-1 | 1 | `common/portability` | `source-policy-common.mjs` | **arc-09** | unguarded platform-specific script outside declared `supportedPlatforms` / guarded-or-in-scope (see checklist relaxation) |
| SEC-1 | 2 | `common/secret-scan` | `source-policy-common-security.mjs` | **arc-09** | secret pattern detected / clean |
| **SEC-2** | **20** | `generic-scanner` (16), `generic-scanner-redaction` (1), `common/security` (3) | `source-policy-common-security-rules.mjs`, `generic-common-scanner.mjs` | **arc-10 (semantics)** / arc-09 (engine only) | SEE SEC-2 decision above — homed in arc-10, NOT counted here |

**Count reconciliation.** 269 common rules total. This crate (arc-09) owns count-parity for **249** of them (269 − 20 SEC-2 delegated to arc-10). SEC-2's 20 rows are owned+proved by **arc-10** (which `deps: arc-09` for the shared `generic-scanner` engine). The `generic-scanner` engine is single-owned by arc-09; the `ts`/`py` slices of that engine's rules are owned by **arc-07 / arc-08** respectively and appear in THIS table only to fix the partition, not to be enumerated here.

- [ ] **Count-parity assertion:** `cargo test -p enforcer-lang-common` includes a parity test that reads `rules.json`, selects every rule with `language == "common"` MINUS the SEC-2 family (delegated to arc-10), and asserts each remaining RuleId (all 249) has a registered `Validator` impl with both a fail and a pass fixture. Missing/extra RuleId fails the test. arc-10's suite asserts the mirror set for SEC-2.1..SEC-2.20.

## Acceptance And Proof
Tier P1. Proof row asserts `cargo test -p enforcer-lang-common` exits 0 with fail/pass fixture coverage per rule AND the count-parity test (every `language==common` RuleId except delegated SEC-2 has a validator + fixtures). Record the artifact path in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Owns only `crates/enforcer-lang-common/**`. Deps arc-01/02/05. Parallel-safe with all sibling lang crates (arc-06..08, arc-10..12) and arc-13/arc-19 — disjoint crate trees.
