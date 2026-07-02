function ruleMetadataEntries(rows) {
  return Object.fromEntries(
    rows.map(([id, title, snippet]) => [id, { title, snippet }]),
  );
}

const RULES_VALUE = Object.freeze({
  ...ruleMetadataEntries([
    ["RR-3.29", "FFI exports must not unwind", "Wrap FFI exports in catch_unwind or use an abort-on-panic profile."],
    ["RR-3.30", "Unsafe crates require Miri proof", "Add MIRI-PROOF evidence for unsafe code or remove unsafe constructs."],
    ["RR-3.31", "Unsafe crates require cargo-geiger report", "Add GEIGER-PROOF evidence for unsafe code or remove unsafe constructs."],
    ["RR-6.44", "Newtypes must have validated constructors", "Add try_new or parse constructors for raw-field newtypes."],
    ["RR-6.50", "Domain value objects must implement Debug intentionally", "Derive or implement Debug for public domain value objects, unless they are secret types."],
    ["RR-6.52", "Secret types must have redacted formatting", "Implement redacted Debug/Display for secret, token, key, credential, and password types."],
    ["RR-8.17", "Do not await while holding lock guards", "Drop lock guards before await or use async-aware locking patterns."],
    ["RR-8.22", "Retry loops require bounded policy", "Use a RetryPolicy or bounded backoff object instead of ad-hoc retry loops."],
    ["RR-8.24", "select! branches require cancellation-safety notes", "Add CANCEL-SAFE notes explaining cancellation behavior for select! branches."],
    ["RR-8.26", "CPU work in async paths must be isolated", "Move CPU-heavy work to spawn_blocking or a worker boundary."],
    ["RR-9.17", "Heavy dependencies need explicit default feature policy", "Set default-features explicitly for heavy runtime crates."],
    ["RR-9.18", "New dependencies require justification", "Add DEPENDENCY-JUSTIFICATION near new direct dependencies or in policy."],
    ["RR-9.19", "Duplicate direct dependency versions are forbidden", "Align direct dependency requirements across workspace members."],
    ["RR-9.20", "Runtime proc-macro dependencies require approval", "Move proc-macro crates to build/dev dependencies or add approval."],
    ["RR-9.21", "Native dependencies require approval", "Add NATIVE-DEPENDENCY-JUSTIFICATION for native/sys/build-linked crates."],
    ["RR-9.23", "Yanked crate versions are forbidden", "Configure dependency policy to deny yanked versions."],
    ["RR-9.24", "Unmaintained crates are forbidden when advisory data exists", "Configure advisory policy to deny or explicitly review unmaintained crates."],
    ["RR-9.26", "Workspace dependencies must use path/workspace linkage", "Use workspace/path dependencies between workspace members, not registry versions."],
    ["RR-9.27", "Dev dependencies must not leak into production", "Move test-only crates to dev-dependencies."],
    ["RR-9.28", "Test-only crates are restricted to dev-dependencies", "Use dev-dependencies for test-only crates such as proptest, rstest, mockall, and criterion."],
    ["RR-9.29", "Runtime crates cannot depend on test-only crates", "Remove test-only crates from runtime dependencies."],
    ["RR-12.16", "Validated constructors require invalid-input tests", "Add invalid-input tests for each try_new constructor."],
    ["RR-12.17", "Parsers require negative and edge-case tests", "Add invalid, empty, oversized, and malformed parser tests."],
    ["RR-12.18", "DTO conversions require negative tests", "Add negative tests for boundary DTO to domain conversions."],
    ["RR-12.19", "Bugfixes require regression tests", "Add a REGRESSION-TEST marker for bugfix behavior."],
    ["RR-12.20", "Domain tests cannot use should_panic casually", "Use Result/assertions instead of #[should_panic] unless testing a panic contract."],
    ["RR-12.21", "Tests cannot unwrap without justification", "Use exact assertions or add TEST-UNWRAP-JUSTIFICATION for exceptional unwraps."],
    ["RR-12.24", "Test bodies cannot be empty", "Add assertions or remove placeholder tests."],
    ["RR-12.25", "Construction-only tests must assert behavior", "Assert validation/output instead of only constructing a value."],
    ["RR-12.26", "Snapshot tests must redact volatile values", "Redact timestamps, UUIDs, random IDs, and secrets in snapshot tests."],
    ["RR-12.27", "Normalizers and parsers require property tests", "Add proptest/quickcheck coverage for normalizers and parsers."],
    ["RR-12.28", "Binary and network parsers require fuzz targets", "Add fuzz target evidence for binary or network parser inputs."],
    ["RR-12.29", "Concurrency code requires cancellation tests", "Add shutdown/cancellation tests for spawn, channel, select, and async loop code."],
    ["RR-12.30", "Unsafe modules require Miri proof", "Add MIRI-PROOF evidence for unsafe modules."],
    ["RR-14.17", "Serialize derives require serialization docs", "Document serialization contract before deriving Serialize on domain types."],
    ["RR-14.19", "serde(default) requires justification", "Add DEFAULT-JUSTIFICATION for serde(default)."],
    ["RR-14.20", "DTO structs must live in boundary modules", "Move DTO/request/response/envelope structs under boundary, serde, transport, or adapter modules."],
    ["RR-14.21", "DTO names must use boundary suffixes", "Name boundary shapes with Dto, Request, Response, Envelope, or configured suffixes."],
    ["RR-14.22", "Domain modules cannot import DTO modules", "Move DTO imports to boundary/adapters and convert into domain types."],
    ["RR-14.23", "Boundary DTOs must convert to domain explicitly", "Add TryFrom, From, or a named mapper from DTO shapes into domain types."],
    ["RR-14.24", "Public serde enums must be tagged", "Use serde(tag = ...) or add SERDE-TAG-JUSTIFICATION."],
    ["RR-14.25", "External schema shapes require round-trip tests", "Add round-trip tests for externally serialized shapes."],
    ["RR-14.26", "Strict config inputs must deny unknown fields", "Add #[serde(deny_unknown_fields)] to strict config/input DTOs."],
    ["RR-14.27", "serde(flatten) requires justification", "Add FLATTEN-JUSTIFICATION for serde(flatten)."],
    ["RR-14.28", "Domain code cannot use raw base64 strings", "Use branded binary/blob/base64 value objects in domain code."],
    ["RR-14.29", "serde_json::from_str is boundary-only", "Decode JSON at boundary modules and pass typed domain values inward."],
    ["RR-14.30", "Do not deserialize directly into domain types", "Deserialize into DTOs, then validate into domain types."],
  ]),
  "RR-1.1": {
    title: "Rust toolchain must be pinned",
    snippet:
      "Add rust-toolchain.toml with channel and rustfmt/clippy components.",
  },
  "RR-1.2": {
    title: "Cargo.lock must exist for deterministic builds",
    snippet:
      "Run cargo generate-lockfile and commit Cargo.lock unless this is a publish-only library with a documented exception.",
  },
  "RR-1.3": {
    title: "Clippy configuration must exist",
    snippet:
      "Add clippy.toml to centralize lint thresholds and future policy knobs.",
  },
  "RR-1.4": {
    title: "Dependency policy must exist",
    snippet: "Add deny.toml and run cargo deny check in CI.",
  },
  "RR-1.5": {
    title: "Cargo.toml must declare rust-version",
    snippet: "Set package.rust-version or workspace.package.rust-version.",
  },
  "RR-2.1": {
    title: "No lint suppression attributes",
    snippet:
      "Do not use #[allow(...)] or #[expect(...)]. Fix the code or strengthen the rule instead of suppressing it.",
  },
  "RR-2.2": {
    title: "No validator suppression comments",
    snippet:
      "Comments such as rust-rules: ignore/allow/skip/disable are forbidden.",
  },
  "RR-3.1": {
    title: "Unsafe Rust is forbidden by default",
    snippet:
      "Use safe Rust. If unsafe is truly required, isolate it in an approved FFI/sys crate and set allowUnsafeCode with owner review.",
  },
  "RR-3.2": {
    title: "Unsafe code requires SAFETY documentation",
    snippet:
      "Every unsafe block requires a nearby // SAFETY: comment explaining the invariants.",
  },
  "RR-3.3": {
    title: "Unsafe functions require a # Safety section",
    snippet: "Document caller obligations under a rustdoc # Safety section.",
  },
  "RR-3.4": {
    title: "Raw pointers are forbidden in public/domain APIs",
    snippet:
      "Use references, NonNull wrappers, or isolated FFI boundary types.",
  },
  "RR-3.16": {
    title: "transmute is forbidden",
    snippet:
      "Replace transmute with typed conversion, checked parsing, or an isolated reviewed boundary.",
  },
  "RR-3.17": {
    title: "MaybeUninit is forbidden outside unsafe owners",
    snippet:
      "Use initialized safe types unless an approved unsafe-owner module carries the invariants.",
  },
  "RR-3.18": {
    title: "ManuallyDrop requires unsafe justification",
    snippet:
      "Avoid ManuallyDrop or isolate it behind reviewed unsafe invariants.",
  },
  "RR-3.19": {
    title: "mem::forget is forbidden",
    snippet:
      "Use explicit ownership transfer or drop behavior instead of leaking values with mem::forget.",
  },
  "RR-3.20": {
    title: "Box::leak requires justification",
    snippet:
      "Avoid permanent leaks or add LEAK-JUSTIFICATION for reviewed singleton state.",
  },
  "RR-3.21": {
    title: "static mut is forbidden",
    snippet:
      "Use OnceLock, LazyLock, atomics, or synchronized ownership instead of static mut.",
  },
  "RR-3.22": {
    title: "UnsafeCell is forbidden outside approved primitives",
    snippet:
      "Use safe synchronization or isolate UnsafeCell in reviewed concurrency primitives.",
  },
  "RR-3.23": {
    title: "unsafe Send/Sync impls require safety proof",
    snippet:
      "Add SAFETY documentation and concurrency tests before unsafe Send or Sync implementations.",
  },
  "RR-3.24": {
    title: "get_unchecked is forbidden",
    snippet:
      "Use checked indexing or typed bounds proofs instead of get_unchecked.",
  },
  "RR-3.25": {
    title: "Raw pointer dereference requires safety proof",
    snippet:
      "Keep raw pointer dereference inside approved unsafe modules with local SAFETY comments.",
  },
  "RR-3.26": {
    title: "FFI extern blocks must live in FFI owner modules",
    snippet:
      "Move extern blocks to ffi/sys modules and wrap them in safe domain APIs.",
  },
  "RR-3.27": {
    title: "FFI structs require repr(C)",
    snippet:
      "Annotate FFI-facing structs with #[repr(C)] and keep them in FFI owner modules.",
  },
  "RR-3.28": {
    title: "no_mangle is restricted to FFI owner modules",
    snippet:
      "Use #[no_mangle] only in configured FFI export modules.",
  },
  "RR-3.32": {
    title: "Unsafe code is forbidden in tests as an escape hatch",
    snippet:
      "Tests must prove safe APIs, not bypass invariants with unsafe code.",
  },
  "RR-3.33": {
    title: "allow(unsafe_code) is forbidden",
    snippet:
      "Use an approved unsafe profile instead of suppressing unsafe policy with attributes.",
  },
  "RR-4.1": {
    title: "No unwrap/expect",
    snippet:
      "Replace .unwrap()/.expect() with ?, match, ok_or_else, or a typed error.",
  },
  "RR-4.2": {
    title: "No panic-like macros in production paths",
    snippet:
      "Replace panic!/todo!/unimplemented!/unreachable! with Result, typed errors, or explicit state modelling.",
  },
  "RR-4.3": {
    title: "No debug/console macros in Rust logic",
    snippet:
      "Use tracing/logging abstractions. dbg!, println!, and eprintln! are not allowed in source logic.",
  },
  "RR-4.4": {
    title: "Domain code must not use erased application errors",
    snippet:
      "Use a typed error enum. anyhow::Result and Box<dyn Error> belong only at application boundaries.",
  },
  "RR-4.7": {
    title: "No stringly Result errors",
    snippet: "Return a typed error enum instead of Result<T, String>.",
  },
  "RR-4.8": {
    title: "No static string Result errors",
    snippet: "Return a typed error enum instead of Result<T, &'static str>.",
  },
  "RR-4.9": {
    title: "No literal Err values",
    snippet: "Return a typed error variant instead of Err(\"...\").",
  },
  "RR-4.10": {
    title: "No formatted string errors",
    snippet:
      "Put dynamic context into typed error fields instead of Err(format!(...)).",
  },
  "RR-4.11": {
    title: "No map_err to_string error erasure",
    snippet:
      "Preserve source errors with #[from], #[source], or typed error variants.",
  },
  "RR-4.12": {
    title: "Boolean success APIs are forbidden",
    snippet:
      "Return Result or a modeled status enum instead of bool from fallible domain operations.",
  },
  "RR-4.13": {
    title: "Sentinel error values are forbidden",
    snippet:
      "Return Option or Result instead of -1, empty string, or other sentinel failure values.",
  },
  "RR-4.14": {
    title: "Fallible constructors must return Result",
    snippet:
      "Use try_new/parse returning Result<Self, Error> for raw constructor inputs.",
  },
  "RR-4.15": {
    title: "main must not swallow fallible errors",
    snippet:
      "Return Result from main or handle every fallible call explicitly.",
  },
  "RR-4.16": {
    title: "No ignored fallible results",
    snippet:
      "Handle the Result, propagate it with ?, or use a typed helper that documents intentional discard.",
  },
  "RR-4.17": {
    title: "No ok() error swallowing",
    snippet:
      "Do not turn Result into Option with .ok(); preserve or map the typed error.",
  },
  "RR-4.18": {
    title: "No unwrap_or_default on fallible domain data",
    snippet:
      "Handle failure explicitly instead of defaulting away parse/config/domain errors.",
  },
  "RR-4.19": {
    title: "No unwrap_or hiding parse/config failures",
    snippet:
      "Handle parse, env, and config failures explicitly instead of replacing them with fallback values.",
  },
  "RR-4.20": {
    title: "Error enums must implement Debug and Error",
    snippet:
      "Derive Debug and thiserror::Error or implement std::error::Error for custom error enums.",
  },
  "RR-4.21": {
    title: "Wrapped source errors require source/from metadata",
    snippet:
      "Mark wrapped error sources with #[source] or #[from] so diagnostics keep provenance.",
  },
  "RR-4.22": {
    title: "Do not log and return the same error",
    snippet:
      "Log at the boundary or return the error, not both in the same function.",
  },
  "RR-5.1": {
    title: "Clone must be justified",
    snippet:
      "Add a nearby CLONE-JUSTIFICATION: comment or refactor to borrow, move, Arc, or Cow.",
  },
  "RR-5.2": {
    title: "String allocation must be justified",
    snippet:
      "Add ALLOC-JUSTIFICATION: near to_string()/to_owned() or avoid allocation with borrowing/domain types.",
  },
  "RR-5.3": {
    title: "Unchecked indexing is forbidden",
    snippet:
      "Use get(), get_mut(), split_at_checked-style logic, or typed indexes that prove bounds.",
  },
  "RR-5.4": {
    title: "Lossy casts require justification",
    snippet:
      "Use TryFrom/TryInto or add CAST-JUSTIFICATION: explaining why the cast is safe.",
  },
  "RR-6.1": {
    title: "No raw string types in domain function signatures",
    snippet:
      "Replace String/&str/str/Cow<str>/PathBuf/OsString with branded domain types. Raw text may exist only in configured boundary or owner modules.",
  },
  "RR-6.2": {
    title: "No unbranded primitive types in domain function signatures",
    snippet:
      "Replace bool/numeric primitive parameters and returns with newtypes, enums, NonZero types, or domain value objects.",
  },
  "RR-6.3": {
    title: "No public raw fields",
    snippet:
      "Public fields must expose branded/domain types, not String, &str, bool, numbers, Vec<String>, or HashMap<String, _>.",
  },
  "RR-6.4": {
    title: "Raw private fields require brand invariants",
    snippet:
      "Private raw fields inside domain types require a nearby BRAND-INVARIANT: comment explaining validation and meaning.",
  },
  "RR-6.5": {
    title: "Type aliases must not disguise raw primitives",
    snippet:
      "Use tuple/newtype structs instead of type UserId = String or type Count = usize.",
  },
  "RR-6.6": {
    title: "Tuple newtypes must not expose raw inner fields",
    snippet:
      "Use pub struct UserId(String); not pub struct UserId(pub String); and validate construction.",
  },
  "RR-6.43": {
    title: "Public newtype fields are forbidden",
    snippet:
      "Keep tuple newtype fields private and expose validated constructors/accessors.",
  },
  "RR-6.27": {
    title: "No AsRef<str> domain parameters",
    snippet:
      "Convert raw text at adapters and accept branded domain types in domain APIs.",
  },
  "RR-6.28": {
    title: "No Into<String> domain parameters",
    snippet:
      "Decode into a branded type before entering the domain instead of accepting Into<String>.",
  },
  "RR-6.29": {
    title: "No Display-based domain identity parameters",
    snippet:
      "Accept branded identity types instead of impl Display for ID-like domain parameters.",
  },
  "RR-6.30": {
    title: "No Cow<str> in domain APIs",
    snippet:
      "Normalize raw text at boundaries and pass branded values through domain APIs.",
  },
  "RR-6.31": {
    title: "No Vec<String> domain APIs",
    snippet:
      "Use a typed collection of branded values instead of Vec<String>.",
  },
  "RR-6.32": {
    title: "No HashMap<String, _> domain APIs",
    snippet:
      "Use a typed key newtype or domain map instead of HashMap<String, _>.",
  },
  "RR-6.33": {
    title: "No BTreeMap<String, _> domain APIs",
    snippet:
      "Use a typed key newtype or domain map instead of BTreeMap<String, _>.",
  },
  "RR-6.34": {
    title: "serde_json::Value is forbidden in domain code",
    snippet:
      "Decode untyped JSON at the boundary and pass modeled domain values inward.",
  },
  "RR-6.35": {
    title: "Option<String> is forbidden in domain structs",
    snippet:
      "Use a domain-specific optional value object instead of optional raw text.",
  },
  "RR-6.36": {
    title: "Option<bool> is forbidden for domain state",
    snippet:
      "Use an enum for tri-state or unknown state instead of Option<bool>.",
  },
  "RR-6.37": {
    title: "No boolean state clusters",
    snippet:
      "Replace multiple boolean state fields with an enum, typestate, or explicit state value object.",
  },
  "RR-6.38": {
    title: "Raw Duration is forbidden for named domain timing values",
    snippet:
      "Use branded timeout, TTL, delay, or deadline value objects instead of raw Duration.",
  },
  "RR-6.39": {
    title: "Raw time types are forbidden in public domain APIs",
    snippet:
      "Use branded timestamps or clock-owned value objects instead of SystemTime or Instant.",
  },
  "RR-6.40": {
    title: "Raw URL strings are forbidden",
    snippet:
      "Decode URL/URI text into branded URL value objects at boundaries.",
  },
  "RR-6.41": {
    title: "Raw path strings are forbidden",
    snippet:
      "Decode file, dir, and path text into branded path value objects at boundaries.",
  },
  "RR-6.42": {
    title: "ID-like raw fields are forbidden",
    snippet:
      "Represent id/ref/key fields with branded newtypes instead of raw strings or integers.",
  },
  "RR-6.45": {
    title: "Newtype constructors must not panic",
    snippet:
      "Return Result from constructors instead of panicking or unwrapping validation.",
  },
  "RR-6.46": {
    title: "Numeric ID newtypes should use NonZero types",
    snippet:
      "Use NonZero numeric wrappers or validated domain IDs instead of raw numeric ID fields.",
  },
  "RR-6.47": {
    title: "No type aliases to domain collections",
    snippet:
      "Use collection newtypes instead of type aliases to Vec/HashMap/BTreeMap.",
  },
  "RR-6.48": {
    title: "Naked tuples are forbidden in public signatures",
    snippet:
      "Return named structs or domain value objects instead of positional tuples.",
  },
  "RR-6.49": {
    title: "Multi-primitive constructors are forbidden",
    snippet:
      "Use a named input value object for constructors with multiple primitive parameters.",
  },
  "RR-6.51": {
    title: "Secret types must not derive Debug or Display",
    snippet:
      "Implement redacted formatting for secret, token, key, and credential types.",
  },
  "RR-6.26": {
    title: "Serialized domain fields must not expose raw identity primitives",
    snippet:
      "Use typed domain newtypes or enums for public serialized id/ref/event/command fields.",
  },
  "RR-7.1": {
    title: "Wildcard imports are forbidden",
    snippet: "Replace use x::* with explicit imports.",
  },
  "RR-7.2": {
    title: "Wildcard public re-exports are forbidden",
    snippet:
      "Replace pub use x::* with direct module imports or explicit call-site paths.",
  },
  "RR-7.3": {
    title: "Public re-exports must match project policy",
    snippet:
      "For strict projects, remove pub use entirely. For facade projects, keep pub use only in configured facade files.",
  },
  "RR-7.4": {
    title: "Dumping-ground modules are forbidden",
    snippet:
      "Rename utils/helpers/common/misc/shared/stuff to a domain-specific module.",
  },
  "RR-7.5": {
    title: "Build scripts are forbidden by default",
    snippet:
      "Remove build.rs or explicitly approve it in config with a deterministic justification.",
  },
  "RR-8.1": {
    title: "Blocking primitives are forbidden inside async modules",
    snippet:
      "Do not combine async/.await with std::sync locks, std::thread::sleep, or blocking std::fs/std::net calls.",
  },
  "RR-8.16": {
    title: "std sync locks are forbidden in async modules",
    snippet:
      "Use async-aware synchronization or isolate blocking state outside async execution paths.",
  },
  "RR-8.2": {
    title: "C-style index loops are forbidden",
    snippet:
      "Use iterators, enumerate(), chunks(), windows(), or typed ranges.",
  },
  "RR-8.18": {
    title: "tokio::spawn handles must be tracked",
    snippet:
      "Store, await, or supervise JoinHandle values instead of fire-and-forget spawning.",
  },
  "RR-8.19": {
    title: "Fire-and-forget spawn requires task justification",
    snippet:
      "Store the JoinHandle or add TASK-JUSTIFICATION explaining supervision and shutdown.",
  },
  "RR-8.20": {
    title: "Unbounded channels require justification",
    snippet:
      "Use bounded channels or add CHANNEL-JUSTIFICATION explaining the backpressure boundary.",
  },
  "RR-8.21": {
    title: "External async I/O must use timeouts",
    snippet:
      "Wrap network and service futures in a timeout policy instead of awaiting indefinitely.",
  },
  "RR-8.23": {
    title: "Async loops require cancellation",
    snippet:
      "Add a cancellation branch or shutdown signal to async loop constructs.",
  },
  "RR-8.25": {
    title: "Blocking I/O is forbidden in async modules",
    snippet:
      "Use async file/network APIs or spawn_blocking for unavoidable blocking work.",
  },
  "RR-8.27": {
    title: "Libraries must not create global Tokio runtimes",
    snippet:
      "Accept a runtime from the application boundary instead of constructing one in a library.",
  },
  "RR-8.28": {
    title: "block_on is forbidden in library/domain code",
    snippet:
      "Keep async execution at application boundaries and expose async APIs inward.",
  },
  "RR-8.29": {
    title: "Sleep-based tests are forbidden",
    snippet:
      "Use controlled clocks, notifications, or deterministic polling instead of sleeping in tests.",
  },
  "RR-8.30": {
    title: "Arc<Mutex<T>> signatures must be domain-named",
    snippet:
      "Wrap shared mutable state in a named domain type instead of exposing raw Arc<Mutex<T>>.",
  },
  "RR-9.1": {
    title: "Dependency versions must be pinned semver requirements",
    snippet: 'Do not use dependency version = "*".',
  },
  "RR-9.2": {
    title: "Git dependencies are forbidden by default",
    snippet:
      "Use crates.io versions or explicitly approve a pinned git revision in dependency policy.",
  },
  "RR-9.3": {
    title: "Path dependencies are forbidden outside policy",
    snippet: "Use workspace members or explicitly approve path dependencies.",
  },
  "RR-9.4": {
    title: "Workspace path dependencies must stay inside the workspace",
    snippet:
      "Move path dependencies under the workspace root or publish/version the dependency.",
  },
  "RR-9.5": {
    title: "Direct dependency versions must not drift",
    snippet:
      "Align direct registry dependency requirements across changed manifests.",
  },
  "RR-9.16": {
    title: "Loose dependency version ranges are forbidden",
    snippet:
      "Use pinned semver requirements instead of *, >=, or major-only dependency ranges.",
  },
  "RR-9.22": {
    title: "GPL and AGPL licenses are forbidden by default",
    snippet:
      "Use an approved license or reviewed dependency policy before introducing copyleft dependencies.",
  },
  "RR-9.25": {
    title: "Cargo.lock must be current after manifest changes",
    snippet:
      "Run cargo generate-lockfile or cargo update after changing Cargo.toml.",
  },
  "RR-9.30": {
    title: "build-dependencies require approval",
    snippet:
      "Avoid build-time code execution or add a reviewed build dependency waiver.",
  },
  "RR-10.1": {
    title: "cargo fmt must pass",
    snippet:
      "Run cargo fmt --all -- --check or the scoped package equivalent before completing the task.",
  },
  "RR-10.2": {
    title: "cargo clippy hard mode must pass",
    snippet: "Run cargo clippy with -D warnings and selected deny lints.",
  },
  "RR-10.3": {
    title: "cargo test must pass",
    snippet: "Run cargo test for the workspace or touched crate.",
  },
  "RR-10.4": {
    title: "rustdoc warnings must be errors",
    snippet:
      "Run cargo doc with RUSTDOCFLAGS denying rustdoc warnings for release/readiness gates.",
  },
  "RR-11.1": {
    title: "cargo-deny must pass",
    snippet:
      "Run cargo deny check to enforce advisories, bans, licenses, and source policy.",
  },
  "RR-11.2": {
    title: "cargo-deny must be installed when dependency policy is required",
    snippet:
      "Install with cargo install --locked cargo-deny or disable only in local scan mode.",
  },
  "RR-11.3": {
    title: "cargo-audit must pass when enabled",
    snippet:
      "Run cargo audit and upgrade vulnerable crates instead of ignoring advisories.",
  },
  "RR-12.22": {
    title: "No weak is_ok assertions",
    snippet:
      "Assert exact Ok values or error variants instead of only checking is_ok().",
  },
  "RR-12.23": {
    title: "No weak is_some assertions",
    snippet:
      "Assert exact Some values instead of only checking is_some().",
  },
  "RR-14.16": {
    title: "Domain structs must not derive Deserialize directly",
    snippet:
      "Deserialize into DTOs at the boundary, then validate into domain types.",
  },
  "RR-14.18": {
    title: "Untagged serde enums require justification",
    snippet:
      "Avoid #[serde(untagged)] ambiguity or add SERDE-UNTAGGED-JUSTIFICATION at the boundary.",
  },
  "RR-18.16": {
    title: "Runtime Rust source must not contain inline string literals",
    snippet:
      "Move stable runtime strings to constants, schemas, protocol crates, or configured owner modules.",
  },
});

