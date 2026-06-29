# Rust Rules

This document is the law for Rust code in this repository. Rules are numbered so humans, AI agents, CI, and validation scripts can point to the exact broken rule.

## Severity and enforcement language

| Term | Meaning |
|---|---|
| Hard: Node | Enforced by `scripts/rust-rules.mjs`. Failure exits non-zero and prints rule ID, reason, doc anchor, and fix snippet. |
| Hard: Cargo | Enforced by Cargo/rustc/rustfmt/Clippy/rustdoc commands invoked by the Node gate. |
| Hard: cargo-deny/audit | Enforced by Cargo dependency policy tools. |
| Hard: Review/Test | Mandatory design rule that must be proven by tests or review. Add deterministic validation when practical. |
| Hard: Process | Mandatory agent/repository process rule. |

Suppression is not a normal escape hatch. `#[allow(...)]`, `#[expect(...)]`, and `rust-rules: ignore` style comments are themselves violations.

## Non-negotiable project posture

1. No raw strings in domain function signatures. Use branded/domain types.
2. No unbranded primitives in domain function signatures. Use newtypes, enums, NonZero types, or value objects.
3. No `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, debug macros, wildcard imports, wildcard re-exports, or barrel-style `pub use` outside the configured project policy.
4. Unsafe Rust is forbidden by default.
5. Validation files are production code and must have tests that prove bad fixtures fail.

## Rule index

- [1. Toolchain, workspace, and deterministic builds](#1-toolchain-workspace-and-deterministic-builds)
- [2. Lint posture and suppression policy](#2-lint-posture-and-suppression-policy)
- [3. Safety, unsafe Rust, and memory invariants](#3-safety-unsafe-rust-and-memory-invariants)
- [4. Error handling, panics, and control flow](#4-error-handling-panics-and-control-flow)
- [5. Ownership, borrowing, allocation, and copying](#5-ownership-borrowing-allocation-and-copying)
- [6. Branded types, domain modelling, strings, and primitives](#6-branded-types-domain-modelling-strings-and-primitives)
- [7. Modules, visibility, imports, and architecture boundaries](#7-modules-visibility-imports-and-architecture-boundaries)
- [8. Async, concurrency, cancellation, and blocking](#8-async-concurrency-cancellation-and-blocking)
- [9. Dependencies, supply chain, and Cargo policy](#9-dependencies-supply-chain-and-cargo-policy)
- [10. Formatting, build, CI, and release gates](#10-formatting-build-ci-and-release-gates)
- [11. Security and input handling](#11-security-and-input-handling)
- [12. Testing, fixtures, properties, and failure proofs](#12-testing-fixtures-properties-and-failure-proofs)
- [13. Documentation and public API ergonomics](#13-documentation-and-public-api-ergonomics)
- [14. Serialization, DTOs, and boundary conversion](#14-serialization-dtos-and-boundary-conversion)
- [15. FFI, platform, filesystem, and OS boundaries](#15-ffi-platform-filesystem-and-os-boundaries)
- [16. Performance, allocation, and measurement](#16-performance-allocation-and-measurement)
- [17. Traits, generics, macros, and language features](#17-traits-generics-macros-and-language-features)
- [18. State, architecture, and maintainability](#18-state-architecture-and-maintainability)
- [19. AI agent conduct and anti-slop rules](#19-ai-agent-conduct-and-anti-slop-rules)
- [20. Migration, maintenance, and governance](#20-migration-maintenance-and-governance)

## 1. Toolchain, workspace, and deterministic builds

<a id="rr-11"></a>
### RR-1.1 — Rust toolchain must be pinned

**Enforcement:** Hard: Node + CI.

**Rule:** Every repo must contain rust-toolchain.toml with a fixed channel and rustfmt/clippy components.

<a id="rr-12"></a>
### RR-1.2 — Cargo.lock must be committed

**Enforcement:** Hard: Node.

**Rule:** Commit Cargo.lock for deterministic dependency resolution. Library-only crates need explicit architecture approval before omitting it.

<a id="rr-13"></a>
### RR-1.3 — clippy.toml must exist

**Enforcement:** Hard: Node.

**Rule:** Keep lint thresholds centralized and version-controlled.

<a id="rr-14"></a>
### RR-1.4 — deny.toml must exist

**Enforcement:** Hard: Node + cargo-deny.

**Rule:** Dependency policy must be executable, not prose.

<a id="rr-15"></a>
### RR-1.5 — Cargo.toml must declare rust-version

**Enforcement:** Hard: Node.

**Rule:** Declare package.rust-version or workspace.package.rust-version.

<a id="rr-16"></a>
### RR-1.6 — Edition must be explicit

**Enforcement:** Hard: Review/Cargo.

**Rule:** Every package must declare edition. Use the current project edition consistently.

<a id="rr-17"></a>
### RR-1.7 — Workspace dependencies must be centralized

**Enforcement:** Hard: Review.

**Rule:** Use [workspace.dependencies] to prevent drift across crates.

<a id="rr-18"></a>
### RR-1.8 — Workspace lints must be centralized when possible

**Enforcement:** Hard: Review.

**Rule:** Prefer [workspace.lints] for common rustc/clippy/rustdoc policy.

<a id="rr-19"></a>
### RR-1.9 — Generated code must be isolated

**Enforcement:** Hard: Review.

**Rule:** Generated files must live under generated/ or OUT_DIR and must not weaken rules in handwritten code.

<a id="rr-110"></a>
### RR-1.10 — No non-deterministic build inputs

**Enforcement:** Hard: Review.

**Rule:** Builds must not depend on local absolute paths, current time, network, or machine-specific environment.

<a id="rr-111"></a>
### RR-1.11 — No hidden code generation

**Enforcement:** Hard: Node.

**Rule:** build.rs is forbidden by default.

<a id="rr-112"></a>
### RR-1.12 — No untracked tool config

**Enforcement:** Hard: Review.

**Rule:** rustfmt, clippy, deny, audit, and CI config must be committed.

## 2. Lint posture and suppression policy

<a id="rr-21"></a>
### RR-2.1 — No lint suppression attributes

**Enforcement:** Hard: Node.

**Rule:** Do not use #[allow(...)] or #[expect(...)]. Fix the code or change the central rule with review.

<a id="rr-22"></a>
### RR-2.2 — No validator suppression comments

**Enforcement:** Hard: Node.

**Rule:** Comments such as rust-rules: ignore/skip/allow/disable are forbidden.

<a id="rr-23"></a>
### RR-2.3 — Warnings are errors

**Enforcement:** Hard: Cargo.

**Rule:** Compile, clippy, and rustdoc warnings must fail CI.

<a id="rr-24"></a>
### RR-2.4 — No local lint downgrades

**Enforcement:** Hard: Review.

**Rule:** Do not downgrade deny/forbid lints inside modules, functions, or test code.

<a id="rr-25"></a>
### RR-2.5 — No blanket allow lists

**Enforcement:** Hard: Review.

**Rule:** No crate-level allow groups for clippy::all, clippy::pedantic, warnings, dead_code, unused, or missing_docs.

<a id="rr-26"></a>
### RR-2.6 — Restriction lints must be cherry-picked

**Enforcement:** Hard: Review.

**Rule:** Use specific Clippy restriction lints; do not enable contradictory groups blindly.

<a id="rr-27"></a>
### RR-2.7 — Forbid unsafe_code by default

**Enforcement:** Hard: Cargo/Node.

**Rule:** Every normal crate should use #![forbid(unsafe_code)].

<a id="rr-28"></a>
### RR-2.8 — Deny unreachable public items

**Enforcement:** Hard: Cargo.

**Rule:** Use unreachable_pub or equivalent visibility review.

<a id="rr-29"></a>
### RR-2.9 — Deny unused must-use values

**Enforcement:** Hard: Cargo.

**Rule:** unused_must_use must be denied; do not discard Results or Futures.

<a id="rr-210"></a>
### RR-2.10 — Deny missing docs for library APIs

**Enforcement:** Hard: Cargo/rustdoc.

**Rule:** Public API documentation is required for libraries.

<a id="rr-211"></a>
### RR-2.11 — Deny broken intra-doc links

**Enforcement:** Hard: rustdoc.

**Rule:** Broken rustdoc links fail documentation builds.

<a id="rr-212"></a>
### RR-2.12 — No dead code accumulation

**Enforcement:** Hard: Review.

**Rule:** Remove dead code instead of hiding it behind allow(dead_code).

## 3. Safety, unsafe Rust, and memory invariants

<a id="rr-31"></a>
### RR-3.1 — Unsafe Rust is forbidden by default

**Enforcement:** Hard: Node + Cargo.

**Rule:** unsafe is not allowed unless the crate is explicitly designated as an unsafe boundary crate.

<a id="rr-32"></a>
### RR-3.2 — Unsafe blocks require SAFETY comments

**Enforcement:** Hard: Node when unsafe enabled.

**Rule:** Every unsafe block requires a nearby // SAFETY: explanation.

Required form: `// SAFETY: <invariants that make this block safe>` immediately near the unsafe block.

