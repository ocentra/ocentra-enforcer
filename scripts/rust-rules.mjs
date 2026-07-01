#!/usr/bin/env node
/*
 * Ocentra Enforcer hard gate.
 * Cross-platform Node.js validator with Effect Schema validated external inputs.
 */
import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import {
  ProductName,
  decodeCodexDoctorRequest,
  decodeCodexInstallRequest,
  decodeCodexUninstallRequest,
  decodeCheckToolArguments,
  decodeEnforcerConfig,
  decodeInitRequest,
  decodeRuleRegistry,
} from "../schemas/effect/enforcer-schemas.mjs";
import { routeRules } from "../src/routing.mjs";
import { GENERIC_RULES, runGenericScan } from "../src/generic-scanners.mjs";
import {
  CHECK_RULES,
  SCANNER_BACKED_CHECKS,
  normalizeCheckName,
  runStandaloneCheck,
} from "../src/checks.mjs";
import {
  applyCodexMcpInstallReport,
  applyCodexUninstallReport,
  createCodexDoctorReport as buildCodexDoctorReport,
  createCodexMcpInstallReport,
  createCodexUninstallReport,
} from "../src/codex-install.mjs";
import {
  applyWaivers,
  applyRulePolicy,
  normalizeFailOn,
  normalizeRuleOverrides,
  normalizeToolPolicies,
  policyForTool,
  splitFindings,
} from "../src/policy.mjs";
import {
  enrichFindingMetadata,
  enrichFindingsMetadata,
  registryRules as loadRegistryRules,
} from "../src/rule-registry.mjs";
import {
  lastFailure,
  listRuns,
  pruneRuns,
  readArtifact,
  resetRuns,
  runDiagnostics,
  runHarness,
  runSummary,
} from "../src/harness.mjs";
import { runCoordinationCli } from "../src/coordination/runner.mjs";
import { runProofCli } from "../src/proof.mjs";

function ruleMetadataEntries(rows) {
  return Object.fromEntries(
    rows.map(([id, title, snippet]) => [id, { title, snippet }]),
  );
}

const RULES = {
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
};

