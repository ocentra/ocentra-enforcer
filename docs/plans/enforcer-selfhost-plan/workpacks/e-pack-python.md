# e-pack-python Python FastAPI Layered And Clean-Arch Rule Family

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Python FastAPI Layered And Clean-Arch Rule Family`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling completion, product status, PR readiness, or broad DONE.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-lang-py/src/rules/fastapi_layered.rs`, `crates/enforcer-lang-py/tests/fixtures/fastapi_layered/**`
- deps: `arc-08`, `d01`, `d16`, `d22`
- tier: `P0/P1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md), [ADBP_GAPS](../ADBP_GAPS.md).

## Where We Are
The `enforcer-lang-py` crate (arc-08) covers only baseline Python language-hygiene validators; there is **no FastAPI-layered / clean-architecture pack**. The ADBP_GAPS rules-python and linters-python rows (layering/DI, enums, security) are unbacked: no layer-boundary checks, no DI discipline, no Python security CWE coverage. There is no `fastapi_layered` rules module in the crate, no `py-fastapi-*` `Validator` impls, and no `tests/fixtures/fastapi_layered/` tree. The enforcer VALIDATES the user's Python code from Rust (via tree-sitter-python symbol/AST analysis); it does not run in Python itself.

## Where We Want To Be
A layered/clean-arch `py-fastapi-*` family shipped as a `rules/fastapi_layered.rs` module inside `enforcer-lang-py`, scaffolded through d01 so every rule ships in 5-way parity (ruleId <-> doc-anchor <-> `Validator` impl <-> {fail+pass fixture} <-> `cargo test` detection test). Each rule is a Rust type implementing the `Validator` trait (from `enforcer-validator`, arc-05), performing symbol/AST-level analysis of the target's Python (tree-sitter-python) and emitting `enforcer-domain` `Finding`s + a `Fix:` hint — not substring matching. Rule records live as typed data in `enforcer-rules` (arc-04). T1 AST/symbol-level checks that block. FSM/enum semantics (StrEnum, transition maps, enum location) are CONSUMED from **d16** (the shared `enforcer-lang-common` FSM/enum validator), not re-implemented. Size/shape caps (nesting depth, catch-all utils) are CONSUMED from **d22** (the shared size/shape validator). This pack ships only the layering/DI + Python-specific structural and security rules.

## Requirement Checklist
Each rule is scaffolded via d01 with a `Validator` impl + fail-fixture + pass-fixture + `cargo test` detection test. Symbol/AST-level where noted.

- [ ] **no-repodep-in-routers** (`py-fastapi-no-repo-in-routers`, T1, symbol-level): a `routers/**` module referencing a `*Repository` symbol is flagged; router depending on a service stays clean.
- [ ] **no-session-in-services** (`py-fastapi-no-session-in-services`, T1): a `Session`/`AsyncSession` param or use inside `services/**` is flagged; service taking a repo stays clean.
- [ ] **no-transaction-in-services** (`py-fastapi-no-transaction-in-services`, T1): `commit()`/`begin()`/`session.rollback()` in `services/**` flagged; tx owned at boundary/unit-of-work stays clean.
- [ ] **no-models-in-services** (`py-fastapi-no-orm-models-in-services`, T1): importing ORM model classes into `services/**` flagged; service using domain DTOs stays clean.
- [ ] **no-sqlalchemy-in-routers** (`py-fastapi-no-sqlalchemy-in-routers`, T1): `from sqlalchemy` / `select(`/`.query(` in `routers/**` flagged; router delegating stays clean.
- [ ] **HTTPException-outside-handlers** (`py-fastapi-httpexception-location`, T1): `raise HTTPException` outside `routers/**` (e.g. in services/domain) flagged; raised only in routers stays clean.
- [ ] **no-repos-in-workflows** (`py-fastapi-no-repos-in-workflows`, T1): a `workflows/**` module using a repository directly flagged; workflow calling a service stays clean.
- [ ] **models-use-Mapped** (`py-fastapi-models-mapped`, T1): a SQLAlchemy model column not typed with `Mapped[...]` flagged; `Mapped[int]`/`mapped_column` stays clean.
- [ ] **StrEnum-only + enum-discipline** (via d16 `py.enum.strenum-only`/`py-fastapi-enum-location`, T1): `class X(Enum)` or non-StrEnum in `enums/` flagged; `class X(StrEnum)` under `enums/` clean. Consumed from d16.
- [ ] **domain-purity / domain-no-ui-exceptions** (`py-fastapi-domain-purity`, T1): `domain/**` importing FastAPI/HTTP or raising `HTTPException` flagged; pure domain stays clean.
- [ ] **no-sync-http** (`py-fastapi-no-sync-http`, T1): `requests.`/sync `httpx.Client` in async request path flagged; `httpx.AsyncClient`/`await` stays clean.
- [ ] **no-direct-repo-instantiation** (`py-fastapi-no-direct-repo-instantiation`, T1): `SomeRepository(...)` constructed inline flagged; injected via `Depends`/DI stays clean.
- [ ] **python security** (`py-fastapi-plaintext-password`, `py-fastapi-insecure-random-token`, `py-fastapi-cors-wildcard`, T1): plaintext password store/compare, `random.*` for a token, and `allow_origins=["*"]` flagged; bcrypt/argon2, `secrets.token_hex`, explicit origin list clean.

## Acceptance And Proof
All rules T1 (blocking). Per-rule fixtures under `crates/enforcer-lang-py/tests/fixtures/fastapi_layered/<ruleId>/{fail,pass}.py`; `Validator` impls in `crates/enforcer-lang-py/src/rules/fastapi_layered.rs`; rule records + doc-anchors in `enforcer-rules`. Detection test `cargo test -p enforcer-lang-py` iterates every fixture pair through each `Validator` (fail-flagged / pass-clean) and runs the d01 parity oracle over the family (every `py-fastapi-*` rule record resolves to a `Validator` + doc anchor + both fixtures). Symbol-level rules (repo-in-router, session-in-service) assert on resolved symbol references from the tree-sitter AST, not substrings, so a comment mentioning `Repository` in a pass fixture stays clean. Named proof rows in TEST_PROOF_EXPECTATIONS.md: `python-fastapi-layered-detection` and `python-fastapi-family-parity`.

## Parallel Ownership Notes
`owns:` is disjoint from every sibling AND from the arc-08 crate skeleton: this pack owns ONLY `crates/enforcer-lang-py/src/rules/fastapi_layered.rs` + `crates/enforcer-lang-py/tests/fixtures/fastapi_layered/**`, and `deps: arc-08` so it sequences after the `enforcer-lang-py` skeleton (Cargo.toml/lib.rs/module-root/Validator registration) exists. It is NOT a new crate — it is a rules module inside `enforcer-lang-py`. Enum/FSM semantics consumed from d16 (do not redefine the transition/enum engine here). Nesting-depth and catch-all-utils caps consumed from d22. Must not touch the crate's `Validator` registration root or `enforcer-rules` routing beyond the `py-fastapi-*` rule records d01's scaffolder writes for these ids; existing Python language-hygiene validators stay untouched.
