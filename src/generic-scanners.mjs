import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import {
  collectFiles,
  normalizeRel,
  repoAbsolute,
  uniqueSorted,
} from "./path-utils.mjs";
import {
  SOURCE_POLICY_RULES,
  scanAdditionalCommonFile,
  scanAdditionalTypeScriptFile,
} from "./source-policy-scanners.mjs";

export const GENERIC_RULES = {
  "TS-1.1": {
    title: "TypeScript/JavaScript re-exports are forbidden",
    snippet:
      "Import from the owning module at the call site; do not add barrel exports or re-export shims.",
  },
  "TS-2.1": {
    title: "TypeScript/JavaScript suppression comments are forbidden",
    snippet:
      "Fix the type/lint issue or add a project policy exception rather than suppressing validation.",
  },
  "TS-3.1": {
    title: "Skipped/focused JavaScript tests are forbidden",
    snippet:
      "Remove .skip/.only/.todo and keep every checked-in test executable.",
  },
  "TS-5.1": {
    title: "TypeScript compiler checks must pass",
    snippet:
      "Run tsc --noEmit through the Enforcer harness and fix compiler diagnostics.",
  },
  "TS-5.2": {
    title: "ESLint JSON diagnostics must pass",
    snippet:
      "Run ESLint with --format json through the Enforcer harness and fix lint diagnostics.",
  },
  "PY-1.1": {
    title: "Python lint suppression comments are forbidden",
    snippet:
      "Fix the Ruff/Pylint issue or move the exception into reviewed policy.",
  },
  "PY-1.2": {
    title: "Python type-ignore comments are forbidden",
    snippet: "Fix the type issue or model the dynamic boundary explicitly.",
  },
  "PY-1.3": {
    title: "Python naked domain string aliases are forbidden",
    snippet:
      "Use typing.NewType, a validated dataclass/value object, or a project-owned schema boundary instead of Alias = str.",
  },
  "PY-4.1": {
    title: "Python Any is forbidden",
    snippet:
      "Use precise types, protocols, or validated boundary models instead of Any.",
  },
  "PY-4.2": {
    title: "Python functions must be typed",
    snippet:
      "Add parameter and return annotations so Pyright/mypy can enforce contracts.",
  },
  "PY-4.3": {
    title: "Python return annotations are required",
    snippet:
      "Add explicit return annotations to every function so contracts are visible to type checkers.",
  },
  "PY-4.4": {
    title: "Python dict[str, Any] domain APIs are forbidden",
    snippet:
      "Use precise typed mappings or decoded value objects instead of dict[str, Any].",
  },
  "PY-4.5": {
    title: "Python raw str ID aliases are forbidden",
    snippet:
      "Use NewType, frozen dataclasses, or schema-branded values for domain identifiers.",
  },
  "PY-4.6": {
    title: "Python raw domain parameters are forbidden",
    snippet:
      "Replace ID/count/state raw str/int/bool parameters with typed domain values.",
  },
  "PY-4.7": {
    title: "TypedDict domain models are forbidden",
    snippet:
      "Keep TypedDict at boundaries and use value objects or dataclasses for domain models.",
  },
  "PY-4.8": {
    title: "Pydantic domain models are forbidden by default",
    snippet:
      "Use boundary decoders plus domain value objects instead of BaseModel in domain code.",
  },
  "PY-4.9": {
    title: "Python Optional field soup is forbidden",
    snippet:
      "Replace clusters of Optional fields with explicit states or value objects.",
  },
  "PY-4.10": {
    title: "Python mutable default arguments are forbidden",
    snippet:
      "Use None plus explicit construction inside the function body.",
  },
  "PY-4.11": {
    title: "Broad Python exception handlers are forbidden",
    snippet:
      "Catch specific exception types and model the failure path.",
  },
  "PY-4.12": {
    title: "Bare Python except handlers are forbidden",
    snippet: "Catch specific exception types; never use bare except.",
  },
  "PY-4.13": {
    title: "Python except pass is forbidden",
    snippet:
      "Handle the exception, return a modeled error, or re-raise with context instead of pass.",
  },
  "PY-4.14": {
    title: "Python print debugging is forbidden",
    snippet:
      "Use project logging domains or structured diagnostics instead of print.",
  },
  "PY-4.15": {
    title: "Python runtime asserts are forbidden",
    snippet:
      "Use explicit validation and modeled errors instead of assert in production code.",
  },
  "PY-4.16": {
    title: "Python dynamic code execution is forbidden",
    snippet: "Remove eval, exec, and compile from project source.",
  },
  "PY-4.17": {
    title: "Python subprocess shell=True is forbidden",
    snippet:
      "Pass argv arrays with shell disabled and validate every boundary input.",
  },
  "PY-4.18": {
    title: "Python os.system is forbidden",
    snippet:
      "Use subprocess with argv arrays and shell disabled instead of os.system.",
  },
  "PY-4.19": {
    title: "Python pickle.loads is forbidden",
    snippet:
      "Use safe structured formats and schema decoding instead of untrusted pickle loading.",
  },
  "PY-4.20": {
    title: "Python yaml.load requires a safe loader",
    snippet:
      "Use yaml.safe_load or pass SafeLoader explicitly.",
  },
  "PY-4.21": {
    title: "Python global mutable state is forbidden",
    snippet:
      "Move mutable state behind explicit owners or dependency-injected services.",
  },
  "PY-4.22": {
    title: "Python dynamic imports are forbidden in domain code",
    snippet:
      "Use static imports at module boundaries instead of importlib or __import__.",
  },
  "PY-4.23": {
    title: "Python naive datetime calls are forbidden",
    snippet:
      "Use timezone-aware datetime values instead of datetime.now() or utcnow().",
  },
  "PY-4.24": {
    title: "Python sleep is forbidden in async code and tests",
    snippet:
      "Use controlled clocks, awaits on events, or deterministic polling instead of time.sleep.",
  },
  "PY-4.25": {
    title: "Python HTTP calls must set timeouts",
    snippet:
      "Pass an explicit timeout to requests calls so tests and services cannot hang.",
  },
  "PY-4.26": {
    title: "Python asyncio tasks must be tracked",
    snippet:
      "Store, await, or supervise create_task results instead of fire-and-forget tasks.",
  },
  "PY-4.27": {
    title: "Python coroutine calls must be awaited or returned",
    snippet:
      "Await coroutine calls or return them to the caller instead of swallowing them.",
  },
  "PY-4.28": {
    title: "Python parent-relative imports are forbidden",
    snippet:
      "Use package-owned absolute imports rather than escaping boundaries with from .. imports.",
  },
  "PY-4.29": {
    title: "Python wildcard imports are forbidden",
    snippet:
      "Import explicit names from owning modules instead of using import *.",
  },
  "PY-4.30": {
    title: "Python from-module wildcard imports are forbidden",
    snippet:
      "Replace from module import * with explicit imports.",
  },
  "PY-4.31": {
    title: "Python dumping-ground module names are forbidden",
    snippet:
      "Rename utils.py, helpers.py, and common.py to domain-specific modules.",
  },
  "PY-4.32": {
    title: "Python dataclass value objects must be frozen and slotted",
    snippet:
      "Use @dataclass(frozen=True, slots=True) for domain value objects.",
  },
  "PY-4.33": {
    title: "Python tuple domain records are forbidden",
    snippet:
      "Use named value objects instead of NamedTuple or tuple positional records.",
  },
  "PY-4.34": {
    title: "Python raw JSON dict domain inputs are forbidden",
    snippet:
      "Decode JSON into typed boundary models before entering domain code.",
  },
  "PY-4.35": {
    title: "Python environment reads must stay in config boundaries",
    snippet:
      "Read os.environ in config modules and pass typed config inward.",
  },
  "PY-6.1": {
    title: "Python skipped/xfail tests are forbidden without waiver",
    snippet:
      "Remove skip, skipif, and xfail markers or move an expiring waiver into policy.",
  },
  "PY-6.2": {
    title: "Weak Python assertions are forbidden",
    snippet:
      "Assert concrete values and modeled outcomes instead of bare truthiness or non-null checks.",
  },
  "PY-6.3": {
    title: "Empty Python tests are forbidden",
    snippet: "Every test must execute meaningful behavior and assertions.",
  },
  "PY-6.4": {
    title: "Python tests must assert behavior",
    snippet: "Add concrete assertions to tests instead of only exercising code.",
  },
  "PY-6.5": {
    title: "Python monkeypatch and mocks are forbidden by default",
    snippet:
      "Use real local collaborators or reviewed test seams instead of monkeypatch and mock.",
  },
  "PY-6.6": {
    title: "Python unit tests must not use the network",
    snippet:
      "Keep unit tests local and deterministic; move network checks to explicit integration proof.",
  },
  "PY-6.7": {
    title: "Python sleep-based tests are forbidden",
    snippet:
      "Use deterministic clocks, events, or bounded polling helpers instead of sleep.",
  },
  "PY-2.1": {
    title: "Skipped/focused Python tests are forbidden",
    snippet:
      "Remove skip/focus markers or move the exception into reviewed test policy.",
  },
  "PY-3.1": {
    title: "Ruff diagnostics must pass",
    snippet:
      "Run Ruff with JSON output through the Enforcer harness and fix diagnostics.",
  },
  "PY-3.2": {
    title: "Python type-check diagnostics must pass",
    snippet:
      "Run Pyright or mypy through the Enforcer harness and fix diagnostics.",
  },
  "SEC-1.1": {
    title: "Inline secrets are forbidden",
    snippet:
      "Move credentials into a secret manager or local ignored environment file.",
  },
  "GEN-1.1": {
    title: "Generated artifacts must not be committed as source",
    snippet:
      "Generate artifacts in CI/build output, not in tracked source folders.",
  },
  "TEST-1.2": {
    title: "Weak assertions are forbidden",
    snippet:
      "Assert concrete behavior and values instead of existence, truthiness, or broad matcher placeholders.",
  },
  "TEST-1.3": {
    title: "Hidden, focused, or ignored tests are forbidden",
    snippet:
      "Remove focused, skipped, todo, ignored, or fixme markers before claiming test coverage.",
  },
  "SRC-1.2": {
    title: "Placeholder implementation markers are forbidden",
    snippet:
      "Replace TODO, placeholder, not-implemented, debug-print, and temporary code with real behavior before landing.",
  },
  "DOC-1.1": {
    title: "Public API documentation is recommended",
    snippet:
      "Add a short rustdoc/JSDoc/docstring for exported or public API, or disable/downgrade this advisory in project policy.",
  },
  "HAR-1.1": {
    title: "Harnessed command failed",
    snippet:
      "Read compact diagnostics first, then inspect bounded raw artifacts only if needed.",
  },
  ...SOURCE_POLICY_RULES,
};

