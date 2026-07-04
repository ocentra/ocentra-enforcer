# Parity gaps — rules-rust-shared

Delta only: ADBP normative points with NO or PARTIAL backing in `rules/rules.json`. Fully-backed points (unwrap/expect/panic/todo!/unimplemented!/unreachable!, thiserror-vs-anyhow choice, `#[from]`/`#[source]` cause preservation, `unwrap_or_default`, file/function line budgets, blocking-in-async + await-while-holding-lock, `get_unchecked`/unchecked indexing, TODO/FIXME/HACK markers, lockfile pinning, cross-platform CI gates, clock/timezone, newtype/AsRef/Display domain params) are omitted.

| ADBP point | ADBP source | Backed? | Tier | Proposed ruleId | Fail-fixture | Pass-fixture |
|---|---|---|---|---|---|---|
| Public error enums MUST use `#[non_exhaustive]` | rust/error-handling.md | NO | T1 | RUST-ERR-NONEXHAUSTIVE | `pub enum ConfigError { NotFound }` (no attr) | `#[non_exhaustive] pub enum ConfigError { NotFound }` |
| Error messages lowercase, no trailing punctuation | rust/error-handling.md | NO | T2 | RUST-ERR-MSG-STYLE | `#[error("File Not Found.")]` | `#[error("config file not found")]` |
| No sentinel return values (`-1`, `""`) to signal failure; return `Result`/`Option` | rust/error-handling.md | NO | T2 | RUST-ERR-SENTINEL | `fn find()->i64 { -1 }` on not-found | `fn find()->Option<T>` / `Result<T,E>` |
| `main` returns `ExitCode`/`anyhow::Result<()>` with documented exit codes | rust/error-handling.md | PARTIAL (1 ExitCode hit) | T2 | RUST-ERR-MAIN-EXITCODE | `fn main() { ...; std::process::exit(1) }` scattered | `fn main() -> ExitCode` mapping categories |
| Checked/saturating arithmetic on untrusted input; no unchecked overflow | rust/error-handling.md | NO | T1 | RUST-SEC-CHECKED-ARITH | `let n = a + user_input;` on attacker math | `a.checked_add(user_input)?` |
| Context added at `?` boundaries via `.with_context(||...)` | rust/error-handling.md | NO | T2 | RUST-ERR-CONTEXT | `read_to_string(p)?` bare in commands/ | `.with_context(|| format!("reading {p}"))?` |
| `unsafe` block requires `// SAFETY:` comment | rust/error-handling.md, patterns.md, typing-style.md | PARTIAL (SAFETY strings present; unsafe-without-SAFETY check unclear) | T1 | RUST-SAFETY-COMMENT | `unsafe { *ptr }` no comment | `// SAFETY: ...\nunsafe { *ptr }` |
| Internal enums: no catch-all `_ =>` arm; list all variants | typing-style.md, patterns.md | NO | T1 | RUST-MATCH-NO-WILDCARD | `match s { A => .., _ => .. }` on internal enum | exhaustive arms per variant |
| Prefer `match` over `if let ... else if let` chains over closed set | typing-style.md | NO | T2 | RUST-MATCH-OVER-IFLET | `if let A=x {} else if let B=x {}` | `match x { A=>.., B=>.. }` |
| Captured identifiers in format strings (`{path}` not `{}, path`) | typing-style.md | NO | T2 | RUST-FMT-CAPTURED-IDENT | `format!("{}", path)` | `format!("{path}")` |
| No lossy `as` casts; use `TryFrom`/`try_into()` | typing-style.md | NO | T1 | RUST-CAST-NO-AS-LOSSY | `x as u8` from u64 | `u8::try_from(x)?` |
| `///` doc comments on every public item (with `# Errors`/`# Panics`) | typing-style.md | PARTIAL ("missing doc" ref) | T2 | RUST-DOC-PUBLIC-ITEM | `pub fn foo()` no `///` | `/// Summary\npub fn foo()` |
| Max 5 parameters; group into struct/builder beyond | patterns.md, typing-style.md | PARTIAL (value-object rule, not count gate) | T1 | RUST-FN-MAX-PARAMS | `fn f(a,b,c,d,e,f)` | `fn f(input: FooInput)` |
| Cyclomatic complexity < 10, nesting depth <= 3 | patterns.md | PARTIAL (complexity/nesting mentioned generically) | T2 | RUST-FN-COMPLEXITY | 12-branch fn, 5-deep nesting | guard clauses, <10 / <=3 |
| Don't introduce trait/generic until two real impls (YAGNI) | patterns.md | NO | T3 | RUST-YAGNI-PREMATURE-TRAIT | trait with single impl | trait with 2+ impls (labeled: intent, unscorable) |
| Wire concrete impls at composition root, not deep in domain | patterns.md | NO | T3 | RUST-DI-COMPOSITION-ROOT | concrete `Postgres::new()` in domain fn | injected `S: UserStore` (labeled) |
| Never block async executor: no `std::thread::sleep`/blocking fs/net in `async fn` | patterns.md | PARTIAL (backed) — but bound-concurrency clause NOT | patterns.md | — | — | — |
| Bound concurrency (semaphore/`buffer_unordered`) vs unbounded spawn | patterns.md | NO | T2 | RUST-ASYNC-BOUNDED-CONCURRENCY | `for x in xs { tokio::spawn(..) }` unbounded | `buffer_unordered(N)` / semaphore |
| No catch-all `utils.rs`/`helpers` module | patterns.md, change-discipline.md | PARTIAL (utils hits generic) | T1 | RUST-NO-UTILS-MODULE | `src/utils.rs` created | responsibility-named module |
| Path traversal: user input never directly in file path; assert within base dir | shared/security.md | NO | T1 | SEC-PATH-TRAVERSAL | `File::open(base.join(user_input))` no canonicalize check | resolve + assert `starts_with(base)` |
| No MD5/SHA1 for security; use SHA-256+/AES-256 | shared/security.md | NO | T1 | SEC-WEAK-HASH | `Md5::new()` / `Sha1` for auth | `Sha256`/vetted crate |
| No general-purpose `random` for tokens/keys; use CSPRNG | shared/security.md | PARTIAL (random hits generic) | T1 | SEC-CSPRNG-TOKENS | `rand::thread_rng()` for token | `rand::rngs::OsRng` / `getrandom` |
| Constant-time secret comparison, never `==` | shared/security.md | PARTIAL (timing 1 hit) | T1 | SEC-CONST-TIME-CMP | `secret == input` | `subtle::ConstantTimeEq` |
| No hardcoded secrets (`sk-`,`ghp_`,`aws_`, high-entropy) | shared/security.md | PARTIAL (secret 41 / sk- 2) | T1 | SEC-HARDCODED-SECRET | `let key = "sk-abc123..."` | env / secret manager |
| Never `eval`/`exec`/deserialize-untrusted (Rust: no unchecked `serde` from untrusted w/o validation) | shared/security.md | PARTIAL (eval/pickle python-shaped) | T1 | SEC-UNTRUSTED-DESER | deserialize+use w/o boundary validation | TryFrom validated at edge |
| Never expose stack traces/internal paths in client responses (CWE-209) | shared/security.md | NO | T2 | SEC-ERR-LEAK-CLIENT | return `format!("{err:?}")` to client | generic client msg, detail logged |
| Scan deps for CVEs before merge (cargo-audit) | shared/security.md, change-discipline.md | PARTIAL (audit/gitleaks present; cargo-audit CVE gate?) | T2 | SEC-SCA-CARGO-AUDIT | no `cargo audit` in CI | `cargo audit` gate green |
| STOP-on-Vulnerability real-time gate (CWE watchlist interrupt) | shared/security.md | NO | T3 | SEC-STOP-GATE | (agent-behavior, not code) | (labeled: advisory, unscorable) |
| Trust-boundary change MUST enumerate failure modes, each handled + tested | shared/resilience.md | NO | T3 | RESIL-FAILURE-MODE-TEST | trust-boundary fn, no failure-mode tests | modes covered by tests (labeled) |
| Idempotency for retried/duplicate/out-of-order events | shared/resilience.md | NO | T2 | RESIL-IDEMPOTENCY | mutation w/o dedup key | idempotency key / upsert |
| Atomic write + rollback for partial-failure/crash recovery | shared/resilience.md | NO | T2 | RESIL-ATOMIC-WRITE | truncate-then-write in place | temp-file + atomic rename |
| Timeouts/retries on external I/O | shared/resilience.md | PARTIAL (retry 2 / timeout 9) | T2 | RESIL-IO-TIMEOUT | bare network call, no timeout | `tokio::time::timeout(..)` |
| `// TODO`/`FIXME`/`HACK` MUST carry tracker ref `(#1234)` | shared/change-discipline.md | PARTIAL (markers removed, tracker-ref form NOT) | T1 | CHG-TODO-TRACKER-REF | `// TODO: fix later` | `// TODO(#1234): reason` |
| Structural change (3+ files/new pattern) documents WHY / updates ARCHITECTURE.md | shared/change-discipline.md | NO | T3 | CHG-STRUCTURAL-EXPLAIN | 3+ file diff, no rationale/doc | ARCHITECTURE.md updated (labeled) |
| Deviation from established patterns recorded as ADR in decisions.md | shared/change-discipline.md | NO | T3 | CHG-ADR-DEVIATION | pattern deviation, no ADR | decisions.md ADR entry (labeled) |
| Refactor in dedicated commits — never mixed with feature work | shared/change-discipline.md | NO | T3 | CHG-REFACTOR-ISOLATED | commit mixing refactor+feature | separate commits (labeled) |
| Minimize deps: check stdlib/existing before adding new package | shared/change-discipline.md | PARTIAL (dependency generic) | T3 | CHG-DEP-JUSTIFY | new Cargo.toml dep, no rationale | documented in ARCHITECTURE.md (labeled) |
