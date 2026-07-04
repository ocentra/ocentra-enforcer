# Testing Constitution + Mutation Testing + Rust Architecture — ingested rule sources (distilled)

Three ingested project rule docs (generic; neutral naming — no product branding). Mechanization inputs for
Track I (test-writing standards + mutation) and a Rust-architecture augmentation. Overlap with existing
rules.json (RR-*, TS-*, TEST-*, d17/d23/h04/a09) is RESOLVED BY PARITY SWEEP — mechanize the DELTA only.
Sibling still missing: ocentra-security-rules.mdc (numbered guarantees G1-G5 / Rules 0.1.1-15.x).

## A. TESTING CONSTITUTION (test-writing standards — generic, all React/Node/TS)
- Test classes (§0.1): every test is exactly ONE of Correctness / Security / Regression; mixing forbidden.
  Regression must cite an incident/CVE/bug id. Security must map to a guarantee+threat.
- Proof standards (§0.2, §20): a test counts only if it fails-when-protection-removed, asserts exact behavior
  (no weak matchers), deterministic (logged seed), isolated (no shared mutable state), documents what failure means.
- Fix-code-not-tests (§1): tests are the spec; never weaken a test to pass — fix the code.
- Prerequisites vs behavior (§2.1): test the ACTUAL behavior, not the prerequisite (don't test CORS/auth/rate-limit
  inside a business-handler test; prerequisites via shared setup helpers; test each prerequisite in its own suite).
- TS strictness (§3): strict:true; TS errors = test failures; no any/unknown/assertions-to-silence; model invalid
  states unrepresentable.
- TDD (§4): RED first (must have seen it fail) -> minimal code -> GREEN -> refactor-without-changing-tests.
- BDD naming (§5): behavior sentences ("returns 401 when token missing"), not function names / internals.
- Must-fail-when-broken (§6): every test verified to fail when impl is broken; else delete it.
- No fake tests / banned weak assertions (§7): FORBID toBeTruthy/toBeFalsy/toBeDefined/not.toThrow/expect(true).toBe(true)/
  broad toMatchObject-without-strict/arrayContaining([])-alone; assert exact values, error types+messages+codes, side effects.
- Happy/error/abuse (§8): every feature has happy + error(4xx) + abuse(malicious) paths; "no crash" != success.
- Mocking discipline (§10) — CRITICAL 3-way distinction:
  EMULATION (preferred; real platform semantics locally, e.g. wrangler/miniflare) is NOT mocking;
  SUBSTITUTION (unit tests only; deterministic contract-bound impls of an interface, no shortcutting) is NOT mocking;
  MOCKING (restricted): mocks MUST NEVER determine business outcomes; never mock business-logic/economic/authz/validation;
  mocks only to assert a dependency was called / side effects. Coverage via mocked logic is INVALID (§10.5).
  Logic modules must be pure/stateless (no cache/memoization/global state). Handlers are thin adapters.
- Determinism & isolation (§11): shared setup helpers; reset state; control time/randomness/env; randomness is an
  injectable dependency, never global; production randomness never stubbed in logic tests (hides replay/collision).
- Interface immutability (§13.1): logic-facing interfaces are SECURITY BOUNDARIES; must be total (no optional/null
  returns unless documented); no silent defaults/fallbacks; interface change = security-boundary change.
- Error preservation (§16.1): logic must NOT swallow/normalize errors (no catch->{success:false}); different failure
  causes stay distinguishable; logic returns data OR throws, not both; only handlers translate to HTTP.
- Async ordering (§16.2): order of awaits in logic is security-critical; AI must NOT introduce Promise.all / reorder /
  parallelize storage ops; concurrency changes need explicit human approval.
- Semantic preservation (§16.3): refactors must not change observable behavior; invariant/equivalence tests required.
- Coverage integrity (§17, §10.5): coverage is signal not goal; coverage-increase-without-new-assertions = INVALID;
  trivial coverage-only tests forbidden.
- Test-layer separation (§18.1): integration tests and logic tests are BOTH mandatory; one never replaces the other.
- Snapshot restrictions (§12): snapshots forbidden for logic/security/economics; only static HTML/structure.
- No-temporary-security (§15.1) + no-security-DRY (§15.2): no TODO/temp/commented-out security; security checks may be
  duplicated (duplication preferred to over-abstraction that creates a single point of failure).