const DEFAULT_CONFIG = Object.freeze({
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

const MERGED_ARRAY_CONFIG_KEYS = new Set(["ignoreDirs", "ignoreFileGlobs"]);
const DEFAULT_ARCHITECTURE_POLICY_CHECKS = Object.freeze([
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
const VERIFY_MODE_CHECKS = Object.freeze({
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

const SCRIPT_PATH = fileURLToPath(import.meta.url);
const PACK_ROOT = path.resolve(path.join(path.dirname(SCRIPT_PATH), ".."));
const RULE_REGISTRY_PATH = path.join(PACK_ROOT, "rules", "rules.json");
const DEFAULT_INIT_ADAPTERS = ["codex", "mcp", "precommit", "github-actions"];
const GITHUB_ACTIONS_ADAPTERS = [
  "github-actions",
  "codeql",
  "dependency-policy",
  "secret-scan",
  "sbom",
];
let cachedRuleDocs = null;
let cachedRegistryRules = null;

function ruleRegistryRules() {
  if (cachedRegistryRules === null) {
    cachedRegistryRules = loadRegistryRules(PACK_ROOT);
  }
  return cachedRegistryRules;
}

function usage() {
  return `Ocentra Enforcer hard gate

Usage:
  ocentra-enforcer init --root <repo> --profile <profile> --adapters codex,mcp,precommit,github-actions
  ocentra-enforcer route [options]
  ocentra-enforcer check <name> [options]
  ocentra-enforcer verify [fast|local|ci] [options]
  ocentra-enforcer scan [options]
  ocentra-enforcer cargo [options]
  ocentra-enforcer doctor [options]
  ocentra-enforcer explain <RULE_ID>
  ocentra-enforcer run --root <repo> --tool <tool> -- <command...>
  ocentra-enforcer runs <list|summary|diagnostics|last-failure|artifact|prune|reset> [options]
  ocentra-enforcer proof <route|run|status|inventory|migrate-legacy|import-legacy|parity|claim|last-failure|diagnostics|artifact|reset|prune|export> [options]
  ocentra-enforcer coordination <ledger-command> [--hub <hub>] [--state-root <path>] [options]
  ocentra-enforcer coordination closeout --lane <lane> [--thread-id <thread>] [--all-owned] [--json]
  ocentra-enforcer coordination repair <legacy-hash|sequence|all> [--hub <hub>] [--state-root <path>] [--write]
  ocentra-enforcer coordination repair stale-claims --paths <path[,path]> [--owner <writer>] [--write]
  ocentra-enforcer ledger <ledger-command> [--hub <hub>] [--state-root <path>] [options]
  ocentra-enforcer architecture check --language rust --scope <files|diff|all> [options]
  ocentra-enforcer codex install --root <repo> --profile <profile> [--dry-run]
  ocentra-enforcer codex uninstall [--dry-run]
  ocentra-enforcer codex doctor --root <repo>

Compatibility aliases:
  rust-rules scan [options]
  rust-rules cargo [options]
  rust-rules doctor [options]
  rust-rules explain <RULE_ID>

Scope options:
  --files <path...>       Scan explicit files or directories.
  --crate <name>          Scan one Cargo package by package.name.
  --workspace, --all      Scan the whole workspace/repo.
  --base <sha> --head <sha>
                          Scan changed files between two git refs.

Common options:
  --root <path>           Repository root. Defaults to current directory.
  --config <path>         Optional config path. Defaults to ocentra-enforcer.config.json, then rust-rules.config.json.
  --profile <name>        Named profile for init output.
  --scan-only             With scan/cargo compatibility: skip Cargo commands.
  --verify-mode <mode>    Verify gate preset: fast, local, or ci. Defaults to local.
  --languages <list>      Comma-separated scan languages. Defaults to profile languages or rust,typescript,python,common.
  --check-config <path>   Optional check-specific config, for example single-source contracts.
  --output <path>         Optional output directory for checks such as sbom.
  --staged                With check secrets: scan staged files.
  --tracked               With check generated-artifacts: include tracked generated paths.
  --strict-empty-test-trees
                          With check required-tests: reject tests/proof trees that only contain .gitkeep.
  --json                  Print machine-readable JSON report.
  --help                  Show this help.

Proof options:
  --proof <id>            Proof definition id.
  --proofs <ids>          Comma-separated proof ids for claim/status.
  --plan <id>             Route by plan/workpack id.
  --capability <name>     Route or run by capability, for example ci, local, android-device, manual-required.
  --include-scripts       Proof inventory only: include bounded script rows instead of summary-only output.
  --legacy-paths <paths>  Comma-separated legacy proof artifact files or directories for import-legacy/parity.
  --script-root <path>    Legacy proof script root for migrate-legacy. Defaults to scripts/test.
  --write                 With migrate-legacy, write the generated profile proof registry and copied scripts.
  --include-all-scripts   With migrate-legacy, include non-proof scripts under the script root too.
  --pin                   Pin a proof run so retention does not prune it.
  --pr-ready              Validate proof claim as a PR-ready claim.
  --allow-dirty           Allow dirty worktree claims when explicitly requested.

Init options:
  --adapters <list>       Comma-separated adapters: codex,mcp,precommit,github-actions,husky,lefthook,codeql,dependency-policy,secret-scan,sbom.
  --dry-run               Print exact file plan without writing.
  --force                 Allow init to overwrite existing target files.

Codex install options:
  --codex-config <path>   Codex global config path. Defaults to CODEX_HOME/config.toml or ~/.codex/config.toml.
  --server-name <name>    MCP server name. Defaults to ocentra-enforcer.
  --ledger-root <path>    Per-PC ledger home. Defaults to <enforcer-install>/.ledger.
`;
}

function defaultArgs() {
  return {
    command: "scan",
    root: process.cwd(),
    rootExplicit: false,
    configPath: null,
    scanOnly: false,
    json: false,
    help: false,
    explainRuleId: null,
    profile: null,
    languages: null,
    adapters: null,
    dryRun: false,
    force: false,
    runTool: null,
    runCommand: [],
    runId: null,
    routeRuleId: null,
    checkName: null,
    checkConfigPath: null,
    output: null,
    staged: false,
    tracked: false,
    strictEmptyTestTrees: false,
    codexConfigPath: null,
    ledgerRoot: null,
    mcpServerName: "ocentra-enforcer",
    installSkill: true,
    installGlobalAgents: true,
    runsCommand: null,
    artifact: null,
    limit: null,
    limitBytes: null,
    severity: null,
    status: null,
    file: null,
    tag: null,
    crateName: null,
    packageName: null,
    domain: null,
    verifyMode: "local",
    scope: { mode: "all" },
  };
}

function parseArgs(argv) {
  const args = defaultArgs();
  const tokens = argv.slice(2);
  if (
    tokens[0] &&
    [
      "init",
      "route",
      "check",
      "verify",
      "scan",
      "cargo",
      "doctor",
      "explain",
      "run",
      "runs",
      "codex",
    ].includes(tokens[0])
  ) {
    args.command = tokens.shift();
  }

  if (args.command === "explain") {
    args.explainRuleId = tokens.shift() ?? null;
  }
  if (args.command === "check") {
    args.checkName =
      tokens[0] && !tokens[0].startsWith("-")
        ? normalizeCheckName(tokens.shift())
        : null;
  }
  if (args.command === "route" && tokens[0] && !tokens[0].startsWith("-")) {
    args.routeRuleId = tokens.shift();
  }
  if (args.command === "verify" && tokens[0] && !tokens[0].startsWith("-")) {
    args.verifyMode = normalizeVerifyMode(tokens.shift());
  }
  if (args.command === "runs") {
    args.runsCommand =
      tokens[0] && !tokens[0].startsWith("-") ? tokens.shift() : "list";
  }
  if (args.command === "codex") {
    const codexCommand =
      tokens[0] && !tokens[0].startsWith("-") ? tokens.shift() : "install";
    if (codexCommand === "install") {
      args.command = "codex-install";
      args.adapters = ["codex", "mcp"];
    } else if (codexCommand === "uninstall") {
      args.command = "codex-uninstall";
    } else if (codexCommand === "doctor") {
      args.command = "codex-doctor";
    } else {
      throw new Error(`Unknown codex command: ${codexCommand}`);
    }
  }

  const explicitFiles = [];
  for (let i = 0; i < tokens.length; i += 1) {
    const arg = tokens[i];
    if (arg === "--root") {
      args.root = tokens[++i];
      args.rootExplicit = true;
    } else if (arg === "--config") {
      args.configPath = tokens[++i];
    } else if (arg === "--profile") {
      args.profile = tokens[++i];
    } else if (arg === "--verify-mode") {
      args.verifyMode = normalizeVerifyMode(tokens[++i] ?? "");
    } else if (arg === "--languages") {
      args.languages = parseAdapterList(tokens[++i] ?? "");
    } else if (arg === "--adapters") {
      args.adapters = parseAdapterList(tokens[++i] ?? "");
    } else if (arg === "--tool") {
      args.runTool = tokens[++i];
    } else if (arg === "--run-id") {
      args.runId = tokens[++i];
    } else if (arg === "--rule-id") {
      args.routeRuleId = tokens[++i];
    } else if (arg === "--check-config") {
      args.checkConfigPath = tokens[++i];
    } else if (arg === "--output") {
      args.output = tokens[++i];
    } else if (arg === "--staged") {
      args.staged = true;
    } else if (arg === "--tracked") {
      args.tracked = true;
    } else if (arg === "--strict-empty-test-trees") {
      args.strictEmptyTestTrees = true;
    } else if (arg === "--codex-config") {
      args.codexConfigPath = tokens[++i];
    } else if (arg === "--ledger-root") {
      args.ledgerRoot = tokens[++i];
    } else if (arg === "--server-name") {
      args.mcpServerName = tokens[++i];
    } else if (arg === "--no-skill") {
      args.installSkill = false;
    } else if (arg === "--no-global-agents") {
      args.installGlobalAgents = false;
    } else if (arg === "--artifact") {
      args.artifact = tokens[++i];
    } else if (arg === "--limit") {
      args.limit = Number(tokens[++i]);
    } else if (arg === "--limit-bytes") {
      args.limitBytes = Number(tokens[++i]);
    } else if (arg === "--severity") {
      args.severity = tokens[++i];
    } else if (arg === "--status") {
      args.status = tokens[++i];
    } else if (arg === "--file") {
      args.file = tokens[++i];
    } else if (arg === "--tag") {
      args.tag = tokens[++i];
    } else if (arg === "--domain") {
      args.domain = tokens[++i];
    } else if (arg === "--package-name") {
      args.packageName = tokens[++i];
    } else if (arg === "--crate-name") {
      args.crateName = tokens[++i];
    } else if (arg === "--dry-run") {
      args.dryRun = true;
    } else if (arg === "--force") {
      args.force = true;
    } else if (arg === "--scan-only") {
      args.scanOnly = true;
    } else if (arg === "--json") {
      args.json = true;
    } else if (arg === "--workspace" || arg === "--all") {
      args.scope = { mode: "all" };
    } else if (arg === "--crate" || arg === "-p" || arg === "--package") {
      const crateName = tokens[++i];
      args.crateName = args.crateName ?? crateName;
      args.scope = { mode: "crate", crateName };
    } else if (arg === "--base") {
      const base = tokens[++i] ?? null;
      const current =
        args.scope.mode === "diff"
          ? args.scope
          : { mode: "diff", base: null, head: null };
      args.scope = { ...current, base };
    } else if (arg === "--head") {
      const head = tokens[++i] ?? null;
      const current =
        args.scope.mode === "diff"
          ? args.scope
          : { mode: "diff", base: null, head: null };
      args.scope = { ...current, head };
    } else if (arg === "--files") {
      for (let fileIndex = i + 1; fileIndex < tokens.length; fileIndex += 1) {
        if (tokens[fileIndex].startsWith("-")) {
          i = fileIndex - 1;
          break;
        }
        explicitFiles.push(tokens[fileIndex]);
        i = fileIndex;
      }
    } else if (arg === "--help" || arg === "-h") {
      args.help = true;
    } else if (arg === "--") {
      args.runCommand = tokens.slice(i + 1);
      break;
    } else if (arg.startsWith("-")) {
      throw new Error(`Unknown argument: ${arg}`);
    } else {
      explicitFiles.push(arg);
    }
  }

  if (explicitFiles.length > 0) {
    args.scope = { mode: "files", files: explicitFiles };
  }
  if (args.scope.mode === "diff" && (!args.scope.base || !args.scope.head)) {
    throw new Error("Diff scope requires --base <sha> --head <sha>.");
  }

  return args;
}

function loadConfig(root, explicitPath, profile = null) {
  const profileConfig = profile ? readProfileConfig(profile) : {};
  const candidate = resolveConfigCandidate(root, explicitPath, profile);
  let userConfig = {};
  if (candidate && fs.existsSync(candidate)) {
    userConfig = JSON.parse(fs.readFileSync(candidate, "utf8"));
  }
  return normalizeConfig(
    decodeEnforcerConfig(
      mergeConfigLayers(DEFAULT_CONFIG, profileConfig, userConfig),
    ),
  );
}

function resolveConfigCandidate(root, explicitPath, profile) {
  if (explicitPath)
    return path.isAbsolute(explicitPath)
      ? explicitPath
      : path.join(root, explicitPath);
  if (profile) {
    return null;
  }
  const targetConfig = resolveDefaultConfigPath(root);
  if (targetConfig) return targetConfig;
  return path.join(PACK_ROOT, "profiles", "strict.json");
}

function readProfileConfig(profile) {
  if (!/^[A-Za-z0-9][A-Za-z0-9_.-]*$/u.test(profile))
    throw new Error(`Invalid profile name: ${profile}`);
  const profilePath = path.join(PACK_ROOT, "profiles", `${profile}.json`);
  if (!fs.existsSync(profilePath))
    throw new Error(
      `Unknown Ocentra Enforcer profile "${profile}". Expected ${profilePath}.`,
    );
  return JSON.parse(fs.readFileSync(profilePath, "utf8"));
}

function mergeConfigLayers(...layers) {
  const merged = {};
  for (const layer of layers) {
    const previousRules = merged.rules;
    const previousTools = merged.tools;
    const previousHarness = merged.harness;
    const previousArrays = Object.fromEntries(
      [...MERGED_ARRAY_CONFIG_KEYS].map((key) => [
        key,
        Array.isArray(merged[key]) ? merged[key] : [],
      ]),
    );
    Object.assign(merged, layer);
    if (layer.rules)
      merged.rules = { ...(previousRules ?? {}), ...layer.rules };
    if (layer.tools)
      merged.tools = { ...(previousTools ?? {}), ...layer.tools };
    if (layer.harness)
      merged.harness = { ...(previousHarness ?? {}), ...layer.harness };
    for (const key of MERGED_ARRAY_CONFIG_KEYS) {
      if (Array.isArray(layer[key]))
        merged[key] = [...new Set([...previousArrays[key], ...layer[key]])];
    }
  }
  return merged;
}

function resolveDefaultConfigPath(root) {
  const branded = path.join(root, "ocentra-enforcer.config.json");
  if (fs.existsSync(branded)) return branded;
  const legacy = path.join(root, "rust-rules.config.json");
  if (fs.existsSync(legacy)) return legacy;
  return null;
}

function normalizeConfig(config) {
  return {
    ...config,
    rawFailOn:
      config.rawFailOn ?? (Array.isArray(config.failOn) ? [...config.failOn] : null),
    failOn: normalizeFailOn(config.failOn),
    rules: normalizeRuleOverrides(config.rules),
    waivers: Array.isArray(config.waivers) ? config.waivers : [],
    tools: normalizeToolPolicies(config.tools),
    runtimeStringLineAllowRegexps: config.runtimeStringLineAllowPatterns.map(
      (pattern) => new RegExp(pattern, "u"),
    ),
    allowedGitDependenciesSet: new Set(config.allowedGitDependencies),
    runtimeCratesSet: new Set(config.runtimeCrates),
    testOnlyCratesSet: new Set(config.testOnlyCrates),
  };
}

function parseAdapterList(value) {
  return value
    .split(",")
    .map((entry) => entry.trim())
    .filter(Boolean);
}

function normalizeVerifyMode(value) {
  const mode = String(value ?? "local").trim().toLowerCase();
  if (mode === "") return "local";
  if (!Object.hasOwn(VERIFY_MODE_CHECKS, mode)) {
    throw new Error(`Unknown verify mode: ${value}`);
  }
  return mode;
}

function createInitReport(args) {
  const request = decodeInitRequest({
    root: path.resolve(args.root ?? process.cwd()),
    profile: args.profile ?? "strict",
    adapters: args.adapters ?? DEFAULT_INIT_ADAPTERS,
    dryRun: args.dryRun,
    force: args.force,
  });
  const root = path.resolve(request.root ?? process.cwd());
  const adapters = expandAdapters(
    root,
    request.adapters ?? DEFAULT_INIT_ADAPTERS,
  );
  const writes = buildInitWrites(
    root,
    request.profile ?? "strict",
    adapters,
    Boolean(request.force),
  );
  return {
    ok: true,
    command: "init",
    productName: ProductName,
    root,
    profile: request.profile ?? "strict",
    dryRun: Boolean(request.dryRun),
    force: Boolean(request.force),
    adapters,
    files: writes,
  };
}

function createCodexInstallReport(args) {
  const request = decodeCodexInstallRequest({
    root: args.rootExplicit ? path.resolve(args.root) : undefined,
    profile: args.profile ?? "strict",
    dryRun: args.dryRun,
    force: args.force,
    codexConfigPath: args.codexConfigPath ?? undefined,
    ledgerRoot: args.ledgerRoot ?? undefined,
    serverName: args.mcpServerName ?? undefined,
    installSkill: args.installSkill,
    installGlobalAgents: args.installGlobalAgents,
  });
  const target = request.root
    ? createInitReport({
        ...args,
        root: request.root,
        profile: request.profile ?? "strict",
        adapters: ["codex", "mcp"],
        dryRun: request.dryRun,
        force: request.force,
      })
    : null;
  const codexMcp = createCodexMcpInstallReport({
    packRoot: PACK_ROOT,
    codexConfigPath: request.codexConfigPath,
    ledgerRoot: request.ledgerRoot,
    serverName: request.serverName ?? "ocentra-enforcer",
    installSkill: request.installSkill ?? true,
    installGlobalAgents: request.installGlobalAgents ?? true,
    dryRun: Boolean(request.dryRun),
  });
  return {
    ok: (target?.ok ?? true) && codexMcp.ok,
    command: "codex-install",
    productName: ProductName,
    root: target?.root ?? null,
    profile: request.profile ?? "strict",
    dryRun: Boolean(request.dryRun),
    force: Boolean(request.force),
    target,
    codexMcp,
  };
}

function applyCodexInstallReport(report) {
  if (!report.dryRun) {
    if (report.target) applyInitReport(report.target);
    report.codexMcp = applyCodexMcpInstallReport(report.codexMcp);
  }
  return report;
}

function createCodexUninstallCliReport(args) {
  const request = decodeCodexUninstallRequest({
    dryRun: args.dryRun,
    codexConfigPath: args.codexConfigPath ?? undefined,
    serverName: args.mcpServerName ?? undefined,
    removeSkill: args.installSkill,
    removeGlobalAgents: args.installGlobalAgents,
  });
  return createCodexUninstallReport({
    packRoot: PACK_ROOT,
    codexConfigPath: request.codexConfigPath,
    serverName: request.serverName ?? "ocentra-enforcer",
    removeSkill: request.removeSkill ?? true,
    removeGlobalAgents: request.removeGlobalAgents ?? true,
    dryRun: Boolean(request.dryRun),
  });
}

function createCodexDoctorReport(args) {
  const request = decodeCodexDoctorRequest({
    root: args.root ? path.resolve(args.root) : undefined,
    codexConfigPath: args.codexConfigPath ?? undefined,
    serverName: args.mcpServerName ?? undefined,
  });
  return buildCodexDoctorReport({
    packRoot: PACK_ROOT,
    root: request.root,
    codexConfigPath: request.codexConfigPath,
    serverName: request.serverName ?? "ocentra-enforcer",
  });
}

function expandAdapters(root, requestedAdapters) {
  const adapters = new Set(
    requestedAdapters.length > 0 ? requestedAdapters : DEFAULT_INIT_ADAPTERS,
  );
  if (adapters.has("github-actions")) {
    for (const adapter of GITHUB_ACTIONS_ADAPTERS) adapters.add(adapter);
  }
  if (adapters.has("precommit") && targetUsesHusky(root)) adapters.add("husky");
  return [...adapters].sort((a, b) => a.localeCompare(b));
}

function targetUsesHusky(root) {
  if (fs.existsSync(path.join(root, ".husky"))) return true;
  const packagePath = path.join(root, "package.json");
  if (!fs.existsSync(packagePath)) return false;
  try {
    const parsed = JSON.parse(fs.readFileSync(packagePath, "utf8"));
    return Boolean(
      parsed.devDependencies?.husky ||
      parsed.dependencies?.husky ||
      parsed.scripts?.prepare?.includes("husky"),
    );
  } catch {
    return false;
  }
}

function buildInitWrites(root, profile, adapters, force) {
  const writes = [
    initWrite(
      root,
      "core",
      "ocentra-enforcer.config.json",
      "generated",
      force,
      {
        content: `${JSON.stringify(
          {
            schemaVersion: 2,
            profileName: profile,
            languages: ["rust", "typescript", "python", "common"],
            failOn: ["error"],
            rules: {},
            tools: {},
            harness: {
              storageDir: ".enforce",
              store: "ndjson-duckdb",
              maxRuns: 50,
              maxRunsPerTool: 20,
              maxFailedRuns: 20,
              pruneAfterDays: 14,
            },
          },
          null,
          2,
        )}\n`,
      },
    ),
    initWrite(root, "core", ".gitignore", "append-line", force, {
      content: ".enforce/\n",
    }),
  ];

  if (adapters.includes("codex")) {
    writes.push(
      initWrite(
        root,
        "codex",
        ".codex/skills/ocentra-enforcer/SKILL.md",
        "template",
        force,
        {
          source: "skills/ocentra-enforcer/SKILL.md",
        },
      ),
    );
  }
  if (adapters.includes("mcp")) {
    writes.push(
      initWrite(root, "mcp", ".mcp.json", "generated", force, {
        content: mcpConfigTemplate(),
      }),
    );
  }
  if (adapters.includes("precommit")) {
    writes.push(
      initWrite(root, "precommit", ".git/hooks/pre-commit", "template", force, {
        source: "adapters/git-hooks/pre-commit.sh",
      }),
    );
  }
  if (adapters.includes("husky")) {
    writes.push(
      initWrite(root, "husky", ".husky/pre-commit", "template", force, {
        source: "adapters/husky/pre-commit",
      }),
    );
  }
  if (adapters.includes("lefthook")) {
    writes.push(
      initWrite(root, "lefthook", "lefthook.yml", "template", force, {
        source: "adapters/lefthook/lefthook.yml",
      }),
    );
  }

  const workflowMap = [
    [
      "github-actions",
      ".github/workflows/ocentra-enforcer.yml",
      "adapters/github-actions/ocentra-enforcer.yml",
    ],
    [
      "codeql",
      ".github/workflows/codeql.yml",
      "adapters/github-actions/codeql.yml",
    ],
    [
      "dependency-policy",
      ".github/workflows/dependency-policy.yml",
      "adapters/github-actions/dependency-policy.yml",
    ],
    [
      "secret-scan",
      ".github/workflows/secret-scan.yml",
      "adapters/github-actions/secret-scan.yml",
    ],
    ["sbom", ".github/workflows/sbom.yml", "adapters/github-actions/sbom.yml"],
  ];
  for (const [adapter, targetPath, source] of workflowMap) {
    if (adapters.includes(adapter))
      writes.push(
        initWrite(root, adapter, targetPath, "template", force, { source }),
      );
  }

  return writes;
}

function initWrite(root, adapter, targetPath, kind, force, details) {
  const absolutePath = path.join(root, targetPath);
  if (kind === "append-line") {
    const existing = fs.existsSync(absolutePath)
      ? fs.readFileSync(absolutePath, "utf8")
      : "";
    const line = details.content.trim();
    return {
      adapter,
      path: targetPath,
      action: existing.split(/\r?\n/u).includes(line)
        ? "skip-existing"
        : "append-line",
      kind,
      ...details,
    };
  }
  return {
    adapter,
    path: targetPath,
    action: fs.existsSync(absolutePath)
      ? force
        ? "overwrite"
        : "skip-existing"
      : "write",
    kind,
    ...details,
  };
}

function mcpConfigTemplate() {
  return `${JSON.stringify(
    {
      mcpServers: {
        "ocentra-enforcer": {
          command: "node",
          args: [toPosix(path.join(PACK_ROOT, "mcp", "ocentra-enforcer-mcp.mjs"))],
        },
      },
    },
    null,
    2,
  )}\n`;
}

function applyInitReport(report) {
  for (const file of report.files) {
    if (file.action === "skip-existing") continue;
    const targetPath = path.join(report.root, file.path);
    fs.mkdirSync(path.dirname(targetPath), { recursive: true });
    if (file.kind === "append-line") {
      const existing = fs.existsSync(targetPath)
        ? fs.readFileSync(targetPath, "utf8")
        : "";
      const prefix =
        existing.length > 0 && !existing.endsWith("\n") ? "\n" : "";
      fs.writeFileSync(
        targetPath,
        `${existing}${prefix}${file.content}`,
        "utf8",
      );
      continue;
    }
    const content =
      file.content ??
      fs.readFileSync(path.join(PACK_ROOT, file.source), "utf8");
    fs.writeFileSync(targetPath, content, "utf8");
    if (file.adapter === "precommit" || file.adapter === "husky") {
      try {
        fs.chmodSync(targetPath, 0o755);
      } catch {
        // Windows does not need executable bits for these generated scripts.
      }
    }
  }
}

function printInitReport(report) {
  const mode = report.dryRun ? "dry-run" : "write";
  console.log(`Ocentra Enforcer init ${mode} for ${report.root}`);
  console.log(`Profile: ${report.profile}`);
  console.log(`Adapters: ${report.adapters.join(", ")}`);
  for (const file of report.files) {
    console.log(`${file.action} ${file.path} (${file.adapter})`);
  }
}

function printCodexInstallReport(report) {
  console.log(
    report.root
      ? `Ocentra Enforcer Codex install for ${report.root}`
      : "Ocentra Enforcer Codex global install",
  );
  console.log(`Profile: ${report.profile}`);
  console.log(`Dry run: ${report.dryRun ? "yes" : "no"}`);
  console.log("");
  if (report.target) {
    console.log("Target repo wiring:");
    for (const file of report.target.files) {
      console.log(`${file.action} ${file.path} (${file.adapter})`);
    }
  } else {
    console.log("Target repo wiring: skipped (no --root passed)");
  }
  console.log("");
  console.log("Codex global MCP:");
  const action = report.codexMcp.changed
    ? report.dryRun
      ? "would-write"
      : "write"
    : "skip-existing";
  console.log(`${action} ${report.codexMcp.codexConfigPath}`);
  console.log(
    `server ${report.codexMcp.serverName}: node ${report.codexMcp.serverPath}`,
  );
  console.log(`ledger root: ${report.codexMcp.ledgerRoot}`);
  if (report.codexMcp.backupPath)
    console.log(`backup ${report.codexMcp.backupPath}`);
  for (const check of report.codexMcp.checks) {
    console.log(`${check.ok ? "PASS" : "FAIL"} ${check.name}: ${check.detail}`);
  }
  console.log("");
  console.log("Codex user skill:");
  console.log(
    `${report.codexMcp.skillChanged ? (report.dryRun ? "would-write" : "write") : "skip-existing"} ${report.codexMcp.skillTarget}`,
  );
  console.log("");
  console.log("Codex global AGENTS.md:");
  console.log(
    `${report.codexMcp.globalAgentsChanged ? (report.dryRun ? "would-write" : "write") : "skip-existing"} ${report.codexMcp.globalAgentsPath}`,
  );
  console.log("");
  console.log(
    "Restart Codex Desktop or start a new Codex thread after install so the MCP server list refreshes.",
  );
}

function printCodexUninstallReport(report) {
  console.log(`Ocentra Enforcer Codex uninstall`);
  console.log(`Dry run: ${report.dryRun ? "yes" : "no"}`);
  console.log("");
  console.log(
    `${report.changed ? (report.dryRun ? "would-write" : "write") : "skip-missing"} ${report.codexConfigPath}`,
  );
  console.log(
    `${report.skillChanged ? (report.dryRun ? "would-remove" : "remove") : "skip-missing"} ${report.skillTarget}`,
  );
  console.log(
    `${report.globalAgentsChanged ? (report.dryRun ? "would-write" : "write") : "skip-missing"} ${report.globalAgentsPath}`,
  );
  if (report.backupPath) console.log(`backup ${report.backupPath}`);
  if (report.globalAgentsBackupPath) console.log(`backup ${report.globalAgentsBackupPath}`);
}

function printCodexDoctorReport(report) {
  console.log(`Ocentra Enforcer Codex doctor: ${report.ok ? "PASS" : "FAIL"}`);
  console.log(`Pack: ${report.packRoot}`);
  if (report.root) console.log(`Target: ${report.root}`);
  console.log(`Codex config: ${report.codexConfigPath}`);
  for (const check of report.checks) {
    const label = check.ok
      ? "PASS"
      : check.severity === "warning"
        ? "WARN"
        : "FAIL";
    console.log(`${label} ${check.name}: ${check.detail}`);
  }
  console.log("");
  for (const step of report.nextSteps) console.log(`next: ${step}`);
}

function normalizeRel(root, filePath) {
  return path.relative(root, path.resolve(filePath)).split(path.sep).join("/");
}

function toPosix(value) {
  return value.split(path.sep).join("/");
}

function repoAbsolute(root, value) {
  return path.isAbsolute(value) ? value : path.join(root, value);
}

function globToRegExp(glob) {
  const special = /[.+^${}()|[\]\\]/g;
  let pattern = "";
  for (let i = 0; i < glob.length; i += 1) {
    const char = glob[i];
    if (char === "*") {
      if (glob[i + 1] === "*") {
        pattern += ".*";
        i += 1;
      } else {
        pattern += "[^/]*";
      }
    } else if (char === "?") {
      pattern += "[^/]";
    } else {
      pattern += char.replace(special, "\\$&");
    }
  }
  return new RegExp(`^${pattern}$`, "u");
}

const globCache = new Map();

function matchesGlob(relPath, glob) {
  if (!globCache.has(glob)) {
    globCache.set(glob, globToRegExp(glob));
  }
  return globCache.get(glob).test(relPath);
}

function matchesAnyGlob(relPath, globs) {
  return globs.some((glob) => matchesGlob(relPath, glob));
}

function isIgnoredPath(relPath, config) {
  return (
    relPath.split("/").some((segment) => config.ignoreDirs.includes(segment)) ||
    matchesAnyGlob(relPath, config.ignoreFileGlobs)
  );
}

function isRustFile(filePath) {
  return path.extname(filePath).toLowerCase() === ".rs";
}

function lineNumberAt(text, index) {
  let line = 1;
  for (let i = 0; i < index; i += 1) {
    if (text.charCodeAt(i) === 10) line += 1;
  }
  return line;
}

function runGit(root, args, label) {
  const result = spawnSync("git", args, {
    cwd: root,
    encoding: "utf8",
    shell: false,
  });
  if (result.error) throw result.error;
  if ((result.status ?? 1) !== 0) {
    throw new Error(
      `${label}: ${result.stderr?.trim() || "git command failed"}`,
    );
  }
  return result.stdout.trim();
}

function walkFiles(root, start, config, collect) {
  if (!fs.existsSync(start)) return;
  const stats = fs.statSync(start);
  const rel = normalizeRel(root, start);
  if (isIgnoredPath(rel, config)) return;
  if (stats.isDirectory()) {
    for (const entry of fs.readdirSync(start, { withFileTypes: true })) {
      walkFiles(root, path.join(start, entry.name), config, collect);
    }
    return;
  }
  if (stats.isFile()) collect(start);
}

function collectAllRustFiles(root, config) {
  const starts = config.rustRoots
    .map((entry) => repoAbsolute(root, entry))
    .filter((entry) => fs.existsSync(entry));
  if (starts.length === 0) starts.push(root);
  const files = [];
  for (const start of starts) {
    walkFiles(root, start, config, (file) => {
      if (isRustFile(file)) files.push(path.resolve(file));
    });
  }
  return uniqueSorted(files);
}

function collectExplicitRustFiles(root, config, entries) {
  const files = [];
  for (const entry of entries) {
    walkFiles(root, repoAbsolute(root, entry), config, (file) => {
      if (isRustFile(file)) files.push(path.resolve(file));
    });
  }
  return uniqueSorted(files);
}

function collectDiffRustFiles(root, config, base, head) {
  const output = runGit(
    root,
    [
      "diff",
      "--name-only",
      "--diff-filter=ACMR",
      base,
      head,
      "--",
      ...config.rustRoots,
    ],
    "failed to list diff files",
  );
  if (output === "") return [];
  return uniqueSorted(
    output
      .split(/\r?\n/u)
      .map((entry) => repoAbsolute(root, entry))
      .filter(
        (entry) =>
          fs.existsSync(entry) &&
          isRustFile(entry) &&
          !isIgnoredPath(normalizeRel(root, entry), config),
      ),
  );
}

function findCargoManifests(root, config) {
  const manifests = [];
  walkFiles(root, root, config, (file) => {
    if (path.basename(file) === "Cargo.toml")
      manifests.push(path.resolve(file));
  });
  return uniqueSorted(manifests);
}

function packageNameFromManifest(manifestPath) {
  const text = fs.readFileSync(manifestPath, "utf8");
  const packageBlock = text.match(
    /(?:^|\n)\s*\[package\]([\s\S]*?)(?:\n\s*\[|$)/u,
  );
  if (!packageBlock) return null;
  const nameMatch = packageBlock[1].match(/(?:^|\n)\s*name\s*=\s*"([^"]+)"/u);
  return nameMatch?.[1] ?? null;
}

function collectCrateRustFiles(root, config, crateName) {
  if (!crateName) throw new Error("--crate requires a package name.");
  const manifest = findCargoManifests(root, config).find(
    (candidate) => packageNameFromManifest(candidate) === crateName,
  );
  if (!manifest)
    throw new Error(
      `No Cargo package named "${crateName}" was found under ${root}.`,
    );
  const crateRoot = path.dirname(manifest);
  return {
    crateName,
    crateRoot,
    manifest,
    files: collectExplicitRustFiles(root, config, [crateRoot]),
  };
}

function resolveScope(root, config, scope) {
  if (scope.mode === "files") {
    return {
      ...scope,
      files: collectExplicitRustFiles(root, config, scope.files),
    };
  }
  if (scope.mode === "diff") {
    return {
      ...scope,
      files: collectDiffRustFiles(root, config, scope.base, scope.head),
    };
  }
  if (scope.mode === "crate") {
    return {
      ...scope,
      ...collectCrateRustFiles(root, config, scope.crateName),
    };
  }
  return { mode: "all", files: collectAllRustFiles(root, config) };
}

function uniqueSorted(files) {
  return [...new Set(files)].sort((a, b) => a.localeCompare(b));
}

function maskRustCode(source) {
  let out = "";
  let state = "code";
  let blockDepth = 0;
  let rawHashes = "";
  const pushMask = (ch) => {
    out += ch === "\n" ? "\n" : " ";
  };

  for (let i = 0; i < source.length; i += 1) {
    const ch = source[i];
    const next = source[i + 1];

    if (state === "code") {
      const rawMatch = source.slice(i).match(/^(?:b|c|br)?r(#+)?"/u);
      if (rawMatch) {
        state = "rawString";
        rawHashes = rawMatch[1] ?? "";
        for (let j = 0; j < rawMatch[0].length; j += 1) pushMask(source[i + j]);
        i += rawMatch[0].length - 1;
        continue;
      }
      if (ch === "/" && next === "/") {
        state = "lineComment";
        pushMask(ch);
        continue;
      }
      if (ch === "/" && next === "*") {
        state = "blockComment";
        blockDepth = 1;
        pushMask(ch);
        continue;
      }
      if (ch === '"' || (ch === "b" && next === '"')) {
        state = "string";
        pushMask(ch);
        continue;
      }
      if (ch === "'") {
        if (/^'[A-Za-z_][A-Za-z0-9_]*\b/u.test(source.slice(i))) {
          out += ch;
          continue;
        }
        state = "char";
        pushMask(ch);
        continue;
      }
      out += ch;
      continue;
    }

    if (state === "lineComment") {
      pushMask(ch);
      if (ch === "\n") state = "code";
      continue;
    }

    if (state === "blockComment") {
      pushMask(ch);
      if (ch === "/" && next === "*") {
        blockDepth += 1;
        pushMask(next);
        i += 1;
      } else if (ch === "*" && next === "/") {
        blockDepth -= 1;
        pushMask(next);
        i += 1;
        if (blockDepth === 0) state = "code";
      }
      continue;
    }

    if (state === "string" || state === "char") {
      pushMask(ch);
      if (ch === "\\") {
        if (i + 1 < source.length) {
          pushMask(source[i + 1]);
          i += 1;
        }
      } else if (
        (state === "string" && ch === '"') ||
        (state === "char" && ch === "'")
      ) {
        state = "code";
      }
      continue;
    }

    if (state === "rawString") {
      pushMask(ch);
      if (
        ch === '"' &&
        source.slice(i + 1, i + 1 + rawHashes.length) === rawHashes
      ) {
        for (let j = 0; j < rawHashes.length; j += 1)
          pushMask(source[i + 1 + j]);
        i += rawHashes.length;
        state = "code";
      }
    }
  }

  return out;
}

function contextHas(lines, index, token, distance = 4) {
  const start = Math.max(0, index - distance);
  return lines
    .slice(start, index + 1)
    .join("\n")
    .includes(token);
}

function firstLineMatching(lines, pattern) {
  const index = lines.findIndex((line) => pattern.test(line));
  return index < 0 ? 1 : index + 1;
}

function escapeRegExp(value) {
  return String(value).replace(/[.*+?^${}()|[\]\\]/gu, "\\$&");
}

function lineNumberAtIndex(source, index) {
  return source.slice(0, index).split(/\r?\n/u).length;
}

function addViolation(
  violations,
  root,
  filePath,
  line,
  ruleId,
  detail,
  sourceLine = null,
) {
  violations.push({
    ...enrichFindingMetadata({
      ruleId,
      detail,
      file: filePath === "." ? "." : normalizeRel(root, filePath),
      line,
      source: sourceLine?.trim() ?? null,
    }, PACK_ROOT, RULES),
  });
}

function applyPolicyAndWaivers(findings, config) {
  const enriched = enrichFindingsMetadata(findings, PACK_ROOT, {
    ...RULES,
    ...CHECK_RULES,
    ...GENERIC_RULES,
  });
  const policyFindings = applyRulePolicy(enriched, config, ruleRegistryRules());
  const { active, waived } = applyWaivers(
    policyFindings,
    config,
    ruleRegistryRules(),
    { ci: process.env.CI === "true" },
  );
  const { violations, warnings, bySeverity } = splitFindings(active, config);
  return {
    violations,
    warnings,
    waived,
    findings: [...active, ...waived],
    bySeverity,
  };
}

function policyPreflightFindings(root, config, options = {}) {
  if (options.skipPolicyPreflight === true) return [];
  return ["config-lockdown", "waiver-policy"].flatMap((checkName) => {
    const report = runStandaloneCheck({
      checkName,
      root,
      config,
      args: { scope: { mode: "all" } },
    });
    return [...(report.violations ?? []), ...(report.warnings ?? [])];
  });
}

const RAW_STRING_TYPE_RE =
  /\b(?:String|str|PathBuf|OsString|CString|CStr)\b|\b(?:std|alloc)::(?:string::String|path::PathBuf|ffi::(?:OsString|CString|CStr))\b|\bCow\s*<[^>]*\bstr\b[^>]*>/u;
const RAW_PRIMITIVE_TYPE_RE =
  /\b(?:bool|u8|u16|u32|u64|u128|usize|i8|i16|i32|i64|i128|isize|f32|f64)\b/u;
const RAW_POINTER_RE = /\*(?:const|mut)\s+[A-Za-z_]/u;
const TYPE_ALIAS_RAW_RE =
  /^\s*(?:pub(?:\([^)]*\))?\s+)?type\s+[A-Z][A-Za-z0-9_]*\s*=\s*([^;]+);/u;
const PUBLIC_SERDE_STRUCT_RE = /^\s*pub\s+struct\s+\w+/u;
const PUBLIC_FIELD_RE =
  /^\s*pub\s+(?<name>[A-Za-z_][A-Za-z0-9_]*)\s*:\s*(?<type>[^,]+),?/u;
const FIELD_RE =
  /^\s*(?:pub(?:\([^)]*\))?\s+)?(?<name>[A-Za-z_][A-Za-z0-9_]*)\s*:\s*(?<type>[^,]+),?/u;
const ID_LIKE_NAME_RE = /(?:^|_)(?:id|ids|key|ref|refs)$/iu;
const URL_LIKE_NAME_RE = /(?:^|_)(?:url|uri|endpoint)$/iu;
const PATH_LIKE_NAME_RE = /(?:^|_)(?:path|file|dir|directory)$/iu;
const TIME_LIKE_NAME_RE = /(?:^|_)(?:timeout|ttl|delay|interval|deadline|duration)$/iu;
const FALLIBLE_FN_NAME_RE = /^(?:save|load|parse|decode|find|get|lookup|create|open|connect|send|remove|delete|update|write)/u;

function collectFunctionSignatures(masked) {
  const signatures = [];
  const fnRe =
    /\b(?:pub(?:\([^)]*\))?\s+)?(?:const\s+)?(?:async\s+)?(?:unsafe\s+)?(?:extern\s+"[^"]+"\s+)?fn\s+[A-Za-z_][A-Za-z0-9_]*\b/gu;
  let match;
  while ((match = fnRe.exec(masked)) !== null) {
    let end = match.index;
    let parenDepth = 0;
    let angleDepth = 0;
    let seenParen = false;
    for (; end < masked.length; end += 1) {
      const ch = masked[end];
      if (ch === "(") {
        parenDepth += 1;
        seenParen = true;
      } else if (ch === ")") {
        parenDepth = Math.max(0, parenDepth - 1);
      } else if (ch === "<") {
        angleDepth += 1;
      } else if (ch === ">") {
        angleDepth = Math.max(0, angleDepth - 1);
      } else if (
        seenParen &&
        parenDepth === 0 &&
        angleDepth === 0 &&
        (ch === "{" || ch === ";")
      ) {
        end += 1;
        break;
      }
    }
    signatures.push({
      text: masked.slice(match.index, end),
      index: match.index,
      line: lineNumberAt(masked, match.index),
    });
    fnRe.lastIndex = Math.max(fnRe.lastIndex, end);
  }
  return signatures;
}

function functionName(signatureText) {
  return signatureText.match(/\bfn\s+([A-Za-z_][A-Za-z0-9_]*)\b/u)?.[1] ?? "";
}

function functionParams(signatureText) {
  const open = signatureText.indexOf("(");
  if (open < 0) return "";
  let depth = 0;
  for (let i = open; i < signatureText.length; i += 1) {
    const ch = signatureText[i];
    if (ch === "(") depth += 1;
    if (ch === ")") {
      depth -= 1;
      if (depth === 0) return signatureText.slice(open + 1, i);
    }
  }
  return "";
}

function normalizedNameTokens(name) {
  return name
    .replace(/([a-z0-9])([A-Z])/gu, "$1_$2")
    .toLowerCase()
    .split(/[^a-z0-9]+/u)
    .filter(Boolean);
}

function isSuspiciousSerializedFieldName(name) {
  const tokens = normalizedNameTokens(name);
  const lastToken = tokens.at(-1);
  const secondToLastToken = tokens.at(-2);
  return (
    lastToken === "id" ||
    lastToken === "ids" ||
    lastToken === "ref" ||
    lastToken === "refs" ||
    (secondToLastToken === "event" && lastToken === "type") ||
    (secondToLastToken === "command" && lastToken === "type")
  );
}

function braceDelta(line) {
  return (line.match(/\{/gu) ?? []).length - (line.match(/\}/gu) ?? []).length;
}

function isTestFile(rel, config) {
  return matchesAnyGlob(rel, config.testFileGlobs);
}

function isRawTypeBoundary(rel, config) {
  return matchesAnyGlob(rel, config.rawTypeBoundaryGlobs);
}

function isBoundaryModulePath(rel, config) {
  return (
    isRawTypeBoundary(rel, config) ||
    /(?:^|\/)(?:boundary|boundaries|serde|transport|adapter|adapters)(?:\/|\.|-)/iu.test(rel)
  );
}

function isRawStringOwner(rel, config) {
  return matchesAnyGlob(rel, config.rawStringOwnerGlobs);
}

function isDomainPrimitiveOwner(rel, config) {
  return matchesAnyGlob(rel, config.domainPrimitiveOwnerGlobs);
}

function isRuntimeStringOwner(rel, config) {
  return matchesAnyGlob(rel, config.runtimeStringOwnerGlobs);
}

function isSerializedDomainOwner(rel, config) {
  return matchesAnyGlob(rel, config.serializedDomainOwnerGlobs);
}

function hasStringLiteral(line) {
  return /"(?:[^"\\]|\\.)*"/u.test(line);
}

function scanRustFile(root, filePath, config) {
  const rel = normalizeRel(root, filePath);
  const violations = [];
  const source = fs.readFileSync(filePath, "utf8");
  const masked = maskRustCode(source);
  const originalLines = source.split(/\r?\n/u);
  const maskedLines = masked.split(/\r?\n/u);
  const isBoundary = isBoundaryModulePath(rel, config);
  const isStringOwner = isRawStringOwner(rel, config);
  const isPrimitiveOwner = isDomainPrimitiveOwner(rel, config);
  const enforceRuntimeStrings =
    config.enforceRuntimeStringLiterals &&
    !isTestFile(rel, config) &&
    !isRuntimeStringOwner(rel, config);
  const enforceSerializedDomainFields =
    config.enforceSerializedPublicDomainPrimitives &&
    !isTestFile(rel, config) &&
    !isSerializedDomainOwner(rel, config);
  const fileName = path.basename(filePath);
  const badModuleFileNames = new Set([
    "utils.rs",
    "helper.rs",
    "helpers.rs",
    "common.rs",
    "misc.rs",
    "stuff.rs",
    "shared.rs",
  ]);

  if (badModuleFileNames.has(fileName)) {
    addViolation(
      violations,
      root,
      filePath,
      1,
      "RR-7.4",
      `Forbidden dumping-ground file name: ${fileName}.`,
    );
  }

  let pendingSerializeDerive = false;
  let pendingSerdeShape = false;
  let trackedSerdeStructDepth = 0;

  maskedLines.forEach((line, idx) => {
    const lineNo = idx + 1;
    const originalLine = originalLines[idx] ?? line;

    if (
      /^\s*#!?\s*\[\s*(?:allow|expect)\s*\(/u.test(line) ||
      /^\s*#!?\s*\[\s*cfg_attr\s*\([^\]]*\b(?:allow|expect)\s*\(/u.test(line)
    ) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-2.1",
        "Lint suppression attribute found.",
        originalLine,
      );
      if (/\bunsafe_code\b/u.test(line)) {
        addViolation(
          violations,
          root,
          filePath,
          lineNo,
          "RR-3.33",
          "allow(unsafe_code) suppression found.",
          originalLine,
        );
      }
    }

    if (/\brustfmt::skip\b|\bclippy::(?:allow|expect)\b/u.test(originalLine)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-2.1",
        "Rust formatter or Clippy suppression found.",
        originalLine,
      );
    }

    if (/rust-rules\s*:\s*(?:ignore|allow|skip|disable)/iu.test(originalLine)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-2.2",
        "Validator suppression comment found.",
        originalLine,
      );
    }

    if (/\bunsafe\b/u.test(line)) {
      if (isTestFile(rel, config)) {
        addViolation(
          violations,
          root,
          filePath,
          lineNo,
          "RR-3.32",
          "unsafe code found in test source.",
          originalLine,
        );
      }
      if (!config.allowUnsafeCode) {
        addViolation(
          violations,
          root,
          filePath,
          lineNo,
          "RR-3.1",
          "unsafe keyword found while allowUnsafeCode=false.",
          originalLine,
        );
      } else if (
        /\bunsafe\s*\{/u.test(line) &&
        !contextHas(originalLines, idx, "SAFETY:", 4)
      ) {
        addViolation(
          violations,
          root,
          filePath,
          lineNo,
          "RR-3.2",
          "unsafe block lacks nearby // SAFETY: comment.",
          originalLine,
        );
      } else if (
        /\bunsafe\s+fn\b/u.test(line) &&
        !contextHas(originalLines, idx, "# Safety", 8)
      ) {
        addViolation(
          violations,
          root,
          filePath,
          lineNo,
          "RR-3.3",
          "unsafe fn lacks rustdoc # Safety section.",
          originalLine,
        );
      }
    }

    if (/\b(?:core|std)::mem::transmute\b|\btransmute\s*(?:::<[^>]+>)?\s*\(/u.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-3.16",
        "transmute found in Rust source.",
        originalLine,
      );
    }

    if (/\bMaybeUninit\b/u.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-3.17",
        "MaybeUninit found outside an approved unsafe owner.",
        originalLine,
      );
    }

    if (/\bManuallyDrop\b/u.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-3.18",
        "ManuallyDrop found without reviewed unsafe invariants.",
        originalLine,
      );
    }

    if (/\b(?:core|std)::mem::forget\b|\bmem::forget\s*\(/u.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-3.19",
        "mem::forget found.",
        originalLine,
      );
    }

    if (/\bBox::leak\s*\(/u.test(line) && !contextHas(originalLines, idx, "LEAK-JUSTIFICATION:", 4)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-3.20",
        "Box::leak lacks LEAK-JUSTIFICATION.",
        originalLine,
      );
    }

    if (/^\s*(?:pub\s+)?static\s+mut\b/u.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-3.21",
        "static mut found in Rust source.",
        originalLine,
      );
    }

    if (/\bUnsafeCell\b/u.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-3.22",
        "UnsafeCell found outside an approved primitive.",
        originalLine,
      );
    }

    if (/^\s*unsafe\s+impl\s+(?:Send|Sync)\b/u.test(line) && !contextHas(originalLines, idx, "SAFETY:", 6)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-3.23",
        "unsafe Send/Sync impl lacks nearby SAFETY proof.",
        originalLine,
      );
    }

    if (/\bget_unchecked(?:_mut)?\s*\(/u.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-3.24",
        "get_unchecked found.",
        originalLine,
      );
    }

    if (/\bunsafe\s*\{[^}]*\*[A-Za-z_][A-Za-z0-9_]*/u.test(line) && !contextHas(originalLines, idx, "SAFETY:", 4)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-3.25",
        "raw pointer dereference lacks nearby SAFETY proof.",
        originalLine,
      );
    }

    if (/^\s*(?:pub\s+)?extern\s+(?:"[^"]+"\s*)?\{/u.test(line) && !/(?:^|\/)(?:ffi|sys)(?:\/|$)/u.test(rel)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-3.26",
        "extern block found outside ffi/sys module.",
        originalLine,
      );
    }

    if (/(?:^|\/)(?:ffi|sys)(?:\/|$)/u.test(rel) && /^\s*pub\s+struct\s+[A-Z][A-Za-z0-9_]*\b/u.test(line) && !contextHas(originalLines, idx, "repr(C)", 2)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-3.27",
        "FFI-facing public struct lacks #[repr(C)].",
        originalLine,
      );
    }

    if (/^\s*#\s*\[\s*no_mangle\s*\]/u.test(line) && !/(?:^|\/)(?:ffi|sys)(?:\/|$)/u.test(rel)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-3.28",
        "#[no_mangle] found outside ffi/sys module.",
        originalLine,
      );
    }

    if ((/^\s*#\s*\[\s*no_mangle\s*\]/u.test(line) || /\bextern\s+"C"\s+fn\b/u.test(line)) && !contextHas(originalLines, idx, "catch_unwind", 12) && !contextHas(originalLines, idx, "PANIC-ABORT", 12)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-3.29",
        "FFI export lacks catch_unwind or PANIC-ABORT evidence.",
        originalLine,
      );
    }

    if (/\.unwrap\s*\(|\.expect\s*\(/u.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-4.1",
        ".unwrap() or .expect() found.",
        originalLine,
      );
    }

    if (!isBoundary && /\bErr\s*\(/u.test(line) && /\bErr\s*\(\s*(?:b?"|b?r#*")/u.test(originalLine)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-4.9",
        "Literal string error found.",
        originalLine,
      );
    }

    if (!isBoundary && /\bErr\s*\(\s*format\s*!/u.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-4.10",
        "Formatted string error found.",
        originalLine,
      );
    }

    if (!isBoundary && /\.map_err[^\n]*\.to_string\s*\(/u.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-4.11",
        "map_err erases the source error into String.",
        originalLine,
      );
    }

    if (
      !isBoundary &&
      /^\s*let\s+_\s*=\s*[^;]*(?:send|write|flush|read|parse|save|load|remove|create|open|connect|try_[A-Za-z0-9_]*)\s*(?:::<[^>]+>)?\s*\(/u.test(
        line,
      )
    ) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-4.16",
        "Fallible-looking result is ignored with let _ = ...;",
        originalLine,
      );
    }

    if (!isBoundary && /\.ok\s*\(\s*\)/u.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-4.17",
        "Result::ok() swallows the typed error.",
        originalLine,
      );
    }

    if (!isBoundary && /\.unwrap_or_default\s*\(\s*\)/u.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-4.18",
        "unwrap_or_default hides a fallible domain/config value.",
        originalLine,
      );
    }

    if (/\b(?:unwrap|expect|panic)\s*(?:!|\()/u.test(line) && contextHas(originalLines, idx, "fn new", 12)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-6.45",
        "newtype constructor panics or unwraps validation.",
        originalLine,
      );
    }

    if (
      !isBoundary &&
      /\.(?:unwrap_or|unwrap_or_else)\s*\(/u.test(line) &&
      /\b(?:parse|env|config|read|load|decode|deserialize)\b/u.test(line)
    ) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-4.19",
        "unwrap_or hides a parse/config/domain failure.",
        originalLine,
      );
    }

    if (/\b(?:panic|todo|unimplemented|unreachable)\s*!\s*\(/u.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-4.2",
        "panic-like macro found.",
        originalLine,
      );
    }

    if (isTestFile(rel, config) && /\.unwrap\s*\(|\.expect\s*\(/u.test(line) && !contextHas(originalLines, idx, "TEST-UNWRAP-JUSTIFICATION:", 4)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-12.21",
        "test unwrap/expect lacks TEST-UNWRAP-JUSTIFICATION.",
        originalLine,
      );
    }

    if (/\b(?:dbg|println|eprintln)\s*!\s*\(/u.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-4.3",
        "debug/console macro found.",
        originalLine,
      );
    }

    if (
      !isBoundary &&
      /\banyhow::Result\b|\bBox\s*<\s*dyn\s+(?:std::error::Error|Error)\b/u.test(
        line,
      )
    ) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-4.4",
        "Erased application error type found in non-boundary code.",
        originalLine,
      );
    }

    if (
      /\.clone\s*\(/u.test(line) &&
      !contextHas(originalLines, idx, "CLONE-JUSTIFICATION:", 4)
    ) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-5.1",
        ".clone() found without nearby CLONE-JUSTIFICATION.",
        originalLine,
      );
    }

    if (
      /\.(?:to_string|to_owned)\s*\(/u.test(line) &&
      !contextHas(originalLines, idx, "ALLOC-JUSTIFICATION:", 4)
    ) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-5.2",
        "String allocation found without nearby ALLOC-JUSTIFICATION.",
        originalLine,
      );
    }

    if (
      /\b[A-Za-z_][A-Za-z0-9_\.]*\s*\[[^\]\n]+\]/u.test(line) &&
      !/\b(?:vec|format|println|assert|assert_eq|assert_ne)!\s*\[/u.test(line)
    ) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-5.3",
        "Unchecked indexing/slicing found.",
        originalLine,
      );
    }

    if (
      /\s+as\s+(?:u8|u16|u32|u64|u128|usize|i8|i16|i32|i64|i128|isize|f32|f64)\b/u.test(
        line,
      ) &&
      !contextHas(originalLines, idx, "CAST-JUSTIFICATION:", 4)
    ) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-5.4",
        "Numeric cast found without nearby CAST-JUSTIFICATION.",
        originalLine,
      );
    }

    const typeAliasMatch = line.match(TYPE_ALIAS_RAW_RE);
    if (
      typeAliasMatch &&
      (RAW_STRING_TYPE_RE.test(typeAliasMatch[1]) ||
        RAW_PRIMITIVE_TYPE_RE.test(typeAliasMatch[1]))
    ) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-6.5",
        "Raw primitive/string type alias found.",
        originalLine,
      );
    }

    if (/^\s*(?:pub(?:\([^)]*\))?\s+)?type\s+[A-Z][A-Za-z0-9_]*\s*=\s*(?:Vec|HashMap|BTreeMap)\s*</u.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-6.47",
        "Domain collection type alias found.",
        originalLine,
      );
    }

    if (
      /^\s*pub\s+struct\s+[A-Z][A-Za-z0-9_]*\s*\(\s*pub\s+/u.test(line) &&
      (RAW_STRING_TYPE_RE.test(line) || RAW_PRIMITIVE_TYPE_RE.test(line))
    ) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-6.43",
        "Public tuple newtype exposes raw inner field.",
        originalLine,
      );
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-6.6",
        "Public tuple newtype exposes raw inner field.",
        originalLine,
      );
    }

    const tupleNewtypeMatch = line.match(/^\s*pub\s+struct\s+([A-Z][A-Za-z0-9_]*(?:Id|ID|Key|Ref))\s*\(\s*(?:pub\s+)?(?<type>u8|u16|u32|u64|u128|usize)\s*\)\s*;/u);
    if (tupleNewtypeMatch) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-6.46",
        "Numeric ID newtype uses a raw integer instead of NonZero or a validated representation.",
        originalLine,
      );
    }

    if (/^\s*#\[derive\([^#\]]*\bDebug\b[^#\]]*\)\]/u.test(line)) {
      const nextLine =
        maskedLines
          .slice(idx + 1, idx + 4)
          .find((candidate) => candidate.trim() !== "" && !/^\s*#\[/u.test(candidate)) ?? "";
      if (/(?:Secret|Token|Key|Credential|Password)/u.test(nextLine)) {
        addViolation(
          violations,
          root,
          filePath,
          lineNo,
          "RR-6.51",
          "Secret-like type derives Debug.",
          originalLine,
        );
      }
    }

    if (/(?:Secret|Token|Key|Credential|Password)/u.test(line) && /\b(?:struct|enum)\b/u.test(line) && !/\bRedacted\b|REDACTED|redact/u.test(source)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-6.52",
        "secret-like type lacks redacted formatting evidence.",
        originalLine,
      );
    }

    if (
      /^\s*(?:pub(?:\([^)]*\))?\s+)?[A-Za-z_][A-Za-z0-9_]*\s*:\s*/u.test(
        line,
      ) &&
      (RAW_STRING_TYPE_RE.test(line) || RAW_PRIMITIVE_TYPE_RE.test(line))
    ) {
      if (/^\s*pub(?:\([^)]*\))?\s+/u.test(line)) {
        addViolation(
          violations,
          root,
          filePath,
          lineNo,
          "RR-6.3",
          "Public raw field found.",
          originalLine,
        );
      } else if (
        !isBoundary &&
        !contextHas(originalLines, idx, "BRAND-INVARIANT:", 6)
      ) {
        addViolation(
          violations,
          root,
          filePath,
          lineNo,
          "RR-6.4",
          "Private raw field lacks nearby BRAND-INVARIANT documentation.",
          originalLine,
        );
      }
    }

    const fieldMatch = line.match(FIELD_RE);
    if (!isBoundary && fieldMatch?.groups) {
      const fieldName = fieldMatch.groups.name;
      const fieldType = fieldMatch.groups.type;
      if (/\bBTreeMap\s*<\s*String\s*,/u.test(fieldType)) {
        addViolation(
          violations,
          root,
          filePath,
          lineNo,
          "RR-6.33",
          "BTreeMap<String, _> found in domain field.",
          originalLine,
        );
      }
      if (/\bserde_json::Value\b|\bValue\b/u.test(fieldType) && /serde_json|json|value/u.test(line)) {
        addViolation(
          violations,
          root,
          filePath,
          lineNo,
          "RR-6.34",
          "serde_json::Value found in domain field.",
          originalLine,
        );
      }
      if (/\bOption\s*<\s*String\s*>/u.test(fieldType)) {
        addViolation(
          violations,
          root,
          filePath,
          lineNo,
          "RR-6.35",
          "Option<String> found in domain field.",
          originalLine,
        );
      }
      if (/\bOption\s*<\s*bool\s*>/u.test(fieldType)) {
        addViolation(
          violations,
          root,
          filePath,
          lineNo,
          "RR-6.36",
          "Option<bool> found in domain field.",
          originalLine,
        );
      }
      if (TIME_LIKE_NAME_RE.test(fieldName) && /\bDuration\b/u.test(fieldType)) {
        addViolation(
          violations,
          root,
          filePath,
          lineNo,
          "RR-6.38",
          "Raw Duration found in named domain timing field.",
          originalLine,
        );
      }
      if (/\b(?:SystemTime|Instant)\b/u.test(fieldType)) {
        addViolation(
          violations,
          root,
          filePath,
          lineNo,
          "RR-6.39",
          "Raw time type found in domain field.",
          originalLine,
        );
      }
      if (URL_LIKE_NAME_RE.test(fieldName) && RAW_STRING_TYPE_RE.test(fieldType)) {
        addViolation(
          violations,
          root,
          filePath,
          lineNo,
          "RR-6.40",
          "URL-like field uses a raw string type.",
          originalLine,
        );
      }
      if (PATH_LIKE_NAME_RE.test(fieldName) && RAW_STRING_TYPE_RE.test(fieldType)) {
        addViolation(
          violations,
          root,
          filePath,
          lineNo,
          "RR-6.41",
          "Path-like field uses a raw string type.",
          originalLine,
        );
      }
      if (ID_LIKE_NAME_RE.test(fieldName) && (RAW_STRING_TYPE_RE.test(fieldType) || RAW_PRIMITIVE_TYPE_RE.test(fieldType))) {
        addViolation(
          violations,
          root,
          filePath,
          lineNo,
          "RR-6.42",
          "ID-like field uses a raw string or primitive type.",
          originalLine,
        );
      }
    }

    if (
      /^\s*pub\s+struct\s+[A-Z][A-Za-z0-9_]*\s*\([^)]*(?:String|str|bool|u8|u16|u32|u64|u128|usize|i8|i16|i32|i64|i128|isize|f32|f64)/u.test(
        line,
      ) &&
      !contextHas(originalLines, idx, "BRAND-INVARIANT:", 6)
    ) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-6.4",
        "Tuple newtype over raw field lacks BRAND-INVARIANT documentation.",
        originalLine,
      );
    }

    if (/^\s*use\s+[^;]*::\*\s*;/u.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-7.1",
        "Wildcard import found.",
        originalLine,
      );
    }

    if (/^\s*pub\s+use\s+[^;]*::\*\s*;/u.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-7.2",
        "Wildcard public re-export found.",
        originalLine,
      );
    }

    if (/^\s*pub\s+use\s+/u.test(line)) {
      const isFacade = matchesAnyGlob(rel, config.facadeFileGlobs);
      if (config.publicReexportPolicy === "forbid") {
        addViolation(
          violations,
          root,
          filePath,
          lineNo,
          "RR-7.3",
          "pub use is forbidden by this profile.",
          originalLine,
        );
      } else if (config.publicReexportPolicy === "facade-only" && !isFacade) {
        addViolation(
          violations,
          root,
          filePath,
          lineNo,
          "RR-7.3",
          "pub use outside configured facade file.",
          originalLine,
        );
      }
    }

    if (
      /^\s*(?:pub\s+)?mod\s+(?:utils|helper|helpers|common|misc|stuff|shared)\s*;/u.test(
        line,
      )
    ) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-7.4",
        "Forbidden dumping-ground module declaration.",
        originalLine,
      );
    }

    if (
      /\basync\s+fn\b|\.await\b/u.test(masked) &&
      /\bstd::sync::(?:Mutex|RwLock)\b|\bstd::thread::sleep\b|\bstd::fs::|\bstd::net::/u.test(
        line,
      )
    ) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-8.1",
        "Blocking primitive in async module.",
        originalLine,
      );
      if (/\bstd::sync::(?:Mutex|RwLock)\b/u.test(line)) {
        addViolation(
          violations,
          root,
          filePath,
          lineNo,
          "RR-8.16",
          "std sync lock found in async module.",
          originalLine,
        );
      }
      if (/\bstd::fs::|\bstd::net::/u.test(line)) {
        addViolation(
          violations,
          root,
          filePath,
          lineNo,
          "RR-8.25",
          "Blocking std I/O found in async module.",
          originalLine,
        );
      }
    }

    if (
      /\btokio::spawn\s*\(/u.test(line) &&
      !/[A-Za-z_][A-Za-z0-9_]*\s*=\s*tokio::spawn\s*\(/u.test(line) &&
      !/\b(?:let|return)\b/u.test(line)
    ) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-8.18",
        "tokio::spawn handle is not tracked.",
        originalLine,
      );
      if (!contextHas(originalLines, idx, "TASK-JUSTIFICATION:", 4)) {
        addViolation(
          violations,
          root,
          filePath,
          lineNo,
          "RR-8.19",
          "fire-and-forget tokio::spawn lacks TASK-JUSTIFICATION.",
          originalLine,
        );
      }
    }

    if (/\.await\b/u.test(line) && /\b(?:MutexGuard|RwLock.*Guard|\.lock\s*\(\s*\)|\.write\s*\(\s*\)|\.read\s*\(\s*\))/u.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-8.17",
        "await appears while a lock guard is held.",
        originalLine,
      );
    }

    if (/\b(?:retry|retries|Retry)\b/u.test(line) && /\b(?:loop|while|for)\b/u.test(line) && !/\bRetryPolicy\b|BACKOFF-JUSTIFICATION|RETRY-JUSTIFICATION/u.test(source)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-8.22",
        "retry loop lacks bounded retry policy.",
        originalLine,
      );
    }

    if (/\bselect!\s*\{/u.test(line) && !contextHas(originalLines, idx, "CANCEL-SAFE:", 8)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-8.24",
        "select! lacks nearby CANCEL-SAFE branch documentation.",
        originalLine,
      );
    }

    if ((/\basync\s+fn\b|\.await\b/u.test(masked)) && /\b(?:for|while)\b/u.test(line) && /\b(?:hash|compress|encode|decode|sort|parse|render|compute)\b/iu.test(line) && !/\bspawn_blocking\b|CPU-JUSTIFICATION|worker/u.test(source)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-8.26",
        "CPU-looking work in async path lacks spawn_blocking/worker boundary.",
        originalLine,
      );
    }

    if (
      /\b(?:tokio::sync::mpsc::|mpsc::)?unbounded_channel\s*(?:::<[^>]+>)?\s*\(/u.test(line) &&
      !contextHas(originalLines, idx, "CHANNEL-JUSTIFICATION:", 4)
    ) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-8.20",
        "Unbounded channel lacks CHANNEL-JUSTIFICATION.",
        originalLine,
      );
    }

    if (
      /\.(?:send|get|post|put|patch|delete|request)\s*\([^;\n]*\)\s*\.await\b/u.test(line) &&
      !/(?:timeout|Timeout|deadline)/u.test(line)
    ) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-8.21",
        "external async I/O await lacks timeout policy.",
        originalLine,
      );
    }

    if (/^\s*loop\s*\{/u.test(line) && /\basync\s+fn\b|\.await\b/u.test(masked) && !contextHas(originalLines, idx, "CANCEL", 8)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-8.23",
        "async loop lacks nearby cancellation marker.",
        originalLine,
      );
    }

    if (/\b(?:tokio::runtime::Runtime::new|tokio::runtime::Builder::new)/u.test(line) && !/(?:^|\/)(?:main|bin)(?:\.rs|\/)/u.test(rel)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-8.27",
        "library/domain source creates a Tokio runtime.",
        originalLine,
      );
    }

    if (/\bblock_on\s*\(/u.test(line) && !/(?:^|\/)(?:main|bin)(?:\.rs|\/)/u.test(rel)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-8.28",
        "block_on found in library/domain source.",
        originalLine,
      );
    }

    if (isTestFile(rel, config) && /\b(?:std::thread::sleep|tokio::time::sleep)\s*\(/u.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-8.29",
        "sleep found in test source.",
        originalLine,
      );
    }

    if (/\bfor\s+[A-Za-z_][A-Za-z0-9_]*\s+in\s+0\s*\.\./u.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-8.2",
        "C-style numeric loop found.",
        originalLine,
      );
    }

    if (
      enforceRuntimeStrings &&
      hasStringLiteral(originalLine) &&
      !config.runtimeStringLineAllowRegexps.some((pattern) =>
        pattern.test(originalLine),
      )
    ) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-18.16",
        "Runtime Rust source contains an inline string literal.",
        originalLine,
      );
    }

    if (
      !isBoundary &&
      /^\s*#\[derive\([^#\]]*\bDeserialize\b[^#\]]*\)\]/u.test(line)
    ) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-14.16",
        "Deserialize derive found in non-boundary Rust domain source.",
        originalLine,
      );
    }

    if (!isBoundary && /^\s*#\[derive\([^#\]]*\bSerialize\b[^#\]]*\)\]/u.test(line) && !contextHas(originalLines, idx, "SERIALIZATION-DOC:", 8)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-14.17",
        "Serialize derive lacks SERIALIZATION-DOC evidence.",
        originalLine,
      );
    }

    if (/^\s*#\[serde\([^#\]]*\bdefault\b[^#\]]*\)\]/u.test(line) && !contextHas(originalLines, idx, "DEFAULT-JUSTIFICATION:", 4)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-14.19",
        "serde(default) lacks DEFAULT-JUSTIFICATION.",
        originalLine,
      );
    }

    if (/^\s*#\[serde\([^#\]]*\bflatten\b[^#\]]*\)\]/u.test(line) && !contextHas(originalLines, idx, "FLATTEN-JUSTIFICATION:", 4)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-14.27",
        "serde(flatten) lacks FLATTEN-JUSTIFICATION.",
        originalLine,
      );
    }

    if (!isBoundary && /\bserde_json::from_str\s*(?:::<[^>]+>)?\s*\(/u.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-14.29",
        "serde_json::from_str found outside boundary source.",
        originalLine,
      );
    }

    if (!isBoundary && /\bserde_json::from_str\s*::?\s*<\s*[A-Z][A-Za-z0-9_]*(?:Domain|Id|State|Config|Policy|Record)?\s*>/u.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-14.30",
        "JSON deserializes directly into a domain-like type.",
        originalLine,
      );
    }

    if (/\buse\s+[^;]*(?:dto|request|response|envelope|transport|serde)[^;]*;/iu.test(line) && /(?:^|\/)(?:domain|core|model|models)(?:\/|$)/u.test(rel)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-14.22",
        "domain module imports DTO/transport/serde module.",
        originalLine,
      );
    }

    if (
      !isBoundary &&
      /^\s*#\[serde\([^#\]]*\buntagged\b[^#\]]*\)\]/u.test(line) &&
      !contextHas(originalLines, idx, "SERDE-UNTAGGED-JUSTIFICATION:", 4)
    ) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-14.18",
        "serde untagged enum lacks SERDE-UNTAGGED-JUSTIFICATION.",
        originalLine,
      );
    }

    if (enforceSerializedDomainFields) {
      if (trackedSerdeStructDepth === 0) {
        if (
          /^\s*#\[derive\([^#\]]*\b(?:Serialize|Deserialize)\b[^#\]]*\)\]/u.test(
            line,
          )
        ) {
        pendingSerializeDerive = true;
          if (!isBoundary && /\bDeserialize\b/u.test(line)) {
            addViolation(
              violations,
              root,
              filePath,
              lineNo,
              "RR-14.16",
              "Deserialize derive found in non-boundary Rust domain source.",
              originalLine,
            );
          }
          return;
        }
        if (/^\s*#\[serde\(/u.test(line)) {
          if (
            !isBoundary &&
            /\buntagged\b/u.test(line) &&
            !contextHas(originalLines, idx, "SERDE-UNTAGGED-JUSTIFICATION:", 4)
          ) {
            addViolation(
              violations,
              root,
              filePath,
              lineNo,
              "RR-14.18",
              "serde untagged enum lacks SERDE-UNTAGGED-JUSTIFICATION.",
              originalLine,
            );
          }
          pendingSerdeShape = true;
          return;
        }
        if (pendingSerializeDerive || pendingSerdeShape) {
          if (/^\s*#\[/u.test(line)) return;
          const shouldTrack =
            pendingSerializeDerive &&
            pendingSerdeShape &&
            PUBLIC_SERDE_STRUCT_RE.test(line);
          pendingSerializeDerive = false;
          pendingSerdeShape = false;
          if (shouldTrack) trackedSerdeStructDepth = braceDelta(line);
          return;
        }
      } else {
        const match = PUBLIC_FIELD_RE.exec(line);
        if (
          match &&
          isSuspiciousSerializedFieldName(match.groups.name) &&
          (RAW_STRING_TYPE_RE.test(match.groups.type) ||
            RAW_PRIMITIVE_TYPE_RE.test(match.groups.type))
        ) {
          addViolation(
            violations,
            root,
            filePath,
            lineNo,
            "RR-6.26",
            "Serialized public struct field uses raw id/ref/event/command primitive outside configured owner crates.",
            originalLine,
          );
        }
        trackedSerdeStructDepth += braceDelta(line);
      }
    }

    if (/\bassert!\s*\(.*\.is_ok\s*\(\s*\).*?\)/u.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-12.22",
        "Weak is_ok assertion found.",
        originalLine,
      );
    }

    if (/\bassert!\s*\(.*\.is_some\s*\(\s*\).*?\)/u.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-12.23",
        "Weak is_some assertion found.",
        originalLine,
      );
    }
  });

  if (!isBoundary) {
    for (const match of source.matchAll(/\bstruct\s+([A-Z][A-Za-z0-9_]*)[^{;]*\{([\s\S]*?)^\s*\}/gmu)) {
      const boolFields = [...match[2].matchAll(/^\s*(?:pub(?:\([^)]*\))?\s+)?[A-Za-z_][A-Za-z0-9_]*\s*:\s*bool\s*,?/gmu)];
      if (boolFields.length >= 2) {
        addViolation(
          violations,
          root,
          filePath,
          lineNumberAtIndex(source, match.index),
          "RR-6.37",
          `Struct ${match[1]} has ${boolFields.length} boolean state fields.`,
          originalLines[lineNumberAtIndex(source, match.index) - 1] ?? null,
        );
      }
    }

    for (const match of source.matchAll(/(?<attrs>(?:^\s*#\[[^\]]+\]\s*\r?\n)*)^\s*(?:pub\s+)?enum\s+(?<name>[A-Z][A-Za-z0-9_]*Error)\b[^{]*\{(?<body>[\s\S]*?)^\s*\}/gmu)) {
      const lineNo = lineNumberAtIndex(source, match.index);
      const attrs = match.groups?.attrs ?? "";
      const body = match.groups?.body ?? "";
      if (!/\bDebug\b/u.test(attrs) || !/\b(?:thiserror::)?Error\b/u.test(attrs)) {
        addViolation(
          violations,
          root,
          filePath,
          lineNo,
          "RR-4.20",
          `Error enum ${match.groups?.name ?? "Error"} does not derive Debug and Error.`,
          originalLines[lineNo - 1] ?? null,
        );
      }
      for (const variant of body.matchAll(/^\s*(?<attrs>(?:#\[[^\]]+\]\s*)*)[A-Z][A-Za-z0-9_]*\s*\(\s*(?<type>(?:std::)?[A-Za-z_:]+Error)\s*\)/gmu)) {
        if (!/\b(?:source|from)\b/u.test(variant.groups?.attrs ?? "")) {
          addViolation(
            violations,
            root,
            filePath,
            lineNumberAtIndex(source, (match.index ?? 0) + variant.index),
            "RR-4.21",
            "Wrapped source error lacks #[source] or #[from].",
            variant[0],
          );
        }
      }
    }

    for (const match of masked.matchAll(/\bfn\s+(?<name>find|get|lookup|parse)[A-Za-z0-9_]*\b[\s\S]*?\{(?<body>[\s\S]*?)^\s*\}/gmu)) {
      const body = match.groups?.body ?? "";
      if (/\breturn\s+(?:-1|""|None)\s*;|=>\s*(?:-1|"")\b/u.test(body)) {
        const lineNo = lineNumberAt(masked, match.index);
        addViolation(
          violations,
          root,
          filePath,
          lineNo,
          "RR-4.13",
          "Lookup/parse function returns a sentinel failure value.",
          originalLines[lineNo - 1] ?? null,
        );
      }
    }

    for (const match of masked.matchAll(/\bfn\s+[A-Za-z_][A-Za-z0-9_]*\b[\s\S]*?\{(?<body>[\s\S]*?)^\s*\}/gmu)) {
      const body = match.groups?.body ?? "";
      if (/\b(?:error|warn)!\s*\(/u.test(body) && /\bErr\s*\(/u.test(body)) {
        const lineNo = lineNumberAt(masked, match.index);
        addViolation(
          violations,
          root,
          filePath,
          lineNo,
          "RR-4.22",
          "Function logs and returns an error.",
          originalLines[lineNo - 1] ?? null,
        );
      }
    }

    const mainMatch = masked.match(/\bfn\s+main\s*\([^)]*\)\s*(?!->)[\s\S]*?\{(?<body>[\s\S]*?)^\s*\}/mu);
    if (mainMatch && /\blet\s+_\s*=\s*[A-Za-z_][A-Za-z0-9_]*(?:::[^(\s]+)?\s*\(/u.test(mainMatch.groups?.body ?? "")) {
      const lineNo = lineNumberAt(masked, mainMatch.index ?? 0);
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-4.15",
        "main swallows a fallible-looking call with let _ =.",
        originalLines[lineNo - 1] ?? null,
      );
    }
  }

  for (const sig of collectFunctionSignatures(masked)) {
    if (isBoundary) continue;
    const originalSigFirstLine = originalLines[sig.line - 1] ?? sig.text;
    const sigName = functionName(sig.text);
    const params = functionParams(sig.text);
    if (RAW_POINTER_RE.test(sig.text)) {
      addViolation(
        violations,
        root,
        filePath,
        sig.line,
        "RR-3.4",
        "Raw pointer found in function signature.",
        originalSigFirstLine,
      );
    }
    if (FALLIBLE_FN_NAME_RE.test(sigName) && /->\s*bool\b/u.test(sig.text)) {
      addViolation(
        violations,
        root,
        filePath,
        sig.line,
        "RR-4.12",
        "Fallible-looking API returns bool instead of Result or a status enum.",
        originalSigFirstLine,
      );
    }
    if (
      /\bfn\s+new\s*\(/u.test(sig.text) &&
      /->\s*Self\b/u.test(sig.text) &&
      (RAW_STRING_TYPE_RE.test(params) || RAW_PRIMITIVE_TYPE_RE.test(params)) &&
      !/Result\s*<\s*Self\s*,/u.test(sig.text)
    ) {
      addViolation(
        violations,
        root,
        filePath,
        sig.line,
        "RR-4.14",
        "new(...) accepts raw input but does not return Result<Self, Error>.",
        originalSigFirstLine,
      );
    }
    if (/\bResult\s*<[^>]*,\s*String\s*>/u.test(sig.text)) {
      addViolation(
        violations,
        root,
        filePath,
        sig.line,
        "RR-4.7",
        "Result uses String as the error type.",
        originalSigFirstLine,
      );
    }
    if (/\bResult\s*<[^>]*,\s*&\s*'static\s+str\s*>/u.test(sig.text)) {
      addViolation(
        violations,
        root,
        filePath,
        sig.line,
        "RR-4.8",
        "Result uses &'static str as the error type.",
        originalSigFirstLine,
      );
    }
    if (/\bAsRef\s*<\s*str\s*>/u.test(sig.text)) {
      addViolation(
        violations,
        root,
        filePath,
        sig.line,
        "RR-6.27",
        "AsRef<str> found in domain function signature.",
        originalSigFirstLine,
      );
    }
    if (/\bInto\s*<\s*String\s*>/u.test(sig.text)) {
      addViolation(
        violations,
        root,
        filePath,
        sig.line,
        "RR-6.28",
        "Into<String> found in domain function signature.",
        originalSigFirstLine,
      );
    }
    if (/\bimpl\s+Display\b/u.test(sig.text) && /\b(?:id|key|ref|name)\s*:/iu.test(sig.text)) {
      addViolation(
        violations,
        root,
        filePath,
        sig.line,
        "RR-6.29",
        "ID-like parameter accepts impl Display.",
        originalSigFirstLine,
      );
    }
    if (/\bCow\s*<[^>]*\bstr\b[^>]*>/u.test(sig.text)) {
      addViolation(
        violations,
        root,
        filePath,
        sig.line,
        "RR-6.30",
        "Cow<str> found in domain function signature.",
        originalSigFirstLine,
      );
    }
    if (/\bVec\s*<\s*String\s*>/u.test(sig.text)) {
      addViolation(
        violations,
        root,
        filePath,
        sig.line,
        "RR-6.31",
        "Vec<String> found in domain function signature.",
        originalSigFirstLine,
      );
    }
    if (/\bHashMap\s*<\s*String\s*,/u.test(sig.text)) {
      addViolation(
        violations,
        root,
        filePath,
        sig.line,
        "RR-6.32",
        "HashMap<String, _> found in domain function signature.",
        originalSigFirstLine,
      );
    }
    if (/\bBTreeMap\s*<\s*String\s*,/u.test(sig.text)) {
      addViolation(
        violations,
        root,
        filePath,
        sig.line,
        "RR-6.33",
        "BTreeMap<String, _> found in domain function signature.",
        originalSigFirstLine,
      );
    }
    if (/\bserde_json::Value\b/u.test(sig.text)) {
      addViolation(
        violations,
        root,
        filePath,
        sig.line,
        "RR-6.34",
        "serde_json::Value found in domain function signature.",
        originalSigFirstLine,
      );
    }
    if (/\b(?:timeout|ttl|delay|interval|deadline|duration)\s*:\s*(?:std::time::)?Duration\b/iu.test(sig.text)) {
      addViolation(
        violations,
        root,
        filePath,
        sig.line,
        "RR-6.38",
        "Raw Duration found in named domain timing parameter.",
        originalSigFirstLine,
      );
    }
    if (/\b(?:SystemTime|Instant)\b/u.test(sig.text)) {
      addViolation(
        violations,
        root,
        filePath,
        sig.line,
        "RR-6.39",
        "Raw time type found in public domain signature.",
        originalSigFirstLine,
      );
    }
    if (/\b(?:url|uri|endpoint)\s*:\s*(?:String|&\s*str|str\b)/iu.test(sig.text)) {
      addViolation(
        violations,
        root,
        filePath,
        sig.line,
        "RR-6.40",
        "URL-like parameter uses raw string type.",
        originalSigFirstLine,
      );
    }
    if (/\b(?:path|file|dir|directory)\s*:\s*(?:String|&\s*str|str\b|PathBuf)/iu.test(sig.text)) {
      addViolation(
        violations,
        root,
        filePath,
        sig.line,
        "RR-6.41",
        "Path-like parameter uses raw string/path type.",
        originalSigFirstLine,
      );
    }
    if (/->\s*\([^)]*,[^)]*\)/u.test(sig.text) || /\([^)]*:\s*\([^)]*,[^)]*\)/u.test(sig.text)) {
      addViolation(
        violations,
        root,
        filePath,
        sig.line,
        "RR-6.48",
        "Naked tuple found in public/domain function signature.",
        originalSigFirstLine,
      );
    }
    if (
      /\bfn\s+new\s*\(/u.test(sig.text) &&
      (params.match(/\b(?:String|str|bool|u8|u16|u32|u64|usize|i8|i16|i32|i64|isize)\b/gu) ?? []).length >= 2
    ) {
      addViolation(
        violations,
        root,
        filePath,
        sig.line,
        "RR-6.49",
        "Constructor accepts multiple primitive/raw parameters.",
        originalSigFirstLine,
      );
    }
    if (/\bArc\s*<\s*(?:std::sync::)?Mutex\s*</u.test(sig.text)) {
      addViolation(
        violations,
        root,
        filePath,
        sig.line,
        "RR-8.30",
        "Raw Arc<Mutex<T>> appears in a function signature.",
        originalSigFirstLine,
      );
    }
    if (!isStringOwner && RAW_STRING_TYPE_RE.test(sig.text)) {
      addViolation(
        violations,
        root,
        filePath,
        sig.line,
        "RR-6.1",
        "Raw string/path type found in function signature.",
        originalSigFirstLine,
      );
    }
    if (!isPrimitiveOwner && RAW_PRIMITIVE_TYPE_RE.test(sig.text)) {
      addViolation(
        violations,
        root,
        filePath,
        sig.line,
        "RR-6.2",
        "Unbranded primitive type found in function signature.",
        originalSigFirstLine,
      );
    }
  }

  const unsafeLine = originalLines.findIndex((line) => /\bunsafe\b/u.test(line));
  if (unsafeLine >= 0 && !/\bMIRI-PROOF:/u.test(source)) {
    addViolation(
      violations,
      root,
      filePath,
      unsafeLine + 1,
      "RR-3.30",
      "unsafe source lacks MIRI-PROOF evidence.",
      originalLines[unsafeLine],
    );
    addViolation(
      violations,
      root,
      filePath,
      unsafeLine + 1,
      "RR-12.30",
      "unsafe module lacks MIRI-PROOF evidence.",
      originalLines[unsafeLine],
    );
  }
  if (unsafeLine >= 0 && !/\bGEIGER-PROOF:/u.test(source)) {
    addViolation(
      violations,
      root,
      filePath,
      unsafeLine + 1,
      "RR-3.31",
      "unsafe source lacks GEIGER-PROOF evidence.",
      originalLines[unsafeLine],
    );
  }

  for (const match of source.matchAll(/pub\s+struct\s+(?<name>[A-Z][A-Za-z0-9_]*)\s*\(\s*(?:pub\s+)?(?<inner>String|&\s*str|str|u8|u16|u32|u64|usize|i8|i16|i32|i64|isize|bool)[^)]*\)\s*;/gu)) {
    const typeName = match.groups?.name ?? "";
    if (!new RegExp(`impl\\s+${escapeRegExp(typeName)}[\\s\\S]*?\\b(?:try_new|parse)\\s*\\(`, "u").test(source)) {
      const lineNo = lineNumberAtIndex(source, match.index ?? 0);
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-6.44",
        `newtype ${typeName} lacks try_new or parse constructor.`,
        originalLines[lineNo - 1] ?? null,
      );
    }
  }

  if (!isBoundary) {
    for (const match of source.matchAll(/(?<attrs>(?:^\s*#\[[^\]]+\]\s*\r?\n)*)^\s*pub\s+(?:struct|enum)\s+(?<name>[A-Z][A-Za-z0-9_]*)(?:\b|[<{(])/gmu)) {
      const name = match.groups?.name ?? "";
      if (/(?:Secret|Token|Key|Credential|Password)/u.test(name)) continue;
      const attrs = match.groups?.attrs ?? "";
      if (!/\bDebug\b/u.test(attrs) && !new RegExp(`impl\\s+(?:std::fmt::|fmt::)?Debug\\s+for\\s+${escapeRegExp(name)}\\b`, "u").test(source)) {
        const lineNo = lineNumberAtIndex(source, match.index ?? 0);
        addViolation(
          violations,
          root,
          filePath,
          lineNo,
          "RR-6.50",
          `public domain value object ${name} lacks intentional Debug implementation.`,
          originalLines[lineNo - 1] ?? null,
        );
      }
    }
  }

  if (!isBoundary && /\b(?:try_new|parse)\s*\(/u.test(masked) && !/\b(?:invalid|reject|malformed|bad input)\b/iu.test(source)) {
    const lineNo = firstLineMatching(originalLines, /\b(?:try_new|parse)\s*\(/u);
    addViolation(violations, root, filePath, lineNo, "RR-12.16", "validated constructor/parser lacks invalid-input test evidence.", originalLines[lineNo - 1] ?? null);
  }
  if (!isBoundary && /\bparse[A-Za-z0-9_]*\s*\(/u.test(masked) && !/\b(?:invalid|empty|oversized|malformed)\b/iu.test(source)) {
    const lineNo = firstLineMatching(originalLines, /\bparse[A-Za-z0-9_]*\s*\(/u);
    addViolation(violations, root, filePath, lineNo, "RR-12.17", "parser lacks invalid/empty/oversized/malformed test evidence.", originalLines[lineNo - 1] ?? null);
  }
  if (/\b(?:TryFrom|From)\s*<[^>]*(?:Dto|Request|Response|Envelope)[^>]*>/u.test(source) && !/\b(?:negative|invalid|reject)\b/iu.test(source)) {
    const lineNo = firstLineMatching(originalLines, /\b(?:TryFrom|From)\s*</u);
    addViolation(violations, root, filePath, lineNo, "RR-12.18", "DTO conversion lacks negative test evidence.", originalLines[lineNo - 1] ?? null);
  }
  if (/\b(?:BUGFIX|FIXES|bugfix|fixes)\b/u.test(source) && !/\bREGRESSION-TEST:/u.test(source)) {
    const lineNo = firstLineMatching(originalLines, /\b(?:BUGFIX|FIXES|bugfix|fixes)\b/u);
    addViolation(violations, root, filePath, lineNo, "RR-12.19", "bugfix marker lacks REGRESSION-TEST evidence.", originalLines[lineNo - 1] ?? null);
  }
  if (isTestFile(rel, config) && /#\s*\[\s*should_panic/u.test(source) && !/\bPANIC-CONTRACT:/u.test(source)) {
    const lineNo = firstLineMatching(originalLines, /#\s*\[\s*should_panic/u);
    addViolation(violations, root, filePath, lineNo, "RR-12.20", "#[should_panic] lacks PANIC-CONTRACT evidence.", originalLines[lineNo - 1] ?? null);
  }
  if (isTestFile(rel, config)) {
    for (const match of source.matchAll(/#\s*\[\s*test\s*\][\s\S]*?fn\s+[A-Za-z_][A-Za-z0-9_]*\s*\([^)]*\)\s*\{\s*\}/gu)) {
      const lineNo = lineNumberAtIndex(source, match.index ?? 0);
      addViolation(violations, root, filePath, lineNo, "RR-12.24", "empty test body found.", originalLines[lineNo - 1] ?? null);
    }
    for (const match of source.matchAll(/#\s*\[\s*test\s*\][\s\S]*?fn\s+[A-Za-z_][A-Za-z0-9_]*\s*\([^)]*\)\s*\{(?<body>[\s\S]*?)^\s*\}/gmu)) {
      const body = match.groups?.body ?? "";
      const lineNo = lineNumberAtIndex(source, match.index ?? 0);
      if (/\b::(?:new|try_new|parse)\s*\(/u.test(body) && !/\bassert(?:_eq|_ne)?!\s*\(|\bmatches!\s*\(/u.test(body)) {
        addViolation(violations, root, filePath, lineNo, "RR-12.25", "construction-only test lacks behavioral assertion.", originalLines[lineNo - 1] ?? null);
      }
      if (/\b(?:toMatchSnapshot|insta::assert|snapshot)\b/iu.test(body) && /\b(?:\d{4}-\d{2}-\d{2}|[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}|random|uuid)\b/iu.test(body) && !/\bREDACT|redact/u.test(body)) {
        addViolation(violations, root, filePath, lineNo, "RR-12.26", "snapshot test includes volatile value without redaction.", originalLines[lineNo - 1] ?? null);
      }
    }
  }
  if (!isTestFile(rel, config) && /\b(?:normalize|parse)[A-Za-z0-9_]*\s*\(/u.test(masked) && !/\b(?:proptest|quickcheck|PROPERTY-TEST:)/u.test(source)) {
    const lineNo = firstLineMatching(originalLines, /\b(?:normalize|parse)[A-Za-z0-9_]*\s*\(/u);
    addViolation(violations, root, filePath, lineNo, "RR-12.27", "normalizer/parser lacks property-test evidence.", originalLines[lineNo - 1] ?? null);
  }
  if (/\b(?:binary|packet|frame|network)\b/iu.test(source) && /\bparse[A-Za-z0-9_]*\s*\(/u.test(masked) && !/\b(?:fuzz|cargo fuzz|FUZZ-TARGET:)/iu.test(source)) {
    const lineNo = firstLineMatching(originalLines, /\bparse[A-Za-z0-9_]*\s*\(/u);
    addViolation(violations, root, filePath, lineNo, "RR-12.28", "binary/network parser lacks fuzz target evidence.", originalLines[lineNo - 1] ?? null);
  }
  if (/\b(?:tokio::spawn|select!|unbounded_channel|mpsc::channel|async\s+fn)\b/u.test(masked) && !/\b(?:shutdown|cancellation|CANCELLATION-TEST:|SHUTDOWN-TEST:)\b/iu.test(source)) {
    const lineNo = firstLineMatching(originalLines, /\b(?:tokio::spawn|select!|unbounded_channel|mpsc::channel|async\s+fn)\b/u);
    addViolation(violations, root, filePath, lineNo, "RR-12.29", "concurrency code lacks cancellation/shutdown test evidence.", originalLines[lineNo - 1] ?? null);
  }

  for (const match of source.matchAll(/^\s*pub\s+struct\s+(?<name>[A-Z][A-Za-z0-9_]*(?:Dto|DTO|Request|Response|Envelope))\b/gmu)) {
    const name = match.groups?.name ?? "";
    const lineNo = lineNumberAtIndex(source, match.index ?? 0);
    if (!isBoundary) {
      addViolation(violations, root, filePath, lineNo, "RR-14.20", `DTO struct ${name} is outside a boundary/serde/transport module.`, originalLines[lineNo - 1] ?? null);
    }
    if (!/\b(?:TryFrom|From)\s*<[^>]*\b/u.test(source) && !/\b(?:map_to_domain|into_domain|to_domain)\b/u.test(source)) {
      addViolation(violations, root, filePath, lineNo, "RR-14.23", `DTO struct ${name} lacks explicit domain conversion.`, originalLines[lineNo - 1] ?? null);
    }
    if (!/\b(?:round[-_ ]?trip|ROUNDTRIP-TEST:)\b/iu.test(source)) {
      addViolation(violations, root, filePath, lineNo, "RR-14.25", `DTO struct ${name} lacks round-trip test evidence.`, originalLines[lineNo - 1] ?? null);
    }
  }
  for (const match of source.matchAll(/^\s*pub\s+struct\s+(?<name>[A-Z][A-Za-z0-9_]*)\b/gmu)) {
    const name = match.groups?.name ?? "";
    const lineNo = lineNumberAtIndex(source, match.index ?? 0);
    if (isBoundary && /\b(?:Serialize|Deserialize)\b/u.test(source.slice(Math.max(0, match.index - 200), match.index)) && !/(?:Dto|DTO|Request|Response|Envelope)$/u.test(name)) {
      addViolation(violations, root, filePath, lineNo, "RR-14.21", `boundary serde struct ${name} lacks DTO/request/response suffix.`, originalLines[lineNo - 1] ?? null);
    }
    if (/\b(?:Config|Input|Options|Settings)\b/u.test(name) && /\bDeserialize\b/u.test(source.slice(Math.max(0, match.index - 200), match.index)) && !/deny_unknown_fields/u.test(source.slice(Math.max(0, match.index - 260), match.index))) {
      addViolation(violations, root, filePath, lineNo, "RR-14.26", `strict config/input ${name} lacks deny_unknown_fields.`, originalLines[lineNo - 1] ?? null);
    }
  }
  for (const match of source.matchAll(/(?<attrs>(?:^\s*#\[[^\]]+\]\s*\r?\n)*)^\s*pub\s+enum\s+(?<name>[A-Z][A-Za-z0-9_]*)\b/gmu)) {
    const attrs = match.groups?.attrs ?? "";
    if (/\b(?:Serialize|Deserialize)\b/u.test(attrs) && !/\bserde\s*\(\s*tag\s*=/u.test(attrs) && !/SERDE-TAG-JUSTIFICATION:/u.test(attrs)) {
      const lineNo = lineNumberAtIndex(source, match.index ?? 0);
      addViolation(violations, root, filePath, lineNo, "RR-14.24", `public serde enum ${match.groups?.name ?? "enum"} lacks tag or justification.`, originalLines[lineNo - 1] ?? null);
    }
  }
  if (!isBoundary && /\b(?:base64|Base64)\b/u.test(source) && RAW_STRING_TYPE_RE.test(source)) {
    const lineNo = firstLineMatching(originalLines, /\b(?:base64|Base64)\b/u);
    addViolation(violations, root, filePath, lineNo, "RR-14.28", "domain source uses raw base64 string shape.", originalLines[lineNo - 1] ?? null);
  }

  return violations;
}

function scanWorkspaceFiles(root, config, scope) {
  const violations = [];
  const cargoToml = path.join(root, "Cargo.toml");
  if (!fs.existsSync(cargoToml) || !config.enforceWorkspaceFiles)
    return violations;

  const required = [
    ["rust-toolchain.toml", "RR-1.1"],
    ["Cargo.lock", "RR-1.2"],
    ["clippy.toml", "RR-1.3"],
    ["deny.toml", "RR-1.4"],
  ];
  for (const [fileName, ruleId] of required) {
    if (!fs.existsSync(path.join(root, fileName))) {
      addViolation(
        violations,
        root,
        path.join(root, fileName),
        1,
        ruleId,
        `${fileName} is missing.`,
      );
    }
  }

  const manifestPaths = manifestPathsForScope(root, config, scope);
  for (const manifest of manifestPaths) {
    scanCargoManifest(root, manifest, config, violations);
  }

  return violations;
}

function manifestPathsForScope(root, config, scope) {
  if (scope.mode === "crate" && scope.manifest) return [scope.manifest];
  if (scope.mode === "files" || scope.mode === "diff") {
    return uniqueSorted(
      scope.files
        .map((file) => nearestCargoManifest(root, file))
        .filter(Boolean),
    );
  }
  return findCargoManifests(root, config).filter(
    (manifest) => !normalizeRel(root, manifest).includes("/target/"),
  );
}

function nearestCargoManifest(root, filePath) {
  const rootPath = path.resolve(root);
  let current = path.dirname(path.resolve(filePath));
  while (true) {
    const relative = path.relative(rootPath, current);
    if (relative.startsWith("..") || path.isAbsolute(relative)) return null;
    const manifest = path.join(current, "Cargo.toml");
    if (fs.existsSync(manifest)) return path.resolve(manifest);
    if (current === rootPath) return null;
    const next = path.dirname(current);
    if (next === current) return null;
    current = next;
  }
}

function scanCargoManifest(root, manifest, config, violations) {
  const cargoText = fs.readFileSync(manifest, "utf8");
  const packageBlock = cargoText.match(
    /(?:^|\n)\s*\[package\]([\s\S]*?)(?:\n\s*\[|$)/u,
  );
  const workspacePackageBlock = cargoText.match(
    /(?:^|\n)\s*\[workspace\.package\]([\s\S]*?)(?:\n\s*\[|$)/u,
  );
  if (
    packageBlock &&
    !/(^|\n)\s*rust-version\s*=\s*"[^"]+"/u.test(packageBlock[1]) &&
    !/(^|\n)\s*rust-version\s*=\s*"[^"]+"/u.test(
      workspacePackageBlock?.[1] ?? "",
    )
  ) {
    addViolation(
      violations,
      root,
      manifest,
      1,
      "RR-1.5",
      "Cargo.toml package does not declare rust-version.",
    );
  }

  if (/(?:^|\n)\s*license\s*=\s*"(?:[^"]*\bA?GPL\b[^"]*)"/iu.test(packageBlock?.[1] ?? "")) {
    addViolation(
      violations,
      root,
      manifest,
      1,
      "RR-9.22",
      "GPL/AGPL package license found.",
    );
  }

  const lockPath = path.join(root, "Cargo.lock");
  if (fs.existsSync(lockPath)) {
    const manifestMtime = fs.statSync(manifest).mtimeMs;
    const lockMtime = fs.statSync(lockPath).mtimeMs;
    if (manifestMtime > lockMtime + 1000) {
      addViolation(
        violations,
        root,
        lockPath,
        1,
        "RR-9.25",
        "Cargo.toml is newer than Cargo.lock.",
      );
    }
  }

  const lines = cargoText.split(/\r?\n/u);
  let currentSection = "";
  const dependencyNamesBySection = new Map();
  const dependencyRequirementsByName = new Map();
  const currentPackageName = packageNameFromManifest(manifest);
  const workspacePackageNames = workspacePackageNamesFromManifests(root, config);
  lines.forEach((line, idx) => {
    const lineNo = idx + 1;
    const sectionMatch = line.match(/^\s*\[([^\]]+)\]\s*$/u);
    if (sectionMatch) currentSection = sectionMatch[1];
    if (/version\s*=\s*"\*"|=\s*"\*"/u.test(line)) {
      addViolation(
        violations,
        root,
        manifest,
        lineNo,
        "RR-9.1",
        "Wildcard dependency versions are forbidden.",
        line,
      );
    }
    const inDependencySection =
      /^(?:dependencies|dev-dependencies|build-dependencies|target\..+\.dependencies)(?:\.|$)/u.test(
        currentSection,
      );
    const inProductionDependencySection =
      /^(?:dependencies|target\..+\.dependencies)(?:\.|$)/u.test(
        currentSection,
      );
    const dependencyName = dependencyNameFromManifestLine(line);
    if (inDependencySection && dependencyName) {
      if (!dependencyNamesBySection.has(currentSection)) dependencyNamesBySection.set(currentSection, new Set());
      dependencyNamesBySection.get(currentSection).add(dependencyName);
      const dependencyRequirement = dependencyRequirementFromManifestLine(line);
      if (inProductionDependencySection && dependencyRequirement) {
        if (!dependencyRequirementsByName.has(dependencyName)) dependencyRequirementsByName.set(dependencyName, new Set());
        dependencyRequirementsByName.get(dependencyName).add(dependencyRequirement);
      }
      if (
        inProductionDependencySection &&
        workspacePackageNames.has(dependencyName) &&
        dependencyName !== currentPackageName &&
        !/\b(?:path|workspace)\s*=/u.test(line)
      ) {
        addViolation(
          violations,
          root,
          manifest,
          lineNo,
          "RR-9.26",
          `${dependencyName} is a workspace member but is not linked by path/workspace dependency syntax.`,
          line,
        );
      }
      if (!contextHas(lines, idx, "DEPENDENCY-JUSTIFICATION:", 4)) {
        addViolation(
          violations,
          root,
          manifest,
          lineNo,
          "RR-9.18",
          `${dependencyName} lacks DEPENDENCY-JUSTIFICATION.`,
          line,
        );
      }
      if (/^(?:dependencies|target\..+\.dependencies)(?:\.|$)/u.test(currentSection) && config.testOnlyCratesSet.has(dependencyName)) {
        addViolation(
          violations,
          root,
          manifest,
          lineNo,
          "RR-9.27",
          `${dependencyName} is test-only but appears in production dependencies.`,
          line,
        );
        addViolation(
          violations,
          root,
          manifest,
          lineNo,
          "RR-9.28",
          `${dependencyName} must be in dev-dependencies only.`,
          line,
        );
        addViolation(
          violations,
          root,
          manifest,
          lineNo,
          "RR-9.29",
          `${dependencyName} must not be a production dependency in a runtime crate.`,
          line,
        );
      }
      if (/^(?:dependencies|target\..+\.dependencies)(?:\.|$)/u.test(currentSection) && /\b(?:syn|quote|proc-macro2|darling|proc-macro-error)\b/u.test(dependencyName) && !contextHas(lines, idx, "PROC-MACRO-DEPENDENCY-JUSTIFICATION:", 4)) {
        addViolation(
          violations,
          root,
          manifest,
          lineNo,
          "RR-9.20",
          `${dependencyName} is a proc-macro ecosystem dependency in runtime dependencies without approval.`,
          line,
        );
      }
      if (/\b(?:openssl|openssl-sys|libsqlite3-sys|ring|rusqlite|bindgen|cc|cmake|pkg-config)\b/u.test(dependencyName) && !contextHas(lines, idx, "NATIVE-DEPENDENCY-JUSTIFICATION:", 4)) {
        addViolation(
          violations,
          root,
          manifest,
          lineNo,
          "RR-9.21",
          `${dependencyName} is native/build-linked and lacks NATIVE-DEPENDENCY-JUSTIFICATION.`,
          line,
        );
      }
      if (/\b(?:tokio|reqwest|sqlx|diesel|aws-sdk|openssl|rusqlite)\b/u.test(dependencyName) && /\{[^}]*version\s*=/u.test(line) && !/\bdefault-features\s*=\s*false\b/u.test(line)) {
        addViolation(
          violations,
          root,
          manifest,
          lineNo,
          "RR-9.17",
          `${dependencyName} must set default-features explicitly to false or document the default feature policy.`,
          line,
        );
      }
    }
    if (
      inDependencySection &&
      /=\s*"(?:>=|>|<=|<)[^"]*"|=\s*"\d+"|version\s*=\s*"(?:>=|>|<=|<)[^"]*"|version\s*=\s*"\d+"/u.test(
        line,
      )
    ) {
      addViolation(
        violations,
        root,
        manifest,
        lineNo,
        "RR-9.16",
        "Loose dependency version range found.",
        line,
      );
    }
    if (!config.allowGitDependencies && /\bgit\s*=/u.test(line)) {
      addViolation(
        violations,
        root,
        manifest,
        lineNo,
        "RR-9.2",
        "Git dependency found.",
        line,
      );
    }
    if (!config.allowPathDependencies && /\bpath\s*=/u.test(line)) {
      addViolation(
        violations,
        root,
        manifest,
        lineNo,
        "RR-9.3",
        "Path dependency found.",
        line,
      );
    }
    if (/^build-dependencies(?:\.|$)/u.test(currentSection)) {
      const isDependencyLine = /^\s*[\w.-]+\s*=/u.test(line);
      if (isDependencyLine && !contextHas(lines, idx, "BUILD-DEPENDENCY-JUSTIFICATION:", 4)) {
        addViolation(
          violations,
          root,
          manifest,
          lineNo,
          "RR-9.30",
          "build-dependency lacks BUILD-DEPENDENCY-JUSTIFICATION.",
          line,
        );
      }
    }
  });

  const prodDeps = new Set([
    ...(dependencyNamesBySection.get("dependencies") ?? []),
    ...[...dependencyNamesBySection.entries()]
      .filter(([section]) => /^target\..+\.dependencies/u.test(section))
      .flatMap(([, names]) => [...names]),
  ]);
  const devDeps = dependencyNamesBySection.get("dev-dependencies") ?? new Set();
  for (const name of devDeps) {
    if (prodDeps.has(name)) {
      addViolation(
        violations,
        root,
        manifest,
        1,
        "RR-9.27",
        `${name} appears in both production and dev dependencies.`,
      );
    }
  }
  for (const [dependencyName, requirements] of dependencyRequirementsByName) {
    if (requirements.size > 1) {
      addViolation(
        violations,
        root,
        manifest,
        1,
        "RR-9.19",
        `Direct dependency ${dependencyName} uses multiple requirements: ${[...requirements].join(", ")}.`,
      );
    }
  }

  const denyPath = path.join(root, "deny.toml");
  if (fs.existsSync(denyPath)) {
    const denyText = fs.readFileSync(denyPath, "utf8");
    if (!/\byanked\s*=\s*"deny"/u.test(denyText)) {
      addViolation(violations, root, denyPath, 1, "RR-9.23", 'deny.toml must deny yanked crate versions.');
    }
    if (!/\bunmaintained\s*=\s*"deny"/u.test(denyText)) {
      addViolation(violations, root, denyPath, 1, "RR-9.24", 'deny.toml must deny unmaintained crates when advisory data is available.');
    }
  }

  const buildRs = path.join(path.dirname(manifest), "build.rs");
  if (!config.allowBuildRs && fs.existsSync(buildRs)) {
    addViolation(
      violations,
      root,
      buildRs,
      1,
      "RR-7.5",
      "build.rs is forbidden by default because it can hide non-deterministic build behavior.",
    );
  }
}

function dependencyNameFromManifestLine(line) {
  const match = line.match(/^\s*([A-Za-z0-9_.-]+)\s*=/u);
  return match?.[1] ?? null;
}

function dependencyRequirementFromManifestLine(line) {
  return (
    line.match(/=\s*"([^"]+)"/u)?.[1] ??
    line.match(/\bversion\s*=\s*"([^"]+)"/u)?.[1] ??
    null
  );
}

function workspacePackageNamesFromManifests(root, config) {
  const names = new Set();
  for (const manifest of findCargoManifests(root, config)) {
    const name = packageNameFromManifest(manifest);
    if (name) names.add(name);
  }
  return names;
}

function loadCargoMetadata(root) {
  const result = spawnSync(
    "cargo",
    ["metadata", "--no-deps", "--format-version", "1"],
    {
      cwd: root,
      encoding: "utf8",
      maxBuffer: 32 * 1024 * 1024,
      shell: false,
    },
  );
  if (result.error) throw result.error;
  if ((result.status ?? 1) !== 0) return null;
  return JSON.parse(result.stdout);
}

function scanCargoMetadata(root, config, scope) {
  const violations = [];
  if (!fs.existsSync(path.join(root, "Cargo.toml"))) return violations;
  if (!commandExists("cargo")) return violations;
  const metadata = loadCargoMetadata(root);
  if (!metadata) return violations;
  const workspaceRoot = toPosix(metadata.workspace_root);
  const scopedManifests = new Set(
    manifestPathsForScope(root, config, scope).map((manifest) =>
      toPosix(manifest),
    ),
  );
  const packageFilter =
    scope.mode === "crate"
      ? (packageInfo) => packageInfo.name === scope.crateName
      : scope.mode === "files" || scope.mode === "diff"
        ? (packageInfo) =>
            scopedManifests.has(toPosix(packageInfo.manifest_path))
        : () => true;
  const packages = (metadata.packages ?? []).filter(packageFilter);
  const workspacePackageNames = new Set((metadata.packages ?? []).map((packageInfo) => packageInfo.name));

  for (const packageInfo of packages) {
    const blockedForPackage = new Set(
      config.blockedProtocolDependencies[packageInfo.name] ?? [],
    );
    for (const dependency of packageInfo.dependencies ?? []) {
      if (
        (dependency.source ?? "").startsWith("git+") &&
        !config.allowedGitDependenciesSet.has(dependency.name)
      ) {
        addViolation(
          violations,
          root,
          packageInfo.manifest_path,
          1,
          "RR-9.2",
          "Git dependency found in cargo metadata.",
        );
      }

      const dependencyPath = dependency.path ?? null;
      if (dependencyPath !== null) {
        const normalizedPath = toPosix(dependencyPath);
        if (!normalizedPath.startsWith(workspaceRoot)) {
          addViolation(
            violations,
            root,
            packageInfo.manifest_path,
            1,
            "RR-9.4",
            "Path dependency points outside the workspace root.",
          );
        }
      }

      if (dependencyPath === null && dependency.req.trim() === "*") {
        addViolation(
          violations,
          root,
          packageInfo.manifest_path,
          1,
          "RR-9.1",
          "Wildcard registry dependency version found.",
        );
      }

      if (blockedForPackage.has(dependency.name)) {
        addViolation(
          violations,
          root,
          packageInfo.manifest_path,
          1,
          "RR-9.4",
          `${packageInfo.name} must not depend on ${dependency.name}.`,
        );
      }

      if (
        config.runtimeCratesSet.has(packageInfo.name) &&
        config.testOnlyCratesSet.has(dependency.name) &&
        dependency.kind !== "dev"
      ) {
        addViolation(
          violations,
          root,
          packageInfo.manifest_path,
          1,
          "RR-9.29",
          "Runtime crate depends on test-only crate outside dev-dependencies.",
        );
      }
      if (workspacePackageNames.has(dependency.name) && dependency.path === null && dependency.source !== null) {
        addViolation(
          violations,
          root,
          packageInfo.manifest_path,
          1,
          "RR-9.26",
          `Workspace member ${packageInfo.name} depends on ${dependency.name} by registry version instead of path/workspace linkage.`,
        );
      }
    }
  }

  const directReqs = new Map();
  for (const packageInfo of packages) {
    for (const dependency of packageInfo.dependencies ?? []) {
      if (
        (dependency.path ?? null) !== null ||
        dependency.kind === "dev" ||
        (dependency.source ?? "").startsWith("git+")
      )
        continue;
      if (!directReqs.has(dependency.name))
        directReqs.set(dependency.name, new Set());
      directReqs.get(dependency.name).add(dependency.req);
    }
  }
  for (const [dependencyName, reqs] of directReqs) {
    if (reqs.size > 1) {
      addViolation(
        violations,
        root,
        ".",
        1,
        "RR-9.5",
        `Direct registry dependency ${dependencyName} uses multiple requirements: ${[...reqs].join(", ")}.`,
      );
      addViolation(
        violations,
        root,
        ".",
        1,
        "RR-9.19",
        `Direct registry dependency ${dependencyName} uses duplicate requirements: ${[...reqs].join(", ")}.`,
      );
    }
  }

  return violations;
}

function runScanner(root, config, scope) {
  const violations = [];
  violations.push(...scanWorkspaceFiles(root, config, scope));
  violations.push(...scanCargoMetadata(root, config, scope));
  for (const filePath of scope.files) {
    violations.push(...scanRustFile(root, filePath, config));
    if (config.failFast && violations.length > 0) break;
  }
  return violations;
}

function commandExists(command) {
  const result = spawnSync(command, ["--version"], {
    encoding: "utf8",
    stdio: "pipe",
    shell: false,
  });
  return result.status === 0;
}

function runCommand(root, command, args, ruleId, env = {}) {
  const result = spawnSync(command, args, {
    cwd: root,
    env: { ...process.env, ...env },
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
    shell: false,
  });
  if (result.status === 0) return [];
  const output = [result.stdout, result.stderr]
    .filter(Boolean)
    .join("\n")
    .trim();
  return [
    {
      ruleId,
      title: RULES[ruleId].title,
      detail: `${command} ${args.join(" ")} failed with exit code ${result.status ?? "unknown"}.`,
      file: ".",
      line: 1,
      snippet: RULES[ruleId].snippet,
      source: output.slice(0, 4000),
    },
  ];
}

function shouldRunCargoForScope(scope, config) {
  if (scope.mode === "all" || scope.mode === "crate") return true;
  if (scope.mode === "files") return config.cargoOnFileScope;
  if (scope.mode === "diff") return config.cargoOnDiffScope;
  return false;
}

function cargoPackageArgs(scope) {
  return scope.mode === "crate"
    ? ["--package", scope.crateName]
    : ["--workspace"];
}

function configuredCargoCommand(
  root,
  config,
  toolId,
  defaultEnabled,
  command,
  args,
  ruleId,
  env = {},
) {
  const toolPolicy = policyForTool(toolId, config, {
    enabled: defaultEnabled,
    severity: "error",
  });
  if (!toolPolicy.enabled) return [];
  return runCommand(root, command, args, ruleId, env).map((finding) => ({
    ...finding,
    severity: toolPolicy.severity,
  }));
}

function strongestEnabledSeverity(policies) {
  const enabled = policies
    .filter((policy) => policy.enabled)
    .map((policy) => policy.severity);
  if (enabled.includes("error")) return "error";
  if (enabled.includes("warning")) return "warning";
  return "info";
}

function runCargoGates(root, config, scope) {
  const violations = [];
  if (!fs.existsSync(path.join(root, "Cargo.toml"))) return violations;
  if (!shouldRunCargoForScope(scope, config)) return violations;

  const cargoToolPolicies = [
    policyForTool("cargoFmt", config, { enabled: true, severity: "error" }),
    policyForTool("cargoClippy", config, { enabled: true, severity: "error" }),
    policyForTool("cargoTest", config, { enabled: true, severity: "error" }),
    policyForTool("cargoDoc", config, {
      enabled: config.runCargoDoc,
      severity: "error",
    }),
    policyForTool("cargoDeny", config, {
      enabled: config.requireCargoDeny,
      severity: "error",
    }),
    policyForTool("cargoAudit", config, {
      enabled: config.requireCargoAudit,
      severity: "error",
    }),
  ];
  if (!cargoToolPolicies.some((policy) => policy.enabled)) return violations;

  if (!commandExists("cargo")) {
    violations.push({
      ruleId: "RR-10.2",
      severity: strongestEnabledSeverity(cargoToolPolicies),
      title: RULES["RR-10.2"].title,
      detail: "cargo is not installed or not on PATH.",
      file: ".",
      line: 1,
      snippet: "Install Rust/Cargo and rerun the gate.",
      source: null,
    });
    return violations;
  }

  const packageArgs = cargoPackageArgs(scope);
  violations.push(
    ...configuredCargoCommand(
      root,
      config,
      "cargoFmt",
      true,
      "cargo",
      ["fmt", "--all", "--", "--check"],
      "RR-10.1",
    ),
  );
  violations.push(
    ...configuredCargoCommand(
      root,
      config,
      "cargoClippy",
      true,
      "cargo",
      [
        "clippy",
        ...packageArgs,
        "--all-targets",
        "--all-features",
        "--",
        "-D",
        "warnings",
        "-D",
        "clippy::unwrap_used",
        "-D",
        "clippy::expect_used",
        "-D",
        "clippy::panic",
        "-D",
        "clippy::todo",
        "-D",
        "clippy::unimplemented",
        "-D",
        "clippy::dbg_macro",
        "-D",
        "clippy::print_stdout",
        "-D",
        "clippy::print_stderr",
        "-D",
        "clippy::wildcard_imports",
        "-D",
        "clippy::enum_glob_use",
        "-D",
        "clippy::clone_on_ref_ptr",
        "-D",
        "clippy::await_holding_lock",
        "-D",
        "clippy::await_holding_refcell_ref",
      ],
      "RR-10.2",
    ),
  );

  const testArgs = ["test", ...packageArgs, "--all-features"];
  if (config.cargoTestThreads !== null) {
    testArgs.push("--", `--test-threads=${config.cargoTestThreads}`);
  }
  violations.push(
    ...configuredCargoCommand(
      root,
      config,
      "cargoTest",
      true,
      "cargo",
      testArgs,
      "RR-10.3",
    ),
  );

  const cargoDocPolicy = policyForTool("cargoDoc", config, {
    enabled: config.runCargoDoc,
    severity: "error",
  });
  if (cargoDocPolicy.enabled) {
    violations.push(
      ...configuredCargoCommand(
        root,
        config,
        "cargoDoc",
        config.runCargoDoc,
        "cargo",
        ["doc", ...packageArgs, "--all-features", "--no-deps"],
        "RR-10.4",
        {
          RUSTDOCFLAGS:
            "-D warnings -D rustdoc::broken_intra_doc_links -D rustdoc::bare_urls -D missing_docs",
        },
      ),
    );
  }

  const cargoDenyPolicy = policyForTool("cargoDeny", config, {
    enabled: config.requireCargoDeny,
    severity: "error",
  });
  if (cargoDenyPolicy.enabled) {
    if (!commandExists("cargo-deny")) {
      violations.push({
        ruleId: "RR-11.2",
        severity: cargoDenyPolicy.severity,
        title: RULES["RR-11.2"].title,
        detail: "cargo-deny is required but not installed or not on PATH.",
        file: ".",
        line: 1,
        snippet: RULES["RR-11.2"].snippet,
        source: null,
      });
    } else {
      violations.push(
        ...configuredCargoCommand(
          root,
          config,
          "cargoDeny",
          config.requireCargoDeny,
          "cargo",
          ["deny", "check"],
          "RR-11.1",
        ),
      );
    }
  }

  const cargoAuditPolicy = policyForTool("cargoAudit", config, {
    enabled: config.requireCargoAudit,
    severity: "error",
  });
  if (cargoAuditPolicy.enabled) {
    if (!commandExists("cargo-audit")) {
      violations.push({
        ruleId: "RR-11.3",
        severity: cargoAuditPolicy.severity,
        title: RULES["RR-11.3"].title,
        detail: "cargo-audit is enabled but not installed or not on PATH.",
        file: ".",
        line: 1,
        snippet: RULES["RR-11.3"].snippet,
        source: null,
      });
    } else {
      violations.push(
        ...configuredCargoCommand(
          root,
          config,
          "cargoAudit",
          config.requireCargoAudit,
          "cargo",
          ["audit"],
          "RR-11.3",
        ),
      );
    }
  }

  return violations;
}

function printHumanReport(report) {
  const warnings = report.warnings ?? [];
  if (report.violations.length === 0 && warnings.length === 0) {
    console.log(
      `Ocentra Enforcer ${report.command} passed for ${report.scope.files.length} file(s).`,
    );
    return;
  }

  if (report.violations.length === 0) {
    console.log(
      `Ocentra Enforcer ${report.command} passed with ${warnings.length} warning${warnings.length === 1 ? "" : "s"} for ${report.scope.files.length} file(s).`,
    );
    printFindingList(warnings, "Warning");
    return;
  }

  console.error(
    `Ocentra Enforcer ${report.command} failed with ${report.violations.length} violation${report.violations.length === 1 ? "" : "s"}.`,
  );
  console.error(`Profile: ${report.profileName}`);
  console.error(`Scope: ${describeScope(report.scope)}`);
  console.error(
    `Failing severities: ${(report.failOn ?? ["error"]).join(", ")}`,
  );
  console.error("");
  printFindingList(report.violations, "Reason");
  if (warnings.length > 0) printFindingList(warnings, "Warning");
}

function printFindingList(findings, label) {
  for (const finding of findings) {
    console.error(
      `${finding.file}:${finding.line}: ${finding.severity ?? "error"} ${finding.ruleId} ${finding.title}`,
    );
    console.error(`  ${label}: ${finding.detail}`);
    console.error(`  Rule: ${finding.doc ?? ruleDocFor(finding.ruleId)}`);
    console.error(`  Fix: ${finding.snippet}`);
    if (finding.source) {
      for (const line of String(finding.source).split(/\r?\n/u).slice(0, 12))
        console.error(`  > ${line}`);
    }
    console.error("");
  }
}

function describeScope(scope) {
  if (scope.mode === "crate") return `crate ${scope.crateName}`;
  if (scope.mode === "diff") return `diff ${scope.base}..${scope.head}`;
  if (scope.mode === "files") return "explicit files";
  return "workspace";
}

function explainRule(ruleId) {
  const normalized = ruleId?.toUpperCase();
  const rule =
    RULES[normalized] ?? GENERIC_RULES[normalized] ?? CHECK_RULES[normalized];
  if (!rule) throw new Error(`Unknown rule ID: ${ruleId}`);
  return { ruleId: normalized, ...rule, anchor: ruleDocFor(normalized) };
}

function ruleDocFor(ruleId) {
  if (cachedRuleDocs === null) {
    cachedRuleDocs = new Map();
    if (fs.existsSync(RULE_REGISTRY_PATH)) {
      const registry = decodeRuleRegistry(
        JSON.parse(fs.readFileSync(RULE_REGISTRY_PATH, "utf8")),
      );
      for (const entry of registry.rules ?? [])
        cachedRuleDocs.set(entry.id, entry.doc);
    }
  }
  return (
    cachedRuleDocs.get(ruleId) ??
    `docs/RustRules.md#${ruleId.toLowerCase().replace(".", "")}`
  );
}

function decorateRuleDocs(report) {
  const completenessFailures = collectReportCompletenessFailures(report);
  for (const key of ["violations", "warnings", "waived", "findings"]) {
    if (!Array.isArray(report[key])) continue;
    report[key] = sortFindings(
      report[key].map((finding) => normalizeReportFinding(finding)),
    );
  }
  enforceReportCompleteness(report, completenessFailures);
  return report;
}

function normalizeReportFinding(finding) {
  const normalized = {
    ruleId: finding.ruleId ?? "ENF-1.8",
    severity: finding.severity ?? "error",
    title: finding.title ?? RULES[finding.ruleId]?.title ?? "Incomplete report finding",
    detail: finding.detail ?? "",
    file: finding.file ?? "",
    line: Number.isInteger(finding.line) ? finding.line : 1,
    snippet:
      finding.snippet ??
      RULES[finding.ruleId]?.snippet ??
      "Emit complete, deterministic findings.",
    source: finding.source ?? null,
    doc: finding.doc ?? ruleDocFor(finding.ruleId),
  };
  for (const [key, value] of Object.entries(finding)) {
    if (!(key in normalized)) normalized[key] = value;
  }
  return normalized;
}

function sortFindings(findings) {
  return [...findings].sort(compareFindings);
}

function compareFindings(a, b) {
  return (
    String(a.file ?? "").localeCompare(String(b.file ?? "")) ||
    Number(a.line ?? 0) - Number(b.line ?? 0) ||
    String(a.ruleId ?? "").localeCompare(String(b.ruleId ?? "")) ||
    String(a.detail ?? "").localeCompare(String(b.detail ?? ""))
  );
}

function collectReportCompletenessFailures(report) {
  const required = [
    "ruleId",
    "severity",
    "title",
    "detail",
    "file",
    "line",
    "snippet",
    "source",
  ];
  const bad = [];
  for (const key of ["violations", "warnings", "findings"]) {
    if (!Array.isArray(report[key])) continue;
    report[key].forEach((finding, index) => {
      const missing = required.filter((field) => !(field in finding));
      if (missing.length > 0) bad.push(`${key}[${index}] missing ${missing.join(",")}`);
    });
  }
  return bad;
}

function enforceReportCompleteness(report, bad) {
  if (bad.length === 0) return;
  const reportFinding = normalizeReportFinding({
    ruleId: "ENF-1.8",
    severity: "error",
    title: "Validation reports must be complete",
    detail: bad.slice(0, 5).join("; "),
    file: report.root ?? process.cwd(),
    line: 1,
    snippet: "Emit ruleId, severity, title, detail, file, line, snippet, source, and doc.",
  });
  report.findings = sortFindings([...(report.findings ?? []), reportFinding]);
  report.violations = sortFindings([...(report.violations ?? []), reportFinding]);
  report.ok = false;
  report.bySeverity = {
    ...(report.bySeverity ?? {}),
    error: Number(report.bySeverity?.error ?? 0) + 1,
  };
}

function doctor(root, config, scope) {
  const checks = [
    { name: "root", ok: fs.existsSync(root), detail: root },
    {
      name: "config schema",
      ok: config.schemaVersion >= 1,
      detail: `schemaVersion=${config.schemaVersion}`,
    },
    {
      name: "cargo",
      ok: commandExists("cargo"),
      detail: "required for cargo gates and metadata dependency checks",
    },
    {
      name: "git",
      ok: commandExists("git"),
      detail: "required for diff scopes",
    },
    {
      name: "cargo-deny",
      ok: !config.requireCargoDeny || commandExists("cargo-deny"),
      detail: config.requireCargoDeny
        ? "required when requireCargoDeny=true"
        : "not required by this profile",
    },
    {
      name: "scope files",
      ok: scope.files.length > 0,
      detail: `${scope.files.length} Rust file(s) selected`,
    },
  ];
  return {
    ok: checks.every((check) => check.ok),
    command: "doctor",
    root,
    profileName: config.profileName,
    scope,
    checks,
    violations: [],
  };
}

function printDoctor(report) {
  for (const check of report.checks) {
    console.log(`${check.ok ? "PASS" : "FAIL"} ${check.name}: ${check.detail}`);
  }
}

function runRunsCommand(args, root, config) {
  const query = {
    root,
    harness: config.harness,
    runId: args.runId,
    limit: args.limit ?? undefined,
    diagnosticLimit: args.limit ?? undefined,
    severity: args.severity ?? undefined,
    status: args.status ?? undefined,
    file: args.file ?? undefined,
    tool: args.runTool ?? undefined,
    crateName: args.crateName ?? undefined,
    packageName: args.packageName ?? undefined,
    domain: args.domain ?? undefined,
    tag: args.tag ?? undefined,
    artifact: args.artifact ?? undefined,
    limitBytes: args.limitBytes ?? undefined,
  };
  switch (args.runsCommand) {
    case "list":
      return { ok: true, runs: listRuns(query) };
    case "summary":
      return { ok: true, summary: runSummary(query) };
    case "diagnostics":
      return runDiagnostics(query);
    case "last-failure":
      return lastFailure(query);
    case "artifact":
      return readArtifact(query);
    case "prune":
      return pruneRuns(query);
    case "reset":
      return resetRuns(query);
    case "ingest":
      return {
        ok: true,
        message:
          "NDJSON manifests are updated at run completion; DuckDB ingestion is optional in this build.",
      };
    default:
      throw new Error(`Unknown runs command: ${args.runsCommand}`);
  }
}

function printRunReport(report) {
  console.log(
    `Ocentra Enforcer run ${report.summary.status}: ${report.summary.runId}`,
  );
  console.log(`Tool: ${report.summary.tool}`);
  if (report.summary.crateName)
    console.log(`Crate: ${report.summary.crateName}`);
  if (report.summary.packageName)
    console.log(`Package: ${report.summary.packageName}`);
  if (report.summary.domain) console.log(`Domain: ${report.summary.domain}`);
  console.log(`Exit: ${report.summary.exitCode}`);
  console.log(`Diagnostics: ${report.summary.diagnosticCount}`);
  console.log(
    `Artifacts: ${Object.values(report.summary.artifacts).join(", ")}`,
  );
  for (const diagnostic of report.diagnostics.slice(0, 10)) {
    console.log(
      `${diagnostic.file}:${diagnostic.line}: ${diagnostic.ruleId} ${diagnostic.message}`,
    );
  }
}

function printRunsReport(command, report) {
  if (command === "list") {
    for (const run of report.runs) {
      const meta = [
        run.crateName ? `crate=${run.crateName}` : null,
        run.packageName ? `package=${run.packageName}` : null,
        run.domain ? `domain=${run.domain}` : null,
        run.tags?.length ? `tags=${run.tags.join(",")}` : null,
      ]
        .filter(Boolean)
        .join(" ");
      console.log(
        `${run.runId} ${run.status} ${run.tool} diagnostics=${run.diagnosticCount}${meta ? ` ${meta}` : ""}`,
      );
    }
    return;
  }
  if (command === "artifact") {
    console.log(report.text ?? report.message ?? "");
    return;
  }
  if (command === "diagnostics") {
    for (const diagnostic of report.diagnostics ?? []) {
      console.log(
        `${diagnostic.file}:${diagnostic.line}: ${diagnostic.ruleId} ${diagnostic.message}`,
      );
    }
    return;
  }
  console.log(JSON.stringify(report, null, 2));
}

function printCheckReport(report) {
  const label = report.check ?? report.command ?? "check";
  if (report.violations.length === 0) {
    console.log(`Ocentra Enforcer ${label} passed.`);
    return;
  }
  console.error(
    `Ocentra Enforcer ${label} failed with ${report.violations.length} violation${report.violations.length === 1 ? "" : "s"}.`,
  );
  console.error(`Profile: ${report.profileName}`);
  console.error("");
  printFindingList(report.violations, "Reason");
}

export function runRustRules(options = {}) {
  const root = path.resolve(options.root ?? process.cwd());
  const config = normalizeConfig({
    ...DEFAULT_CONFIG,
    ...(options.config ??
      loadConfig(root, options.configPath, options.profile)),
  });
  const scope = resolveScope(root, config, options.scope ?? { mode: "all" });
  const command = options.command ?? "scan";
  const scannerViolations = runScanner(root, config, scope);
  const cargoViolations =
    options.scanOnly || command === "scan"
      ? []
      : runCargoGates(root, config, scope);
  const {
    violations,
    warnings,
    waived,
    findings,
    bySeverity,
  } = applyPolicyAndWaivers(
    [
      ...policyPreflightFindings(root, config, options),
      ...scannerViolations,
      ...cargoViolations,
    ],
    config,
  );
  return decorateRuleDocs({
    ok: violations.length === 0,
    command,
    violations,
    warnings,
    waived,
    findings,
    bySeverity,
    failOn: config.failOn,
    root,
    profileName: config.profileName,
    scanOnly: Boolean(options.scanOnly || command === "scan"),
    scope: {
      ...scope,
      files: scope.files.map((file) => normalizeRel(root, file)),
    },
  });
}

export function runEnforcerScan(options = {}) {
  const root = path.resolve(options.root ?? process.cwd());
  const config = normalizeConfig({
    ...DEFAULT_CONFIG,
    ...(options.config ??
      loadConfig(root, options.configPath, options.profile)),
  });
  const activeLanguages = resolveScanLanguages(options.languages, config);
  const rawScope = options.rawScope ?? options.scope ?? { mode: "all" };
  const rustReport = activeLanguages.includes("rust")
    ? runRustRules({
        root,
        config,
        scope: rawScope,
        command: options.command ?? "scan",
        scanOnly: options.scanOnly,
      })
    : {
        ok: true,
        command: options.command ?? "scan",
        violations: [],
        root,
        profileName: config.profileName,
        scanOnly: Boolean(options.scanOnly || options.command === "scan"),
        scope: { ...rawScope, files: [] },
      };
  const genericLanguages = activeLanguages.filter(
    (language) => language !== "rust",
  );
  const genericReport =
    genericLanguages.length === 0
      ? { files: [], violations: [] }
      : runGenericScan({
          root,
          scope: rawScope,
          config,
          languages: genericLanguages,
        });
  const genericPolicy = applyPolicyAndWaivers(
    [
      ...(activeLanguages.includes("rust")
        ? []
        : policyPreflightFindings(root, config, options)),
      ...genericReport.violations,
    ],
    config,
  );
  const findings = [
    ...(rustReport.violations ?? []),
    ...(rustReport.warnings ?? []),
    ...genericPolicy.violations,
    ...genericPolicy.warnings,
  ];
  const waived = [...(rustReport.waived ?? []), ...genericPolicy.waived];
  const { violations, warnings, bySeverity } = splitFindings(findings, config);
  const scopeFiles = uniqueSorted([
    ...(rustReport.scope.files ?? []),
    ...genericReport.files,
  ]);
  return decorateRuleDocs({
    ...rustReport,
    ok: violations.length === 0,
    command: options.command ?? rustReport.command,
    violations,
    warnings,
    waived,
    findings: [...findings, ...waived],
    bySeverity,
    failOn: config.failOn,
    languages: activeLanguages,
    scope: { ...rustReport.scope, files: scopeFiles },
  });
}

export function runEnforcerCheck(options = {}) {
  const root = path.resolve(options.root ?? process.cwd());
  const config = normalizeConfig({
    ...DEFAULT_CONFIG,
    ...(options.config ??
      loadConfig(root, options.configPath, options.profile)),
  });
  const rawScope = options.rawScope ?? options.scope ?? { mode: "all" };
  const decoded = decodeCheckToolArguments({
    root,
    configPath: options.configPath ?? undefined,
    profile: options.profile ?? undefined,
    check: normalizeCheckName(options.checkName ?? options.check),
    scope: rawScope.mode === "all" ? "workspace" : rawScope.mode,
    files: rawScope.files ?? undefined,
    crateName: rawScope.crateName ?? undefined,
    base: rawScope.base ?? undefined,
    head: rawScope.head ?? undefined,
    checkConfigPath: options.checkConfigPath ?? undefined,
    output: options.output ?? undefined,
    dryRun: options.dryRun ?? undefined,
    staged: options.staged ?? undefined,
    tracked: options.tracked ?? undefined,
    strictEmptyTestTrees: options.strictEmptyTestTrees ?? undefined,
  });
  const checkName = decoded.check;
  if (checkName === "architecture-policy") {
    return runArchitecturePolicyCheck({
      ...options,
      root,
      config,
      rawScope,
      decoded,
    });
  }
  const scannerBacked = SCANNER_BACKED_CHECKS[checkName];
  if (scannerBacked) {
    const report = runEnforcerScan({
      root,
      config,
      rawScope,
      command: "check",
      scanOnly: true,
      languages: scannerBacked.languages,
    });
    const allowed = new Set(scannerBacked.ruleIds);
    const findings = [
      ...(report.violations ?? []),
      ...(report.warnings ?? []),
    ].filter((finding) =>
      allowed.has(finding.ruleId),
    );
    const waived = (report.waived ?? []).filter((finding) =>
      allowed.has(finding.ruleId),
    );
    const { violations, warnings, bySeverity } = splitFindings(
      findings,
      config,
    );
    return decorateRuleDocs({
      ok: violations.length === 0,
      command: "check",
      check: checkName,
      root,
      profileName: report.profileName,
      violations,
      warnings,
      waived,
      findings: [...findings, ...waived],
      bySeverity,
      scope: report.scope,
      languages: scannerBacked.languages,
    });
  }
  return decorateRuleDocs(
    runStandaloneCheck({
      checkName,
      root,
      config,
      args: {
        scope: rawScope,
        checkConfigPath: decoded.checkConfigPath,
        output: decoded.output,
        dryRun: decoded.dryRun,
        staged: decoded.staged,
        tracked: decoded.tracked,
        strictEmptyTestTrees: decoded.strictEmptyTestTrees,
      },
    }),
  );
}

export function runEnforcerVerify(options = {}) {
  const root = path.resolve(options.root ?? process.cwd());
  const config = normalizeConfig({
    ...DEFAULT_CONFIG,
    ...(options.config ?? loadConfig(root, options.configPath, options.profile)),
  });
  const rawScope = options.rawScope ?? options.scope ?? { mode: "all" };
  const verifyMode = normalizeVerifyMode(options.verifyMode ?? "local");
  const scanReport = runEnforcerScan({
    root,
    config,
    rawScope,
    command: "verify",
    scanOnly: true,
    languages: options.languages,
  });
  const checkNames = VERIFY_MODE_CHECKS[verifyMode];
  const checkReports = checkNames.map((checkName) =>
    runEnforcerCheck({
      root,
      config,
      rawScope,
      checkName,
      configPath: options.configPath,
      profile: options.profile,
    }),
  );
  const reports = [scanReport, ...checkReports];
  const findings = reports.flatMap((report) => [
    ...(report.violations ?? []),
    ...(report.warnings ?? []),
  ]);
  const waived = reports.flatMap((report) => report.waived ?? []);
  const { violations, warnings, bySeverity } = splitFindings(findings, config);
  return decorateRuleDocs({
    ok: reports.every((report) => report.ok) && violations.length === 0,
    command: "verify",
    verifyMode,
    root,
    profileName: config.profileName ?? "strict",
    violations,
    warnings,
    waived,
    findings: [...findings, ...waived],
    bySeverity,
    scope: scanReport.scope,
    checks: reports.map((report) => ({
      command: report.command,
      check: report.check ?? report.command,
      ok: report.ok,
      violations: (report.violations ?? []).length,
      warnings: (report.warnings ?? []).length,
    })),
  });
}

function runArchitecturePolicyCheck({
  root,
  config,
  rawScope,
  decoded,
  ...options
}) {
  const checks =
    config.architecturePolicyChecks ?? DEFAULT_ARCHITECTURE_POLICY_CHECKS;
  const reports = [];
  for (const check of checks) {
    if (check === "architecture-policy") continue;
    reports.push(
      runEnforcerCheck({
        ...options,
        root,
        config,
        rawScope,
        checkName: check,
        checkConfigPath: decoded.checkConfigPath,
        output: decoded.output,
        dryRun: decoded.dryRun,
        staged: decoded.staged,
        tracked: decoded.tracked,
        strictEmptyTestTrees: decoded.strictEmptyTestTrees,
      }),
    );
  }
  const findings = reports.flatMap((report) => [
    ...(report.violations ?? []),
    ...(report.warnings ?? []),
  ]);
  const waived = reports.flatMap((report) => report.waived ?? []);
  const { violations, warnings, bySeverity } = splitFindings(findings, config);
  return decorateRuleDocs({
    ok: violations.length === 0,
    command: "check",
    check: "architecture-policy",
    root,
    profileName: config.profileName ?? "strict",
    violations,
    warnings,
    waived,
    findings: [...findings, ...waived],
    bySeverity,
    scope: reports.find((report) => report.scope)?.scope ?? {
      mode: rawScope.mode === "all" ? "workspace" : rawScope.mode,
      files: [],
    },
    checks: reports.map((report) => ({
      check: report.check,
      ok: report.ok,
      violations: report.violations.length,
    })),
    languages: [
      ...new Set(reports.flatMap((report) => report.languages ?? [])),
    ],
  });
}

function resolveScanLanguages(optionLanguages, config) {
  const languages = optionLanguages ?? config.languages ?? [
    "rust",
    "typescript",
    "python",
    "common",
  ];
  const allowed = new Set(["rust", "typescript", "python", "common"]);
  const normalized = languages
    .map((language) => String(language).trim())
    .filter(Boolean);
  for (const language of normalized) {
    if (!allowed.has(language))
      throw new Error(`Unknown scan language: ${language}`);
  }
  return normalized.length > 0
    ? normalized
    : ["rust", "typescript", "python", "common"];
}

function isMainModule() {
  return process.argv[1] && path.resolve(process.argv[1]) === SCRIPT_PATH;
}

if (isMainModule()) {
  try {
    const rawCommand = process.argv[2];
    if (rawCommand === "proof") {
      const report = runProofCli(process.argv.slice(3), {
        packRoot: PACK_ROOT,
        defaultRoot: process.cwd(),
      });
      if (report.json) console.log(JSON.stringify(report.result, null, 2));
      else console.log(report.text);
      process.exit(report.exitCode);
    }
    if (rawCommand === "coordination" || rawCommand === "ledger") {
      await runCoordinationCli(process.argv.slice(3));
      process.exit(process.exitCode ?? 0);
    }
    if (rawCommand === "architecture") {
      const report = runArchitectureCli(process.argv.slice(3));
      if (report.json) console.log(JSON.stringify(report.result, null, 2));
      else printCheckReport(report.result);
      process.exit(report.result.ok ? 0 : 1);
    }

    const args = parseArgs(process.argv);
    if (args.help) {
      console.log(usage());
      process.exit(0);
    }

    if (args.command === "init") {
      const report = createInitReport(args);
      if (!report.dryRun) applyInitReport(report);
      if (args.json) console.log(JSON.stringify(report, null, 2));
      else printInitReport(report);
      process.exit(0);
    }

    if (args.command === "codex-install") {
      const report = applyCodexInstallReport(createCodexInstallReport(args));
      if (args.json) console.log(JSON.stringify(report, null, 2));
      else printCodexInstallReport(report);
      process.exit(0);
    }

    if (args.command === "codex-uninstall") {
      const report = applyCodexUninstallReport(createCodexUninstallCliReport(args));
      if (args.json) console.log(JSON.stringify(report, null, 2));
      else printCodexUninstallReport(report);
      process.exit(0);
    }

    if (args.command === "codex-doctor") {
      const report = createCodexDoctorReport(args);
      if (args.json) console.log(JSON.stringify(report, null, 2));
      else printCodexDoctorReport(report);
      process.exit(report.ok ? 0 : 1);
    }

    const root = path.resolve(args.root);
    const config = loadConfig(root, args.configPath, args.profile);

    if (args.command === "route") {
      const report = routeRules({
        root,
        configPath: args.configPath,
        profile: args.profile ?? config.profileName,
        scope: args.scope.mode === "all" ? "workspace" : args.scope.mode,
        files: args.scope.files ?? [],
        crateName: args.scope.crateName,
        base: args.scope.base,
        head: args.scope.head,
        ruleId: args.routeRuleId,
      });
      console.log(JSON.stringify(report, null, 2));
      process.exit(0);
    }

    if (args.command === "run") {
      const report = runHarness({
        root,
        profile: args.profile,
        tool: args.runTool,
        language: args.languages?.[0],
        harness: config.harness,
        command: args.runCommand,
        runId: args.runId,
        crateName: args.crateName,
        packageName: args.packageName,
        domain: args.domain,
        tags: args.tag ? [args.tag] : undefined,
      });
      if (args.json) console.log(JSON.stringify(report, null, 2));
      else printRunReport(report);
      process.exit(report.ok ? 0 : 1);
    }

    if (args.command === "runs") {
      const report = runRunsCommand(args, root, config);
      if (args.json) console.log(JSON.stringify(report, null, 2));
      else printRunsReport(args.runsCommand, report);
      process.exit(report?.ok === false ? 1 : 0);
    }

    if (args.command === "verify") {
      const report = runEnforcerVerify({
        root,
        config,
        rawScope: args.scope,
        configPath: args.configPath,
        profile: args.profile,
        languages: args.languages,
        verifyMode: args.verifyMode,
      });
      if (args.json) console.log(JSON.stringify(report, null, 2));
      else printCheckReport(report);
      process.exit(report.ok ? 0 : 1);
    }

    if (args.command === "check") {
      const report = runEnforcerCheck({
        root,
        config,
        rawScope: args.scope,
        checkName: args.checkName,
        configPath: args.configPath,
        profile: args.profile,
        checkConfigPath: args.checkConfigPath,
        output: args.output,
        dryRun: args.dryRun,
        staged: args.staged,
        tracked: args.tracked,
        strictEmptyTestTrees: args.strictEmptyTestTrees,
      });
      if (args.json) console.log(JSON.stringify(report, null, 2));
      else printCheckReport(report);
      process.exit(report.ok ? 0 : 1);
    }

    const scope = resolveScope(root, config, args.scope);

    if (args.command === "explain") {
      const report = explainRule(args.explainRuleId);
      if (args.json) console.log(JSON.stringify(report, null, 2));
      else {
        console.log(`${report.ruleId} ${report.title}`);
        console.log(`Rule: ${report.anchor}`);
        console.log(`Fix: ${report.snippet}`);
      }
      process.exit(0);
    }

    if (args.command === "doctor") {
      const report = doctor(root, config, scope);
      if (args.json) console.log(JSON.stringify(report, null, 2));
      else printDoctor(report);
      process.exit(report.ok ? 0 : 1);
    }

    const report = runEnforcerScan({
      root,
      config,
      rawScope: args.scope,
      command: args.command,
      scanOnly: args.scanOnly,
      languages: args.languages,
      profile: args.profile,
    });
    if (args.json) console.log(JSON.stringify(report, null, 2));
    else printHumanReport(report);
    process.exit(report.ok ? 0 : 1);
  } catch (error) {
    console.error(
      `Ocentra Enforcer internal error: ${error instanceof Error ? error.message : String(error)}`,
    );
    process.exit(2);
  }
}

function runArchitectureCli(tokens) {
  const subcommand = tokens[0];
  if (subcommand !== "check") {
    throw new Error("usage: ocentra-enforcer architecture check --language rust --scope <files|diff|all>");
  }
  const args = {
    root: process.cwd(),
    language: "rust",
    scopeName: "files",
    files: [],
    base: null,
    head: null,
    configPath: null,
    profile: null,
    json: false,
  };
  for (let index = 1; index < tokens.length; index += 1) {
    const token = tokens[index];
    if (token === "--root") args.root = tokens[++index];
    else if (token === "--config") args.configPath = tokens[++index];
    else if (token === "--profile") args.profile = tokens[++index];
    else if (token === "--language") args.language = tokens[++index];
    else if (token === "--scope") args.scopeName = tokens[++index];
    else if (token === "--base") args.base = tokens[++index];
    else if (token === "--head") args.head = tokens[++index];
    else if (token === "--all" || token === "--workspace") args.scopeName = "all";
    else if (token === "--json") args.json = true;
    else if (token === "--files") {
      args.scopeName = "files";
      while (tokens[index + 1] && !tokens[index + 1].startsWith("-")) {
        args.files.push(tokens[++index]);
      }
    } else if (token.startsWith("-")) {
      throw new Error(`Unknown architecture argument: ${token}`);
    } else {
      args.files.push(token);
    }
  }
  if (args.language !== "rust") {
    throw new Error("architecture check currently supports --language rust");
  }
  let rawScope;
  if (args.scopeName === "all" || args.scopeName === "workspace") {
    rawScope = { mode: "all" };
  } else if (args.scopeName === "diff") {
    if (!args.base || !args.head) {
      throw new Error("architecture diff scope requires --base <sha> --head <sha>");
    }
    rawScope = { mode: "diff", base: args.base, head: args.head };
  } else {
    rawScope = { mode: "files", files: args.files };
  }
  return {
    json: args.json,
    result: runEnforcerCheck({
      root: args.root,
      configPath: args.configPath,
      profile: args.profile,
      rawScope,
      checkName: "reexports",
    }),
  };
}