<a id="rr-33"></a>
### RR-3.3 — Unsafe functions require # Safety docs

**Enforcement:** Hard: Node when unsafe enabled.

**Rule:** Every unsafe fn or trait must document caller obligations.

<a id="rr-34"></a>
### RR-3.4 — Raw pointers are forbidden in public/domain APIs

**Enforcement:** Hard: Node.

**Rule:** Raw pointers may appear only in approved FFI/sys modules.

<a id="rr-35"></a>
### RR-3.5 — Unsafe must be behind safe abstractions

**Enforcement:** Hard: Review/Test.

**Rule:** Public APIs must remain safe unless the entire API is explicitly unsafe by design.

<a id="rr-36"></a>
### RR-3.6 — Unsafe scope must be minimal

**Enforcement:** Hard: Review.

**Rule:** Keep unsafe blocks as small as possible.

<a id="rr-37"></a>
### RR-3.7 — No unchecked arithmetic without proof

**Enforcement:** Hard: Review/Clippy.

**Rule:** Use checked/saturating/wrapping operations intentionally, with domain meaning.

<a id="rr-38"></a>
### RR-3.8 — No unchecked indexing

**Enforcement:** Hard: Node.

**Rule:** Use get/get_mut or typed bounded indexes.

<a id="rr-39"></a>
### RR-3.9 — No panic across FFI boundaries

**Enforcement:** Hard: Review/Test.

**Rule:** FFI entry points must catch unwind or compile with panic abort policy as designed.

<a id="rr-310"></a>
### RR-3.10 — No aliasing violations

**Enforcement:** Hard: Compiler/Review.

**Rule:** Never create mutable aliasing or invalid references through unsafe.

<a id="rr-311"></a>
### RR-3.11 — No uninitialized memory exposure

**Enforcement:** Hard: Review/Miri.

**Rule:** MaybeUninit must be encapsulated and tested.

<a id="rr-312"></a>
### RR-3.12 — No transmute unless proven

**Enforcement:** Hard: Review.

**Rule:** Prefer safe conversions; transmute requires layout proof and tests.

<a id="rr-313"></a>
### RR-3.13 — No manual Send/Sync without proof

**Enforcement:** Hard: Review.

**Rule:** unsafe impl Send/Sync requires invariant documentation and concurrency tests.

<a id="rr-314"></a>
### RR-3.14 — Run Miri for unsafe crates

**Enforcement:** Hard: CI for unsafe crates.

**Rule:** Unsafe crates must include a Miri job where feasible.

<a id="rr-315"></a>
### RR-3.15 — Pointer lifetimes must not be fabricated

**Enforcement:** Hard: Review.

**Rule:** Do not extend lifetimes with unsafe, transmute, leaked boxes, or static hacks.

## 4. Error handling, panics, and control flow

<a id="rr-41"></a>
### RR-4.1 — No unwrap/expect

**Enforcement:** Hard: Node + Clippy.

**Rule:** Replace unwrap/expect with ?, match, ok_or_else, or typed errors.

Bad: `let item = value.unwrap();`

Good: `let item = value.ok_or_else(|| DomainError::MissingItem)?;`

<a id="rr-42"></a>
### RR-4.2 — No panic-like macros in production paths

**Enforcement:** Hard: Node + Clippy.

**Rule:** No panic!, todo!, unimplemented!, or unreachable! in source logic.

<a id="rr-43"></a>
### RR-4.3 — No debug/console macros in Rust logic

**Enforcement:** Hard: Node + Clippy.

**Rule:** No dbg!, println!, or eprintln! in logic; use tracing/logging.

<a id="rr-44"></a>
### RR-4.4 — Domain code must not use erased application errors

**Enforcement:** Hard: Node.

**Rule:** Use typed errors in domain/library crates; anyhow is for outer application boundaries.

<a id="rr-45"></a>
### RR-4.5 — Recoverable failure returns Result

**Enforcement:** Hard: Review.

**Rule:** All expected failures must be modelled as Result or domain state.

<a id="rr-46"></a>
### RR-4.6 — Absence returns Option only when absence is valid

**Enforcement:** Hard: Review.

**Rule:** Do not use Option to hide errors that require context.

<a id="rr-47"></a>
### RR-4.7 — No stringly errors

**Enforcement:** Hard: Review.

**Rule:** Errors must be typed; no Err(String), &str errors, or ad hoc string errors in domain code.

<a id="rr-48"></a>
### RR-4.8 — Attach context at boundaries

**Enforcement:** Hard: Review.

**Rule:** Boundary/application layers add context while preserving source errors.

<a id="rr-49"></a>
### RR-4.9 — No ignored Results

**Enforcement:** Hard: Cargo.

**Rule:** Do not use let _ = result unless the API is explicitly fire-and-forget and documented.

<a id="rr-410"></a>
### RR-4.10 — No boolean success/failure APIs

**Enforcement:** Hard: Review/Node via primitive ban.

**Rule:** Use Result or typed outcome enums.

<a id="rr-411"></a>
### RR-4.11 — No panic-based validation

**Enforcement:** Hard: Review.

**Rule:** Constructors validate with Result, not panic.

<a id="rr-412"></a>
### RR-4.12 — No sentinel values

**Enforcement:** Hard: Review.

**Rule:** Use Option/Result/enums, not empty string, zero, -1, or magic numbers as error states.

<a id="rr-413"></a>
### RR-4.13 — Do not lose source errors

**Enforcement:** Hard: Review.

**Rule:** Custom errors should use #[source] or #[from] where applicable.

<a id="rr-414"></a>
### RR-4.14 — Top-level errors are rendered once

**Enforcement:** Hard: Review.

**Rule:** Avoid logging and returning the same error at multiple layers.

## 5. Ownership, borrowing, allocation, and copying

<a id="rr-51"></a>
### RR-5.1 — Clone must be justified

**Enforcement:** Hard: Node.

**Rule:** Every .clone() requires a nearby CLONE-JUSTIFICATION: comment.

Required form: `// CLONE-JUSTIFICATION: <why this clone is semantically required>` immediately near the clone.

<a id="rr-52"></a>
### RR-5.2 — String allocation must be justified

**Enforcement:** Hard: Node.

**Rule:** Every to_string()/to_owned() requires nearby ALLOC-JUSTIFICATION: unless in boundary modules.

<a id="rr-53"></a>
### RR-5.3 — Unchecked indexing is forbidden

**Enforcement:** Hard: Node.

**Rule:** No array/slice/string indexing; use checked accessors.

<a id="rr-54"></a>
### RR-5.4 — Lossy casts require justification

**Enforcement:** Hard: Node.

**Rule:** Numeric as casts require CAST-JUSTIFICATION: or TryFrom/TryInto.

<a id="rr-55"></a>
### RR-5.5 — Prefer borrowing over ownership

**Enforcement:** Hard: Review.

**Rule:** Take references/slices where ownership is not consumed.

<a id="rr-56"></a>
### RR-5.6 — Move when consuming

**Enforcement:** Hard: Review.

**Rule:** Take owned values only when the function consumes or stores them.