Object.assign(GENERIC_RULES, {
  "SEC-2.1": {
    title: "GitHub tokens are forbidden",
    snippet: "Move GitHub tokens to a secret manager and keep only placeholders in examples.",
  },
  "SEC-2.2": {
    title: "AWS access keys are forbidden",
    snippet: "Remove AWS keys from source and rotate exposed credentials.",
  },
  "SEC-2.3": {
    title: "Google service account JSON is forbidden",
    snippet: "Keep service account JSON in secret storage, not source or fixtures.",
  },
  "SEC-2.4": {
    title: "Azure credentials are forbidden",
    snippet: "Move Azure tenant/client secrets to managed secret storage.",
  },
  "SEC-2.5": {
    title: "Slack and Discord tokens are forbidden",
    snippet: "Remove chat tokens and use local ignored environment values.",
  },
  "SEC-2.6": {
    title: "JWT-looking secrets are forbidden",
    snippet: "Do not commit JWT tokens or bearer credentials.",
  },
  "SEC-2.7": {
    title: "Private key blocks are forbidden",
    snippet: "Remove private keys from source and rotate the exposed key material.",
  },
  "SEC-2.8": {
    title: "Package registry tokens are forbidden",
    snippet: "Keep npm, PyPI, and Cargo tokens in CI secret storage.",
  },
  "SEC-2.9": {
    title: "Stripe keys are forbidden",
    snippet: "Remove Stripe keys from source and rotate exposed values.",
  },
  "SEC-2.10": {
    title: "High-entropy secret assignments are forbidden",
    snippet: "Replace real-looking high-entropy values with obvious placeholders.",
  },
  "SEC-2.11": {
    title: ".env.example may contain placeholders only",
    snippet: "Use placeholder values such as example, changeme, or <TOKEN>.",
  },
  "SEC-2.12": {
    title: ".env.template may contain placeholders only",
    snippet: "Do not put real-looking secrets in template environment files.",
  },
  "SEC-2.13": {
    title: "Secrets are forbidden in snapshots",
    snippet: "Redact snapshots before committing them.",
  },
  "SEC-2.14": {
    title: "Fixture secrets require fake markers",
    snippet: "Mark fixture credentials as fake and keep them non-secret-looking.",
  },
  "SEC-2.15": {
    title: "Secret diagnostics must redact matched values",
    snippet: "Report secret classes and locations, never raw secret values.",
  },
  "SEC-2.16": {
    title: "Secret scanners must emit SARIF",
    snippet: "Configure secret scanners to emit SARIF or structured output before CI consumes findings.",
  },
  "SEC-2.17": {
    title: "Gitleaks findings must be normalized",
    snippet: "Run Gitleaks through Enforcer or emit SARIF so findings map to SEC rule IDs.",
  },
  "SEC-2.18": {
    title: "TruffleHog findings must be normalized",
    snippet: "Run TruffleHog through Enforcer or emit JSON so findings map to SEC rule IDs.",
  },
  "SEC-2.19": {
    title: "Committed SSH keys are forbidden",
    snippet: "Remove SSH key files and rotate exposed keys.",
  },
  "SEC-2.20": {
    title: "Mobile secret config files are forbidden",
    snippet: "Do not commit GoogleService-Info.plist or google-services.json.",
  },
  "GEN-2.1": {
    title: "Generated directories require ignore policy",
    snippet: "Keep generated outputs ignored unless the profile explicitly tracks them.",
  },
  "GEN-2.2": {
    title: "Generated files require source owner provenance",
    snippet: "Stamp generated files with generator and source-owner provenance.",
  },
  "GEN-2.3": {
    title: "Generated files cannot be edited manually",
    snippet: "Regenerate artifacts from their source owner instead of hand-editing output.",
  },
  "GEN-2.4": {
    title: "Generated contract artifacts require source hash",
    snippet: "Regenerate contract artifacts with a source schema or contract hash.",
  },
  "GEN-2.5": {
    title: "Generated schema files must be reproducible",
    snippet: "Stamp generated schemas with deterministic generator and schema hash metadata.",
  },
  "GEN-2.6": {
    title: "Runtime output directories cannot be tracked",
    snippet: "Keep coverage, playwright-report, test-results, and output directories out of source.",
  },
  "GEN-2.7": {
    title: "Generated files cannot be single source of truth",
    snippet: "Move ownership to source schemas/templates and treat generated files as outputs.",
  },
  "GEN-2.8": {
    title: "Generated code cannot contain suppressions",
    snippet: "Fix the generator or generated source policy instead of emitting bypass comments.",
  },
  "GEN-2.9": {
    title: "Generated code cannot live in domain modules",
    snippet: "Move generated artifacts under configured generated/boundary folders.",
  },
  "GEN-2.10": {
    title: "Generated snapshots must be stable",
    snippet: "Redact timestamps, random IDs, and machine-local paths from generated snapshots.",
  },
  "SRC-2.8": {
    title: "Dumping-ground source filenames are forbidden",
    snippet: "Rename utils, helpers, common, misc, shared, and stuff to domain-specific names.",
  },
  "SRC-2.9": {
    title: "Temporary code comments are forbidden",
    snippet: "Replace temporary, for now, hack, and quick fix markers with real implementation.",
  },
  "SRC-2.10": {
    title: "Placeholder implementation markers are forbidden",
    snippet: "Remove TODO, FIXME, placeholder, scaffold, and not-implemented code.",
  },
  "SRC-2.11": {
    title: "Copied huge source blocks are forbidden",
    snippet: "Extract shared logic or generate code instead of pasting large copied blocks.",
  },
  "SRC-2.12": {
    title: "Duplicate function names in one module are forbidden",
    snippet: "Merge duplicate functions or give each behavior one owned name.",
  },
  "SRC-2.13": {
    title: "Mixed responsibility source files are forbidden",
    snippet: "Split files that import UI, domain, data, network, and infrastructure layers together.",
  },
  "SRC-2.14": {
    title: "Internal modules cannot expose public API",
    snippet: "Keep internal modules private and expose reviewed facades from public boundaries.",
  },
  "SRC-2.15": {
    title: "Dependency direction violations are forbidden",
    snippet: "Keep lower-level domain modules independent from app, UI, adapter, and infra layers.",
  },
  "PY-5.1": {
    title: "pyproject.toml is required for Python projects",
    snippet: "Keep Python tool policy in pyproject.toml.",
  },
  "PY-5.2": {
    title: "Ruff configuration is required",
    snippet: "Add [tool.ruff] configuration to pyproject.toml.",
  },
  "PY-5.3": {
    title: "Pyright or mypy configuration is required",
    snippet: "Add [tool.pyright] or [tool.mypy] strict type-check configuration.",
  },
  "PY-5.4": {
    title: "Python type checker strict mode is required",
    snippet: "Enable strict Pyright or mypy settings.",
  },
  "PY-5.5": {
    title: "Ruff diagnostics must be structured",
    snippet: "Run Ruff with JSON output through the Enforcer harness.",
  },
  "PY-5.6": {
    title: "Python type diagnostics must be structured",
    snippet: "Run Pyright with --outputjson or mypy with structured output through the harness.",
  },
  "PY-5.7": {
    title: "Python lockfile is required",
    snippet: "Commit uv.lock, poetry.lock, or the configured Python lockfile.",
  },
  "PY-5.8": {
    title: "Unpinned Python requirements are forbidden",
    snippet: "Pin requirements with exact versions or hashes.",
  },
  "PY-5.9": {
    title: "Python git dependencies are forbidden",
    snippet: "Use published pinned packages instead of git dependencies.",
  },
  "PY-5.10": {
    title: "Python local path dependencies require waiver",
    snippet: "Use published packages or explicit workspace policy instead of path dependencies.",
  },
  "PY-6.8": {
    title: "Python validators require invalid-input tests",
    snippet: "Add negative tests for validator/parser rejection paths.",
  },
  "PY-6.9": {
    title: "Python exception paths require tests",
    snippet: "Assert raised exceptions with pytest.raises instead of leaving exception paths unproved.",
  },
  "PY-6.10": {
    title: "Python parsers and normalizers require property tests",
    snippet: "Use Hypothesis/given tests for parser and normalizer behavior.",
  },
  "BOUND-1.1": {
    title: "Boundary modules require invariant documentation",
    snippet: "Add BOUNDARY-INVARIANT documentation to every boundary module.",
  },
  "BOUND-1.2": {
    title: "Raw boundary input must be converted",
    snippet: "Convert raw input into domain types at the boundary before passing inward.",
  },
  "BOUND-1.3": {
    title: "Boundary modules cannot contain domain decisions",
    snippet: "Move domain decisions out of boundary/adapter files and into domain owners.",
  },
  "BOUND-1.4": {
    title: "Domain modules cannot import boundary modules",
    snippet: "Invert the dependency: boundary/adapters may call domain, not the reverse.",
  },
  "BOUND-1.5": {
    title: "Boundary modules require negative tests",
    snippet: "Add invalid-input tests for boundary decoders and converters.",
  },
  "BOUND-1.6": {
    title: "Boundary raw type count is budgeted",
    snippet: "Keep raw DTO/input type count small and convert to domain types quickly.",
  },
  "BOUND-1.7": {
    title: "Boundary glob additions require waiver",
    snippet: "Add an explicit waiver or owner note for new boundary glob expansions.",
  },
  "BOUND-1.8": {
    title: "Boundary utility filenames are forbidden",
    snippet: "Use domain-specific boundary file names instead of utils or helpers.",
  },
  "BOUND-1.9": {
    title: "Boundary DTOs cannot leak into domain signatures",
    snippet: "Return domain types from boundary conversion functions, not raw DTO/input shapes.",
  },
  "BOUND-1.10": {
    title: "Boundary conversion functions return typed errors",
    snippet: "Return Result/Either/typed error shapes from boundary conversion functions.",
  },
  "ARCH-1.1": {
    title: "Domain cannot import infrastructure",
    snippet: "Keep infrastructure dependencies outside domain modules.",
  },
  "ARCH-1.2": {
    title: "Domain cannot import UI",
    snippet: "Keep UI dependencies outside domain modules.",
  },
  "ARCH-1.3": {
    title: "Domain cannot import database clients",
    snippet: "Pass persistence through ports/adapters instead of importing DB clients in domain.",
  },
  "ARCH-1.4": {
    title: "Domain cannot import HTTP clients or servers",
    snippet: "Keep HTTP transport code at boundaries and pass domain values inward.",
  },
  "ARCH-1.5": {
    title: "Adapters cannot be imported by domain",
    snippet: "Adapters depend on domain; domain must not depend on adapters.",
  },
  "ARCH-1.6": {
    title: "Generated code cannot depend on domain internals",
    snippet: "Generated outputs may use public contracts, not domain/internal modules.",
  },
  "ARCH-1.7": {
    title: "Production source cannot import test support",
    snippet: "Move test helpers under tests and keep production source independent.",
  },
  "ARCH-1.8": {
    title: "CLI/main depends on application boundary only",
    snippet: "Keep CLI/main imports routed through app/application boundary modules.",
  },
  "ARCH-1.9": {
    title: "Circular imports are forbidden",
    snippet: "Break circular module references by extracting an owner or port.",
  },
  "ARCH-1.10": {
    title: "Import boundary config requires tests",
    snippet: "Add tests for importBoundaryPolicies before relying on them.",
  },
  "ARCH-1.11": {
    title: "Public API surface is budgeted",
    snippet: "Reduce public exports or split the facade into stable owned APIs.",
  },
  "ARCH-1.12": {
    title: "Barrel/facade files require explicit profile",
    snippet: "Do not add barrel/facade exports unless profile policy explicitly permits them.",
  },
  "ARCH-1.13": {
    title: "Public facade can expose only stable API",
    snippet: "Remove internal, experimental, and unstable exports from public facades.",
  },
  "ARCH-1.14": {
    title: "Internal modules cannot leak through public types",
    snippet: "Do not export public signatures that reference internal modules or raw internal types.",
  },
  "ARCH-1.15": {
    title: "Package and crate ownership files are required",
    snippet: "Add OWNERS, CODEOWNERS, or an ownership README for packages and crates.",
  },
});

