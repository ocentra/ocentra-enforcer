# arc-08 Crate enforcer-lang-py

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Crate enforcer-lang-py`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-lang-py/**`
- deps: `arc-01`, `arc-02`, `arc-05`
- tier: `P1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
Python-family rule detection lives in the generic/python scanner `.mjs` (`src`/`scripts` generic-scanner + python shape logic) as ad hoc JS. No crate implements the Python family against the `Validator` trait. The enforcer validates Python from Rust.

## Where We Want To Be
`enforcer-lang-py` is the per-family validator crate for Python: `Validator` impls (built on `enforcer-validator`) covering the Python rule family, each with fail/pass fixtures and a `cargo test` detection test.

## Rule inventory (per-prefix)
<!-- machine-inventory: crate=enforcer-lang-py source=rules/rules.json filter=language==python total=61 -->

Source of truth: `rules/rules.json`, filtered `"language": "python"` = **61 rules** across 6 PY-* prefixes. The arc-08 matrix historically enumerates only ~6 (PY-1/2/3); PY-4/PY-5/PY-6 ride a single generic bullet. This table homes every one. Each prefix ships fail/pass fixtures wired through the arc-05 `enforcer-validator` parity harness, and a `cargo test -p enforcer-lang-py` detection test.

| Prefix | Count | ruleIds | Validator(s) in rules.json | Backing source (.mjs to port) |
| --- | --- | --- | --- | --- |
| PY-1 | 3 | PY-1.1, PY-1.2, PY-1.3 | `python/source-scan` | `src` generic-python source scanner + `python/*` shape logic |
| PY-2 | 1 | PY-2.1 | `python/test-scan` | python test-shape scanner (`python/*` tests) |
| PY-3 | 2 | PY-3.1, PY-3.2 | `python/ruff-json`, `python/typecheck` | Ruff-JSON + Pyright/mypy toolchain adapters |
| PY-4 | 35 | PY-4.1 .. PY-4.35 | `python/source-scan` | `src` generic-python source scanner + `python/*` shape logic |
| PY-5 | 10 | PY-5.1 .. PY-5.10 | `generic-scanner` (5.1,5.2,5.3,5.4,5.7,5.8,5.9,5.10 = 8), `python/toolchain` (5.5,5.6 = 2) | `generic-scanner.mjs` (PY SLICE only) + python toolchain shape |
| PY-6 | 10 | PY-6.1 .. PY-6.10 | `python/source-scan` (6.1,6.3,6.4,6.5,6.6,6.7 = 6), `python/test-scan` (6.2 = 1), `python/tests` (6.8,6.9,6.10 = 3) | python test-shape + tests-required scanners |

Validator rollup (must equal 61): `python/source-scan` = 44 (PY-1×3 + PY-4×35 + PY-6×6); `python/test-scan` = 2 (PY-2.1 + PY-6.2); `python/ruff-json` = 1; `python/typecheck` = 1; `python/toolchain` = 2 (PY-5.5, PY-5.6); `python/tests` = 3 (PY-6.8/6.9/6.10); `generic-scanner` = 8 (PY-5 remainder). Sum = 61. ✓

Provable rows — each is a fail fixture that MUST trip and a pass fixture that MUST stay silent, run under the arc-05 parity harness:
- PY-1: `noqa` / `type: ignore` / `Alias = str` fail fixture trips; clean-suppression pass fixture silent.
- PY-2: `pytest.mark.skip` / `unittest.skip` fail fixture trips; enabled-test pass fixture silent.
- PY-3: Ruff-JSON diagnostic + Pyright/mypy diagnostic fail fixtures trip via the toolchain adapters; clean-toolchain pass fixtures silent.
- PY-4: representative fail fixtures across all 35 (`Any`, missing annotations, `except Exception`, `eval`, `shell=True`, `pickle.loads`, `dict[str, Any]`, mutable defaults, wildcard imports, naive `datetime.now()`, unfrozen dataclass, …) each trip; typed/guarded pass fixtures silent.
- PY-5: pyproject/ruff/type-config/strict/lockfile/pinning/git-dep/path-dep fail fixtures trip via the PY SLICE of `generic-scanner` (8) + structured-diagnostics fail fixtures via `python/toolchain` (2); conforming manifest pass fixtures silent.
- PY-6: weak-assert / empty-test / no-assert / monkeypatch / network-in-test / sleep-in-test / skip-xfail fail fixtures trip; required-negative/exception/property-test fail fixtures trip via `python/tests`; well-formed test pass fixtures silent.

Count-parity assertion: `cargo test -p enforcer-lang-py` includes a coverage test that loads every `language == "python"` ruleId from `enforcer-rules`, asserts a `Validator` impl is registered for each, and asserts the loaded count equals **61** — so a new PY rule added to `rules.json` fails the build until a validator + fixtures exist. This test owns the PY slice count; it does not assert anything about non-python rules.

### Shared-engine boundary (do NOT double-own)
`generic-scanner` is SHARED across `language == common` + `python` + `typescript`. arc-08 owns ONLY the **PY SLICE** — the 8 PY-5 rules whose validator is `generic-scanner`. The shared `generic-scanner` engine itself and the cross-language partition spec are owned by **arc-09** (common) per AUDIT_FINDINGS WAVE 3 MIS-MAP. arc-08 consumes the engine and registers only its PY-keyed rules; it must not port or re-own the engine, and must not claim any `common`/`typescript` `generic-scanner` rules (those go to arc-09 / arc-07). Partition contract lives in arc-09.

## Requirement Checklist
- [ ] Implement the Python-family `Validator` impls per RUST_ARCHITECTURE.md, keyed to their `RuleId`s in `enforcer-rules`.
- [ ] Port the corresponding `.mjs` Python detection logic (generic-scanner + python shape rules) to Rust validators.
- [ ] Cover every PY-* prefix in the Rule inventory (61 rules across PY-1/2/3/4/5/6), not just the ~6 in the matrix.
- [ ] Provide fail/pass fixtures per rule; wire them through the `enforcer-validator` (arc-05) parity harness.
- [ ] `cargo test -p enforcer-lang-py` passes: every validator fires on its fail fixture and is silent on its pass fixture, AND the count-parity test asserts a validator exists for all 61 `language == "python"` ruleIds.
- [ ] Register only the PY SLICE of `generic-scanner` (the 8 PY-5 rules); do not re-own the shared engine or partition spec (arc-09).
- [ ] Clean `cargo clippy` / `cargo fmt --check`.

## Acceptance And Proof
Tier P1. Proof row asserts `cargo test -p enforcer-lang-py` exits 0 with fail/pass fixture coverage per rule. Record the artifact path in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Owns only `crates/enforcer-lang-py/**`. Deps arc-01/02/05. Parallel-safe with all sibling lang crates (arc-06/07, arc-09..12) and arc-13/arc-19 — disjoint crate trees.