<a id="rr-57"></a>
### RR-5.7 — No borrow-checker escape cloning

**Enforcement:** Hard: Node/Review.

**Rule:** Do not clone to avoid understanding lifetimes.

<a id="rr-58"></a>
### RR-5.8 — No global mutable state

**Enforcement:** Hard: Review.

**Rule:** Use explicit state objects, dependency injection, or synchronization wrappers.

<a id="rr-59"></a>
### RR-5.9 — Use Arc only for shared ownership across threads/tasks

**Enforcement:** Hard: Review/Clippy.

**Rule:** Do not use Arc as a default ownership escape hatch.

<a id="rr-510"></a>
### RR-5.10 — Use Rc only for single-threaded graph ownership

**Enforcement:** Hard: Review.

**Rule:** Rc is forbidden in Send/Sync or async multi-threaded contexts.

<a id="rr-511"></a>
### RR-5.11 — No RefCell as design shortcut

**Enforcement:** Hard: Review/Clippy.

**Rule:** Prefer ordinary mutability and explicit ownership.

<a id="rr-512"></a>
### RR-5.12 — Avoid heap allocation in hot paths

**Enforcement:** Hard: Review/Bench.

**Rule:** Use slices, SmallVec, arrays, or arenas when justified.

<a id="rr-513"></a>
### RR-5.13 — No accidental large copies

**Enforcement:** Hard: Clippy/Review.

**Rule:** Large structs should not derive Copy without measurement.

<a id="rr-514"></a>
### RR-5.14 — No lifetime widening hacks

**Enforcement:** Hard: Review.

**Rule:** Do not use Box::leak or static storage to satisfy lifetime errors.

<a id="rr-515"></a>
### RR-5.15 — Avoid unnecessary collection

**Enforcement:** Hard: Clippy/Review.

**Rule:** Prefer iterator chains or streaming APIs over collect-then-process.

## 6. Branded types, domain modelling, strings, and primitives

<a id="rr-61"></a>
### RR-6.1 — No raw string types in domain function signatures

**Enforcement:** Hard: Node.

**Rule:** No String, &str, str, Cow<str>, PathBuf, OsString, CString, or CStr in domain signatures; wrap them in branded types.

Bad: `pub fn find_user(id: &str) -> Result<User, Error>`

Good: `pub fn find_user(id: UserId) -> Result<User, LookupError>`

<a id="rr-62"></a>
### RR-6.2 — No unbranded primitive types in domain function signatures

**Enforcement:** Hard: Node.

**Rule:** No bool or numeric primitive parameters/returns in domain signatures; use newtypes/enums.

Bad: `pub fn retry(count: u32, enabled: bool)`

Good: `pub fn retry(policy: RetryPolicy)`

<a id="rr-63"></a>
### RR-6.3 — No public raw fields

**Enforcement:** Hard: Node.

**Rule:** Public fields must expose branded/domain types only.

<a id="rr-64"></a>
### RR-6.4 — Raw private fields require brand invariants

**Enforcement:** Hard: Node.

**Rule:** Private raw fields inside domain types require BRAND-INVARIANT documentation.

Required form: `/// BRAND-INVARIANT: <validation rule and semantic meaning>` near a newtype over raw data.

<a id="rr-65"></a>
### RR-6.5 — Type aliases must not disguise raw primitives

**Enforcement:** Hard: Node.

**Rule:** No type UserId = String or type Count = usize.

<a id="rr-66"></a>
### RR-6.6 — Tuple newtypes must not expose raw inner fields

**Enforcement:** Hard: Node.

**Rule:** Use private tuple fields and validated constructors.

<a id="rr-67"></a>
### RR-6.7 — Boolean blindness is forbidden

**Enforcement:** Hard: Node/Review.

**Rule:** No bool arguments or bool state flags; use enums or dedicated types.

<a id="rr-68"></a>
### RR-6.8 — Make invalid states unrepresentable

**Enforcement:** Hard: Review.

**Rule:** Use enums, typestates, and constrained constructors.

<a id="rr-69"></a>
### RR-6.9 — Every domain primitive wrapper has a validation constructor

**Enforcement:** Hard: Review/Test.

**Rule:** Newtypes over raw data must validate invariants.

<a id="rr-610"></a>
### RR-6.10 — Every domain primitive wrapper documents invariants

**Enforcement:** Hard: Node/Docs.

**Rule:** Use BRAND-INVARIANT comments and rustdoc.

<a id="rr-611"></a>
### RR-6.11 — Do not expose Stringly maps at domain boundaries

**Enforcement:** Hard: Review/Node partial.

**Rule:** No HashMap<String, _> in domain APIs; key types must be branded.

<a id="rr-612"></a>
### RR-6.12 — Do not expose Vec<String> or Vec<primitive> in domain APIs

**Enforcement:** Hard: Review/Node partial.

**Rule:** Use collection value objects.

<a id="rr-613"></a>
### RR-6.13 — Use NonZero types when zero is invalid

**Enforcement:** Hard: Review.

**Rule:** Prefer NonZeroU64 etc inside branded wrappers.

<a id="rr-614"></a>
### RR-6.14 — Use units-of-measure types

**Enforcement:** Hard: Review.

**Rule:** No raw duration/count/bytes/pixels/percentage primitives across APIs.

<a id="rr-615"></a>
### RR-6.15 — Use typestate for staged operations

**Enforcement:** Hard: Review.

**Rule:** Represent unvalidated/validated/authorized states in the type system.

<a id="rr-616"></a>
### RR-6.16 — Separate raw input from validated domain value

**Enforcement:** Hard: Review.

**Rule:** Boundary raw input must be converted immediately.

<a id="rr-617"></a>
### RR-6.17 — No public setters that break invariants

**Enforcement:** Hard: Review.

**Rule:** Mutation must preserve invariants or return Result.

<a id="rr-618"></a>
### RR-6.18 — No partial domain objects

**Enforcement:** Hard: Review.

**Rule:** Avoid structs full of Option fields unless the state truly is optional.

<a id="rr-619"></a>
### RR-6.19 — No enum catch-all without policy

**Enforcement:** Hard: Review.

**Rule:** Other/Unknown variants must carry documented compatibility semantics.

<a id="rr-620"></a>
### RR-6.20 — No primitive IDs

**Enforcement:** Hard: Node/Review.

**Rule:** UserId, ProjectId, DeviceId, etc. must be distinct newtypes.

<a id="rr-621"></a>
### RR-6.21 — No raw paths in domain APIs

**Enforcement:** Hard: Node.

**Rule:** Use branded path types that encode purpose and validation.

<a id="rr-622"></a>
### RR-6.22 — No raw URLs in domain APIs

**Enforcement:** Hard: Review.

**Rule:** Use Url-like validated types or project-specific newtypes.

<a id="rr-623"></a>
### RR-6.23 — No raw time stamps in domain APIs

**Enforcement:** Hard: Review.

**Rule:** Use domain time types and timezone policy.

<a id="rr-624"></a>
### RR-6.24 — No naked tuples for domain data

**Enforcement:** Hard: Review.

**Rule:** Use named structs for semantic fields.

<a id="rr-625"></a>
### RR-6.25 — No positional constructor ambiguity

**Enforcement:** Hard: Review.

**Rule:** Prefer builders or named constructors when multiple values share the same type.

<a id="rr-626"></a>
### RR-6.26 — Serialized domain fields must not expose raw identity primitives

**Enforcement:** Hard: Node when `enforceSerializedPublicDomainPrimitives` is enabled.

**Rule:** Public serialized fields whose names carry identity, reference, event type, or command type meaning must use typed domain newtypes or enums unless the file is in `serializedDomainOwnerGlobs`.

## 7. Modules, visibility, imports, and architecture boundaries

<a id="rr-71"></a>
### RR-7.1 — Wildcard imports are forbidden

**Enforcement:** Hard: Node + Clippy.

**Rule:** Use explicit imports only.

<a id="rr-72"></a>
### RR-7.2 — Wildcard public re-exports are forbidden

**Enforcement:** Hard: Node.

**Rule:** No pub use x::*.