export const RULES = RULES_VALUE;

export const DEFAULT_CONFIG = Object.freeze({
  schemaVersion: 2,
  profileName: "strict",
  failOn: ["error"],
  failFast: false,
  enforceWorkspaceFiles: true,
  requireCargoDeny: true,
  requireCargoAudit: false,
  runCargoDoc: false,
  cargoOnFileScope: false,
  cargoOnDiffScope: false,
  cargoTestThreads: null,
  allowUnsafeCode: false,
  allowBuildRs: false,
  allowGitDependencies: false,
  allowPathDependencies: false,
  publicReexportPolicy: "forbid",
  languages: ["rust", "typescript", "python", "common"],
  rules: {},
  waivers: [],
  tools: {},
  harness: {
    store: "ndjson-duckdb",
    storageDir: ".enforce",
    maxArtifactBytes: 8000,
    maxRuns: 50,
    maxRunsPerTool: 20,
    maxFailedRuns: 20,
    pruneAfterDays: 14,
  },
  ignoreDirs: [
    ".git",
    ".ledger",
    ".hub",
    ".turbo",
    ".wrangler",
    ".enforce",
    "target",
    "node_modules",
    ".next",
    "dist",
    "build",
    "coverage",
    "output",
  ],
  ignoreFileGlobs: [],
  rustRoots: ["src", "crates", "tools"],
  crateRootGlobs: ["crates/*", "tools/*", "."],
  testFileGlobs: [
    "**/tests/**",
    "**/*_test.rs",
    "**/*_tests.rs",
    "**/*_test_support.rs",
    "**/*_test_fixture.rs",
    "**/*_test_fixtures.rs",
  ],
  // boundaryOwnerNote: Enforcer-owned Rust boundary defaults; edits require policy-integrity and self-scan validation.
  rawTypeBoundaryGlobs: [
    "src/bin/**",
    "src/main.rs",
  ],
  facadeFileGlobs: [
    "src/lib.rs",
    "src/api.rs",
    "src/prelude.rs",
    "src/**/api.rs",
    "src/**/prelude.rs",
  ],
  rawStringOwnerGlobs: [],
  domainPrimitiveOwnerGlobs: [],
  enforceRuntimeStringLiterals: false,
  runtimeStringOwnerGlobs: [],
  runtimeStringLineAllowPatterns: [
    "env!\\(",
    "#\\[tokio::main",
    "#\\[serde",
    "serde\\(",
    "cfg\\(",
    "#\\[path\\s*=",
    "panic!\\(",
    "format!\\(",
    '^\\s*(?:pub\\s+)?const\\s+[A-Z0-9_]+\\s*:\\s*&str\\s*=\\s*"',
  ],
  enforceSerializedPublicDomainPrimitives: false,
  serializedDomainOwnerGlobs: [],
  blockedProtocolDependencies: {},
  runtimeCrates: [],
  testOnlyCrates: [
    "criterion",
    "mockall",
    "pretty_assertions",
    "proptest",
    "rstest",
    "wiremock",
  ],
  allowedGitDependencies: [],
});

export const MERGED_ARRAY_CONFIG_KEYS = new Set(["ignoreDirs", "ignoreFileGlobs"]);
export const DEFAULT_ARCHITECTURE_POLICY_CHECKS = Object.freeze([
  "reexports",
  "validation-bypass",
  "placeholder-implementation",
  "skipped-focused-tests",
  "weak-assertions",
  "rust-string-boundaries",
  "no-zod-source",
  "no-naked-domain-strings",
  "no-test-doubles",
  "cross-platform-script-commands",
  "generated-artifacts",
]);
export const VERIFY_MODE_CHECKS = Object.freeze({
  fast: ["rule-coverage", "policy-integrity"],
  local: [
    "rule-coverage",
    "policy-integrity",
    "ci-integrity",
    "repo-governance",
    "package-determinism",
  ],
  ci: [
    "rule-coverage",
    "policy-integrity",
    "ci-integrity",
    "repo-governance",
    "package-determinism",
    "secrets",
    "dependency-policy",
    "sbom",
  ],
});