const TS_EXTENSIONS = new Set([
  ".ts",
  ".tsx",
  ".js",
  ".jsx",
  ".mjs",
  ".cjs",
  ".mts",
  ".cts",
]);
const PY_EXTENSIONS = new Set([".py"]);
const COMMON_TEXT_EXTENSIONS = new Set([
  ".md",
  ".mdx",
  ".txt",
  ".json",
  ".jsonc",
  ".toml",
  ".yaml",
  ".yml",
  ".env",
]);
const SECRET_RE =
  /\b(?:[A-Z0-9_/-]*(?:api[_-]?key|secret|token|password|private[_-]?key))\b\s*[:=]\s*["'][A-Za-z0-9_./+=:@-]{16,}["']/iu;
const OPENAI_KEY_RE = /\bsk-(?:proj-)?[A-Za-z0-9_-]{20,}\b/u;
const COMMON_SECRET_RULES = [
  { ruleId: "SEC-2.1", pattern: /\bgh[pousr]_[A-Za-z0-9_]{20,}\b/u, detail: "GitHub token found." },
  { ruleId: "SEC-2.2", pattern: /\bAKIA[0-9A-Z]{16}\b/u, detail: "AWS access key found." },
  { ruleId: "SEC-2.3", pattern: /"type"\s*:\s*"service_account"|"private_key_id"\s*:/u, detail: "Google service account JSON marker found." },
  { ruleId: "SEC-2.4", pattern: /\bAZURE_(?:CLIENT_SECRET|TENANT_ID|CLIENT_ID)\b\s*[:=]/iu, detail: "Azure credential assignment found." },
  { ruleId: "SEC-2.5", pattern: /\bxox[baprs]-[A-Za-z0-9-]{10,}\b|discord(?:app)?\.[A-Za-z0-9_-]{20,}/iu, detail: "Slack or Discord token found." },
  { ruleId: "SEC-2.6", pattern: /\beyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\b/u, detail: "JWT-looking token found." },
  { ruleId: "SEC-2.7", pattern: /-----BEGIN (?:RSA |OPENSSH |EC |DSA |)?PRIVATE KEY-----/u, detail: "private key block found." },
  { ruleId: "SEC-2.8", pattern: /\b(?:npm_[A-Za-z0-9]{20,}|pypi-[A-Za-z0-9_-]{20,}|CARGO_REGISTRY_TOKEN\s*=)/u, detail: "package registry token found." },
  { ruleId: "SEC-2.9", pattern: /\b(?:sk|pk)_(?:live|test)_[A-Za-z0-9]{20,}\b/u, detail: "Stripe key found." },
  { ruleId: "SEC-2.10", pattern: /\b(?:secret|token|password|key)\b\s*[:=]\s*["'][A-Za-z0-9+/=_-]{32,}["']/iu, detail: "high-entropy secret assignment found." },
];
const ENV_PLACEHOLDER_ALLOWED = /(?:example|placeholder|changeme|replace_me|dummy|fake|test|<[^>]+>|\$\{[^}]+\})/iu;
const SSH_KEY_PATH_RE = /(?:^|\/)(?:id_rsa|id_ed25519|id_ecdsa|id_dsa)(?:\.pub)?$/iu;
const MOBILE_SECRET_CONFIG_RE = /(?:^|\/)(?:google-services\.json|GoogleService-Info\.plist)$/u;
const GENERATED_OUTPUT_DIR_RE = /^(?:coverage|playwright-report|test-results|output)(?:\/|$)/iu;
const GENERATED_DOMAIN_PATH_RE = /(?:^|\/)(?:domain|domains|core)\/.*(?:generated|auto-generated)|(?:^|\/)generated\/.*(?:domain|domains)\//iu;
const GENERATED_SUPPRESSION_RE = /(?:@ts-(?:ignore|expect-error|nocheck)|eslint-disable|allow\(|noqa|type:\s*ignore)/iu;
const GENERATED_SNAPSHOT_VOLATILE_RE = /(?:\.snap$|snapshot).*?(?:\d{4}-\d{2}-\d{2}T|[0-9a-f]{8}-[0-9a-f]{4}|random|uuid|timestamp)/iu;
const BAD_SOURCE_BASENAME_RE = /^(?:utils?|helpers?|common|misc|shared|stuff)\.(?:ts|tsx|js|jsx|mjs|cjs|rs|py)$/iu;
const GENERATED_PATH_RE = /(?:^|\/)generated(?:\/|$)|(?:generated|auto-generated|contracts?|schemas?)\.(?:ts|tsx|js|json|rs|py)$/iu;
const GENERATED_PROVENANCE_RE = /(?:@generated|<auto-generated>|generated by|source(?:Owner| owner|):|generator:)/iu;
const GENERATED_HASH_RE = /(?:sourceHash|schemaHash|contractHash|templateHash|sha256)\s*[:=]/iu;
const COPIED_BLOCK_RE = /\b(?:copied from|copy-pasted|copy pasted|BEGIN COPIED BLOCK|END COPIED BLOCK)\b/iu;
const BOUNDARY_PATH_RE = /(?:^|\/)(?:boundary|boundaries|adapter|adapters|codec|codecs|decoder|decoders|transport)(?:\/|\.|-)/iu;
const DOMAIN_PATH_RE = /(?:^|\/)(?:domain|domains|core|model|models)(?:\/|$)/iu;
const FACADE_PATH_RE = /(?:^|\/)(?:index|facade|public-api|api)\.(?:ts|tsx|js|jsx|mjs|rs)$/iu;
const LAYER_IMPORTS = [
  ["ui", /from\s+["'][^"']*(?:\/ui|\/components|\/views|\/pages|react)[^"']*["']|import\s+.*react/iu],
  ["domain", /from\s+["'][^"']*(?:\/domain|\/core|\/model|\/schema)[^"']*["']/iu],
  ["data", /from\s+["'][^"']*(?:\/data|\/db|\/repo|\/repository|\/store)[^"']*["']/iu],
  ["network", /from\s+["'][^"']*(?:\/api|\/http|\/client|\/transport|axios|fetch)[^"']*["']/iu],
  ["infra", /from\s+["'][^"']*(?:\/infra|\/adapter|\/adapters|\/platform|\/fs|\/process)[^"']*["']/iu],
];
const weakAssertionPatterns = [
  { pattern: /\.toBeDefined\s*\(/u, detail: "toBeDefined() is too weak." },
  { pattern: /\.toBeTruthy\s*\(/u, detail: "toBeTruthy() is too weak." },
  { pattern: /\.toBeFalsy\s*\(/u, detail: "toBeFalsy() is too weak." },
  { pattern: /\.not\.toThrow\s*\(/u, detail: "not.toThrow() is too weak." },
  {
    pattern: /\.toMatchObject\s*\(\s*\{\s*\}\s*\)/u,
    detail: "empty toMatchObject({}) is too weak.",
  },
  {
    pattern: /expect\.anything\s*\(\s*\)/u,
    detail: "expect.anything() is too weak.",
  },
  {
    pattern: /expect\.any\s*\(\s*(?:String|Number)\s*\)/u,
    detail: "expect.any(String|Number) is too weak.",
  },
  {
    pattern: /assert!\(\s*.*\.is_some\(\).*?\)/u,
    detail: "assert!(value.is_some()) is too weak.",
  },
  {
    pattern: /assert!\(\s*.*\.is_ok\(\).*?\)/u,
    detail: "assert!(result.is_ok()) is too weak.",
  },
  {
    pattern: /assert!\(\s*.*\.is_err\(\).*?\)/u,
    detail: "assert!(result.is_err()) is too weak.",
  },
  {
    pattern: /assert!\(\s*[^)]+\.len\(\)\s*>\s*0\s*\)/u,
    detail: "length > 0 assertion is too weak.",
  },
  {
    pattern: /assert!\(\s*!\s*[^)]+\.is_empty\(\)\s*\)/u,
    detail: "!is_empty() assertion is too weak.",
  },
  {
    pattern: /assert!\(\s*[^)]+\.contains\([^)]+\)\s*\)/u,
    detail: "contains() assertion is too weak without checking meaning.",
  },
  {
    pattern: /^\s*assert\s+(?:True|False)\b/u,
    detail: "literal Python assert is too weak.",
  },
  {
    pattern: /^\s*assert\s+[^#\n]+\s+is\s+not\s+None\b/u,
    detail: "is not None assertion is too weak.",
  },
  {
    pattern: /^\s*assert\s+len\([^)]+\)\s*>\s*0\b/u,
    detail: "len(...) > 0 assertion is too weak.",
  },
];
const word = (...parts) => parts.join("");
const placeholderCommentPatterns = [
  { pattern: /\bTODO\b/u, detail: "TODO marker found in production source." },
  { pattern: /\bFIXME\b/u, detail: "FIXME marker found in production source." },
  { pattern: /\bTBD\b/u, detail: "TBD marker found in production source." },
  {
    pattern: new RegExp(String.raw`\b${word("place", "holder")}\b`, "iu"),
    detail: "placeholder marker found in production source.",
  },
  {
    pattern: new RegExp(String.raw`\b${word("st", "ub")}\b`, "iu"),
    detail: "stub marker found in production source.",
  },
  {
    pattern: new RegExp(String.raw`\b${word("fa", "ke")}\b`, "iu"),
    detail: "fake marker found in production source.",
  },
  {
    pattern: /\btemporary\b/iu,
    detail: "temporary marker found in production source.",
  },
  {
    pattern: /\bfor now\b/iu,
    detail: "for now marker found in production source.",
  },
  {
    pattern: /\bscaffold[- ]only\b/iu,
    detail: "scaffold-only marker found in production source.",
  },
];
const placeholderDirectPatterns = [
  {
    pattern: /throw new Error\(\s*['"`]not implemented['"`]\s*\)/iu,
    detail: "not implemented throw found.",
  },
  {
    pattern: /raise\s+NotImplementedError\b/u,
    detail: "NotImplementedError found.",
  },
  { pattern: /return\s+null\s+as\s+any/u, detail: "return null as any found." },
  {
    pattern: /return\s+\{\s*\}\s+as\s+any/u,
    detail: "return {} as any found.",
  },
  { pattern: /\btodo!\s*\(\s*\)/u, detail: "todo!() found." },
  { pattern: /\bunimplemented!\s*\(\s*\)/u, detail: "unimplemented!() found." },
  {
    pattern: /panic!\(\s*['"`]not implemented['"`]\s*\)/iu,
    detail: "not implemented panic found.",
  },
  { pattern: /\bdbg!\s*\(/u, detail: "dbg!() found." },
  { pattern: /\bprintln!\s*\(/u, detail: "println!() found." },
  { pattern: /\beprintln!\s*\(/u, detail: "eprintln!() found." },
  {
    pattern: /\bunreachable!\s*\(\s*\)/u,
    detail: "bare unreachable!() found.",
  },
];
const pythonNakedDomainAliasPattern =
  /^\s*([A-Z]\w*(?:Id|ID|Path|Key|Name|Hash|URL|Url|Type|Slug|Route|Label|Title|Description|Status|Version)\w*)\s*(?::\s*TypeAlias)?\s*=\s*str\s*(?:#.*)?$/u;
const pythonAnyPattern =
  /\b(?:from\s+typing\s+import\s+.*\bAny\b|import\s+typing\b|:\s*(?:typing\.)?Any\b|->\s*(?:typing\.)?Any\b|list\[(?:typing\.)?Any\]|dict\[[^\]]*(?:typing\.)?Any[^\]]*\])/u;
const pythonFunctionPattern =
  /^\s*(?:async\s+)?def\s+([A-Za-z_]\w*)\s*\(([^)]*)\)\s*(?:->\s*([^:]+))?:/u;
const pythonMutableDefaultPattern =
  /=\s*(?:\[\s*\]|\{\s*\}|set\(\s*\)|dict\(\s*\)|list\(\s*\))/u;
const pythonBroadExceptPattern = /^\s*except\s+Exception\s*(?:as\s+\w+)?\s*:/u;
const pythonBareExceptPattern = /^\s*except\s*:/u;
const pythonPrintPattern = /^\s*print\s*\(/u;
const pythonRuntimeAssertPattern = /^\s*assert\s+.+/u;
const pythonSubprocessShellPattern = /\bsubprocess\.(?:run|call|check_call|check_output|Popen)\s*\([^#\n]*\bshell\s*=\s*True/u;
const pythonWildcardImportPattern = /^\s*from\s+[\w.]+\s+import\s+\*/u;
const pythonRequestsCallPattern = /\brequests\.(?:get|post|put|patch|delete|head|options)\s*\(/u;
const pythonNaiveDatetimePattern = /\bdatetime\.(?:now|utcnow)\s*\(\s*\)/u;
const pythonDynamicExecPattern = /\b(?:eval|exec|compile)\s*\(/u;
const pythonOsSystemPattern = /\bos\.system\s*\(/u;
const pythonPickleLoadsPattern = /\bpickle\.loads\s*\(/u;
const pythonYamlUnsafeLoadPattern = /\byaml\.load\s*\((?![^)]*(?:SafeLoader|safe_load))/u;
const pythonDynamicImportPattern = /\b(?:importlib\.import_module|__import__)\s*\(/u;
const pythonSleepPattern = /\b(?:time\.sleep|asyncio\.sleep)\s*\(/u;
const pythonCreateTaskPattern = /\basyncio\.create_task\s*\(/u;
const pythonCoroutineCallPattern = /^\s*(?!await\b|return\b|async\s+with\b)(?:[A-Za-z_]\w*\.)?[A-Za-z_]\w*_async\s*\([^#\n]*\)\s*$/u;
const pythonParentRelativeImportPattern = /^\s*from\s+\.\./u;
const pythonBadModuleNamePattern = /(?:^|\/)(?:utils|helpers|common)\.py$/iu;
const pythonDataclassPattern = /^\s*@dataclass(?:\((?<args>[^)]*)\))?/u;
const pythonNamedTuplePattern = /\b(?:NamedTuple|typing\.NamedTuple|collections\.namedtuple)\b/u;
const pythonRawJsonDictPattern = /\b(?:payload|json|data|body)\s*:\s*dict\s*(?:\[|$)/iu;
const pythonEnvReadPattern = /\bos\.environ(?:\.|\[)|\bos\.getenv\s*\(/u;

export function runGenericScan({ root, scope, config, languages = [] }) {
  const activeLanguages = new Set(languages);
  const files = collectGenericScopeFiles(root, scope, config, activeLanguages);
  const violations = [];
  for (const filePath of files) {
    const ext = path.extname(filePath).toLowerCase();
    if (
      activeLanguages.has("typescript") &&
      (TS_EXTENSIONS.has(ext) || isTypeScriptConfigPath(filePath))
    ) {
      violations.push(...scanTypeScriptFile(root, filePath));
    }
    if (activeLanguages.has("python") && PY_EXTENSIONS.has(ext)) {
      violations.push(...scanPythonFile(root, filePath));
    }
    if (activeLanguages.has("common")) {
      violations.push(...scanCommonFile(root, filePath));
    }
    if (config.failFast && violations.length > 0) break;
  }
  return {
    files: files.map((file) => normalizeRel(root, file)),
    violations,
  };
}

export function collectGenericScopeFiles(root, scope, config, activeLanguages) {
  const entries =
    scope.mode === "files"
      ? (scope.files ?? [])
      : scope.mode === "crate" && scope.crateRoot
        ? [scope.crateRoot]
        : [];
  if (scope.mode === "diff") {
    const output = runGitDiff(root, scope.base, scope.head);
    if (output === "") return [];
    return uniqueSorted(
      output
        .split(/\r?\n/u)
        .map((entry) => repoAbsolute(root, entry))
        .filter(
          (file) => fs.existsSync(file) && isGenericFile(file, activeLanguages),
        ),
    );
  }
  return collectFiles(root, entries, config, (file) =>
    isGenericFile(file, activeLanguages),
  );
}

export function scanTypeScriptFile(root, filePath) {
  const rel = normalizeRel(root, filePath);
  const lines = fs.readFileSync(filePath, "utf8").split(/\r?\n/u);
  const violations = [];
  lines.forEach((line, idx) => {
    const lineNo = idx + 1;
    const comment = jsStyleCommentText(line);
    if (
      /^\s*export\s+(?:\*\s+from|\*\s+as\s+[A-Za-z_$][\w$]*\s+from|(?:type\s+)?\{[^}]*\}\s+from)/u.test(
        line,
      )
    ) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "TS-1.1",
        "Barrel-style re-export found.",
        line,
      );
    }
    if (
      /(?:\b(?:eslint-disable|biome-ignore|oxlint-disable|prettier-ignore)\b|@ts-(?:ignore|expect-error|nocheck)\b)/u.test(
        comment,
      )
    ) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "TS-2.1",
        "TypeScript/JavaScript validation suppression found.",
        line,
      );
    }
    if (
      isTestPath(rel) &&
      /\b(?:describe|it|test)\s*\.\s*(?:skip|only|todo)\s*\(/u.test(line)
    ) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "TS-3.1",
        "Skipped or focused test found.",
        line,
      );
    }
    if (
      isTestPath(rel) &&
      /\btest\s*\.\s*(?:fixme|skip|only)\s*\(/u.test(line)
    ) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "TS-3.1",
        "Playwright skipped or focused test found.",
        line,
      );
    }
    if (
      isTestPath(rel) &&
      /\bexpect\s*\(\s*(?:true|false|null|undefined)\s*\)\s*\.\s*(?:toBe|toEqual)\s*\(/u.test(
        line,
      )
    ) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "TEST-1.2",
        "literal truth assertion is too weak.",
        line,
      );
    }
  });
  violations.push(...scanAdditionalTypeScriptFile(root, filePath));
  return violations;
}

export function scanPythonFile(root, filePath) {
  const rel = normalizeRel(root, filePath);
  const lines = fs.readFileSync(filePath, "utf8").split(/\r?\n/u);
  const violations = [];
  if (pythonBadModuleNamePattern.test(rel)) {
    addViolation(
      violations,
      root,
      filePath,
      1,
      "PY-4.31",
      "dumping-ground Python module name found.",
      rel,
    );
  }
  lines.forEach((line, idx) => {
    const lineNo = idx + 1;
    const comment = hashCommentText(line);
    if (/#\s*noqa\b|\bpylint:\s*disable\b/iu.test(comment)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "PY-1.1",
        "Python lint suppression found.",
        line,
      );
    }
    if (/#\s*type:\s*ignore\b/iu.test(comment)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "PY-1.2",
        "Python type-ignore suppression found.",
        line,
      );
    }
    const nakedDomainAlias = line.match(pythonNakedDomainAliasPattern);
    if (nakedDomainAlias) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "PY-1.3",
        `naked domain string alias ${nakedDomainAlias[1]}`,
        line,
      );
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "PY-4.5",
        `raw str ID alias ${nakedDomainAlias[1]}`,
        line,
      );
    }
    if (
      isPythonTestPath(rel) &&
      /@pytest\.mark\.(?:skip|skipif|xfail|focus)|pytest\.skip\s*\(|unittest\.skip/u.test(
        line,
      )
    ) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "PY-2.1",
        "Skipped or focused Python test found.",
        line,
      );
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "PY-6.1",
        "pytest skip/xfail marker found.",
        line,
      );
    }
    if (pythonAnyPattern.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "PY-4.1",
        "Python Any usage found.",
        line,
      );
      if (/\bdict\s*\[\s*str\s*,\s*(?:typing\.)?Any\s*\]/u.test(line)) {
        addViolation(
          violations,
          root,
          filePath,
          lineNo,
          "PY-4.4",
          "dict[str, Any] domain API found.",
          line,
        );
      }
    }
    if (/\bTypedDict\b/u.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "PY-4.7",
        "TypedDict domain model found.",
        line,
      );
    }
    if (/\bBaseModel\b/u.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "PY-4.8",
        "Pydantic BaseModel domain model found.",
        line,
      );
    }
    if (/\b(?:Optional\s*\[|None\s*\|)/u.test(line) && /^\s*[A-Za-z_]\w*\s*:/u.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "PY-4.9",
        "Optional field found in domain model.",
        line,
      );
    }
    const functionMatch = pythonFunctionPattern.exec(line);
    if (functionMatch && !isPythonTestPath(rel)) {
      const params = functionMatch[2]
        .split(",")
        .map((param) => param.trim())
        .filter(Boolean)
        .filter((param) => !/^(?:self|cls)(?:\s|$)/u.test(param));
      const untypedParam = params.find((param) => !param.includes(":"));
      if (untypedParam || !functionMatch[3]) {
        addViolation(
          violations,
          root,
          filePath,
          lineNo,
          "PY-4.2",
          "Python function is missing parameter or return type annotations.",
          line,
        );
      }
      if (!functionMatch[3]) {
        addViolation(
          violations,
          root,
          filePath,
          lineNo,
          "PY-4.3",
          "Python function is missing a return annotation.",
          line,
        );
      }
      if (/(?:^|[,\s(])(?:\w+_)?(?:id|key|count|state|enabled|flag|status)\s*:\s*(?:str|int|bool)\b/iu.test(functionMatch[2])) {
        addViolation(
          violations,
          root,
          filePath,
          lineNo,
          "PY-4.6",
          "raw domain-like parameter type found.",
          line,
        );
      }
      if (pythonRawJsonDictPattern.test(functionMatch[2])) {
        addViolation(
          violations,
          root,
          filePath,
          lineNo,
          "PY-4.34",
          "raw JSON dict domain input found.",
          line,
        );
      }
      if (pythonMutableDefaultPattern.test(functionMatch[2])) {
        addViolation(
          violations,
          root,
          filePath,
          lineNo,
          "PY-4.10",
          "Mutable default argument found.",
          line,
        );
      }
    }
    if (pythonBroadExceptPattern.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "PY-4.11",
        "Broad except Exception handler found.",
        line,
      );
    }
    if (pythonBareExceptPattern.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "PY-4.12",
        "Bare except handler found.",
        line,
      );
    }
    if (pythonBroadExceptPattern.test(line) || pythonBareExceptPattern.test(line)) {
      const nextSignificant = lines
        .slice(idx + 1, idx + 4)
        .find((candidate) => candidate.trim() !== "" && !candidate.trim().startsWith("#"));
      if (/^\s*pass\s*(?:#.*)?$/u.test(nextSignificant ?? "")) {
        addViolation(
          violations,
          root,
          filePath,
          lineNo + 1,
          "PY-4.13",
          "except block contains pass.",
          nextSignificant,
        );
      }
    }
    if (pythonPrintPattern.test(line) && !isPythonTestPath(rel)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "PY-4.14",
        "print debugging found in Python source.",
        line,
      );
    }
    if (pythonRuntimeAssertPattern.test(line) && !isPythonTestPath(rel)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "PY-4.15",
        "runtime assert found in Python source.",
        line,
      );
    }
    if (pythonDynamicExecPattern.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "PY-4.16",
        "eval/exec/compile found.",
        line,
      );
    }
    if (pythonSubprocessShellPattern.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "PY-4.17",
        "subprocess shell=True found.",
        line,
      );
    }
    if (pythonOsSystemPattern.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "PY-4.18",
        "os.system found.",
        line,
      );
    }
    if (pythonPickleLoadsPattern.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "PY-4.19",
        "pickle.loads found.",
        line,
      );
    }
    if (pythonYamlUnsafeLoadPattern.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "PY-4.20",
        "yaml.load without SafeLoader found.",
        line,
      );
    }
    if (
      !isPythonTestPath(rel) &&
      /^\s*[A-Z_][A-Z0-9_]*\s*=\s*(?:\[\s*\]|\{\s*\}|set\(\s*\)|dict\(\s*\)|list\(\s*\))/u.test(line)
    ) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "PY-4.21",
        "global mutable state found.",
        line,
      );
    }
    if (pythonDynamicImportPattern.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "PY-4.22",
        "dynamic import found.",
        line,
      );
    }
    if (pythonNaiveDatetimePattern.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "PY-4.23",
        "naive datetime call found.",
        line,
      );
    }
    if (pythonSleepPattern.test(line) && (isPythonTestPath(rel) || /async\s+def/u.test(lines.slice(Math.max(0, idx - 12), idx + 1).join("\n")))) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        isPythonTestPath(rel) ? "PY-6.7" : "PY-4.24",
        isPythonTestPath(rel) ? "sleep found in Python test." : "sleep found in async Python code.",
        line,
      );
    }
    if (
      pythonRequestsCallPattern.test(line) &&
      !/\btimeout\s*=/u.test(line)
    ) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "PY-4.25",
        "requests call without timeout found.",
        line,
      );
    }
    if (pythonCreateTaskPattern.test(line) && !/^\s*(?:\w+\s*=|return\s+|await\s+)/u.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "PY-4.26",
        "asyncio.create_task result is not tracked.",
        line,
      );
    }
    if (pythonCoroutineCallPattern.test(line) && !isPythonTestPath(rel)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "PY-4.27",
        "coroutine-like call is not awaited or returned.",
        line,
      );
    }
    if (pythonParentRelativeImportPattern.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "PY-4.28",
        "parent-relative import found.",
        line,
      );
    }
    if (pythonWildcardImportPattern.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "PY-4.29",
        "wildcard import found.",
        line,
      );
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "PY-4.30",
        "from-module wildcard import found.",
        line,
      );
    }
    const dataclassMatch = line.match(pythonDataclassPattern);
    if (dataclassMatch) {
      const args = dataclassMatch.groups?.args ?? "";
      if (!/\bfrozen\s*=\s*True\b/u.test(args) || !/\bslots\s*=\s*True\b/u.test(args)) {
        addViolation(
          violations,
          root,
          filePath,
          lineNo,
          "PY-4.32",
          "dataclass lacks frozen=True and slots=True.",
          line,
        );
      }
    }
    if (pythonNamedTuplePattern.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "PY-4.33",
        "NamedTuple or tuple domain record found.",
        line,
      );
    }
    if (pythonEnvReadPattern.test(line) && !isPythonConfigBoundary(rel)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "PY-4.35",
        "environment read found outside config boundary.",
        line,
      );
    }
    if (isPythonTestPath(rel) && /\b(?:monkeypatch|unittest\.mock|mock\.|Mock\s*\(|patch\s*\()/u.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "PY-6.5",
        "monkeypatch/mock usage found in Python test.",
        line,
      );
    }
    if (isPythonTestPath(rel) && /\b(?:requests\.|httpx\.|urllib\.request|socket\.)/u.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "PY-6.6",
        "network API found in Python unit test.",
        line,
      );
    }
    if (isPythonTestPath(rel) && pythonRuntimeAssertPattern.test(line) && isWeakPythonAssert(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "PY-6.2",
        "weak Python assertion found.",
        line,
      );
    }
  });
  if (isPythonTestPath(rel)) {
    violations.push(...scanPythonTestBlocks(root, filePath, lines));
  }
  return violations;
}