<a id="rr-73"></a>
### RR-7.3 — Public re-exports must match project policy

**Enforcement:** Hard: Node.

**Rule:** With `publicReexportPolicy: "forbid"`, every `pub use` fails. With `publicReexportPolicy: "facade-only"`, `pub use` is allowed only in `facadeFileGlobs`.

<a id="rr-74"></a>
### RR-7.4 — Dumping-ground modules are forbidden

**Enforcement:** Hard: Node.

**Rule:** No utils/helpers/common/misc/shared/stuff module names.

<a id="rr-75"></a>
### RR-7.5 — Build scripts are forbidden by default

**Enforcement:** Hard: Node.

**Rule:** No build.rs unless approved by config and review.

<a id="rr-76"></a>
### RR-7.6 — Organize by domain, not technical layer only

**Enforcement:** Hard: Review.

**Rule:** Prefer account/policy/device over giant services/models/utils buckets.

<a id="rr-77"></a>
### RR-7.7 — Keep module boundaries narrow

**Enforcement:** Hard: Review.

**Rule:** Expose the smallest public surface.

<a id="rr-78"></a>
### RR-7.8 — No pub(crate) by default

**Enforcement:** Hard: Review.

**Rule:** Use private visibility unless cross-module access is truly needed.

<a id="rr-79"></a>
### RR-7.9 — No leaking internal modules through public API

**Enforcement:** Hard: rustdoc/Review.

**Rule:** Public types must not expose private internals or unstable layout.

<a id="rr-710"></a>
### RR-7.10 — Facade modules are curated

**Enforcement:** Hard: Review.

**Rule:** prelude/api modules must explicitly re-export stable API only.

<a id="rr-711"></a>
### RR-7.11 — Do not mix domain and infrastructure

**Enforcement:** Hard: Review.

**Rule:** Domain modules must not depend on database, network, filesystem, or UI adapters.

<a id="rr-712"></a>
### RR-7.12 — Boundary modules convert, domain modules decide

**Enforcement:** Hard: Review.

**Rule:** Adapters parse input and map output; domain owns invariants and policy.

<a id="rr-713"></a>
### RR-7.13 — No circular module dependencies

**Enforcement:** Hard: Review.

**Rule:** Break cycles with traits or explicit ports.

<a id="rr-714"></a>
### RR-7.14 — File names must describe domain responsibility

**Enforcement:** Hard: Node/Review.

**Rule:** No vague file names.

<a id="rr-715"></a>
### RR-7.15 — No mega-files

**Enforcement:** Hard: Review.

**Rule:** Split files that accumulate unrelated responsibilities.

## 8. Async, concurrency, cancellation, and blocking

<a id="rr-81"></a>
### RR-8.1 — Blocking primitives are forbidden inside async modules

**Enforcement:** Hard: Node + Clippy.

**Rule:** No std::sync locks, thread sleep, blocking fs/net in async code.

<a id="rr-82"></a>
### RR-8.2 — C-style index loops are forbidden

**Enforcement:** Hard: Node.

**Rule:** Use iterators or typed ranges.

<a id="rr-83"></a>
### RR-8.3 — No await while holding lock

**Enforcement:** Hard: Clippy/Review.

**Rule:** Do not hold Mutex/RwLock/RefCell guards across await.

<a id="rr-84"></a>
### RR-8.4 — Async tasks must be cancellation-safe

**Enforcement:** Hard: Review/Test.

**Rule:** select! and spawned tasks must handle cancellation and cleanup.

<a id="rr-85"></a>
### RR-8.5 — Spawned tasks must be tracked

**Enforcement:** Hard: Review.

**Rule:** Do not fire-and-forget JoinHandles unless explicitly supervised.

<a id="rr-86"></a>
### RR-8.6 — Use bounded channels by default

**Enforcement:** Hard: Review.

**Rule:** Unbounded channels require backpressure justification.

<a id="rr-87"></a>
### RR-8.7 — No blocking CPU work on async executor

**Enforcement:** Hard: Review.

**Rule:** Use spawn_blocking or dedicated worker pools.

<a id="rr-88"></a>
### RR-8.8 — No shared mutable state when message passing fits

**Enforcement:** Hard: Review.

**Rule:** Prefer channels or actors for concurrent ownership transfer.

<a id="rr-89"></a>
### RR-8.9 — All shared state wrappers must encode ownership policy

**Enforcement:** Hard: Review.

**Rule:** Arc<Mutex<T>> must be justified; prefer dedicated state types.

<a id="rr-810"></a>
### RR-8.10 — Timeouts are required for external I/O

**Enforcement:** Hard: Review/Test.

**Rule:** Network and IPC calls must have timeout/cancellation policy.

<a id="rr-811"></a>
### RR-8.11 — Retries require bounded policy

**Enforcement:** Hard: Review.

**Rule:** Use jitter/backoff and stop conditions.

<a id="rr-812"></a>
### RR-8.12 — No task leaks

**Enforcement:** Hard: Test/Review.

**Rule:** Long-running tasks must have shutdown signals.

<a id="rr-813"></a>
### RR-8.13 — No Send violations hidden by local runtimes

**Enforcement:** Hard: Review.

**Rule:** Multi-threaded runtimes require Send-safe futures.

<a id="rr-814"></a>
### RR-8.14 — No lock poisoning blindness

**Enforcement:** Hard: Review.

**Rule:** Handle poisoned locks intentionally if using std locks.

<a id="rr-815"></a>
### RR-8.15 — No global runtime creation in libraries

**Enforcement:** Hard: Review.

**Rule:** Libraries must not secretly create Tokio runtimes.

## 9. Dependencies, supply chain, and Cargo policy

<a id="rr-91"></a>
### RR-9.1 — Dependency versions must be pinned semver requirements

**Enforcement:** Hard: Node + cargo-deny.

**Rule:** No dependency version = "*".

<a id="rr-92"></a>
### RR-9.2 — Git dependencies are forbidden by default

**Enforcement:** Hard: Node + cargo-deny.

**Rule:** Use crates.io releases unless explicitly approved.

<a id="rr-93"></a>
### RR-9.3 — Path dependencies are forbidden by default outside workspace policy

**Enforcement:** Hard: Node.

**Rule:** Use workspace members or explicit approval.

<a id="rr-94"></a>
### RR-9.4 — cargo-deny must check advisories

**Enforcement:** Hard: cargo-deny.

**Rule:** Security advisories fail or warn according to deny.toml policy.

<a id="rr-95"></a>
### RR-9.5 — cargo-deny must check licenses

**Enforcement:** Hard: cargo-deny.

**Rule:** Unapproved licenses fail.

<a id="rr-96"></a>
### RR-9.6 — cargo-deny must check bans

**Enforcement:** Hard: cargo-deny.

**Rule:** Banned crates and wildcard versions fail.

<a id="rr-97"></a>
### RR-9.7 — cargo-deny must check sources

**Enforcement:** Hard: cargo-deny.

**Rule:** Unknown registries and git sources fail.

<a id="rr-98"></a>
### RR-9.8 — No casual dependency additions

**Enforcement:** Hard: Review.

**Rule:** Every new dependency must have purpose, alternatives, and maintenance check.

<a id="rr-99"></a>
### RR-9.9 — Prefer std/core/alloc first

**Enforcement:** Hard: Review.

**Rule:** Do not add crates for trivial helpers.

<a id="rr-910"></a>
### RR-9.10 — Feature flags must be minimal

**Enforcement:** Hard: Review.

**Rule:** Disable default features when unnecessary.

<a id="rr-911"></a>
### RR-9.11 — No duplicate dependency versions without reason

**Enforcement:** Hard: cargo-deny/Review.

**Rule:** Resolve or justify duplicate major versions.

<a id="rr-912"></a>
### RR-9.12 — No abandoned crates for core paths

**Enforcement:** Hard: cargo-deny/Review.

**Rule:** Avoid unmaintained dependencies in production logic.

<a id="rr-913"></a>
### RR-9.13 — No proc-macro dependency sprawl

**Enforcement:** Hard: Review.