- AI override (§19): AI must prefer failing over passing when unsure, write strict assertions, never downgrade for green.

## B. MUTATION TESTING (precision instrument, opt-in)
- Opt-in only (§1,§2): NO implicit mutation targets; only symbols annotated `@mutation(...)` are eligible; mutation
  without decorator = violation.
- Function-local scope (§2.1): mutate ONLY the annotated symbol; never whole files / siblings / helpers / shared deps.
- Allowed targets (§3): deterministic business logic with DECISIONS carrying stateable invariants (economic calc,
  authz, scoring, state machines, rule engines, matching, multi-branch validation). Same-input->same-output, no IO/time/random.
- Forbidden targets (§4): regex, URL/route/path builders, constants/config/enums/flags, UI/React/JSX/hooks, adapters/
  glue/HTTP-handlers/DTO-mappers/serialization, simple pass-through/identity/one-line-wrappers/accessors.
- Invariant-first (§5,§6,§7): mutation valid only if explicit falsifiable invariants exist (state/decision/ordering/
  conservation). ANTI-invariants (forbidden): "does not crash", "returns something", "is defined", truthy/falsy,
  snapshot equality, coverage increase.
- Surviving mutant (§9): a survivor = missing invariant OR invalid target; VALID response = add invariant assertion /
  strengthen / split branches / reject scope; INVALID = disable/lower-threshold/accept-without-reasoning/weak-assert.
- Scope binding (§9.1): run ONLY the tests asserting the mutated target's invariants; never the full suite; never fix
  unrelated tests to kill a mutant. (Stryker coverageAnalysis: perTest.)
- AI enforcement (§11): check for @mutation, refuse mutation without invariants, ask before expanding scope, prefer
  under- to over-mutation; never mutate UI/glue or invent invariants. If uncertain -> STOP AND ASK.

## C. RUST ARCHITECTURE GUIDELINE (RAG) — augment existing RR-* (mechanize the delta)
- Ownership/clone (1.1-1.3, 12.1): justify every .clone() (prefer borrow / Arc / iterate-by-ref); no move-in-loop of
  a still-needed value; move large data (cheap) vs deep-copy; Copy only for small POD.
- Borrowing/lifetimes (2.x, 3.x): &str params over String; explicit lifetimes when >1 input ref; struct lifetime
  annotations for stored refs; 'static sparingly.
- Memory safety (4.x): Option<T> not null; RAII/Drop for resources; right smart pointer (Box/Rc/Arc/RefCell/Mutex/RwLock).
- Error handling (5.x, 12.2): NO .unwrap()/.expect()/panic! in production paths (tests/examples OK); Result + ? ;
  thiserror for libs / anyhow for bins; preserve cause; custom error enums.
- Concurrency/async (6.x,7.x): message-passing (mpsc) preferred; RwLock for read-heavy; NO blocking in async (spawn_blocking);
  select! for cancellation.
- Type system (8.x, 13.3): newtype pattern for domain primitives (UserId not i32); make impossible states unrepresentable
  (enum states over boolean clusters); phantom types; no stringly-typed APIs; associated types.
- Traits/generics (9.x): traits for polymorphism; trait objects for open sets; derive std traits.
- Macros (10.x): prefer functions over macros.
- Unsafe/FFI (11.x): minimize+isolate unsafe behind safe API; every unsafe has // SAFETY: comment; FFI #[repr(C)],
  no-panic-across-FFI (catch_unwind), libc/c_char types.
- Anti-patterns (13.x): no RefCell overuse; don't fight borrow-checker (redesign); no stringly-typed; no iterator invalidation.
- Module org (14.x): domain-driven structure; clear pub/pub(crate)/private boundaries.
- Perf (15.x): zero-cost abstractions; #[inline] judiciously on small hot fns; Cow/SmallVec/cache-friendly; SIMD w/ fallback.
- Best practices (16.x): naming conventions; doc all public APIs with runnable examples (# Examples/# Panics/# Errors).
- Deps/security/CI (18.x-19.x, 21.x): cargo fmt --check + clippy -D warnings + test + audit + deny/geiger in CI;
  pin rust-toolchain.toml; sanitize inputs; serde-safe deserialization; OsRng for crypto randomness; no-std when possible.
- No catch-all utils/helpers module (also in ADBP): >50-line utils must split into responsibility-named modules.