export function scanCommonFile(root, filePath) {
  const rel = normalizeRel(root, filePath);
  const lines = fs.readFileSync(filePath, "utf8").split(/\r?\n/u);
  const text = lines.join("\n");
  const ext = path.extname(filePath).toLowerCase();
  const violations = [];
  if (GENERATED_PATH_RE.test(rel)) {
    if (!GENERATED_PROVENANCE_RE.test(text)) {
      addViolation(
        violations,
        root,
        filePath,
        1,
        "GEN-2.2",
        "generated file lacks source owner or generator provenance.",
        rel,
      );
    }
    if (/(?:contract|contracts)/iu.test(rel) && !GENERATED_HASH_RE.test(text)) {
      addViolation(
        violations,
        root,
        filePath,
        1,
        "GEN-2.4",
        "generated contract artifact lacks source schema hash.",
        rel,
      );
    }
    if (/(?:schema|schemas|\.json$)/iu.test(rel) && !GENERATED_HASH_RE.test(text)) {
      addViolation(
        violations,
        root,
        filePath,
        1,
        "GEN-2.5",
        "generated schema artifact lacks reproducibility hash.",
        rel,
      );
    }
    if (/\b(?:single source of truth|SOURCE_OF_TRUTH|authoritative)\b/iu.test(text)) {
      addViolation(
        violations,
        root,
        filePath,
        1,
        "GEN-2.7",
        "generated file claims to be source of truth.",
        rel,
      );
    }
  }
  if (/ocentra-enforcer\.config\.json|rust-rules\.config\.json/iu.test(rel) && /"importBoundaryPolicies"\s*:/u.test(text) && !/"architecturePolicyChecks"\s*:/u.test(text)) {
    addViolation(
      violations,
      root,
      filePath,
      1,
      "ARCH-1.10",
      "import boundary policy config lacks architecturePolicyChecks tests.",
      rel,
    );
  }
  if (/(?:^|\/)(?:package\.json|Cargo\.toml)$/u.test(rel) && !hasOwnershipFile(path.dirname(filePath))) {
    addViolation(
      violations,
      root,
      filePath,
      1,
      "ARCH-1.15",
      "package/crate has no OWNERS, CODEOWNERS, or ownership README.",
      rel,
    );
  }
  lines.forEach((line, idx) => {
    const lineNo = idx + 1;
    const comment = sourceCommentText(ext, line);
    const commandLike = isCommandLikeLine(line);
    if (commandLike && /\bgitleaks\s+(?:detect|protect|dir|git)\b/iu.test(line)) {
      if (!/\bsarif\b|--report-format\s+sarif/iu.test(line)) {
        addViolation(
          violations,
          root,
          filePath,
          lineNo,
          "SEC-2.16",
          "Gitleaks command does not emit SARIF.",
          line,
        );
      }
      if (!/\bocentra-enforcer\b|--report-format\s+sarif/iu.test(line)) {
        addViolation(
          violations,
          root,
          filePath,
          lineNo,
          "SEC-2.17",
          "Gitleaks command is not normalized through Enforcer/SARIF.",
          line,
        );
      }
    }
    if (commandLike && /\btrufflehog\b/iu.test(line) && !/--json\b|\bjson\b|\bocentra-enforcer\b/iu.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "SEC-2.18",
        "TruffleHog command is not normalized through JSON/Enforcer.",
        line,
      );
    }
    if (commandLike && /\bruff\s+check\b/iu.test(line) && !/--output-format\s+json|--format\s+json|\bocentra-enforcer\b/iu.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "PY-5.5",
        "Ruff command does not emit JSON diagnostics.",
        line,
      );
    }
    if (commandLike && /\bpyright\b/iu.test(line) && !/--outputjson\b|\bocentra-enforcer\b/iu.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "PY-5.6",
        "Pyright command does not emit JSON diagnostics.",
        line,
      );
    }
    if (commandLike && /\bmypy\b/iu.test(line) && !/--junit-xml\b|--json-report\b|\bjson\b|\bocentra-enforcer\b/iu.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "PY-5.6",
        "mypy command does not emit structured diagnostics.",
        line,
      );
    }
    if (OPENAI_KEY_RE.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "SEC-1.1",
        "OpenAI key found.",
        redactOpenAiKey(line),
      );
    }
    if (SECRET_RE.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "SEC-1.1",
        "Inline secret-like assignment found.",
        redact(line),
      );
    }
    for (const rule of COMMON_SECRET_RULES) {
      if (rule.pattern.test(line)) {
        addViolation(
          violations,
          root,
          filePath,
          lineNo,
          rule.ruleId,
          rule.detail,
          redact(line),
        );
      }
    }
    if (/\.env\.(?:example|sample)$/iu.test(rel) && !ENV_PLACEHOLDER_ALLOWED.test(line) && SECRET_RE.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "SEC-2.11",
        ".env.example contains a real-looking secret.",
        redact(line),
      );
    }
    if (/\.env\.template$/iu.test(rel) && !ENV_PLACEHOLDER_ALLOWED.test(line) && SECRET_RE.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "SEC-2.12",
        ".env.template contains a real-looking secret.",
        redact(line),
      );
    }
    if (isTestPath(rel) && COMMON_SECRET_RULES.some((rule) => rule.pattern.test(line))) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "SEC-2.13",
        "secret-looking value found in snapshot/test artifact.",
        redact(line),
      );
    }
    if (/fixtures?\//iu.test(rel) && !/\bfake\b|\bfixture\b|\bexample\b/iu.test(line) && COMMON_SECRET_RULES.some((rule) => rule.pattern.test(line))) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "SEC-2.14",
        "fixture secret lacks explicit fake marker.",
        redact(line),
      );
    }
    if (SSH_KEY_PATH_RE.test(rel)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "SEC-2.19",
        "SSH key file found in source scope.",
        rel,
      );
    }
    if (MOBILE_SECRET_CONFIG_RE.test(rel)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "SEC-2.20",
        "mobile secret config file found in source scope.",
        rel,
      );
    }
    if (
      isGeneratedMarkerSourceFile(rel, ext) &&
      /@generated|<auto-generated>|Generated by/iu.test(comment) &&
      !/\.d\.ts$/iu.test(rel)
    ) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "GEN-1.1",
        "Generated artifact marker found in tracked source scope.",
        line,
      );
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "GEN-2.3",
        "generated file marker found; regenerate instead of editing manually.",
        line,
      );
    }
    if (GENERATED_OUTPUT_DIR_RE.test(rel)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "GEN-2.6",
        "runtime output path is in source scope.",
        rel,
      );
    }
    if (/(?:^|\/)generated(?:\/|$)/iu.test(rel)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "GEN-2.1",
        "generated directory file is in source scope.",
        rel,
      );
      if (GENERATED_SUPPRESSION_RE.test(line)) {
        addViolation(
          violations,
          root,
          filePath,
          lineNo,
          "GEN-2.8",
          "generated code contains validation suppression.",
          line,
        );
      }
    }
    if (/(?:@generated|<auto-generated>|Generated by)/iu.test(comment) && GENERATED_DOMAIN_PATH_RE.test(rel)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "GEN-2.9",
        "generated code is under a domain module path.",
        rel,
      );
    }
    if (isGeneratedLikePath(rel) && GENERATED_SNAPSHOT_VOLATILE_RE.test(`${rel} ${line}`)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "GEN-2.10",
        "generated snapshot contains volatile value.",
        line,
      );
    }
    if (isTestPath(rel) || isPythonTestPath(rel)) {
      for (const rule of weakAssertionPatterns) {
        if (rule.pattern.test(line)) {
          addViolation(
            violations,
            root,
            filePath,
            lineNo,
            "TEST-1.2",
            rule.detail,
            line,
          );
        }
      }
    }
    if (
      ext === ".rs" &&
      isTestPath(rel) &&
      /#\s*\[\s*ignore\s*\]/u.test(line)
    ) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "TEST-1.3",
        "Rust #[ignore] test found.",
        line,
      );
    }
    if (isProductionSourcePath(rel, ext)) {
      for (const rule of placeholderDirectPatterns) {
        if (rule.pattern.test(line)) {
          addViolation(
            violations,
            root,
            filePath,
            lineNo,
            "SRC-1.2",
            rule.detail,
            line,
          );
          addViolation(
            violations,
            root,
            filePath,
            lineNo,
            "SRC-2.10",
            rule.detail,
            line,
          );
        }
      }
      if (comment !== "") {
        for (const rule of placeholderCommentPatterns) {
          if (rule.pattern.test(comment)) {
            addViolation(
              violations,
              root,
              filePath,
              lineNo,
              "SRC-1.2",
              rule.detail,
              line,
            );
          }
        }
      }
      if (comment !== "") {
        for (const rule of [
          { pattern: /\btemporary\b/iu, detail: "temporary comment marker found." },
          { pattern: /\bfor now\b/iu, detail: "for now comment marker found." },
          { pattern: /\bhack\b/iu, detail: "hack comment marker found." },
          { pattern: /\bquick fix\b/iu, detail: "quick fix comment marker found." },
        ]) {
          if (rule.pattern.test(comment)) {
            addViolation(
              violations,
              root,
              filePath,
              lineNo,
              "SRC-2.9",
              rule.detail,
              line,
            );
          }
        }
      }
      if (BAD_SOURCE_BASENAME_RE.test(path.basename(rel))) {
        addViolation(
          violations,
          root,
          filePath,
          lineNo,
          "SRC-2.8",
          "dumping-ground source filename found.",
          rel,
        );
      }
    }
  });
  if (isProductionSourcePath(rel, ext)) {
    violations.push(...scanSourceOwnershipPolicy(root, filePath, rel, lines));
  }
  if (path.basename(rel).toLowerCase() === "pyproject.toml") {
    violations.push(...scanPythonToolchainPolicy(root, filePath, lines));
  }
  if (path.basename(rel).toLowerCase() === "requirements.txt") {
    violations.push(...scanPythonRequirementsPolicy(root, filePath, lines));
  }
  violations.push(...scanAdditionalCommonFile(root, filePath, lines));
  violations.push(...scanDocumentationHints(root, filePath, rel, lines));
  return violations;
}