**Rule:** Procedural macros add compile cost and audit surface.

<a id="rr-914"></a>
### RR-9.14 — No hidden native dependencies

**Enforcement:** Hard: Review.

**Rule:** Crates with native build requirements must be approved.

<a id="rr-915"></a>
### RR-9.15 — Security updates are mandatory

**Enforcement:** Hard: cargo-audit/cargo-deny.

**Rule:** Vulnerabilities must be fixed, not ignored without policy.

## 10. Formatting, build, CI, and release gates

<a id="rr-101"></a>
### RR-10.1 — cargo fmt must pass

**Enforcement:** Hard: Cargo.

**Rule:** Run cargo fmt --all -- --check.

<a id="rr-102"></a>
### RR-10.2 — cargo clippy hard mode must pass

**Enforcement:** Hard: Cargo.

**Rule:** Run clippy with -D warnings and selected deny lints.

<a id="rr-103"></a>
### RR-10.3 — cargo test must pass

**Enforcement:** Hard: Cargo.

**Rule:** Run all workspace tests with all features.

<a id="rr-104"></a>
### RR-10.4 — rustdoc warnings must be errors

**Enforcement:** Hard: Cargo/rustdoc.

**Rule:** Run cargo doc with RUSTDOCFLAGS denying warnings and doc lints.

<a id="rr-105"></a>
### RR-10.5 — Validator tests must pass

**Enforcement:** Hard: Node test.

**Rule:** Run node --test tests/rust-rules.test.mjs.

<a id="rr-106"></a>
### RR-10.6 — CI must run the same script as local dev

**Enforcement:** Hard: CI.

**Rule:** No separate weaker CI command.

<a id="rr-107"></a>
### RR-10.7 — No weakening validation in PRs

**Enforcement:** Hard: Review.

**Rule:** Changes to scripts/config/rules require owner review.

<a id="rr-108"></a>
### RR-10.8 — Pre-commit should run scanner

**Enforcement:** Hard: Local policy.

**Rule:** Use npm run rust:rules:scan for fast feedback.

<a id="rr-109"></a>
### RR-10.9 — Release builds must be reproducible

**Enforcement:** Hard: Review/CI.

**Rule:** Pin toolchain and dependencies.

<a id="rr-1010"></a>
### RR-10.10 — All features must compile

**Enforcement:** Hard: Cargo.

**Rule:** Use --all-features or feature matrix.

<a id="rr-1011"></a>
### RR-10.11 — No platform-specific breakage

**Enforcement:** Hard: CI.

**Rule:** Run target/platform matrix if code is cross-platform.

<a id="rr-1012"></a>
### RR-10.12 — No warnings in examples/benches/tests

**Enforcement:** Hard: Cargo.

**Rule:** all-targets is required.

<a id="rr-1013"></a>
### RR-10.13 — No ignored failing tests

**Enforcement:** Hard: Review.

**Rule:** Ignored tests require tracked issue and owner approval.

<a id="rr-1014"></a>
### RR-10.14 — No flaky tests

**Enforcement:** Hard: Review/CI.

**Rule:** Tests must not depend on timing, global state, or network unless isolated.

<a id="rr-1015"></a>
### RR-10.15 — Benchmark-critical changes need measurement

**Enforcement:** Hard: Review/Bench.

**Rule:** Performance claims require benchmark evidence.

## 11. Security and input handling

<a id="rr-111"></a>
### RR-11.1 — cargo-deny must pass

**Enforcement:** Hard: Cargo.

**Rule:** Run cargo deny check.

<a id="rr-112"></a>
### RR-11.2 — cargo-deny must be installed when dependency policy is required

**Enforcement:** Hard: Node/Cargo.

**Rule:** The hard gate fails if cargo-deny is required and missing.

<a id="rr-113"></a>
### RR-11.3 — cargo-audit must pass when enabled

**Enforcement:** Hard: Cargo.

**Rule:** Run cargo audit when requireCargoAudit=true.

<a id="rr-114"></a>
### RR-11.4 — All external input is untrusted

**Enforcement:** Hard: Review/Test.

**Rule:** Validate at boundaries before constructing domain types.

<a id="rr-115"></a>
### RR-11.5 — No string concatenated commands

**Enforcement:** Hard: Review.

**Rule:** Use Command with explicit args, never shell string composition.

<a id="rr-116"></a>
### RR-11.6 — No path traversal

**Enforcement:** Hard: Review/Test.

**Rule:** Canonicalize and validate paths with branded path types.

<a id="rr-117"></a>
### RR-11.7 — No insecure randomness

**Enforcement:** Hard: Review.

**Rule:** Use OS/cryptographic RNG for secrets/tokens.

<a id="rr-118"></a>
### RR-11.8 — No secrets in logs

**Enforcement:** Hard: Review/Test.

**Rule:** Secret types must redact Debug/Display.

<a id="rr-119"></a>
### RR-11.9 — No secret String persistence

**Enforcement:** Hard: Review.

**Rule:** Use secret-handling wrappers and zeroization where required.

<a id="rr-1110"></a>
### RR-11.10 — No deserialization without validation

**Enforcement:** Hard: Review/Test.

**Rule:** Deserialize to raw/boundary DTOs, then validate to domain types.

<a id="rr-1111"></a>
### RR-11.11 — No default-permit authorization

**Enforcement:** Hard: Review/Test.

**Rule:** Authorization policy must fail closed.

<a id="rr-1112"></a>
### RR-11.12 — No unchecked size inputs

**Enforcement:** Hard: Review/Test.

**Rule:** Bound sizes before allocation or parsing.

<a id="rr-1113"></a>
### RR-11.13 — No regex denial-of-service risk

**Enforcement:** Hard: Review.

**Rule:** Use safe regex engines and bounded input.

<a id="rr-1114"></a>
### RR-11.14 — No time-of-check/time-of-use path bugs

**Enforcement:** Hard: Review.

**Rule:** Avoid separate check/use flows for files where possible.

<a id="rr-1115"></a>
### RR-11.15 — No network calls without timeout/TLS policy

**Enforcement:** Hard: Review/Test.

**Rule:** External calls require timeout and security configuration.

## 12. Testing, fixtures, properties, and failure proofs

<a id="rr-121"></a>
### RR-12.1 — Non-trivial logic requires tests

**Enforcement:** Hard: Review.

**Rule:** Tests must cover behavior, not just construction.

<a id="rr-122"></a>
### RR-12.2 — Every error path has a test

**Enforcement:** Hard: Review.

**Rule:** Expected failures must be exercised.

<a id="rr-123"></a>
### RR-12.3 — Boundary validation has negative tests

**Enforcement:** Hard: Review.

**Rule:** Invalid input must fail with typed errors.

<a id="rr-124"></a>
### RR-12.4 — Domain invariants have tests

**Enforcement:** Hard: Review.

**Rule:** Newtypes and constructors must prove constraints.

<a id="rr-125"></a>
### RR-12.5 — No deleting tests to pass validation

**Enforcement:** Hard: Review.

**Rule:** Removing tests requires justification.

<a id="rr-126"></a>
### RR-12.6 — Property tests for parsers/normalizers

**Enforcement:** Hard: Review.

**Rule:** Use proptest or equivalent for invariant-heavy code.

<a id="rr-127"></a>
### RR-12.7 — Fuzz tests for hostile inputs

**Enforcement:** Hard: Review.

**Rule:** Parsers and binary/network inputs should be fuzzed.

<a id="rr-128"></a>
### RR-12.8 — Concurrency has race/cancellation tests

**Enforcement:** Hard: Review.

**Rule:** Async/concurrent code must test shutdown and cancellation.

<a id="rr-129"></a>
### RR-12.9 — Unsafe abstractions have dedicated tests

**Enforcement:** Hard: Review/Miri.

**Rule:** Unsafe wrappers require boundary and aliasing tests.

<a id="rr-1210"></a>
### RR-12.10 — Snapshot tests must be stable

**Enforcement:** Hard: Review.

**Rule:** Snapshots cannot hide nondeterministic fields.

<a id="rr-1211"></a>
### RR-12.11 — No network-dependent unit tests

