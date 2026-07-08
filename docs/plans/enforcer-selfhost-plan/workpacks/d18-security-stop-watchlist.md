# d18 Security Stop Watchlist

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Security Stop Watchlist`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling completion, product status, PR readiness, or broad DONE.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-lang-security/src/rules/security_stop.rs, crates/enforcer-lang-security/tests/fixtures/security_stop/**`
- deps: `d01, arc-10, arc-19, arc-05, arc-04`
- tier: `P0`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [ADBP_GAPS](../ADBP_GAPS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
The `enforcer-rules` registry has generic entropy/secret rules and python-shaped eval/pickle bans, but the real-time CWE STOP watchlist ADBP mandates (rows 68-81, 103 of ADBP_GAPS.md) is not backed as a deterministic pattern/AST `Validator` family: no SQL-injection f-string ban, no path-traversal canonicalization gate, no weak-crypto ban, no CSPRNG-for-tokens gate, no constant-time-compare gate, no plaintext-password ban, no CORS-wildcard ban, no error-leak-to-client ban, no untrusted-deserialize/shell-exec gate, and no frontend XSS/localStorage-token/`NEXT_PUBLIC_`-secret bans. (These detect vulnerabilities in the user's target-language code.)

## Where We Want To Be
A cross-language security STOP `Validator` family in `enforcer-lang-security` (arc-10) — one `src/rules/security_stop.rs` module, fixtures `crates/enforcer-lang-security/tests/fixtures/security_stop/**` — scaffolded via d01 (arc-14) with full 5-way parity, with the threat-mapped rule records (CWE/OWASP `ThreatId` newtypes) carried in `enforcer-rules` (arc-04) alongside the Track H `enforcer-security` (arc-19) money-critical/security surface. Each rule is a `Validator` impl (built on the `enforcer-validator` trait) that parses the target language with the right frontend — `tree-sitter`/`swc` for Python/TS/JS/TSX, `syn` for Rust — and emits structured `Finding`s carrying the CWE `ThreatId` plus a terse `Fix:` hint. These are deterministic pattern/AST detections and block (T1).

## Requirement Checklist
Each rule is scaffolded via d01, landing a doc-anchor in its `enforcer-rules` record, a `Validator` in `src/rules/security_stop.rs`, and a fail+pass fixture pair under `crates/enforcer-lang-security/tests/fixtures/security_stop/<rule>/{bad,good}/`.

- [ ] **T1 py.sec.sql-injection-fstring — SQL injection (CWE-89).** fail `cursor.execute(f"... {table}")` / `.format()` / interpolation into raw SQL; pass parameterized `execute(sql, params)`.
- [ ] **T1 SEC-PATH-TRAVERSAL / py-fastapi-path-traversal / SEC-PATH-1.1 — path traversal.** fail `File::open(base.join(user_input))` / `open(f"/data/{name}")` with no canonicalization; pass resolve + assert `starts_with(base)` / `is_relative_to(BASE_DIR)`.
- [ ] **T1 SEC-WEAK-HASH — weak crypto.** fail `Md5::new()` / `Sha1` / `hashlib.md5` for security; pass `Sha256`+ / AES-256 / vetted crate.
- [ ] **T1 SEC-CSPRNG-TOKENS / py-fastapi-insecure-random-token — CSPRNG for tokens.** fail `rand::thread_rng()` / `random.choices()` for a token/key; pass `OsRng`/`getrandom`/`secrets.token_hex`.
- [ ] **T1 SEC-CONST-TIME-CMP / py-fastapi-nonconstant-token-compare — constant-time secret compare.** fail `secret == input` / `if token == stored:`; pass `subtle::ConstantTimeEq` / `hmac.compare_digest`.
- [ ] **T1 py-fastapi-plaintext-password — passwords hashed bcrypt/argon2.** fail password stored/compared as plaintext str; pass `bcrypt`/`argon2` hash.
- [ ] **T1 py-fastapi-cors-wildcard — no `allow_origins=["*"]`.** fail `CORSMiddleware(allow_origins=["*"])`; pass explicit origin list.
- [ ] **T1 py.sec.hardcoded-secret-formats / SEC-HARDCODED-SECRET — token-prefix secret formats (CWE-798).** fail `key = "sk-..."` / `ghp_...` / `aws_...` / `password=` literal; pass `os.environ["KEY"]` / `String.fromEnvironment`.
- [ ] **T1/T2 SEC-ERR-LEAK-CLIENT / py-fastapi-exception-str-in-response / CF-SEC-3.1 — no stack trace / exception str to client (CWE-209).** fail returning `format!("{err:?}")` / `str(e)` / `cfcatch.tagContext` to client; pass generic client msg, detail logged. (T1 for the direct `str(e)`-in-response shapes; T2 where the response path is inferred.)
- [ ] **T1 SEC-UNTRUSTED-DESER / SEC-SHELL-1.1 — no untrusted deserialize / shell-string exec.** fail `yaml.load(...)` / `pickle.loads(untrusted)` / `Command::new("sh").arg("-c", user)`; pass `TryFrom`-validated at edge / arg-vector, no shell.
- [ ] **T1/T2 RUST-SEC-CHECKED-ARITH / SEC-ARITH-1.1 — checked arithmetic on untrusted input.** fail `let n = a + user_input;` on attacker math; pass `a.checked_add(user_input)?`. (T1 where operand is a tainted symbol; T2 otherwise.)
- [ ] **T1 TSX-XSS-1.1 — no `dangerouslySetInnerHTML` without sanitization.** fail `dangerouslySetInnerHTML={{__html: user}}`; pass DOMPurify-sanitized value.
- [ ] **T1 TSX-TOKEN-1.1 — no tokens in `localStorage`.** fail `localStorage.setItem('token', ...)`; pass httpOnly cookie.
- [ ] **T1 TSX-ENVPUB-1.1 / FE-CFG-1.2 — no secrets under `NEXT_PUBLIC_`/`VITE_`.** fail secret assigned to `NEXT_PUBLIC_*`/`VITE_*`; pass server-only env.
- [ ] **T2 py.sec.supply-chain-setup-py — supply-chain `setup.py`.** fail `setup.py` with `os.system(...)`/`subprocess`/exfil import/typosquat dep (score over threshold); pass clean `setup.py`/`pyproject`.
- [ ] **T2 SEC-SCA-CARGO-AUDIT / CIGATE-1.5 — target-project dep-CVE audit gate.** fail target project with no cargo-audit/pip-audit/pnpm-audit gate in CI (score over threshold); pass dep-audit gate present.
- [ ] **T1 (prompt-assembly) / T3 (agent behavior, labeled) SEC-STOP-GATE / DISP-1.1 — STOP-on-Vulnerability real-time gate.** T1: the SECURITY STOP watchlist block must be present verbatim in every assembled implementation-subagent dispatch prompt — fail dispatch prompt missing Block 1, pass prompt contains it verbatim (validator scans the assembled prompt text). T3 (advisory, no mechanization possible + the subagent's runtime honoring of the interrupt is an LLM-runtime behavior no static check can observe): labeled prose; the presence of the label is itself T1-enforced.

## Acceptance And Proof
Tier P0. Prove via `cargo test -p enforcer-lang-security`. The T1 rules are deterministic pattern/AST detections and block; the two audit/supply-chain rows are T2 scored (fixtures assert the score crosses the fail threshold and stays under it on pass). The only T3 residue is the subagent runtime-honoring of the STOP interrupt, carried with the label `advisory, no mechanization possible + runtime LLM behavior is unobservable to a static gate`; the label's presence is T1-enforced and the prompt-assembly half of SEC-STOP-GATE is fully T1 (a `Validator` scans the assembled dispatch-prompt text). For every ruleId the fail-fixture is flagged and the pass-fixture stays clean under its `Validator` impl in `src/rules/security_stop.rs`; the detection test asserts both. Re-run the d01 `rule-scaffold-parity` oracle and record detection-test artifact paths in TEST_PROOF_EXPECTATIONS.md.

Representative triples (fixtures are target-language sample code parsed by the Rust `Validator`; the STOP-gate fixture is prompt text):
- py.sec.sql-injection-fstring: fail `crates/enforcer-lang-security/tests/fixtures/security_stop/sqli/bad/fstring.py`, pass `.../good/parameterized.py`, `#[test] sec_sqli`.
- SEC-WEAK-HASH: fail `crates/enforcer-lang-security/tests/fixtures/security_stop/weak_hash/bad/md5.rs`, pass `.../good/sha256.rs`, `#[test] sec_weak_hash`.
- SEC-STOP-GATE (prompt-assembly T1): fail `crates/enforcer-lang-security/tests/fixtures/security_stop/stop_gate/bad/prompt_missing_block1.txt`, pass `.../good/prompt_with_block1.txt`, `#[test] sec_stop_gate`.

## Parallel Ownership Notes
Owns `crates/enforcer-lang-security/src/rules/security_stop.rs` and `crates/enforcer-lang-security/tests/fixtures/security_stop/**` exclusively; disjoint from siblings. Lands inside the `enforcer-lang-security` crate whose skeleton arc-10 owns — must not edit that crate skeleton or the `enforcer-security` (arc-19) money-critical surface; the CWE/OWASP `ThreatId` rule records are added through d01's scaffolder into `enforcer-rules` (arc-04), not by hand. Depends on d01, arc-05 (Validator trait), arc-04 (rule records). Distinct from the Rust error-handling pack (d17) and the FSM pack (d16); the frontend XSS/token/env rows here are the security-only slice and must not overlap the broader `FE-*` family owned by e-pack-frontend-react. The SEC-STOP-GATE prompt-assembly rule reads the dispatch-prompt surface owned by d26-dispatch-prompt-assembly (DISP-1.1, in `enforcer-coordination`); this pack owns only the SECURITY-STOP block content/presence `Validator`, d26 owns block ordering and must not be edited from here.