function scanSourceOwnershipPolicy(root, filePath, rel, lines) {
  const violations = [];
  const text = lines.join("\n");
  const importText = lines.filter(isImportLikeLine).join("\n");
  const boundaryFile = BOUNDARY_PATH_RE.test(rel);
  const domainFile = DOMAIN_PATH_RE.test(rel);
  const generatedFile = GENERATED_PATH_RE.test(rel);
  const facadeFile = FACADE_PATH_RE.test(rel);
  if (boundaryFile) {
    if (!/\bBOUNDARY-INVARIANT:/u.test(text)) {
      addViolation(violations, root, filePath, 1, "BOUND-1.1", "boundary file lacks BOUNDARY-INVARIANT documentation.", rel);
    }
    if (/\braw(?:Input|Dto|DTO|Payload|Body)?\b|:\s*(?:unknown|any|dict\[|Record<string,\s*unknown>)/u.test(text) && !/\b(?:toDomain|fromRaw|parse|decode|validate)\b/u.test(text)) {
      addViolation(violations, root, filePath, 1, "BOUND-1.2", "raw boundary input is not converted to a domain type.", rel);
    }
    if (/\b(?:if|switch|match)\b[\s\S]{0,120}\b(?:business|domain|role|plan|entitlement|policy)\b/iu.test(text)) {
      addViolation(violations, root, filePath, firstMatchingLine(lines, /\b(?:business|domain|role|plan|entitlement|policy)\b/iu), "BOUND-1.3", "domain decision logic found in boundary file.", rel);
    }
    if (!/\b(?:invalid|malformed|negative|reject|throws?|pytest\.raises)\b/iu.test(text)) {
      addViolation(violations, root, filePath, 1, "BOUND-1.5", "boundary file lacks negative invalid-input coverage marker.", rel);
    }
    const rawTypeCount = countTextMatches(text, /\b(?:Raw[A-Z]\w+|[A-Z]\w+(?:Dto|DTO|Payload|Body|Request))\b/g);
    if (rawTypeCount > 3) {
      addViolation(violations, root, filePath, 1, "BOUND-1.6", `boundary raw type count ${rawTypeCount} exceeds budget 3.`, rel);
    }
    if (!/\b(?:BOUNDARY-WAIVER|boundaryOwnerNote|waiverId)\b/u.test(text)) {
      addViolation(violations, root, filePath, 1, "BOUND-1.7", "boundary file lacks waiver/owner marker for boundary expansion.", rel);
    }
    if (/^(?:utils?|helpers?)\./iu.test(path.basename(rel))) {
      addViolation(violations, root, filePath, 1, "BOUND-1.8", "boundary file uses utility/helper filename.", rel);
    }
    if (/\b(?:export\s+)?(?:function|const|def|fn)\s+\w+[^){]*\([^)]*(?:Dto|DTO|Payload|Raw|Request)[^)]*\)[^{;]*(?:Dto|DTO|Payload|Raw|Request)/u.test(text)) {
      addViolation(violations, root, filePath, firstMatchingLine(lines, /(?:Dto|DTO|Payload|Raw|Request)/u), "BOUND-1.9", "boundary DTO leaks into public/domain signature.", rel);
    }
    if (/\b(?:toDomain|fromRaw|parse|decode|convert)\w*\s*\([^)]*\)\s*(?::|->)\s*(?:string|str|boolean|bool|void|unknown|any)\b/iu.test(text)) {
      addViolation(violations, root, filePath, firstMatchingLine(lines, /\b(?:toDomain|fromRaw|parse|decode|convert)/iu), "BOUND-1.10", "boundary conversion returns untyped primitive/error shape.", rel);
    }
  }
  if (domainFile && /(?:\/boundary|\/boundaries|\/transport|\/codec|\/decoder|\/adapter|\/adapters)/iu.test(importText)) {
    addViolation(violations, root, filePath, firstMatchingLine(lines, /(?:^\s*import\b|^\s*export\b.*\bfrom\b|^\s*(?:const|let|var)\s+\w+\s*=\s*require\(|^\s*use\s+)/u), "BOUND-1.4", "domain file imports boundary/adapter module.", rel);
  }
  if ((rawConfigBoundaryText(text) || /rawTypeBoundaryGlobs/u.test(text)) && !/\b(?:boundaryOwnerNote|waiverId|BOUNDARY-WAIVER)\b/u.test(text)) {
    addViolation(violations, root, filePath, 1, "BOUND-1.7", "boundary glob addition lacks waiver or owner note.", rel);
  }

  if (COPIED_BLOCK_RE.test(text) || hasLargeRepeatedBlock(lines)) {
    addViolation(
      violations,
      root,
      filePath,
      firstMatchingLine(lines, COPIED_BLOCK_RE),
      "SRC-2.11",
      "copied or repeated source block found.",
      rel,
    );
  }

  const duplicateFunction = firstDuplicateFunctionName(lines);
  if (duplicateFunction) {
    addViolation(
      violations,
      root,
      filePath,
      duplicateFunction.line,
      "SRC-2.12",
      `duplicate function name ${duplicateFunction.name} found.`,
      duplicateFunction.source,
    );
  }

  const importedLayers = new Set();
  for (const line of lines) {
    for (const [layer, pattern] of LAYER_IMPORTS) {
      if (pattern.test(line)) importedLayers.add(layer);
    }
  }
  if (importedLayers.size >= 3) {
    addViolation(
      violations,
      root,
      filePath,
      1,
      "SRC-2.13",
      `mixed responsibility imports found: ${[...importedLayers].sort().join(", ")}`,
      rel,
    );
  }

  if (/(?:^|\/)internal(?:\/|$)/iu.test(rel) && /\b(?:export\s+|pub\s+)/u.test(text)) {
    addViolation(
      violations,
      root,
      filePath,
      firstMatchingLine(lines, /\b(?:export\s+|pub\s+)/u),
      "SRC-2.14",
      "internal module exposes public API.",
      rel,
    );
  }

  if (/(?:^|\/)(?:domain|core|model|models)(?:\/|$)/iu.test(rel) && /from\s+["'][^"']*(?:\/apps?|\/ui|\/components|\/adapters?|\/infra|\/platform)[^"']*["']|use\s+crate::(?:app|ui|adapter|infra|platform)::/iu.test(text)) {
    addViolation(
      violations,
      root,
      filePath,
      firstMatchingLine(lines, /(?:\/apps?|\/ui|\/components|\/adapters?|\/infra|\/platform|crate::(?:app|ui|adapter|infra|platform)::)/iu),
      "SRC-2.15",
      "domain/core module imports higher-level app, UI, adapter, or infra dependency.",
      rel,
    );
  }
  if (domainFile) {
    for (const [ruleId, pattern, detail] of [
      ["ARCH-1.1", /(?:\/infra|\/platform|node:fs|node:child_process|std::fs|std::process)/iu, "domain imports infrastructure dependency."],
      ["ARCH-1.2", /(?:\/ui|\/components|\/views|react|tsx?["'])/iu, "domain imports UI dependency."],
      ["ARCH-1.3", /(?:\/db|\/database|\/repo|prisma|typeorm|sqlx|diesel)/iu, "domain imports database dependency."],
      ["ARCH-1.4", /(?:\/http|\/api|\/server|axios|fetch|reqwest|hyper)/iu, "domain imports HTTP dependency."],
      ["ARCH-1.5", /(?:\/adapter|\/adapters)/iu, "domain imports adapter dependency."],
    ]) {
      if (pattern.test(importText)) {
        addViolation(violations, root, filePath, firstMatchingLine(lines, /(?:^\s*import\b|^\s*export\b.*\bfrom\b|^\s*(?:const|let|var)\s+\w+\s*=\s*require\(|^\s*use\s+)/u), ruleId, detail, rel);
      }
    }
  }
  if (generatedFile && /(?:\/domain\/internal|\/internal|private|unstable)/iu.test(importText)) {
    addViolation(violations, root, filePath, firstMatchingLine(lines, /(?:^\s*import\b|^\s*export\b.*\bfrom\b|^\s*(?:const|let|var)\s+\w+\s*=\s*require\(|^\s*use\s+)/u), "ARCH-1.6", "generated code depends on domain/internal module.", rel);
  }
  if (!isTestPath(rel) && /(?:\/test-support|\/tests?\/helpers|__tests__|vitest|pytest|unittest)/iu.test(importText)) {
    addViolation(violations, root, filePath, firstMatchingLine(lines, /(?:^\s*import\b|^\s*export\b.*\bfrom\b|^\s*(?:const|let|var)\s+\w+\s*=\s*require\(|^\s*use\s+)/u), "ARCH-1.7", "production source imports test support.", rel);
  }
  if (!isCoordinationVendorToolingPath(rel) && /(?:^|\/)(?:main|cli|bin)\.(?:ts|tsx|js|mjs|rs|py)$/iu.test(rel) && /(?:\/domain|\/core|\/infra|\/db)/iu.test(importText) && !/(?:\/app|\/application|\/boundary)/iu.test(importText)) {
    addViolation(violations, root, filePath, 1, "ARCH-1.8", "CLI/main imports outside application boundary.", rel);
  }
  if (/(?:circular import|cycle detected|imports itself)/iu.test(text) || importsOwnModule(rel, text)) {
    addViolation(violations, root, filePath, 1, "ARCH-1.9", "circular import marker or self-import found.", rel);
  }
  const exportCount = countTextMatches(text, /^\s*export\s+(?:class|function|const|let|var|type|interface|enum|default|\{|\*)/gmu);
  if (exportCount > 10 && !/\bPUBLIC-API-BUDGET-JUSTIFICATION:/u.test(text)) {
    addViolation(violations, root, filePath, 1, "ARCH-1.11", `public export count ${exportCount} exceeds budget 10.`, rel);
  }
  if (facadeFile && /^\s*export\s+(?:\*|\{[^}]+\}\s+from)/mu.test(text) && !/\b(?:facadeProfile|publicFacadeAllowed|stable-api)\b/u.test(text)) {
    addViolation(violations, root, filePath, 1, "ARCH-1.12", "barrel/facade export lacks explicit profile marker.", rel);
  }
  if (facadeFile && /\b(?:internal|unstable|experimental|private)\b/iu.test(text)) {
    addViolation(violations, root, filePath, firstMatchingLine(lines, /\b(?:internal|unstable|experimental|private)\b/iu), "ARCH-1.13", "public facade exports unstable/internal API.", rel);
  }
  if (/\bexport\s+(?:type|interface|class|function|const)[\s\S]{0,120}\b(?:Internal|internal|Private|private|Raw[A-Z]\w+)/u.test(text)) {
    addViolation(violations, root, filePath, firstMatchingLine(lines, /\bexport\s+(?:type|interface|class|function|const)/u), "ARCH-1.14", "public API leaks internal/raw type.", rel);
  }

  return violations;
}

function scanPythonToolchainPolicy(root, filePath, lines) {
  const violations = [];
  const text = lines.join("\n");
  const dir = path.dirname(filePath);
  if (!/\[tool\.ruff(?:\.|])/.test(text)) {
    addViolation(violations, root, filePath, 1, "PY-5.2", "pyproject.toml is missing Ruff configuration.", "pyproject.toml");
  }
  const hasPyright = /\[tool\.pyright]/.test(text);
  const hasMypy = /\[tool\.mypy]/.test(text);
  if (!hasPyright && !hasMypy) {
    addViolation(violations, root, filePath, 1, "PY-5.3", "pyproject.toml is missing Pyright or mypy configuration.", "pyproject.toml");
  }
  if ((!hasPyright && !hasMypy) || (hasPyright && !/typeCheckingMode\s*=\s*["']strict["']/.test(text)) || (hasMypy && !/strict\s*=\s*true/.test(text))) {
    addViolation(violations, root, filePath, 1, "PY-5.4", "Python type checker strict mode is not enabled.", "pyproject.toml");
  }
  if (!["uv.lock", "poetry.lock", "pdm.lock"].some((name) => fs.existsSync(path.join(dir, name)))) {
    addViolation(violations, root, filePath, 1, "PY-5.7", "Python lockfile is missing.", "pyproject.toml");
  }
  lines.forEach((line, index) => {
    if (/git\+|@\s*git(?:hub|lab)?\.com|https:\/\/github\.com/iu.test(line)) {
      addViolation(violations, root, filePath, index + 1, "PY-5.9", "Python git dependency found.", line);
    }
    if (/(?:path|file)\s*=\s*["'][^"']+["']|(?:\.\.\/|\.\/)/u.test(line)) {
      addViolation(violations, root, filePath, index + 1, "PY-5.10", "Python local path dependency found.", line);
    }
  });
  return violations;
}

function scanPythonRequirementsPolicy(root, filePath, lines) {
  const violations = [];
  if (!fs.existsSync(path.join(path.dirname(filePath), "pyproject.toml"))) {
    addViolation(violations, root, filePath, 1, "PY-5.1", "requirements.txt has no pyproject.toml owner.", "requirements.txt");
  }
  lines.forEach((line, index) => {
    const trimmed = line.trim();
    if (trimmed === "" || trimmed.startsWith("#")) return;
    if (/git\+|https:\/\/github\.com/iu.test(trimmed)) {
      addViolation(violations, root, filePath, index + 1, "PY-5.9", "Python git dependency found.", line);
      return;
    }
    if (/(?:^|-e\s+)(?:\.{1,2}\/|file:)/u.test(trimmed)) {
      addViolation(violations, root, filePath, index + 1, "PY-5.10", "Python local path dependency found.", line);
      return;
    }
    if (!/==[^=\s]+/.test(trimmed) && !/--hash=/.test(trimmed)) {
      addViolation(violations, root, filePath, index + 1, "PY-5.8", "unpinned Python requirement found.", line);
    }
  });
  return violations;
}

function scanDocumentationHints(root, filePath, rel, lines) {
  if (isTestPath(rel) || isPythonTestPath(rel) || /\.d\.ts$/iu.test(rel))
    return [];
  const ext = path.extname(filePath).toLowerCase();
  if (TS_EXTENSIONS.has(ext))
    return scanTypeScriptDocumentationHints(root, filePath, lines);
  if (PY_EXTENSIONS.has(ext))
    return scanPythonDocumentationHints(root, filePath, lines);
  if (ext === ".rs") return scanRustDocumentationHints(root, filePath, lines);
  return [];
}

function scanTypeScriptDocumentationHints(root, filePath, lines) {
  const violations = [];
  lines.forEach((line, idx) => {
    if (
      !/^\s*export\s+(?:async\s+)?(?:function|class|interface|type|const|let|var|enum)\s+[A-Za-z_$][\w$]*/u.test(
        line,
      )
    )
      return;
    if (hasLeadingDocComment(lines, idx, "/**")) return;
    addViolation(
      violations,
      root,
      filePath,
      idx + 1,
      "DOC-1.1",
      "Exported TypeScript/JavaScript API has no leading JSDoc comment.",
      line,
    );
  });
  return violations;
}

function scanPythonDocumentationHints(root, filePath, lines) {
  const violations = [];
  lines.forEach((line, idx) => {
    if (
      !/^(?:async\s+)?def\s+[A-Za-z_]\w*\s*\(|^class\s+[A-Za-z_]\w*/u.test(line)
    )
      return;
    if (hasPythonDocstringAfter(lines, idx)) return;
    addViolation(
      violations,
      root,
      filePath,
      idx + 1,
      "DOC-1.1",
      "Top-level Python function/class has no docstring.",
      line,
    );
  });
  return violations;
}

function scanRustDocumentationHints(root, filePath, lines) {
  const violations = [];
  lines.forEach((line, idx) => {
    if (
      !/^\s*pub(?:\([^)]*\)|\s+)?\s*(?:async\s+)?(?:fn|struct|enum|trait)\s+[A-Za-z_]\w*/u.test(
        line,
      )
    )
      return;
    if (
      hasLeadingDocComment(lines, idx, "///") ||
      hasLeadingDocComment(lines, idx, "#[doc")
    )
      return;
    addViolation(
      violations,
      root,
      filePath,
      idx + 1,
      "DOC-1.1",
      "Public Rust API has no leading rustdoc comment.",
      line,
    );
  });
  return violations;
}

function hasLeadingDocComment(lines, index, marker) {
  for (let cursor = index - 1; cursor >= 0; cursor -= 1) {
    const line = lines[cursor]?.trim() ?? "";
    if (line === "") continue;
    return line.startsWith(marker) || line.endsWith("*/");
  }
  return false;
}

function hasPythonDocstringAfter(lines, index) {
  for (let cursor = index + 1; cursor < lines.length; cursor += 1) {
    const line = lines[cursor]?.trim() ?? "";
    if (line === "") continue;
    return line.startsWith('"""') || line.startsWith("'''");
  }
  return false;
}

function isWeakPythonAssert(line) {
  return /^\s*assert\s+[A-Za-z_][\w.]*\s*(?:#.*)?$/u.test(line) ||
    /^\s*assert\s+[^#\n]+\s+is\s+not\s+None\b/u.test(line) ||
    /^\s*assert\s+len\([^)]+\)\s*>\s*0\b/u.test(line);
}

function isPythonConfigBoundary(rel) {
  return /(?:^|\/)(?:config|settings|env)(?:\/|_|\.py$)|(?:^|\/)(?:config|settings)\.py$/iu.test(
    rel,
  );
}

function scanPythonTestBlocks(root, filePath, lines) {
  const violations = [];
  const rel = normalizeRel(root, filePath);
  const text = lines.join("\n");
  if (/(?:validator|parser|decoder|normalizer)/iu.test(rel) && !/\b(?:invalid|malformed|bad input|reject|raises|pytest\.raises)\b/iu.test(text)) {
    addViolation(
      violations,
      root,
      filePath,
      1,
      "PY-6.8",
      "validator/parser test file lacks invalid-input coverage.",
      rel,
    );
  }
  if (/(?:exception|error|failure|raises)/iu.test(rel) && !/\bpytest\.raises\b/u.test(text)) {
    addViolation(
      violations,
      root,
      filePath,
      1,
      "PY-6.9",
      "exception-path test file lacks pytest.raises coverage.",
      rel,
    );
  }
  if (/(?:parser|normalizer)/iu.test(rel) && !/\b(?:hypothesis|@given|given\s*\()\b/u.test(text)) {
    addViolation(
      violations,
      root,
      filePath,
      1,
      "PY-6.10",
      "parser/normalizer test file lacks property-based coverage.",
      rel,
    );
  }
  for (let index = 0; index < lines.length; index += 1) {
    const line = lines[index] ?? "";
    const match = line.match(/^\s*def\s+(test_[A-Za-z_]\w*)\s*\([^)]*\)\s*(?:->\s*[^:]+)?\s*:/u);
    if (!match) continue;
    const body = [];
    for (let cursor = index + 1; cursor < lines.length; cursor += 1) {
      const candidate = lines[cursor] ?? "";
      if (/^\s*def\s+test_/u.test(candidate)) break;
      if (/^\S/u.test(candidate) && candidate.trim() !== "") break;
      body.push(candidate);
    }
    const meaningful = body.filter((entry) => {
      const trimmed = entry.trim();
      return trimmed !== "" && !trimmed.startsWith("#");
    });
    if (meaningful.length === 0 || meaningful.every((entry) => entry.trim() === "pass")) {
      addViolation(
        violations,
        root,
        filePath,
        index + 1,
        "PY-6.3",
        `empty Python test ${match[1]} found.`,
        line,
      );
      continue;
    }
    if (!meaningful.some((entry) => /^\s*assert\b/u.test(entry))) {
      addViolation(
        violations,
        root,
        filePath,
        index + 1,
        "PY-6.4",
        `Python test ${match[1]} has no assertion.`,
        line,
      );
    }
  }
  return violations;
}

function hasLargeRepeatedBlock(lines) {
  let repeated = 0;
  let previous = "";
  for (const line of lines) {
    const normalized = line.trim();
    if (normalized.length < 20) {
      repeated = 0;
      previous = normalized;
      continue;
    }
    if (normalized === previous) {
      repeated += 1;
      if (repeated >= 7) return true;
    } else {
      repeated = 0;
      previous = normalized;
    }
  }
  return false;
}

function firstDuplicateFunctionName(lines) {
  const seen = new Map();
  const pattern = /^\s*(?:export\s+|pub\s+)?(?:async\s+)?(?:function|fn|def)\s+([A-Za-z_]\w*)\b/u;
  for (let index = 0; index < lines.length; index += 1) {
    const line = lines[index] ?? "";
    const match = line.match(pattern);
    if (!match) continue;
    const name = match[1];
    if (seen.has(name)) return { name, line: index + 1, source: line };
    seen.set(name, index + 1);
  }
  return null;
}

function firstMatchingLine(lines, pattern) {
  const index = lines.findIndex((line) => pattern.test(line));
  return index < 0 ? 1 : index + 1;
}

function countTextMatches(text, pattern) {
  return [...String(text ?? "").matchAll(pattern)].length;
}

function rawConfigBoundaryText(text) {
  return /(?:rawTypeBoundaryGlobs|facadeFileGlobs|runtimeStringOwnerGlobs|importBoundaryPolicies)/u.test(text);
}

function importsOwnModule(rel, text) {
  const basename = path.basename(rel, path.extname(rel));
  if (!basename || basename === "index") return false;
  return new RegExp(`from\\s+["'][^"']*/${escapeRegExp(basename)}["']|use\\s+.*::${escapeRegExp(basename)}::`, "iu").test(text);
}

function hasOwnershipFile(dir) {
  return ["OWNERS", "CODEOWNERS", "README.md"].some((name) => fs.existsSync(path.join(dir, name)));
}

function escapeRegExp(value) {
  return String(value).replace(/[.*+?^${}()|[\]\\]/gu, "\\$&");
}

function sourceCommentText(ext, line) {
  if (TS_EXTENSIONS.has(ext) || ext === ".rs") return jsStyleCommentText(line);
  if (PY_EXTENSIONS.has(ext)) return hashCommentText(line);
  return line;
}

function jsStyleCommentText(line) {
  const single = line.indexOf("//");
  const block = line.indexOf("/*");
  const indexes = [single, block].filter((index) => index >= 0);
  if (indexes.length === 0) return "";
  return line.slice(Math.min(...indexes));
}

function hashCommentText(line) {
  const index = line.indexOf("#");
  return index >= 0 ? line.slice(index) : "";
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
  const rule = GENERIC_RULES[ruleId] ?? { title: "Unknown rule", snippet: "" };
  violations.push({
    ruleId,
    title: rule.title,
    detail,
    file: normalizeRel(root, filePath),
    line,
    snippet: rule.snippet,
    source: sourceLine?.trim() ?? null,
  });
}

function isGenericFile(filePath, activeLanguages) {
  const ext = path.extname(filePath).toLowerCase();
  if (
    activeLanguages.has("typescript") &&
    (TS_EXTENSIONS.has(ext) || isTypeScriptConfigPath(filePath))
  )
    return true;
  if (activeLanguages.has("python") && PY_EXTENSIONS.has(ext)) return true;
  if (
    activeLanguages.has("common") &&
    (TS_EXTENSIONS.has(ext) ||
      PY_EXTENSIONS.has(ext) ||
      COMMON_TEXT_EXTENSIONS.has(ext) ||
      ext === ".rs" ||
      isPolicyFile(filePath) ||
      isSensitiveOrGeneratedPath(filePath))
  ) {
    return true;
  }
  return false;
}

function isTypeScriptConfigPath(filePath) {
  return /^tsconfig(?:\.[^.]+)?\.json$/iu.test(path.basename(filePath));
}

function isPolicyFile(filePath) {
  const name = path.basename(filePath).toLowerCase();
  return [
    "package.json",
    "pyproject.toml",
    "requirements.txt",
    "cargo.toml",
    "deny.toml",
    ".env",
    ".env.local",
  ].includes(name);
}

function isSensitiveOrGeneratedPath(filePath) {
  const normalized = filePath.split(path.sep).join("/");
  const name = path.basename(normalized).toLowerCase();
  return (
    /(?:^|\/)(?:output|test-results|playwright-report)\//iu.test(normalized) ||
    name === "google-services.json" ||
    name === "googleservice-info.plist" ||
    name === "id_rsa" ||
    name === "id_rsa.pub" ||
    /^\.env(?:\..+)?$/iu.test(name) ||
    /\.(?:pem|p12|pfx|key)$/iu.test(name)
  );
}

function isCommandLikeLine(line) {
  return /^\s*(?:run:\s*)?(?:-\s+|>\s+)?(?:npx\s+|npm\s+|pnpm\s+|yarn\s+|node\s+|python(?:3)?\s+|uv\s+run\s+|cargo\s+|ruff\s+|pyright\b|mypy\b|gitleaks\b|trufflehog\b|\.\/|[A-Za-z]:[\\/])/iu.test(line) ||
    /\b(?:execSync|spawnSync|spawn|exec)\s*\(\s*["'`][^"'`]*(?:gitleaks|trufflehog|ruff|pyright|mypy|npm\s+install)/iu.test(line);
}

function isGeneratedLikePath(rel) {
  return /(?:^|\/)(?:generated|__generated__|snapshots?|__snapshots__|test-results|output)\//iu.test(rel) ||
    /(?:^|\/)[^/]*snapshot[^/]*\.(?:[cm]?[jt]sx?|json|txt|md|snap)$/iu.test(rel) ||
    /\.(?:snap|snapshot)$/iu.test(rel);
}

function isImportLikeLine(line) {
  return /^\s*(?:import\b|export\b.*\bfrom\b|(?:const|let|var)\s+\w+\s*=\s*require\(|use\s+)/u.test(line);
}

function isCoordinationVendorToolingPath(rel) {
  return /^src\/coordination\/vendor\//u.test(rel);
}

function isTestPath(rel) {
  return /(?:^|\/)(?:test|tests|__tests__)\/|(?:\.test|\.spec)\.[cm]?[jt]sx?$/iu.test(
    rel,
  );
}

function isPythonTestPath(rel) {
  return /(?:^|\/)(?:test|tests)\/|(?:^|\/)test_[^/]+\.py$|_test\.py$/iu.test(
    rel,
  );
}

function isProductionSourcePath(rel, ext) {
  if (!(TS_EXTENSIONS.has(ext) || PY_EXTENSIONS.has(ext) || ext === ".rs"))
    return false;
  if (isTestPath(rel) || isPythonTestPath(rel)) return false;
  if (/(?:^|\/)(?:build\.rs|fixtures?|__fixtures__)\//iu.test(rel))
    return false;
  return /^(?:src|apps|packages|crates|tools|scripts)\//u.test(rel);
}

function isGeneratedMarkerSourceFile(rel, ext) {
  if (!(TS_EXTENSIONS.has(ext) || PY_EXTENSIONS.has(ext) || ext === ".rs"))
    return false;
  if (isTestPath(rel) || isPythonTestPath(rel)) return false;
  return /^(?:src|apps|packages|crates|tools|scripts)\//u.test(rel);
}

function runGitDiff(root, base, head) {
  if (!base || !head) return "";
  const result = spawnSync(
    "git",
    ["diff", "--name-only", "--diff-filter=ACMR", base, head],
    {
      cwd: root,
      encoding: "utf8",
      shell: false,
    },
  );
  if ((result.status ?? 1) !== 0)
    throw new Error(result.stderr?.trim() || "failed to list diff files");
  return result.stdout.trim();
}

function redact(value) {
  return value
    .replace(/(["'])[A-Za-z0-9_./+=:@-]{8,}\1/gu, "$1[REDACTED]$1")
    .replace(/\bgh[pousr]_[A-Za-z0-9_]{20,}\b/gu, "[REDACTED_GITHUB_TOKEN]")
    .replace(/\bAKIA[0-9A-Z]{16}\b/gu, "[REDACTED_AWS_KEY]")
    .replace(/\b(?:sk|pk)_(?:live|test)_[A-Za-z0-9]{20,}\b/gu, "[REDACTED_STRIPE_KEY]")
    .replace(/\beyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\b/gu, "[REDACTED_JWT]");
}

function redactOpenAiKey(value) {
  return value.replace(OPENAI_KEY_RE, "[REDACTED_OPENAI_KEY]");
}