**Enforcement:** Hard: Review.

**Rule:** Use mocks/fakes/local test servers.

<a id="rr-1212"></a>
### RR-12.12 — No time-dependent sleeps in tests

**Enforcement:** Hard: Review.

**Rule:** Use controlled clocks or deterministic synchronization.

<a id="rr-1213"></a>
### RR-12.13 — Validation scripts test both pass and fail cases

**Enforcement:** Hard: Node test.

**Rule:** The validator must prove it fails when it should.

<a id="rr-1214"></a>
### RR-12.14 — Tests must assert meaningful behavior

**Enforcement:** Hard: Review.

**Rule:** No empty tests or tests that only call functions without assertions.

<a id="rr-1215"></a>
### RR-12.15 — Regression tests for every bug fix

**Enforcement:** Hard: Review.

**Rule:** Bug fixes require a test that fails before the fix.

## 13. Documentation and public API ergonomics

<a id="rr-131"></a>
### RR-13.1 — Public items require rustdoc

**Enforcement:** Hard: rustdoc.

**Rule:** Public structs/enums/traits/functions must be documented.

<a id="rr-132"></a>
### RR-13.2 — Rustdoc examples must compile

**Enforcement:** Hard: rustdoc/test.

**Rule:** Examples should be runnable unless marked no_run/ignore with reason.

<a id="rr-133"></a>
### RR-13.3 — Document errors

**Enforcement:** Hard: Review.

**Rule:** Fallible functions must explain error variants.

<a id="rr-134"></a>
### RR-13.4 — Document panics

**Enforcement:** Hard: Review.

**Rule:** Any intentional panic must be documented; production panic is otherwise forbidden.

<a id="rr-135"></a>
### RR-13.5 — Document safety

**Enforcement:** Hard: Node/rustdoc.

**Rule:** Unsafe APIs require # Safety.

<a id="rr-136"></a>
### RR-13.6 — Document brand invariants

**Enforcement:** Hard: Node/Review.

**Rule:** Newtypes over raw data require invariant docs.

<a id="rr-137"></a>
### RR-13.7 — APIs must be hard to misuse

**Enforcement:** Hard: Review.

**Rule:** Prefer constructors and types that guide correct usage.

<a id="rr-138"></a>
### RR-13.8 — No ambiguous names

**Enforcement:** Hard: Review.

**Rule:** Names must encode domain meaning, not storage type.

<a id="rr-139"></a>
### RR-13.9 — No leaking implementation details

**Enforcement:** Hard: Review.

**Rule:** Do not expose internal dependency types in stable domain APIs.

<a id="rr-1310"></a>
### RR-13.10 — No excessive generic APIs

**Enforcement:** Hard: Review.

**Rule:** Generics must improve callers, not hide unclear design.

<a id="rr-1311"></a>
### RR-13.11 — Use #[must_use] for value-returning domain APIs

**Enforcement:** Hard: Review/Clippy.

**Rule:** Important results/value objects should not be silently discarded.

<a id="rr-1312"></a>
### RR-13.12 — Display and Debug must be intentional

**Enforcement:** Hard: Review.

**Rule:** Secrets redact; domain values format clearly.

<a id="rr-1313"></a>
### RR-13.13 — Public API changes need changelog

**Enforcement:** Hard: Review.

**Rule:** Breaking or visible changes must be documented.

<a id="rr-1314"></a>
### RR-13.14 — No boolean builder switches

**Enforcement:** Hard: Review/Node via primitive ban.

**Rule:** Use typed options/enums.

<a id="rr-1315"></a>
### RR-13.15 — No overloaded constructors

**Enforcement:** Hard: Review.

**Rule:** Prefer named constructors for different validation semantics.

## 14. Serialization, DTOs, and boundary conversion

<a id="rr-141"></a>
### RR-14.1 — Deserialize to DTOs, not directly to domain

**Enforcement:** Hard: Review.

**Rule:** Boundary structs are raw; domain conversion validates.

<a id="rr-142"></a>
### RR-14.2 — Boundary raw strings are quarantined

**Enforcement:** Hard: Node.

**Rule:** Raw strings allowed only in configured boundary globs.

<a id="rr-143"></a>
### RR-14.3 — DTOs do not leak into domain logic

**Enforcement:** Hard: Review.

**Rule:** Convert once at the boundary.

<a id="rr-144"></a>
### RR-14.4 — Domain types serialize intentionally

**Enforcement:** Hard: Review.

**Rule:** Serialization shape is public API and must be tested.

<a id="rr-145"></a>
### RR-14.5 — No serde untagged without reason

**Enforcement:** Hard: Review.

**Rule:** Untagged enums can produce ambiguous parsing.

<a id="rr-146"></a>
### RR-14.6 — No default values that hide missing input

**Enforcement:** Hard: Review/Test.

**Rule:** Defaults must be domain-valid and documented.

<a id="rr-147"></a>
### RR-14.7 — Version external schemas

**Enforcement:** Hard: Review.

**Rule:** API/event formats need compatibility strategy.

<a id="rr-148"></a>
### RR-14.8 — Reject unknown fields where appropriate

**Enforcement:** Hard: Review.

**Rule:** Use deny_unknown_fields for strict inputs when compatible.

<a id="rr-149"></a>
### RR-14.9 — Validate size before allocation

**Enforcement:** Hard: Review/Test.

**Rule:** DTO collection lengths must be bounded.

<a id="rr-1410"></a>
### RR-14.10 — Separate transport errors from domain errors

**Enforcement:** Hard: Review.

**Rule:** Transport parse failure is not a domain rule failure.

<a id="rr-1411"></a>
### RR-14.11 — No raw JSON values in domain

**Enforcement:** Hard: Review.

**Rule:** serde_json::Value belongs at boundaries.

<a id="rr-1412"></a>
### RR-14.12 — No base64/string blobs in domain

**Enforcement:** Hard: Review.

**Rule:** Use branded binary/blob types with size rules.

<a id="rr-1413"></a>
### RR-14.13 — Normalize before validation only when safe

**Enforcement:** Hard: Review/Test.

**Rule:** Normalization can change meaning and must be explicit.

<a id="rr-1414"></a>
### RR-14.14 — Round-trip tests for public serialization

**Enforcement:** Hard: Review/Test.

**Rule:** Public formats require compatibility tests.

<a id="rr-1415"></a>
### RR-14.15 — No partial parsing side effects

**Enforcement:** Hard: Review.

**Rule:** Parsing should be pure and rollback-safe.

## 15. FFI, platform, filesystem, and OS boundaries

<a id="rr-151"></a>
### RR-15.1 — FFI is boundary-only

**Enforcement:** Hard: Node/Review.

**Rule:** extern functions and raw pointers belong in ffi/sys modules.

<a id="rr-152"></a>
### RR-15.2 — FFI structs require repr(C)

**Enforcement:** Hard: Review.

**Rule:** FFI layout must be explicit.

<a id="rr-153"></a>
### RR-15.3 — FFI must not unwind

**Enforcement:** Hard: Review/Test.

**Rule:** Catch unwind or abort according to policy.

<a id="rr-154"></a>
### RR-15.4 — C strings are boundary-only

**Enforcement:** Hard: Node/Review.

**Rule:** CString/CStr do not enter domain APIs.

<a id="rr-155"></a>
### RR-15.5 — OS strings are boundary-only

**Enforcement:** Hard: Node.

**Rule:** OsString/PathBuf in domain signatures are forbidden.

<a id="rr-156"></a>
### RR-15.6 — Filesystem paths are branded

**Enforcement:** Hard: Node/Review.

**Rule:** Use purpose-specific validated path types.

<a id="rr-157"></a>
### RR-15.7 — No current-dir assumptions

**Enforcement:** Hard: Review/Test.

**Rule:** Use explicit roots and configuration.

<a id="rr-158"></a>
### RR-15.8 — No platform cfg sprawl

**Enforcement:** Hard: Review.

**Rule:** Contain platform-specific code in platform modules.

<a id="rr-159"></a>
### RR-15.9 — No shell invocation without policy

**Enforcement:** Hard: Review.

**Rule:** Prefer Command args; shell use needs security review.

<a id="rr-1510"></a>
### RR-15.10 — External process output is untrusted

**Enforcement:** Hard: Review/Test.

**Rule:** Parse and validate stdout/stderr before use.

## 16. Performance, allocation, and measurement

<a id="rr-161"></a>
### RR-16.1 — Measure before claiming performance

**Enforcement:** Hard: Review/Bench.

**Rule:** Performance changes need benchmark evidence.

<a id="rr-162"></a>
### RR-16.2 — No premature unsafe for speed

**Enforcement:** Hard: Node/Review.

**Rule:** Try safe design first.

<a id="rr-163"></a>
### RR-16.3 — Avoid allocations in hot paths

**Enforcement:** Hard: Review/Bench.

**Rule:** Use borrowing, arenas, SmallVec, or preallocation when measured.

<a id="rr-164"></a>
### RR-16.4 — Avoid string churn

**Enforcement:** Hard: Node/Review.

**Rule:** to_string/to_owned requires allocation justification.

<a id="rr-165"></a>
### RR-16.5 — Prefer iterators over index loops

**Enforcement:** Hard: Node/Clippy.

**Rule:** No C-style index loops.

<a id="rr-166"></a>
### RR-16.6 — Use cache-friendly layout when relevant

**Enforcement:** Hard: Review/Bench.

**Rule:** Select data structures based on access pattern.

<a id="rr-167"></a>
### RR-16.7 — Do not overuse trait objects in hot loops

**Enforcement:** Hard: Review/Bench.

**Rule:** Dynamic dispatch requires measured justification.

<a id="rr-168"></a>
### RR-16.8 — Do not overuse generics that bloat compile time

**Enforcement:** Hard: Review.

**Rule:** Balance monomorphization with API needs.

<a id="rr-169"></a>
### RR-16.9 — Use inline only with reason

**Enforcement:** Hard: Review.

**Rule:** #[inline] is not a decoration.

<a id="rr-1610"></a>
### RR-16.10 — Avoid large async futures

**Enforcement:** Hard: Clippy/Review.

**Rule:** Box or split large futures when measured/necessary.

<a id="rr-1611"></a>
### RR-16.11 — No accidental blocking I/O

**Enforcement:** Hard: Node/Review.

**Rule:** Blocking I/O must be isolated.

<a id="rr-1612"></a>
### RR-16.12 — Memory pools require tests

**Enforcement:** Hard: Review/Test.

**Rule:** Pools must prove reuse safety and bounded behavior.

<a id="rr-1613"></a>
### RR-16.13 — SIMD requires safe fallback

**Enforcement:** Hard: Review/Test.

**Rule:** CPU feature detection and fallback path are required.

<a id="rr-1614"></a>
### RR-16.14 — Benchmark both old and new paths

**Enforcement:** Hard: Review/Bench.

**Rule:** Migration/performance changes compare baseline.

<a id="rr-1615"></a>
### RR-16.15 — Do not optimize away clarity without data

**Enforcement:** Hard: Review.

**Rule:** Performance choices must be justified.

## 17. Traits, generics, macros, and language features

<a id="rr-171"></a>
### RR-17.1 — Prefer functions over macros

**Enforcement:** Hard: Review.

**Rule:** Macros require syntax-level need.

<a id="rr-172"></a>
### RR-17.2 — Macros live in macro modules/crates

**Enforcement:** Hard: Review.

**Rule:** Do not scatter macro_rules! through domain modules.

<a id="rr-173"></a>
### RR-17.3 — Procedural macros require approval

**Enforcement:** Hard: Review.

**Rule:** Proc macros increase compile/audit cost.

<a id="rr-174"></a>
### RR-17.4 — Traits model behavior, not storage buckets

**Enforcement:** Hard: Review.

**Rule:** Avoid traits with unrelated method collections.

<a id="rr-175"></a>
### RR-17.5 — Use associated types for coupled types

**Enforcement:** Hard: Review.

**Rule:** Prefer associated types when implementor owns the type relationship.

<a id="rr-176"></a>
### RR-17.6 — Avoid over-generic bounds

**Enforcement:** Hard: Review.

**Rule:** Use the weakest useful bound and keep API readable.

<a id="rr-177"></a>
### RR-17.7 — Trait objects for open sets

**Enforcement:** Hard: Review.

**Rule:** Use dyn traits when extension by external implementations is required.

<a id="rr-178"></a>
### RR-17.8 — Enums for closed sets

**Enforcement:** Hard: Review.

**Rule:** Use enums when all variants are known.

<a id="rr-179"></a>
### RR-17.9 — No blanket impl surprises

**Enforcement:** Hard: Review.

**Rule:** Blanket impls must not cause coherence/API lock-in problems.

<a id="rr-1710"></a>
### RR-17.10 — No Deref abuse

**Enforcement:** Hard: Review/Clippy.

**Rule:** Deref is for smart-pointer-like types, not arbitrary forwarding.

<a id="rr-1711"></a>
### RR-17.11 — Derive only when semantics are correct

**Enforcement:** Hard: Review.

**Rule:** Clone/Copy/Ord/Hash derive must match domain meaning.

<a id="rr-1712"></a>
### RR-17.12 — Eq/Ord must be domain-correct

**Enforcement:** Hard: Review/Test.

**Rule:** Ordering and equality need clear semantics.

<a id="rr-1713"></a>
### RR-17.13 — Default must be valid

**Enforcement:** Hard: Review/Test.

**Rule:** Default cannot create invalid domain state.

<a id="rr-1714"></a>
### RR-17.14 — From must be infallible

**Enforcement:** Hard: Review.

**Rule:** Use TryFrom for fallible conversions.

<a id="rr-1715"></a>
### RR-17.15 — Into is usually caller-side

**Enforcement:** Hard: Review.

**Rule:** Implement From; let Into derive from it.

## 18. State, architecture, and maintainability

<a id="rr-181"></a>
### RR-18.1 — State machines use enums/typestate

**Enforcement:** Hard: Review.

**Rule:** No multiple booleans to represent states.

<a id="rr-182"></a>
### RR-18.2 — No invalid state structs

**Enforcement:** Hard: Review.

**Rule:** Do not combine flags/options that permit impossible combinations.

<a id="rr-183"></a>
### RR-18.3 — Command/query separation

**Enforcement:** Hard: Review.

**Rule:** Mutating operations and read operations should be explicit.

<a id="rr-184"></a>
### RR-18.4 — Domain layer is pure where possible

**Enforcement:** Hard: Review/Test.

**Rule:** Keep side effects in adapters.

<a id="rr-185"></a>
### RR-18.5 — No service god objects

**Enforcement:** Hard: Review.

**Rule:** Split large service structs by responsibility.

<a id="rr-186"></a>
### RR-18.6 — No temporal coupling without types

**Enforcement:** Hard: Review.

**Rule:** Use typestate or transaction objects for required sequences.

<a id="rr-187"></a>
### RR-18.7 — Configuration is typed

**Enforcement:** Hard: Review/Node partial.

**Rule:** No raw env/config strings beyond boundaries.

<a id="rr-188"></a>
### RR-18.8 — Policies are explicit types

**Enforcement:** Hard: Review.

**Rule:** Do not encode policy in scattered if statements.

<a id="rr-189"></a>
### RR-18.9 — No duplicated domain logic

**Enforcement:** Hard: Review/Test.

**Rule:** Centralize invariants in constructors/value objects.

<a id="rr-1810"></a>
### RR-18.10 — No hidden side effects in getters

**Enforcement:** Hard: Review.

**Rule:** Accessors should not perform I/O or mutation.

<a id="rr-1811"></a>
### RR-18.11 — No global singletons for test convenience

**Enforcement:** Hard: Review.

**Rule:** Inject dependencies explicitly.

<a id="rr-1812"></a>
### RR-18.12 — Ports/adapters are explicit

**Enforcement:** Hard: Review.

**Rule:** External systems are behind traits or concrete boundary modules.

<a id="rr-1813"></a>
### RR-18.13 — Transactions are explicit

**Enforcement:** Hard: Review.

**Rule:** Do not hide transactional behavior inside low-level helpers.

<a id="rr-1814"></a>
### RR-18.14 — Migration is incremental and tested

**Enforcement:** Hard: Review.

**Rule:** Refactors preserve behavior with tests.

<a id="rr-1815"></a>
### RR-18.15 — Architecture docs update with rule changes

**Enforcement:** Hard: Review.

**Rule:** Rules and code must not drift.

<a id="rr-1816"></a>
### RR-18.16 — Runtime Rust source must not contain inline string literals

**Enforcement:** Hard: Node when `enforceRuntimeStringLiterals` is enabled.

**Rule:** Stable runtime strings must move to constants, schema/protocol owners, or configured owner files. Boundary, test, and attribute exceptions must be explicit in config.

## 19. AI agent conduct and anti-slop rules

<a id="rr-191"></a>
### RR-19.1 — AI must run the gate before completion

**Enforcement:** Hard: Process.

**Rule:** A Rust task is incomplete until npm run rust:rules passes.

<a id="rr-192"></a>
### RR-19.2 — AI must not weaken validation

**Enforcement:** Hard: Review.

**Rule:** Do not edit scripts/config/rules to make bad code pass.

<a id="rr-193"></a>
### RR-19.3 — AI must not add allow/expect

**Enforcement:** Hard: Node.

**Rule:** Suppression is forbidden.

<a id="rr-194"></a>
### RR-19.4 — AI must prefer domain types over raw data

**Enforcement:** Hard: Node/Review.

**Rule:** No raw string/primitive signatures.

<a id="rr-195"></a>
### RR-19.5 — AI must not clone to appease borrow checker

**Enforcement:** Hard: Node/Review.

**Rule:** Clone requires justification.

<a id="rr-196"></a>
### RR-19.6 — AI must not use unwrap/expect

**Enforcement:** Hard: Node/Clippy.

**Rule:** Use real error handling.

<a id="rr-197"></a>
### RR-19.7 — AI must not invent helper modules

**Enforcement:** Hard: Node.

**Rule:** No utils/helpers/common/misc/shared/stuff.

<a id="rr-198"></a>
### RR-19.8 — AI must not use pub use as a barrel escape

**Enforcement:** Hard: Node.

**Rule:** Re-exports follow the configured project policy; strict projects should forbid `pub use` entirely.

<a id="rr-199"></a>
### RR-19.9 — AI must not delete tests

**Enforcement:** Hard: Review.

**Rule:** Fix code, not evidence.

<a id="rr-1910"></a>
### RR-19.10 — AI must not silence compiler errors with broader types

**Enforcement:** Hard: Review.

**Rule:** Do not replace precise types with String, Box<dyn Error>, or anyhow in domain.

<a id="rr-1911"></a>
### RR-19.11 — AI must explain invariant changes

**Enforcement:** Hard: Review.

**Rule:** New branded types need invariant documentation.

<a id="rr-1912"></a>
### RR-19.12 — AI must include negative tests for validation

**Enforcement:** Hard: Review.

**Rule:** Boundary code must test invalid inputs.

<a id="rr-1913"></a>
### RR-19.13 — AI must avoid dependency sprawl

**Enforcement:** Hard: Review/cargo-deny.

**Rule:** No new crate without reason.

<a id="rr-1914"></a>
### RR-19.14 — AI must not use unsafe unless explicitly requested and approved

**Enforcement:** Hard: Node.

**Rule:** Unsafe is forbidden by default.

<a id="rr-1915"></a>
### RR-19.15 — AI must not use println debugging

**Enforcement:** Hard: Node.

**Rule:** Use tracing if output is part of behavior.

<a id="rr-1916"></a>
### RR-19.16 — AI must not hide TODOs

**Enforcement:** Hard: Node.

**Rule:** todo!/unimplemented! fail.

<a id="rr-1917"></a>
### RR-19.17 — AI must preserve architecture boundaries

**Enforcement:** Hard: Review.

**Rule:** Do not move domain logic into adapters or vice versa.

<a id="rr-1918"></a>
### RR-19.18 — AI must write deterministic tests

**Enforcement:** Hard: Review.

**Rule:** No network/time/random flakiness.

<a id="rr-1919"></a>
### RR-19.19 — AI must keep generated code isolated

**Enforcement:** Hard: Review.

**Rule:** Generated code is not mixed with handwritten domain code.

<a id="rr-1920"></a>
### RR-19.20 — AI must report unresolved rule conflicts

**Enforcement:** Hard: Process.

**Rule:** If two rules appear to conflict, stop and identify the rule IDs rather than guessing.

## 20. Migration, maintenance, and governance

<a id="rr-201"></a>
### RR-20.1 — Rules have stable IDs

**Enforcement:** Hard: Process.

**Rule:** Do not renumber existing rules; append or deprecate.

<a id="rr-202"></a>
### RR-20.2 — Rule changes require tests

**Enforcement:** Hard: Node test.

**Rule:** Every new Node-enforced rule needs pass/fail fixtures.

<a id="rr-203"></a>
### RR-20.3 — Validation output must be helpful

**Enforcement:** Hard: Node test.

**Rule:** Failures include rule ID, reason, doc anchor, and fix snippet.

<a id="rr-204"></a>
### RR-20.4 — Legacy violations require tracked migration

**Enforcement:** Hard: Review.

**Rule:** Do not normalize broken code; create explicit remediation tasks.

<a id="rr-205"></a>
### RR-20.5 — No permanent grandfathering

**Enforcement:** Hard: Review.

**Rule:** Temporary migration config must have removal plan.

<a id="rr-206"></a>
### RR-20.6 — Measure rule false positives

**Enforcement:** Hard: Review.

**Rule:** Improve validators instead of teaching agents to ignore them.

<a id="rr-207"></a>
### RR-20.7 — Audit validation scripts themselves

**Enforcement:** Hard: Node test/Review.

**Rule:** The validator is production code and needs tests.

<a id="rr-208"></a>
### RR-20.8 — Keep CI and local behavior identical

**Enforcement:** Hard: CI.

**Rule:** Same command, same config.

<a id="rr-209"></a>
### RR-20.9 — Protect rule files with CODEOWNERS

**Enforcement:** Hard: Repo policy.

**Rule:** Rules/scripts/config require senior review.

<a id="rr-2010"></a>
### RR-20.10 — Document accepted boundary modules

**Enforcement:** Hard: Review.

**Rule:** Raw input quarantine paths must be intentional.

<a id="rr-2011"></a>
### RR-20.11 — Review dependency policy regularly

**Enforcement:** Hard: Maintenance.

**Rule:** Update deny/audit policy with project needs.

<a id="rr-2012"></a>
### RR-20.12 — Review Rust toolchain updates deliberately

**Enforcement:** Hard: Maintenance.

**Rule:** Toolchain bumps run full gate and fix new lints.

<a id="rr-2013"></a>
### RR-20.13 — No silent validation skips

**Enforcement:** Hard: CI.

**Rule:** CI must not ignore exit codes.

<a id="rr-2014"></a>
### RR-20.14 — Keep examples compliant

**Enforcement:** Hard: Cargo/Review.

**Rule:** Examples teach agents; they must follow the rules.

<a id="rr-2015"></a>
### RR-20.15 — Prefer smaller enforceable rules over vague principles

**Enforcement:** Hard: Process.

**Rule:** Every new rule should say how it is enforced or tested.

## Validator contract

The Node validator must:

- exit with code `1` when any hard violation is found;
- include the rule ID, title, file, line, reason, rule-doc anchor, fix snippet, and source line;
- preserve deterministic output ordering;
- avoid network access;
- use no npm dependencies by default;
- include tests that verify both passing and failing fixtures.

## Boundary quarantine

Raw strings and raw OS/path values may only exist at explicit boundaries configured in `rust-rules.config.json`. Boundary modules must convert raw input into branded types immediately. Domain modules must not accept raw text, raw paths, raw numbers, or booleans in function signatures.
