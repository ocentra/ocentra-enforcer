import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  collectFiles,
  lineNumberAt,
  matchesAnyGlob,
  normalizeRel,
  repoAbsolute,
  uniqueSorted,
} from "./path-utils.mjs";
import { GENERIC_RULES, runGenericScan } from "./generic-scanners.mjs";
import {
  buildRegistryPolicyMap,
  buildRegistrySeverityMap,
  isSeverityDowngrade,
  isStrictProfile,
  normalizeFailOn,
  policyForRule,
  rulePolicyCapabilities,
} from "./policy.mjs";

const PACK_ROOT = path.resolve(path.join(path.dirname(fileURLToPath(import.meta.url)), ".."));

function ruleMetadataEntries(rows) {
  return Object.fromEntries(
    rows.map(([id, title, snippet]) => [id, { title, snippet }]),
  );
}

export const CHECK_RULES = {
  "SRC-1.1": {
    title: "Source files must stay within shape limits",
    snippet:
      "Split oversized files, long functions, and dumping-ground modules before adding behavior.",
  },
  "SRC-2.1": {
    title: "File line budget must be respected",
    snippet: "Split oversized files before adding behavior.",
  },
  "SRC-2.2": {
    title: "Function line budget must be respected",
    snippet: "Split long functions into smaller owned operations.",
  },
  "SRC-2.3": {
    title: "Export count budget must be respected",
    snippet: "Reduce public surface area or split modules by ownership.",
  },
  "SRC-2.4": {
    title: "Type count budget must be respected",
    snippet: "Split type-heavy modules by domain concept.",
  },
  "SRC-2.5": {
    title: "Class/struct count budget must be respected",
    snippet: "Keep class and struct ownership focused.",
  },
  "SRC-2.6": {
    title: "Nesting depth budget must be respected",
    snippet: "Flatten nested control flow and extract decision objects.",
  },
  "SRC-2.7": {
    title: "Branch budget must be respected",
    snippet: "Reduce conditional complexity or split branching logic.",
  },
  "TEST-2.1": {
    title: "Source workspaces must have test scaffolds",
    snippet: "Add package/crate tests before treating source work as complete.",
  },
  "TEST-2.2": {
    title: "Tests must live in organized test roots",
    snippet:
      "Move inline unit tests out of source files and into tests/ or another configured test root.",
  },
  "CONTRACT-1.1": {
    title: "Single-source contract values must not be copied",
    snippet:
      "Import or derive values from the owner contract instead of duplicating literals.",
  },
  "DEP-1.1": {
    title: "Dependency security audit must pass",
    snippet:
      "Fix high npm audit findings or cargo audit advisories instead of suppressing them.",
  },
  "DEP-1.2": {
    title: "External npm package licenses must match policy",
    snippet:
      "Use approved licenses or add a reviewed project policy exception.",
  },
  "NPM-1.1": {
    title: "package-lock.json is required",
    snippet: "Commit a package-lock.json and use npm ci so installs are reproducible.",
  },
  "NPM-1.2": {
    title: "npm ci is required in CI",
    snippet: "Use npm ci in workflow and local parity gates instead of npm install.",
  },
  "NPM-1.3": {
    title: "Enforcer dependencies must be pinned",
    snippet: "Use exact package versions; avoid ^, ~, *, latest, git:, file:, and path ranges.",
  },
  "NPM-1.4": {
    title: "packageManager must pin npm",
    snippet: "Set packageManager to an exact npm version such as npm@11.7.0.",
  },
  "NPM-1.5": {
    title: "Node engine must be bounded",
    snippet: "Use a bounded Node engine range such as >=20 <23 instead of an open-ended range.",
  },
  "NPM-1.6": {
    title: "Dependency install scripts require approval",
    snippet: "Remove dependency install scripts or add reviewed package policy before release.",
  },
  "NPM-1.7": {
    title: "Git dependencies are forbidden",
    snippet: "Use published exact package versions instead of git dependencies.",
  },
  "NPM-1.8": {
    title: "File and path dependencies are forbidden",
    snippet: "Use workspace-approved packages or published exact versions instead of file/path dependencies.",
  },
  "NPM-1.9": {
    title: "npm audit high and critical findings must fail",
    snippet: "Fix high or critical npm audit findings instead of suppressing dependency risk.",
  },
  "NPM-1.10": {
    title: "Dependency licenses must match policy",
    snippet: "Use approved dependency licenses or add reviewed package policy.",
  },
  "NPM-1.11": {
    title: "Suspicious dependency names are forbidden",
    snippet: "Review typo-squat-like package names before they enter dependencies.",
  },
  "NPM-1.12": {
    title: "SBOM must be generated for release",
    snippet: "Generate SBOM metadata before release or PR-ready claims.",
  },
  "NPM-1.13": {
    title: "Published package files must be explicit",
    snippet: "Set package.json files so releases cannot accidentally publish private harness state.",
  },
  "NPM-1.14": {
    title: "Package bin paths must exist",
    snippet: "Keep every package.json bin target present in the package and runnable by Node.",
  },
  "NPM-1.15": {
    title: "Package export paths must exist",
    snippet: "Keep every package.json exports target pointing at a tracked file or directory.",
  },
  "CI-1.1": {
    title: "CI must use npm ci",
    snippet: "Use npm ci in workflows; npm install is not deterministic enough for gates.",
  },
  "CI-1.2": {
    title: "CI must run npm test",
    snippet: "Keep the full Node test suite in the required CI parity gate.",
  },
  "CI-1.3": {
    title: "CI must run rule and policy tests",
    snippet: "Run the rule/policy tests in CI before accepting validator changes.",
  },
  "CI-1.4": {
    title: "CI must run multi-language tests",
    snippet: "Run multi-language Rust, TypeScript, Python, and common checks in CI.",
  },
  "CI-1.5": {
    title: "CI must run MCP tests",
    snippet: "Run MCP tests and smoke checks so CLI/MCP parity does not drift.",
  },
  "CI-1.6": {
    title: "CI must run Enforcer self-scan",
    snippet: "Run the Enforcer against itself in CI.",
  },
  "CI-1.7": {
    title: "CI must validate schemas",
    snippet: "Run registry/schema/policy integrity checks in CI.",
  },
  "CI-1.8": {
    title: "CI must run secret scan",
    snippet: "Run the common secret scanner in CI before publishing changes.",
  },
  "CI-1.9": {
    title: "CI must run dependency policy",
    snippet: "Run dependency policy checks in CI.",
  },
  "CI-1.10": {
    title: "CI must run SBOM check",
    snippet: "Run SBOM generation/checks in CI before release or merge readiness.",
  },
  "CI-1.11": {
    title: "Hard CI gates cannot continue on error",
    snippet: "Remove continue-on-error from hard Enforcer, test, lint, and policy jobs.",
  },
  "CI-1.12": {
    title: "CI must not hide failing hard gates",
    snippet: "Remove || true and other shell-level exit-code bypasses from hard gates.",
  },
  "CI-1.13": {
    title: "CI action versions must be pinned",
    snippet: "Pin workflow actions by major version at minimum; do not use main, master, or latest.",
  },
  "CI-1.14": {
    title: "CI workflows must declare least-privilege permissions",
    snippet: "Add an explicit permissions block, usually contents: read.",
  },
  "CI-1.15": {
    title: "CI must run on pull requests and main",
    snippet: "Trigger hard gates for pull_request and pushes to main.",
  },
  "CI-1.16": {
    title: "CI must cover Linux, Windows, and macOS",
    snippet: "Run cross-platform gates on ubuntu-latest, windows-latest, and macos-latest.",
  },
  "CI-1.17": {
    title: "CI workflow must match Enforcer adapter contract",
    snippet: "Run the local Enforcer parity gate from workflow templates instead of custom weaker gates.",
  },
  "CI-1.18": {
    title: "CI cannot call legacy weaker commands",
    snippet: "Call ocentra-enforcer or ci:local, not legacy rust-rules-only gates.",
  },
  "CI-1.19": {
    title: "Branch protection policy is required",
    snippet: "Document required branch protection and required checks.",
  },
  "CI-1.20": {
    title: "Required checks must include Enforcer",
    snippet: "Branch protection docs must name the Enforcer local/CI gate as required.",
  },
  "REPO-1.1": {
    title: "CODEOWNERS is required",
    snippet: "Add CODEOWNERS so policy-critical files require accountable review.",
  },
  "REPO-1.2": {
    title: "CODEOWNERS must protect rules",
    snippet: "Add a CODEOWNERS entry for rules/**.",
  },
  "REPO-1.3": {
    title: "CODEOWNERS must protect enforcement source",
    snippet: "Protect scripts/**, src/**, and mcp/**.",
  },
  "REPO-1.4": {
    title: "CODEOWNERS must protect schemas and adapters",
    snippet: "Protect schemas/**, profiles/**, and adapters/**.",
  },
  "REPO-1.5": {
    title: "CODEOWNERS must protect workflows",
    snippet: "Protect .github/workflows/**.",
  },
  "REPO-1.6": {
    title: "Package lockfile is required",
    snippet: "Commit the lockfile used by CI.",
  },
  "REPO-1.7": {
    title: "packageManager is required",
    snippet: "Set packageManager to an exact package-manager version.",
  },
  "REPO-1.8": {
    title: "Node version policy must be bounded",
    snippet: "Use a bounded Node engine policy.",
  },
  "REPO-1.9": {
    title: "Dependency versions must be deterministic",
    snippet: "Use exact dependency versions in package manifests.",
  },
  "REPO-1.10": {
    title: "License file is required",
    snippet: "Add a root LICENSE file so consumers know the legal terms.",
  },
  "REPO-1.11": {
    title: "Security policy is required",
    snippet: "Add SECURITY.md with vulnerability reporting and supported-version policy.",
  },
  "REPO-1.12": {
    title: "Contributing guide must explain rule changes",
    snippet: "Document how rules, validators, fixtures, schemas, and docs change together.",
  },
  "REPO-1.13": {
    title: "Changelog is required for rule behavior changes",
    snippet: "Keep CHANGELOG.md so validator behavior changes are visible to consumers.",
  },
  "REPO-1.14": {
    title: "Release policy must be documented",
    snippet: "Document release tagging, signing, and package publication policy.",
  },
  "REPO-1.15": {
    title: "Generated schema artifacts must not drift",
    snippet: "Regenerate schema artifacts and keep the working tree clean after schema tests.",
  },
  "SBOM-1.1": {
    title: "SBOM generation must complete",
    snippet:
      "Generate package metadata artifacts without committing generated output to source.",
  },
  "AI-1.1": {
    title: "Agent rule docs must be indexed",
    snippet:
      "Keep AGENTS and rule docs routed through a small index instead of broad rulebook loading.",
  },
  "ENF-1.1": {
    title: "Rule docs and registry must stay in sync",
    snippet:
      "Add a registry entry for each routed rule doc ID or remove the stale doc reference.",
  },
  "ENF-1.2": {
    title: "Registry docs must point to stable anchors",
    snippet:
      "Point each registry rule at an existing routed doc anchor such as rules/common/policy.md#covered-rules.",
  },
  "ENF-1.3": {
    title: "Scanner-emitted rule IDs must be registered",
    snippet:
      "Register every emitted ruleId in rules/rules.json before shipping the scanner.",
  },
  "ENF-1.4": {
    title: "Enforced rules must have fixture evidence",
    snippet:
      "Add fail and pass fixture evidence for validator-backed rules or explicitly mark review-only rules.",
  },
  "ENF-1.5": {
    title: "Rule IDs must be locked",
    snippet:
      "Update rules/rule-id-lock.json intentionally when adding rules; never remove or renumber existing IDs silently.",
  },
  "ENF-1.6": {
    title: "Rule IDs must be unique",
    snippet: "Keep exactly one registry row per ruleId.",
  },
  "ENF-1.7": {
    title: "Rule metadata must not drift",
    snippet:
      "Keep title and snippet in rules/rules.json aligned with validator metadata or remove duplicate metadata sources.",
  },
  "ENF-1.8": {
    title: "Violation reports must be complete",
    snippet:
      "Every finding must include ruleId, title, file, line, detail, doc, snippet, and source.",
  },
  "ENF-1.9": {
    title: "JSON output must be deterministic",
    snippet: "Sort output and avoid volatile fields in scanner/check JSON reports.",
  },
  "ENF-1.10": {
    title: "Human output must be deterministic",
    snippet: "Sort human-readable findings by normalized file, line, and rule ID.",
  },
  "ENF-1.11": {
    title: "Validators must not use undeclared network access",
    snippet: "Keep scanner/checker code offline unless the rule explicitly declares a network capability.",
  },
  "ENF-1.12": {
    title: "Validator source must be self-scanned",
    snippet: "Run Enforcer self-scan in CI over scripts, src, mcp, rules, and schemas.",
  },
  "ENF-1.13": {
    title: "Enforcer source cannot carry temporary bypasses",
    snippet: "Remove TODO, FIXME, HACK, temporary, and bypass markers from policy-critical source.",
  },
  "ENF-1.14": {
    title: "Generated JSON schemas must match Effect schemas",
    snippet: "Regenerate schema artifacts when Effect schema contracts change.",
  },
  "ENF-1.15": {
    title: "CLI and MCP behavior must match",
    snippet: "Keep CLI and MCP route, explain, scan, check, and coordination behavior in parity tests.",
  },
  "ENF-2.1": {
    title: "Policy-critical mutations require stronger proof",
    snippet:
      "Run rule coverage, policy integrity, fixture tests, MCP parity, self-scan, and CI governance before accepting policy-critical edits.",
  },
  "DOCENF-1.1": {
    title: "Rule docs must include required teaching sections",
    snippet:
      "Add Covered Rules, Fails, Passes, Fix Recipe, and Validator sections to routed rule docs.",
  },
  "DOCENF-1.2": {
    title: "Source rule docs must include fail and pass code blocks",
    snippet: "Add one bad and one good fenced code example for source-level rule docs.",
  },
  "DOCENF-1.4": {
    title: "Fix snippets must stay compact",
    snippet: "Keep registry fix snippets at or below 240 characters.",
  },
  "DOCENF-1.6": {
    title: "Immutable rule docs must use mandatory language",
    snippet: "Use must, forbidden, or required instead of should for immutable rules.",
  },
  "DOCENF-1.7": {
    title: "Docs must not make legacy aliases canonical",
    snippet: "Document rust-rules as a compatibility alias, not the canonical product name.",
  },
  ...ruleMetadataEntries([
    ["DOCENF-1.3", "Tagged rule doc code blocks must stay parseable", "Fix malformed fenced JSON or unbalanced tagged examples before shipping docs."],
    ["DOCENF-1.5", "Registry doc anchors must be stable lowercase anchors", "Use lowercase markdown anchors generated from headings; do not invent mixed-case or unstable anchors."],
    ["DOCENF-1.8", "Docs cannot describe the pack as Rust-only", "Update stale Rust-only wording when TS, Python, or common rules are registered."],
    ["DOCENF-1.9", "Advisory rule docs must explain promotion", "Document how a profile can promote advisory warnings to errors."],
    ["DOCENF-1.10", "Review and proof rules must name proof evidence", "Add a proof, checklist, or review evidence section for review/proof-oriented rules."],
    ["HAR-2.1", "Harness runs must identify command lifecycle", "Keep runId, command, cwd, startedAt, endedAt, and exitCode in every harness summary."],
    ["HAR-2.2", "Raw harness logs must be bounded and redacted", "Bound artifact reads and redact secrets before logs or artifacts are returned."],
    ["HAR-2.3", "Harness diagnostics must be sorted deterministically", "Sort diagnostics by normalized file, line, rule ID, and message."],
    ["HAR-2.4", "Harness parsers must not throw on malformed output", "Emit a parser diagnostic instead of crashing on malformed tool output."],
    ["HAR-2.5", "Cargo JSON diagnostics must normalize", "Normalize rustc/cargo compiler-message JSON into rule, file, line, severity diagnostics."],
    ["HAR-2.6", "ESLint JSON diagnostics must normalize", "Normalize ESLint JSON filePath/messages output into compact diagnostics."],
    ["HAR-2.7", "Python tool diagnostics must normalize", "Normalize Ruff, Pyright, mypy-style, and pytest diagnostics into compact records."],
    ["HAR-2.8", "SARIF diagnostics must normalize", "Normalize SARIF runs/results into file, line, severity, rule ID diagnostics."],
    ["HAR-2.9", "Last failure must avoid raw terminal dumps", "Expose compact failed-run diagnostics through last-failure queries."],
    ["HAR-2.10", "Harness artifacts cannot escape storage", "Reject artifact paths that resolve outside the target repository storage root."],
    ["HAR-2.11", "Pinned proof runs must survive pruning", "Keep pinned PR-ready/proof runs even when normal retention would prune old runs."],
    ["HAR-2.12", "Failed harness commands must fail process gates", "Return ok false and non-zero CLI status when a command or guard fails."],
    ["HAR-2.13", "Harness JSON output must have schema artifacts", "Keep run report and diagnostic JSON schemas generated for MCP and non-Effect consumers."],
    ["HAR-2.14", "Harness human output must redact secrets", "Redact tokens, private keys, and credentials before exposing compact or raw artifacts."],
    ["HAR-2.15", "Harness commands must avoid shell by default", "Run commands through argument arrays with shell disabled unless explicitly requested by policy."],
    ["MCP-1.1", "MCP must expose CLI scan and check equivalents", "Keep route, scan, check, doctor, explain, run, proof, and coordination tools available over MCP."],
    ["MCP-1.2", "MCP inputs must be schema decoded", "Decode every external MCP input through Effect Schema or equivalent strict schemas."],
    ["MCP-1.3", "MCP must reject unknown tool arguments", "Use additionalProperties:false and decoder checks so unknown MCP args do not get ignored."],
    ["MCP-1.4", "MCP summary output must be bounded", "Respect summaryOnly/includeScope controls for compact MCP responses."],
    ["MCP-1.5", "MCP diagnostic limits must be enforced", "Apply diagnosticLimit before returning findings to Codex."],
    ["MCP-1.6", "Stale MCP write processes must fail closed", "Refuse coordination writes when the loaded MCP server fingerprint is stale."],
    ["MCP-1.7", "MCP status must include version and hash", "Expose pack version, process identity, file hashes, and reload status."],
    ["MCP-1.8", "MCP explain must match CLI explain", "Keep explain behavior routed through the same registry/doc source as CLI."],
    ["MCP-1.9", "MCP route must match CLI route", "Keep route behavior routed through the same registry decision tree as CLI."],
    ["MCP-1.10", "MCP scan must not mutate target repositories", "Run scan/check/explain/route paths as read-only operations unless the tool is explicitly write-capable."],
    ["MCP-1.11", "MCP write tools cannot be generic action dispatchers", "Use dedicated claim/release/message tools or reject mismatched action fields."],
    ["MCP-1.12", "MCP errors must be structured JSON", "Return JSON error objects instead of raw text or stack dumps."],
    ["PROOF-1.1", "PR-ready claims require fresh proof", "Reject PR-ready proof claims without a fresh matching proof run."],
    ["PROOF-1.2", "Proof freshness must bind commit and scope", "Store commit, branch, files, and profile with every proof run and claim."],
    ["PROOF-1.3", "Manual-required proof cannot auto-pass", "Represent manual/device proof as manual-required or unavailable unless explicit evidence is attached."],
    ["PROOF-1.4", "Required proof artifacts must exist", "Fail proof claims when required artifacts are missing or empty."],
    ["PROOF-1.5", "Proof artifacts must hash-match", "Compare recorded artifact hashes against claim-time artifacts."],
    ["PROOF-1.6", "Dirty worktrees invalidate PR-ready proof", "Reject PR-ready claims from dirty trees unless allowDirty is explicit."],
    ["PROOF-1.7", "Waived proof must remain visible", "Report waived or unavailable proof as explicit evidence state, not pass."],
    ["PROOF-1.8", "Proof command cannot be empty", "Require a command or classify the proof as manual-required/unavailable."],
    ["PROOF-1.9", "Proof command cannot be a shell string", "Accept command argv arrays; reject shell-string execution for proof commands."],
    ["PROOF-1.10", "Proof registry docs paths must exist", "Keep every proof definition linked to an existing proof/rule doc."],
    ["PROOF-1.11", "Proof capabilities must match environment", "Check declared capabilities before treating proof as executable."],
    ["PROOF-1.12", "Device proof cannot auto-pass on desktop", "Mark Android/iOS/device proof unavailable/manual unless matching capability is present."],
    ["PROOF-1.13", "Proof claims must list proved and unproved claims", "Record claimsProved and claimsNotProved in proof runs and claim output."],
    ["PROOF-1.14", "Proof output must be compact by default", "Return bounded summaries and diagnostics unless artifact text is explicitly requested."],
    ["PROOF-1.15", "Proof exports must redact secrets", "Redact secret-like values from proof artifacts and exported proof manifests."],
    ["SCAN-1.1", "Scanner must mask string literals where appropriate", "Mask strings before scanning patterns that should only match code structure."],
    ["SCAN-1.2", "Scanner must mask comments where appropriate", "Mask comments before structural scans while keeping suppression-comment checks explicit."],
    ["SCAN-1.3", "Scanner must still detect suppression comments", "Keep lint-disable, noqa, ts-ignore, and type-ignore detection active."],
    ["SCAN-1.4", "Scanner must handle CRLF and LF identically", "Split and count lines with CRLF/LF-safe logic."],
    ["SCAN-1.5", "Scanner must support Unicode paths", "Normalize paths without assuming ASCII-only file names."],
    ["SCAN-1.6", "Scanner must support spaces in paths", "Treat file paths as data, not shell fragments."],
    ["SCAN-1.7", "Scanner must support Windows drive paths", "Resolve absolute Windows paths correctly before normalizing reports."],
    ["SCAN-1.8", "Scanner symlink policy must be explicit", "Skip or classify symlinks intentionally instead of following them accidentally."],
    ["SCAN-1.9", "Scanner must avoid symlink loops", "Use lstat/realpath guards and never recurse indefinitely through symlinks."],
    ["SCAN-1.10", "Scanner output must be sorted", "Sort findings by normalized path, line, rule ID, and message."],
    ["SCAN-1.11", "Scanner must bound file reads", "Use size limits or streaming before scanning arbitrary files."],
    ["SCAN-1.12", "Scanner must skip binary files safely", "Detect binary payloads and avoid UTF-8 parser crashes."],
    ["SCAN-1.13", "Invalid UTF-8 must produce bounded diagnostics", "Handle decode failures with a compact diagnostic instead of crashing."],
    ["SCAN-1.14", "Unknown extensions must not trigger false language scans", "Route files by configured language extensions and return no detailed docs for unknown extensions."],
    ["SCAN-1.15", "Diff scope must handle deleted and renamed files", "Resolve diff entries deterministically and skip deleted files safely."],
    ["SCAN-1.16", "File scope must not scan the whole repo", "Limit file-scope checks to the provided paths."],
    ["SCAN-1.17", "Workspace scope must scan configured roots", "Use configured roots and ignore globs for workspace scans."],
    ["SCAN-1.18", "Crate/package scope must resolve manifests", "Resolve Cargo/package manifests before scoped native checks."],
    ["SCAN-1.19", "Scope reports must include included/excluded counts", "Report scope file counts and ignored/excluded paths in compact output."],
    ["SCAN-1.20", "Doctor must expose ignore globs", "Show ignore directories and file globs in doctor output."],
    ["SCAN-2.1", "Rust strict mode must use cargo metadata", "Use cargo metadata for workspace/package dependency context."],
    ["SCAN-2.2", "Rust strict mode must use parser-backed checks", "Use parser/native checks for signatures, fields, and syntax-sensitive Rust rules where regex is insufficient."],
    ["SCAN-2.3", "Rust strict mode must ingest Clippy JSON", "Normalize clippy JSON diagnostics through the harness."],
    ["SCAN-2.4", "Rust strict mode must ingest rustdoc warnings", "Normalize rustdoc JSON or warning output through the harness."],
    ["SCAN-2.5", "TypeScript strict mode must use compiler or ESLint JSON", "Ingest tsc and ESLint diagnostics instead of relying only on regex preflight."],
    ["SCAN-2.6", "Python strict mode must ingest Ruff JSON", "Normalize Ruff JSON diagnostics through the harness."],
    ["SCAN-2.7", "Python strict mode must ingest Pyright or mypy output", "Normalize Pyright/mypy diagnostics through the harness."],
    ["SCAN-2.8", "Security strict mode must ingest SARIF", "Normalize SARIF results from security tools."],
    ["SCAN-2.9", "Regex scanner must remain fast preflight", "Keep regex scanners as bounded preflight before heavier native tools."],
    ["SCAN-2.10", "Native and regex reports must merge without duplicate spam", "Dedupe merged diagnostics by tool, rule, file, line, and message."],
  ]),
  "CFG-1.1": {
    title: "Strict profiles must fail on errors",
    snippet: "Keep failOn including error; use warnings for advisory debt, not disabled hard failures.",
  },
  "CFG-1.2": {
    title: "Immutable rules cannot be disabled",
    snippet:
      "Remove enabled:false from immutable rules or change the registry lock only through a reviewed rule change.",
  },
  "CFG-1.3": {
    title: "Immutable rules cannot be downgraded",
    snippet:
      "Do not lower immutable error rules to warning or info; fix the code or file a narrow waiver where allowed.",
  },
  "CFG-1.4": {
    title: "Unsafe code requires governed waiver",
    snippet:
      "Keep allowUnsafeCode false unless a narrow owner-approved waiver is present.",
  },
  "CFG-1.5": {
    title: "Public re-export allow mode is forbidden in strict profiles",
    snippet:
      "Use facade-only or forbid; do not globally allow barrel/public re-export policy in strict mode.",
  },
  "CFG-1.6": {
    title: "Build scripts and non-registry dependencies require waiver",
    snippet:
      "Keep allowBuildRs, allowGitDependencies, and allowPathDependencies false unless a narrow waiver exists.",
  },
  "CFG-1.7": {
    title: "Boundary glob changes require owner note",
    snippet:
      "Add a narrow owner note when changing raw type, facade, runtime string, or import boundary globs.",
  },
  "CFG-1.8": {
    title: "Rule disable requires expiry",
    snippet:
      "Even overridable/advisory disables must carry waiverId, owner, issue, reason, scope, expires, and remediation.",
  },
  "CFG-1.9": {
    title: "Unknown config keys are forbidden",
    snippet: "Remove misspelled or unsupported config keys; schema drift must be explicit.",
  },
  "CFG-1.10": {
    title: "Config precedence must be explicit",
    snippet: "Declare profileName and schemaVersion so profile plus local config layering is unambiguous.",
  },
  "CFG-1.11": {
    title: "Profile name must be known",
    snippet: "Use a known locked profile name such as strict or ocentra-enforcer.",
  },
  "CFG-1.12": {
    title: "Config changes require policy self-check",
    snippet: "When config changes, run and record policy-integrity or rule-coverage validation.",
  },
  "WAIVER-1.1": {
    title: "Waivers must include required metadata",
    snippet:
      "Add ruleId, waiverId, owner, issue, reason, exact scope, expires, remediation, and ciAllowed.",
  },
  "WAIVER-1.2": {
    title: "Waiver scope must be narrow",
    snippet:
      "Use exact files or narrow globs; repo-wide and language-wide waivers are forbidden.",
  },
  "WAIVER-1.3": {
    title: "Expired waivers fail",
    snippet: "Refresh or remove expired waiver debt before claiming the gate passes.",
  },
  "WAIVER-1.4": {
    title: "Immutable rules cannot be waived unless marked waivable",
    snippet:
      "Do not add waivers for immutable rules unless the registry explicitly permits waiving that rule.",
  },
  "WAIVER-1.5": {
    title: "CI waiver behavior must be explicit",
    snippet:
      "Set ciAllowed intentionally; local-only waivers cannot satisfy CI gates.",
  },
  "WAIVER-1.6": {
    title: "Waivers must remain visible in output",
    snippet: "Keep waiver IDs attached to waived findings so debt stays visible.",
  },
  "WAIVER-1.7": {
    title: "Active waiver count is budgeted",
    snippet: "Reduce active waivers or raise maxActiveWaivers through reviewed policy.",
  },
  "WAIVER-1.8": {
    title: "Permanent waiver grandfathering is forbidden",
    snippet: "Use short expiry windows for waivers; long-lived waivers need renewed ownership.",
  },
  "WAIVER-1.9": {
    title: "Waiver owner must be a human or team",
    snippet: "Use a real accountable owner, not codex, ai, agent, or empty metadata.",
  },
  "WAIVER-1.10": {
    title: "Waivers require remediation plans",
    snippet:
      "Explain the concrete cleanup path so waiver debt expires into work, not permanent bypass.",
  },
  "TS-4.1": {
    title: "Import boundary policy must be respected",
    snippet:
      "Move code to the owning package or add a reviewed import-boundary policy exception instead of crossing layers directly.",
  },
};

const DEFAULT_ALLOWED_LICENSES = new Set([
  "0BSD",
  "Apache-2.0 OR MIT",
  "Apache-2.0",
  "BSD-2-Clause",
  "BSD-3-Clause",
  "BlueOak-1.0.0",
  "CC-BY-4.0",
  "ISC",
  "MIT",
  "MPL-2.0",
  "Python-2.0",
]);

const CHECK_ALIASES = new Map([
  ["check-source-shape", "source-shape"],
  ["check-required-tests", "required-tests"],
  ["check-single-source-contracts", "single-source-contracts"],
  ["check-ai-rule-index", "ai-rule-index"],
  ["check-dependency-policy", "dependency-policy"],
  ["write-sbom", "sbom"],
]);

export const SCANNER_BACKED_CHECKS = {
  "no-zod-source": {
    languages: ["typescript", "common"],
    ruleIds: ["TS-1.2"],
  },
  "no-naked-domain-strings": {
    languages: ["rust", "typescript", "python", "common"],
    ruleIds: ["RR-6.1", "RR-6.5", "RR-18.16", "TS-1.3", "PY-1.3"],
  },
  "no-test-doubles": {
    languages: ["typescript", "python", "common"],
    ruleIds: ["TEST-1.1"],
  },
  "weak-assertions": {
    languages: ["typescript", "python", "common"],
    ruleIds: ["TEST-1.2"],
  },
  "skipped-focused-tests": {
    languages: ["typescript", "python", "common"],
    ruleIds: ["TS-3.1", "PY-2.1", "TEST-1.3"],
  },
  "validation-bypass": {
    languages: ["rust", "typescript", "python", "common"],
    ruleIds: ["RR-2.1", "RR-2.2", "TS-2.1", "PY-1.1", "PY-1.2"],
  },
  "placeholder-implementation": {
    languages: ["rust", "typescript", "python", "common"],
    ruleIds: ["RR-4.2", "RR-4.3", "SRC-1.2"],
  },
  reexports: {
    languages: ["rust", "typescript"],
    ruleIds: ["RR-7.2", "RR-7.3", "TS-1.1"],
  },
  "cross-platform-script-commands": {
    languages: ["common"],
    ruleIds: ["PORT-1.1"],
  },
  "rust-string-boundaries": {
    languages: ["rust"],
    ruleIds: ["RR-18.16"],
  },
};

export function normalizeCheckName(value) {
  const normalized = String(value ?? "")
    .trim()
    .replace(/^check-/u, "");
  return CHECK_ALIASES.get(normalized) ?? normalized;
}

export function listStandaloneChecks() {
  return [
    ...Object.keys(SCANNER_BACKED_CHECKS),
    "source-shape",
    "required-tests",
    "single-source-contracts",
    "dependency-policy",
    "sbom",
    "ai-rule-index",
    "generated-artifacts",
    "secrets",
    "import-boundaries",
    "rule-coverage",
    "policy-integrity",
    "config-lockdown",
    "waiver-policy",
    "docs-completeness",
    "ci-integrity",
    "repo-governance",
    "scanner-fixtures",
    "package-determinism",
    "mutation-risk",
    "harness-contracts",
    "proof-contracts",
    "mcp-contracts",
    "scanner-contracts",
  ];
}

export function runStandaloneCheck({
  checkName,
  root,
  config = {},
  args = {},
}) {
  const normalized = normalizeCheckName(checkName);
  const scope = args.scope ?? { mode: "all" };
  switch (normalized) {
    case "source-shape":
      return buildReport({
        root,
        config,
        checkName: normalized,
        findings: collectSourceShapeFindings(root, config, scope),
        scope,
      });
    case "required-tests":
      return buildReport({
        root,
        config,
        checkName: normalized,
        findings: collectRequiredTestFindings(root, config, scope, args),
        scope,
      });
    case "single-source-contracts":
      return buildReport({
        root,
        config,
        checkName: normalized,
        findings: collectSingleSourceContractFindings(
          root,
          args.checkConfigPath,
          scope,
          config,
        ),
        scope,
      });
    case "dependency-policy":
      return buildReport({
        root,
        config,
        checkName: normalized,
        findings: collectDependencyPolicyFindings(root, config),
      });
    case "sbom":
      return buildReport({
        root,
        config,
        checkName: normalized,
        findings: runSbomCheck(root, args),
      });
    case "ai-rule-index":
      return buildReport({
        root,
        config,
        checkName: normalized,
        findings: collectAiRuleIndexFindings(root, config),
      });
    case "generated-artifacts":
      return buildReport({
        root,
        config,
        checkName: normalized,
        findings: collectGeneratedArtifactFindings(root, config, scope, args),
        scope,
      });
    case "secrets":
      return buildReport({
        root,
        config,
        checkName: normalized,
        findings: collectSecretFindings(root, config, scope, args),
        scope,
      });
    case "import-boundaries":
      return buildReport({
        root,
        config,
        checkName: normalized,
        findings: collectImportBoundaryFindings(root, config, scope),
        scope,
      });
    case "rule-coverage":
    case "scanner-fixtures":
      return buildReport({
        root,
        config,
        checkName: normalized,
        findings: collectRuleCoverageFindings(root, config, args),
        scope,
      });
    case "docs-completeness":
      return buildReport({
        root,
        config,
        checkName: normalized,
        findings: collectDocsCompletenessFindings(root, args),
        scope,
      });
    case "policy-integrity":
      return buildReport({
        root,
        config,
        checkName: normalized,
        findings: [
          ...collectConfigLockdownFindings(root, config),
          ...collectWaiverPolicyFindings(root, config),
          ...collectRuleCoverageFindings(root, config, args),
          ...collectHarnessContractFindings(root, args),
          ...collectProofContractFindings(root, args),
          ...collectMcpContractFindings(root, args),
          ...collectScannerContractFindings(root, args),
        ],
        scope,
      });
    case "config-lockdown":
      return buildReport({
        root,
        config,
        checkName: normalized,
        findings: collectConfigLockdownFindings(root, config),
        scope,
      });
    case "waiver-policy":
      return buildReport({
        root,
        config,
        checkName: normalized,
        findings: collectWaiverPolicyFindings(root, config),
        scope,
      });
    case "ci-integrity":
      return buildReport({
        root,
        config,
        checkName: normalized,
        findings: collectCiIntegrityFindings(root),
        scope,
      });
    case "repo-governance":
      return buildReport({
        root,
        config,
        checkName: normalized,
        findings: collectRepoGovernanceFindings(root),
        scope,
      });
    case "package-determinism":
      return buildReport({
        root,
        config,
        checkName: normalized,
        findings: collectPackageDeterminismFindings(root),
        scope,
      });
    case "mutation-risk":
      return buildReport({
        root,
        config,
        checkName: normalized,
        findings: collectMutationRiskFindings(root, scope),
        scope,
      });
    case "harness-contracts":
      return buildReport({
        root,
        config,
        checkName: normalized,
        findings: collectHarnessContractFindings(root, args),
        scope,
      });
    case "proof-contracts":
      return buildReport({
        root,
        config,
        checkName: normalized,
        findings: collectProofContractFindings(root, args),
        scope,
      });
    case "mcp-contracts":
      return buildReport({
        root,
        config,
        checkName: normalized,
        findings: collectMcpContractFindings(root, args),
        scope,
      });
    case "scanner-contracts":
      return buildReport({
        root,
        config,
        checkName: normalized,
        findings: collectScannerContractFindings(root, args),
        scope,
      });
    default:
      throw new Error(`Unknown standalone check: ${checkName}`);
  }
}

function collectSourceShapeFindings(root, config, scope = { mode: "all" }) {
  const policies = config.sourceShapePolicies ?? [
    {
      roots: ["src", "apps"],
      extensions: [".ts", ".tsx"],
      kind: "typescript",
      maxClasses: 1,
      maxExports: 35,
      maxFunctionLines: 80,
      maxLines: 1000,
    },
    {
      roots: ["packages"],
      extensions: [".ts", ".tsx"],
      kind: "typescript",
      maxClasses: 1,
      maxExports: 45,
      maxFunctionLines: 80,
      maxLines: 1000,
    },
    {
      roots: ["src", "crates"],
      extensions: [".rs"],
      kind: "rust",
      maxFunctionLines: 80,
      maxFunctions: 18,
      maxLines: 1000,
      maxTypes: 24,
    },
    {
      roots: ["src", "apps", "packages", "tools"],
      extensions: [".py"],
      kind: "python",
      maxClasses: 4,
      maxFunctionLines: 80,
      maxFunctions: 30,
      maxLines: 800,
    },
  ];

  const findings = [];
  for (const policy of policies) {
    for (const file of collectPolicyFiles(root, config, policy, scope)) {
      const rel = normalizeRel(root, file);
      const text = fs.readFileSync(file, "utf8");
      const effectivePolicy = applySourceShapeOverrides(config, rel, policy);
      if (effectivePolicy.kind === "rust")
        findings.push(...inspectRustShape(root, file, text, effectivePolicy));
      else if (effectivePolicy.kind === "python")
        findings.push(...inspectPythonShape(root, file, text, effectivePolicy));
      else
        findings.push(
          ...inspectTypeScriptShape(root, file, text, effectivePolicy),
        );
      const lines = countLines(text);
      if (lines > effectivePolicy.maxLines) {
        findings.push(
          finding(
            root,
            file,
            effectivePolicy.maxLines + 1,
            "SRC-1.1",
            `file has ${lines} lines; maximum is ${effectivePolicy.maxLines}`,
            null,
          ),
        );
        findings.push(
          finding(
            root,
            file,
            effectivePolicy.maxLines + 1,
            "SRC-2.1",
            `file has ${lines} lines; maximum is ${effectivePolicy.maxLines}`,
            null,
          ),
        );
      }
    }
  }
  return findings;
}

function applySourceShapeOverrides(config, rel, policy) {
  let effectivePolicy = { ...policy };
  for (const override of config.sourceShapeOverrides ?? []) {
    const matchesPath =
      override.path === rel ||
      (Array.isArray(override.paths) && override.paths.includes(rel));
    const matchesGlob =
      (typeof override.glob === "string" &&
        matchesAnyGlob(rel, [override.glob])) ||
      (Array.isArray(override.globs) && matchesAnyGlob(rel, override.globs));
    if (!matchesPath && !matchesGlob) continue;
    const {
      path: _path,
      paths: _paths,
      glob: _glob,
      globs: _globs,
      note: _note,
      ...limits
    } = override;
    effectivePolicy = { ...effectivePolicy, ...limits };
  }
  return effectivePolicy;
}

function inspectTypeScriptShape(root, file, text, policy) {
  const findings = [];
  const lines = text.split(/\r?\n/u);
  const maxNestingDepth = maxBraceNestingDepth(lines);
  const branchCount = countMatches(lines, /\b(?:if|else\s+if|for|while|switch|case|catch)\b|\?\s*[^:]+:/u);
  const classCount = countMatches(
    lines,
    /^\s*(?:export\s+)?class\s+[A-Za-z_$]/u,
  );
  const exportCount = countMatches(
    lines,
    /^\s*export\s+(?:class|function|const|let|var|type|interface|enum|default|\{|\*)/u,
  );
  const functionStarts = [];

  if (maxNestingDepth > (policy.maxNestingDepth ?? 4)) {
    findings.push(
      finding(
        root,
        file,
        1,
        "SRC-2.6",
        `file nesting depth is ${maxNestingDepth}; maximum is ${policy.maxNestingDepth ?? 4}`,
        null,
      ),
    );
  }
  if (branchCount > (policy.maxBranches ?? 12)) {
    findings.push(
      finding(
        root,
        file,
        1,
        "SRC-2.7",
        `file has ${branchCount} branch points; maximum is ${policy.maxBranches ?? 12}`,
        null,
      ),
    );
  }

  lines.forEach((line, index) => {
    if (
      /^\s*(?:export\s+)?(?:async\s+)?function\s+[A-Za-z_$]|\)\s*=>\s*\{|\b(?:async\s+)?[A-Za-z_$][\w$]*\s*\([^)]*\)\s*\{/u.test(
        line,
      )
    ) {
      functionStarts.push(index);
    }
  });

  if (classCount > policy.maxClasses) {
    findings.push(
      finding(
        root,
        file,
        1,
        "SRC-1.1",
        `file has ${classCount} classes; maximum is ${policy.maxClasses}`,
        null,
      ),
    );
    findings.push(
      finding(
        root,
        file,
        1,
        "SRC-2.5",
        `file has ${classCount} classes; maximum is ${policy.maxClasses}`,
        null,
      ),
    );
  }
  if (exportCount > policy.maxExports) {
    findings.push(
      finding(
        root,
        file,
        1,
        "SRC-1.1",
        `file has ${exportCount} exports; maximum is ${policy.maxExports}`,
        null,
      ),
    );
    findings.push(
      finding(
        root,
        file,
        1,
        "SRC-2.3",
        `file has ${exportCount} exports; maximum is ${policy.maxExports}`,
        null,
      ),
    );
  }
  for (const start of functionStarts) {
    const end = findBlockEnd(lines, start);
    const span = end - start + 1;
    if (span > policy.maxFunctionLines) {
      findings.push(
        finding(
          root,
          file,
          start + 1,
          "SRC-1.1",
          `function has ${span} lines; maximum is ${policy.maxFunctionLines}`,
          lines[start],
        ),
      );
      findings.push(
        finding(
          root,
          file,
          start + 1,
          "SRC-2.2",
          `function has ${span} lines; maximum is ${policy.maxFunctionLines}`,
          lines[start],
        ),
      );
    }
  }

  return findings;
}

function inspectPythonShape(root, file, text, policy) {
  const findings = [];
  const lines = text.split(/\r?\n/u);
  const maxNestingDepth = maxPythonIndentDepth(lines);
  const branchCount = countMatches(lines, /^\s*(?:if|elif|for|while|try|except|with|match|case)\b/u);
  const classStarts = [];
  const functionStarts = [];

  if (maxNestingDepth > (policy.maxNestingDepth ?? 4)) {
    findings.push(
      finding(
        root,
        file,
        1,
        "SRC-2.6",
        `file nesting depth is ${maxNestingDepth}; maximum is ${policy.maxNestingDepth ?? 4}`,
        null,
      ),
    );
  }
  if (branchCount > (policy.maxBranches ?? 12)) {
    findings.push(
      finding(
        root,
        file,
        1,
        "SRC-2.7",
        `file has ${branchCount} branch points; maximum is ${policy.maxBranches ?? 12}`,
        null,
      ),
    );
  }

  lines.forEach((line, index) => {
    if (/^\s*class\s+[A-Za-z_]\w*/u.test(line)) classStarts.push(index);
    if (/^\s*(?:async\s+)?def\s+[A-Za-z_]\w*\s*\(/u.test(line))
      functionStarts.push(index);
  });

  if (classStarts.length > policy.maxClasses) {
    findings.push(
      finding(
        root,
        file,
        1,
        "SRC-1.1",
        `file has ${classStarts.length} classes; maximum is ${policy.maxClasses}`,
        null,
      ),
    );
    findings.push(
      finding(
        root,
        file,
        1,
        "SRC-2.5",
        `file has ${classStarts.length} classes; maximum is ${policy.maxClasses}`,
        null,
      ),
    );
  }
  if (functionStarts.length > policy.maxFunctions) {
    findings.push(
      finding(
        root,
        file,
        1,
        "SRC-1.1",
        `file has ${functionStarts.length} functions; maximum is ${policy.maxFunctions}`,
        null,
      ),
    );
  }
  for (const start of functionStarts) {
    const end = findPythonBlockEnd(lines, start);
    const span = end - start + 1;
    if (span > policy.maxFunctionLines) {
      findings.push(
        finding(
          root,
          file,
          start + 1,
          "SRC-1.1",
          `function has ${span} lines; maximum is ${policy.maxFunctionLines}`,
          lines[start],
        ),
      );
      findings.push(
        finding(
          root,
          file,
          start + 1,
          "SRC-2.2",
          `function has ${span} lines; maximum is ${policy.maxFunctionLines}`,
          lines[start],
        ),
      );
    }
  }

  return findings;
}

function inspectRustShape(root, file, text, policy) {
  const findings = [];
  const lines = text.split(/\r?\n/u);
  const maxNestingDepth = maxBraceNestingDepth(lines);
  const branchCount = countMatches(lines, /\b(?:if|else\s+if|for|while|loop|match)\b|=>/u);
  const functionStarts = [];
  let typeCount = 0;

  if (maxNestingDepth > (policy.maxNestingDepth ?? 4)) {
    findings.push(
      finding(
        root,
        file,
        1,
        "SRC-2.6",
        `file nesting depth is ${maxNestingDepth}; maximum is ${policy.maxNestingDepth ?? 4}`,
        null,
      ),
    );
  }
  if (branchCount > (policy.maxBranches ?? 12)) {
    findings.push(
      finding(
        root,
        file,
        1,
        "SRC-2.7",
        `file has ${branchCount} branch points; maximum is ${policy.maxBranches ?? 12}`,
        null,
      ),
    );
  }

  lines.forEach((line, index) => {
    if (/^\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+\w+/u.test(line))
      functionStarts.push(index);
    if (/^\s*(?:pub\s+)?(?:struct|enum)\s+\w+/u.test(line)) typeCount += 1;
  });

  if (functionStarts.length > policy.maxFunctions) {
    findings.push(
      finding(
        root,
        file,
        1,
        "SRC-1.1",
        `file has ${functionStarts.length} functions; maximum is ${policy.maxFunctions}`,
        null,
      ),
    );
  }
  if (typeCount > policy.maxTypes) {
    findings.push(
      finding(
        root,
        file,
        1,
        "SRC-1.1",
        `file has ${typeCount} structs/enums; maximum is ${policy.maxTypes}`,
        null,
      ),
    );
    findings.push(
      finding(
        root,
        file,
        1,
        "SRC-2.4",
        `file has ${typeCount} structs/enums; maximum is ${policy.maxTypes}`,
        null,
      ),
    );
  }
  for (const start of functionStarts) {
    const end = findBlockEnd(lines, start);
    const span = end - start + 1;
    if (span > policy.maxFunctionLines) {
      findings.push(
        finding(
          root,
          file,
          start + 1,
          "SRC-1.1",
          `function has ${span} lines; maximum is ${policy.maxFunctionLines}`,
          lines[start],
        ),
      );
      findings.push(
        finding(
          root,
          file,
          start + 1,
          "SRC-2.2",
          `function has ${span} lines; maximum is ${policy.maxFunctionLines}`,
          lines[start],
        ),
      );
    }
  }
  return findings;
}

function collectRequiredTestFindings(
  root,
  config,
  scope = { mode: "all" },
  args = {},
) {
  const findings = [];
  const scopedRoots = scopedProjectRoots(root, config, scope);
  const strictEmptyTestTrees =
    args.strictEmptyTestTrees === true || config.strictEmptyTestTrees === true;
  for (const workspaceRoot of ["packages", "apps"]) {
    for (const dir of childDirs(path.join(root, workspaceRoot))) {
      if (scopedRoots !== null && !scopedRoots.has(normalizeRel(root, dir)))
        continue;
      const packageJsonPath = path.join(dir, "package.json");
      const srcPath = path.join(dir, "src");
      if (!fs.existsSync(packageJsonPath) || !fs.existsSync(srcPath)) continue;
      const manifest = JSON.parse(fs.readFileSync(packageJsonPath, "utf8"));
      const hasTests = hasFile(path.join(dir, "tests"), (file) =>
        /\.(?:test|spec)\.[cm]?tsx?$/u.test(file),
      );
      if (!hasTests) {
        findings.push(
          finding(
            root,
            packageJsonPath,
            1,
            "TEST-2.1",
            `${manifest.name ?? normalizeRel(root, dir)} is missing tests/*.test.ts`,
            null,
          ),
        );
      }
      collectInlineSourceTestFindings(root, srcPath, config, findings);
      collectStrictEmptyTestTreeFindings(
        root,
        dir,
        strictEmptyTestTrees,
        findings,
      );
    }
  }

  for (const dir of childDirs(path.join(root, "crates"))) {
    if (scopedRoots !== null && !scopedRoots.has(normalizeRel(root, dir)))
      continue;
    const cargoPath = path.join(dir, "Cargo.toml");
    if (!fs.existsSync(cargoPath)) continue;
    const hasIntegrationTest = hasFile(path.join(dir, "tests"), (file) =>
      file.endsWith(".rs"),
    );
    if (!hasIntegrationTest) {
      findings.push(
        finding(
          root,
          cargoPath,
          1,
          "TEST-2.1",
          `${normalizeRel(root, dir)} is missing organized Rust tests under tests/`,
          null,
        ),
      );
    }
    collectInlineSourceTestFindings(root, path.join(dir, "src"), config, findings);
    collectStrictEmptyTestTreeFindings(
      root,
      dir,
      strictEmptyTestTrees,
      findings,
    );
  }

  return findings.filter((entry) => !isIgnored(entry.file, config));
}

function collectInlineSourceTestFindings(root, srcPath, config, findings) {
  if (!fs.existsSync(srcPath)) return;
  const files = collectFiles(
    root,
    [normalizeRel(root, srcPath)],
    config,
    (file) => isInlineTestSourceCandidate(file),
  );
  for (const file of files) {
    const text = fs.readFileSync(file, "utf8");
    const lines = text.split(/\r?\n/u);
    const pattern = inlineTestPatternForFile(file);
    for (const [index, line] of lines.entries()) {
      if (!pattern.test(line)) continue;
      findings.push(
        finding(
          root,
          file,
          index + 1,
          "TEST-2.2",
          `${normalizeRel(root, file)} contains inline test code; move it under an organized test root`,
          line,
        ),
      );
      break;
    }
  }
}

function isInlineTestSourceCandidate(file) {
  return /\.(?:rs|[cm]?[jt]sx?|py)$/u.test(file);
}

function inlineTestPatternForFile(file) {
  if (file.endsWith(".rs")) {
    return /#\s*\[\s*(?:cfg\s*\(\s*test\s*\)|(?:tokio::|async_std::)?test)\s*(?:\([^)]*\))?\s*\]|^\s*mod\s+tests?\b/u;
  }
  if (file.endsWith(".py")) {
    return /^\s*(?:def\s+test_\w+|class\s+Test\w*)\b/u;
  }
  return /^\s*(?:describe|it|test)\s*\(/u;
}

function collectStrictEmptyTestTreeFindings(
  root,
  projectRoot,
  strictEmptyTestTrees,
  findings,
) {
  if (!strictEmptyTestTrees) return;
  for (const treeRoot of ["tests", "proof"]) {
    const treePath = path.join(projectRoot, treeRoot);
    if (!fs.existsSync(treePath)) continue;
    collectEmptyPlaceholderTrees(root, treePath, findings);
  }
}

function collectEmptyPlaceholderTrees(root, treePath, findings) {
  const stats = fs.statSync(treePath);
  if (!stats.isDirectory()) return { hasRealFile: false, reported: false };

  const entries = fs.readdirSync(treePath, { withFileTypes: true });
  let hasRealFile = false;
  let childReported = false;
  let immediateFileCount = 0;
  let placeholderFileCount = 0;

  for (const entry of entries) {
    const childPath = path.join(treePath, entry.name);
    if (entry.isDirectory()) {
      const childResult = collectEmptyPlaceholderTrees(
        root,
        childPath,
        findings,
      );
      hasRealFile ||= childResult.hasRealFile;
      childReported ||= childResult.reported;
    } else if (entry.isFile() && entry.name !== ".gitkeep") {
      immediateFileCount += 1;
      hasRealFile = true;
    } else if (entry.isFile()) {
      immediateFileCount += 1;
      placeholderFileCount += 1;
    }
  }

  const reported = !hasRealFile && !childReported;
  if (reported) {
    const detail =
      immediateFileCount === 0
        ? `${normalizeRel(root, treePath)}: empty test/proof category tree has no files`
        : `${normalizeRel(root, treePath)}: empty test/proof category tree contains only ${placeholderFileCount} .gitkeep placeholder file${placeholderFileCount === 1 ? "" : "s"}`;
    findings.push(finding(root, treePath, 1, "TEST-2.1", detail, null));
  }
  return { hasRealFile, reported };
}

function collectSingleSourceContractFindings(
  root,
  explicitConfigPath,
  scope = { mode: "all" },
  enforcerConfig = {},
) {
  const configPath = resolveContractConfigPath(root, explicitConfigPath);
  if (!configPath) return [];
  const contractConfig = JSON.parse(fs.readFileSync(configPath, "utf8"));
  const findings = [];
  const scopedFiles =
    scope.mode === "all"
      ? null
      : scopeRelativeFiles(root, scope, enforcerConfig);
  enforceRequiredMirrorCoverage(
    root,
    configPath,
    contractConfig,
    scopedFiles,
    findings,
  );

  for (const rawContract of contractConfig.contracts ?? []) {
    const contract = loadContract(root, rawContract);
    const files =
      scopedFiles === null
        ? collectContractScanFiles(root, contract, enforcerConfig)
        : scopedFiles
            .filter((filePath) =>
              contract.scanRoots.some(
                (scanRoot) =>
                  filePath === scanRoot || filePath.startsWith(`${scanRoot}/`),
              ),
            )
            .filter((filePath) => !isNonBlockingContractPath(filePath))
            .filter((filePath) => !contract.allowedPaths.has(filePath));
    for (const rel of files) {
      const file = repoAbsolute(root, rel);
      if (!fs.existsSync(file)) continue;
      const text = fs.readFileSync(file, "utf8");
      for (const value of contract.values) {
        if (value.pattern.test(text)) {
          findings.push(
            finding(
              root,
              file,
              1,
              "CONTRACT-1.1",
              `copied ${contract.name}.${value.name} ${value.text}; import or derive from ${contract.ownerPath}`,
              null,
            ),
          );
        }
      }
    }
  }

  return findings;
}

function collectGeneratedArtifactFindings(
  root,
  config,
  scope = { mode: "all" },
  args = {},
) {
  const genericReport = runGenericScan({
    root,
    scope,
    config,
    languages: ["common"],
  });
  const tracked =
    args.tracked === true ||
    config.generatedArtifactsMode === "tracked" ||
    config.generatedArtifactsTracked === true;
  const findings = (genericReport.violations ?? []).filter(
    (entry) =>
      entry.ruleId === "GEN-1.1" || (!tracked && entry.ruleId === "GEN-1.2"),
  );
  if (!tracked) return findings;

  const trackedFiles = trackedScopeFiles(root, scope);
  for (const rel of trackedFiles) {
    if (!isGeneratedArtifactPath(rel)) continue;
    findings.push(
      genericFinding(
        root,
        repoAbsolute(root, rel),
        1,
        "GEN-1.2",
        "tracked generated artifact path is in source control",
        rel,
      ),
    );
  }
  return findings;
}

function collectSecretFindings(
  root,
  config,
  scope = { mode: "all" },
  args = {},
) {
  if (args.staged === true) {
    const files = stagedFiles(root);
    if (files.length === 0) return [];
    const genericReport = runGenericScan({
      root,
      scope: { mode: "files", files },
      config,
      languages: ["common"],
    });
    return (genericReport.violations ?? []).filter(
      (entry) => entry.ruleId === "SEC-1.1" || entry.ruleId === "SEC-1.2",
    );
  }
  const effectiveScope = scope;
  const genericReport = runGenericScan({
    root,
    scope: effectiveScope,
    config,
    languages: ["common"],
  });
  return (genericReport.violations ?? []).filter(
    (entry) => entry.ruleId === "SEC-1.1" || entry.ruleId === "SEC-1.2",
  );
}

function collectImportBoundaryFindings(root, config, scope = { mode: "all" }) {
  const policies = config.importBoundaryPolicies ?? [];
  if (policies.length === 0) return [];
  const files = scopeFilesByExtensions(
    root,
    scope,
    config,
    new Set([".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs", ".mts", ".cts"]),
  );
  const findings = [];
  for (const file of files) {
    const rel = normalizeRel(root, file);
    const text = fs.readFileSync(file, "utf8");
    const lines = text.split(/\r?\n/u);
    for (const policy of policies) {
      if (!isUnderRoots(rel, policy.roots ?? [])) continue;
      lines.forEach((line, index) => {
        const spec = importSpecifier(line);
        if (!spec) return;
        const forbidden = matchesAnyGlob(spec, policy.forbiddenImports ?? []);
        const allowed = matchesAnyGlob(spec, policy.allowedImports ?? []);
        if (!forbidden || allowed) return;
        findings.push(
          finding(
            root,
            file,
            index + 1,
            "TS-4.1",
            policy.message ?? `import "${spec}" crosses a configured boundary`,
            line,
          ),
        );
      });
    }
  }
  return findings;
}

function collectDependencyPolicyFindings(root, config) {
  const findings = [];
  const packageLockPath = path.join(root, "package-lock.json");
  if (fs.existsSync(packageLockPath)) {
    const audit = spawnInRoot(root, "npm", [
      "audit",
      "--audit-level=high",
      "--json",
    ]);
    if (audit.status !== 0) {
      findings.push(
        finding(
          root,
          packageLockPath,
          1,
          "NPM-1.9",
          "npm audit reported high-or-higher vulnerabilities",
          compactProcessOutput(audit),
        ),
      );
      findings.push(
        finding(
          root,
          packageLockPath,
          1,
          "DEP-1.1",
          "npm audit reported high-or-higher vulnerabilities",
          compactProcessOutput(audit),
        ),
      );
    }
    const lock = JSON.parse(fs.readFileSync(packageLockPath, "utf8"));
    const allowed = new Set(
      config.allowedExternalLicenses ?? [...DEFAULT_ALLOWED_LICENSES],
    );
    for (const [lockPath, packageEntry] of Object.entries(
      lock.packages ?? {},
    )) {
      if (!lockPath.includes("node_modules")) continue;
      const packageName = lockPath.split("node_modules/").at(-1);
      if (
        packageName?.startsWith("@ocentra-parent/") ||
        packageName?.startsWith("@ocentra/")
      )
        continue;
      const license = packageEntry.license;
      if (typeof license !== "string" || !allowed.has(license)) {
        findings.push(
          finding(
            root,
            packageLockPath,
            1,
            "NPM-1.10",
            `${lockPath}: ${license ?? "MISSING"}`,
            null,
          ),
        );
        findings.push(
          finding(
            root,
            packageLockPath,
            1,
            "DEP-1.2",
            `${lockPath}: ${license ?? "MISSING"}`,
            null,
          ),
        );
      }
    }
  }

  if (fs.existsSync(path.join(root, "Cargo.lock"))) {
    const cargoAudit = spawnInRoot(root, "cargo", [
      "audit",
      "--deny",
      "warnings",
    ]);
    if (cargoAudit.error?.code === "ENOENT") {
      findings.push(
        finding(
          root,
          path.join(root, "Cargo.lock"),
          1,
          "DEP-1.1",
          "cargo audit is not installed",
          "Install cargo-audit or disable this check in project policy.",
        ),
      );
    } else if (cargoAudit.status !== 0) {
      findings.push(
        finding(
          root,
          path.join(root, "Cargo.lock"),
          1,
          "DEP-1.1",
          "cargo audit reported advisories",
          compactProcessOutput(cargoAudit),
        ),
      );
    }
  }

  return findings;
}

function collectPackageDeterminismFindings(root) {
  const findings = [];
  const packageJsonPath = path.join(root, "package.json");
  if (!fs.existsSync(packageJsonPath)) return findings;

  let manifest;
  try {
    manifest = JSON.parse(fs.readFileSync(packageJsonPath, "utf8"));
  } catch (error) {
    findings.push(
      finding(
        root,
        packageJsonPath,
        1,
        "NPM-1.3",
        `package.json is not valid JSON: ${error instanceof Error ? error.message : String(error)}`,
        null,
      ),
    );
    return findings;
  }

  if (!fs.existsSync(path.join(root, "package-lock.json"))) {
    findings.push(
      finding(
        root,
        packageJsonPath,
        1,
        "NPM-1.1",
        "package-lock.json is missing",
        null,
      ),
    );
  }

  const packageLockPath = path.join(root, "package-lock.json");
  if (fs.existsSync(packageLockPath)) {
    const lock = JSON.parse(fs.readFileSync(packageLockPath, "utf8"));
    for (const [lockPath, packageEntry] of Object.entries(lock.packages ?? {})) {
      if (packageEntry?.hasInstallScript === true) {
        findings.push(
          finding(
            root,
            packageLockPath,
            1,
            "NPM-1.6",
            `${lockPath || "."} declares an install script`,
            null,
          ),
        );
      }
    }
  }

  if (!Array.isArray(manifest.files) || manifest.files.length === 0) {
    findings.push(
      finding(
        root,
        packageJsonPath,
        lineForJsonKey(packageJsonPath, "files"),
        "NPM-1.13",
        "package.json must declare an explicit files allowlist for publishing",
        null,
      ),
    );
  }

  for (const [name, target] of Object.entries(manifest.bin ?? {})) {
    const targetPath = path.join(root, String(target));
    if (!fs.existsSync(targetPath)) {
      findings.push(
        finding(
          root,
          packageJsonPath,
          lineForJsonKey(packageJsonPath, name),
          "NPM-1.14",
          `bin ${name} points at missing path ${target}`,
          null,
        ),
      );
    }
  }

  for (const target of packageExportTargets(manifest.exports)) {
    const targetPath = path.join(root, target);
    const exists = target.includes("*")
      ? fs.existsSync(path.join(root, target.split("*")[0]))
      : fs.existsSync(targetPath);
    if (!exists) {
      findings.push(
        finding(
          root,
          packageJsonPath,
          lineForJsonKey(packageJsonPath, "exports"),
          "NPM-1.15",
          `exports target ${target} does not exist`,
          null,
        ),
      );
    }
  }

  if (!/^npm@\d+\.\d+\.\d+$/u.test(String(manifest.packageManager ?? ""))) {
    findings.push(
      finding(
        root,
        packageJsonPath,
        lineForJsonKey(packageJsonPath, "packageManager"),
        "NPM-1.4",
        "packageManager must pin an exact npm version, for example npm@11.7.0",
        null,
      ),
    );
  }

  const nodeEngine = manifest.engines?.node;
  if (!isBoundedNodeEngine(nodeEngine)) {
    findings.push(
      finding(
        root,
        packageJsonPath,
        lineForJsonKey(packageJsonPath, "engines"),
        "NPM-1.5",
        `engines.node must be bounded; found ${nodeEngine ?? "MISSING"}`,
        null,
      ),
    );
  }

  for (const [sectionName, dependencies] of dependencySections(manifest)) {
    for (const [dependencyName, version] of Object.entries(dependencies)) {
      const versionText = String(version ?? "").trim();
      if (/^(?:git\+|github:|git:|https?:\/\/.*\.git)/iu.test(versionText)) {
        findings.push(
          finding(
            root,
            packageJsonPath,
            lineForJsonKey(packageJsonPath, dependencyName),
            "NPM-1.7",
            `${sectionName}.${dependencyName} uses git dependency ${versionText}`,
            null,
          ),
        );
        continue;
      }
      if (/^(?:file:|link:|workspace:)|^\.\.?\//iu.test(versionText)) {
        findings.push(
          finding(
            root,
            packageJsonPath,
            lineForJsonKey(packageJsonPath, dependencyName),
            "NPM-1.8",
            `${sectionName}.${dependencyName} uses file/path dependency ${versionText}`,
            null,
          ),
        );
        continue;
      }
      if (isSuspiciousDependencyName(dependencyName)) {
        findings.push(
          finding(
            root,
            packageJsonPath,
            lineForJsonKey(packageJsonPath, dependencyName),
            "NPM-1.11",
            `${sectionName}.${dependencyName} has suspicious package name`,
            null,
          ),
        );
      }
      if (!isDeterministicDependencyVersion(version)) {
        findings.push(
          finding(
            root,
            packageJsonPath,
            lineForJsonKey(packageJsonPath, dependencyName),
            "NPM-1.3",
            `${sectionName}.${dependencyName} uses non-deterministic version ${version}`,
            null,
          ),
        );
      }
    }
  }

  return findings;
}

function packageExportTargets(exportsField) {
  const targets = [];
  const visit = (value) => {
    if (typeof value === "string") {
      if (value.startsWith("./")) targets.push(value.slice(2));
      return;
    }
    if (Array.isArray(value)) {
      value.forEach(visit);
      return;
    }
    if (value && typeof value === "object") {
      Object.values(value).forEach(visit);
    }
  };
  visit(exportsField);
  return uniqueSorted(targets);
}

function collectCiIntegrityFindings(root) {
  const findings = [];
  const workflowRoot = path.join(root, ".github", "workflows");
  const packageText = fs.existsSync(path.join(root, "package.json"))
    ? fs.readFileSync(path.join(root, "package.json"), "utf8")
    : "";
  const localCiText = fs.existsSync(path.join(root, "scripts", "ci-local.mjs"))
    ? fs.readFileSync(path.join(root, "scripts", "ci-local.mjs"), "utf8")
    : "";
  const ciSurfaceText = `${packageText}\n${localCiText}`;
  if (!fs.existsSync(workflowRoot)) {
    return findings;
  }
  for (const file of collectSourceFiles(workflowRoot, [".yml", ".yaml"])) {
    const text = fs.readFileSync(file, "utf8");
    const lines = text.split(/\r?\n/u);
    if (!/\bpull_request\s*:/u.test(text) || !/\bpush\s*:/u.test(text) || !/\bbranches\s*:\s*\[[^\]]*\bmain\b/u.test(text)) {
      findings.push(
        finding(
          root,
          file,
          1,
          "CI-1.15",
          "workflow must run on pull_request and pushes to main",
          null,
        ),
      );
    }
    if (!/^permissions\s*:/mu.test(text)) {
      findings.push(
        finding(root, file, 1, "CI-1.14", "workflow is missing explicit permissions block", null),
      );
    }
    const hasUbuntu = /\bubuntu-latest\b/u.test(text);
    const hasWindows = /\bwindows-latest\b/u.test(text);
    const hasMacos = /\bmacos-latest\b/u.test(text);
    if (!(hasUbuntu && hasWindows && hasMacos)) {
      findings.push(
        finding(
          root,
          file,
          1,
          "CI-1.16",
          "workflow matrix must include ubuntu-latest, windows-latest, and macos-latest",
          null,
        ),
      );
    }
    lines.forEach((line, index) => {
      if (/continue-on-error\s*:\s*true/u.test(line)) {
        findings.push(
          finding(root, file, index + 1, "CI-1.11", "continue-on-error bypass found", line),
        );
      }
      if (/\|\|\s*true\b/u.test(line)) {
        findings.push(
          finding(root, file, index + 1, "CI-1.12", "shell exit-code bypass found", line),
        );
      }
      if (/\brun\s*:\s*npm\s+install\b/u.test(line)) {
        findings.push(
          finding(root, file, index + 1, "CI-1.1", "workflow uses npm install instead of npm ci", line),
        );
        findings.push(
          finding(root, file, index + 1, "NPM-1.2", "workflow uses npm install instead of npm ci", line),
        );
      }
      const action = line.match(/^\s*-\s+uses\s*:\s*([^\s#]+)\s*$/u);
      if (action && !isPinnedActionReference(action[1])) {
        findings.push(
          finding(root, file, index + 1, "CI-1.13", `workflow action is not pinned by major or SHA: ${action[1]}`, line),
        );
      }
      if (/\brust-rules\b/u.test(line) && !/compatibility alias/iu.test(line)) {
        findings.push(
          finding(root, file, index + 1, "CI-1.18", "workflow calls legacy rust-rules command directly", line),
        );
      }
    });
    if (/\bpackage-lock\.json\b|package\.json\b|npm\b/u.test(text) && !/\brun\s*:\s*npm\s+ci\b/u.test(text)) {
      findings.push(
        finding(root, file, 1, "CI-1.1", "workflow does not run npm ci", null),
      );
      findings.push(
        finding(root, file, 1, "NPM-1.2", "workflow does not run npm ci", null),
      );
    }
    const usesLocalParity = /\brun\s*:\s*npm\s+run\s+ci:local\b/u.test(text);
    if (!usesLocalParity) {
      findings.push(
        finding(root, file, 1, "CI-1.17", "workflow does not run npm run ci:local parity gate", null),
      );
    }
    const ciText = usesLocalParity ? `${text}\n${ciSurfaceText}` : text;
    for (const requirement of CI_COMMAND_REQUIREMENTS) {
      if (!requirement.pattern.test(ciText)) {
        findings.push(
          finding(root, file, 1, requirement.ruleId, requirement.detail, null),
        );
      }
    }
  }
  const branchProtectionPath = path.join(root, "docs", "BRANCH_PROTECTION.md");
  if (!fs.existsSync(branchProtectionPath)) {
    findings.push(
      finding(root, root, 1, "CI-1.19", "docs/BRANCH_PROTECTION.md is required", null),
    );
    findings.push(
      finding(root, root, 1, "CI-1.20", "required checks policy must include Enforcer", null),
    );
  } else {
    const branchProtection = fs.readFileSync(branchProtectionPath, "utf8");
    if (!/\bbranch protection\b|\brequired checks\b/iu.test(branchProtection)) {
      findings.push(
        finding(root, branchProtectionPath, 1, "CI-1.19", "branch protection document must describe required checks", null),
      );
    }
    if (!/\bocentra enforcer\b|\bci:local\b|\benforcer\b/iu.test(branchProtection)) {
      findings.push(
        finding(root, branchProtectionPath, 1, "CI-1.20", "required checks document must include Enforcer", null),
      );
    }
  }
  return findings;
}

const CI_COMMAND_REQUIREMENTS = [
  {
    ruleId: "CI-1.2",
    pattern: /\bnpm(?:Step)?\(\s*\[\s*["']test["']\s*\]\s*\)|\bnpm\s+test\b|\bnpm\s+run\s+test\b/u,
    detail: "CI parity gate does not run npm test",
  },
  {
    ruleId: "CI-1.3",
    pattern: /\btest:policy\b|\brule-coverage\b|\benforcer:coverage\b/u,
    detail: "CI parity gate does not run rule/policy tests",
  },
  {
    ruleId: "CI-1.4",
    pattern: /\btest:multilang\b|\brust:rules:scan\b|\benforcer:self\b/u,
    detail: "CI parity gate does not run multi-language/self scan coverage",
  },
  {
    ruleId: "CI-1.5",
    pattern: /\btest:mcp\b|\bmcp:smoke\b/u,
    detail: "CI parity gate does not run MCP tests/smoke checks",
  },
  {
    ruleId: "CI-1.6",
    pattern: /\benforcer:self\b|\brust:rules:scan\b/u,
    detail: "CI parity gate does not run Enforcer self-scan",
  },
  {
    ruleId: "CI-1.7",
    pattern: /\benforcer:verify\b|\bpolicy-integrity\b|\benforcer:policy\b/u,
    detail: "CI parity gate does not run schema/policy validation",
  },
  {
    ruleId: "CI-1.8",
    pattern: /\bsecrets\b|\bsecret scan\b|\bscan-staged-secrets\b/u,
    detail: "CI parity gate does not run a secret scan",
  },
  {
    ruleId: "CI-1.9",
    pattern: /\bdependency-policy\b/u,
    detail: "CI parity gate does not run dependency policy",
  },
  {
    ruleId: "CI-1.10",
    pattern: /\bsbom\b/u,
    detail: "CI parity gate does not run SBOM check",
  },
];

function collectRepoGovernanceFindings(root) {
  const findings = [];
  const codeownersPath = findCodeownersPath(root);
  if (!codeownersPath) {
    findings.push(
      finding(root, root, 1, "REPO-1.1", "CODEOWNERS file is missing", null),
    );
  } else {
    const codeowners = fs.readFileSync(codeownersPath, "utf8");
    for (const [ruleId, requiredPatterns] of [
      ["REPO-1.2", ["rules/**"]],
      ["REPO-1.3", ["scripts/**", "src/**", "mcp/**"]],
      ["REPO-1.4", ["schemas/**", "profiles/**", "adapters/**"]],
      ["REPO-1.5", [".github/workflows/**"]],
    ]) {
      const missing = requiredPatterns.filter(
        (pattern) => !codeownersIncludesPattern(codeowners, pattern),
      );
      if (missing.length > 0) {
        findings.push(
          finding(
            root,
            codeownersPath,
            1,
            ruleId,
            `CODEOWNERS missing protection for ${missing.join(", ")}`,
            null,
          ),
        );
      }
    }
  }

  const packageFindings = collectPackageDeterminismFindings(root);
  for (const packageFinding of packageFindings) {
    const ruleId =
      packageFinding.ruleId === "NPM-1.1"
        ? "REPO-1.6"
        : packageFinding.ruleId === "NPM-1.4"
          ? "REPO-1.7"
          : packageFinding.ruleId === "NPM-1.5"
            ? "REPO-1.8"
            : packageFinding.ruleId === "NPM-1.3"
              ? "REPO-1.9"
              : packageFinding.ruleId;
    findings.push({
      ...packageFinding,
      ruleId,
      title: CHECK_RULES[ruleId]?.title ?? packageFinding.title,
      snippet: CHECK_RULES[ruleId]?.snippet ?? packageFinding.snippet,
    });
  }

  for (const requiredDoc of REPO_GOVERNANCE_DOCS) {
    const docPath = path.join(root, requiredDoc.path);
    if (!fs.existsSync(docPath)) {
      findings.push(
        finding(root, root, 1, requiredDoc.ruleId, `${requiredDoc.path} is required`, null),
      );
      continue;
    }
    const text = fs.readFileSync(docPath, "utf8");
    if (requiredDoc.pattern && !requiredDoc.pattern.test(text)) {
      findings.push(
        finding(
          root,
          docPath,
          1,
          requiredDoc.ruleId,
          `${requiredDoc.path} is missing required governance content`,
          null,
        ),
      );
    }
  }

  return findings;
}

const REPO_GOVERNANCE_DOCS = [
  {
    ruleId: "REPO-1.10",
    path: "LICENSE",
    pattern: /\b(?:MIT|Apache|BSD|ISC|MPL|GPL|Proprietary|Copyright)\b/u,
  },
  {
    ruleId: "REPO-1.11",
    path: "SECURITY.md",
    pattern: /\b(?:vulnerability|security|report)\b/iu,
  },
  {
    ruleId: "REPO-1.12",
    path: "CONTRIBUTING.md",
    pattern: /\b(?:rule|validator|fixture|registry|schema)\b/iu,
  },
  {
    ruleId: "REPO-1.13",
    path: "CHANGELOG.md",
    pattern: /\b(?:rule|validator|enforcer|change)\b/iu,
  },
  {
    ruleId: "REPO-1.14",
    path: path.join("docs", "RELEASE_POLICY.md"),
    pattern: /\b(?:tag|sign|publish|release)\b/iu,
  },
];

const POLICY_CRITICAL_PATTERNS = [
  "rules/**",
  "schemas/**",
  "profiles/**",
  "scripts/**",
  "src/policy*",
  "src/checks*",
  "src/generic-scanners*",
  "src/source-policy-scanners*",
  "mcp/**",
  ".github/workflows/**",
  "package.json",
  "package-lock.json",
  "Cargo.toml",
  "Cargo.lock",
  "deny.toml",
  "rust-toolchain.toml",
];

const MUTATION_RISK_REQUIRED_PROOFS = [
  "ocentra-enforcer scan --workspace",
  "ocentra-enforcer check rule-coverage --root <repo>",
  "ocentra-enforcer check policy-integrity --root <repo>",
  "ocentra-enforcer check ci-integrity --root <repo>",
  "ocentra-enforcer check repo-governance --root <repo>",
  "npm test",
  "npm run test:mcp",
];

function collectMutationRiskFindings(root, scope = { mode: "all" }) {
  const changedFiles = changedFilesForMutationRisk(root, scope);
  const criticalFiles = changedFiles.filter((file) =>
    matchesAnyGlob(normalizeRel(root, file), POLICY_CRITICAL_PATTERNS),
  );
  return criticalFiles.map((file) =>
    finding(
      root,
      file,
      1,
      "ENF-2.1",
      `policy-critical file changed: ${normalizeRel(root, file)}`,
      `Required proof set: ${MUTATION_RISK_REQUIRED_PROOFS.join("; ")}`,
    ),
  );
}

function changedFilesForMutationRisk(root, scope = { mode: "all" }) {
  if (scope.mode === "files") {
    return uniqueSorted((scope.files ?? []).map((file) => repoAbsolute(root, file)));
  }
  if (scope.mode === "diff") {
    return diffFiles(root, scope.base, scope.head);
  }
  return gitStatusChangedFiles(root);
}

function gitStatusChangedFiles(root) {
  const output = spawnSync("git", ["status", "--porcelain"], {
    cwd: root,
    encoding: "utf8",
    shell: false,
  });
  if (output.status !== 0) return [];
  return uniqueSorted(
    String(output.stdout ?? "")
      .split(/\r?\n/u)
      .map((line) => line.trimEnd())
      .filter(Boolean)
      .map((line) => {
        const rawPath = line.slice(3).trim();
        const renamedPath = rawPath.includes(" -> ")
          ? rawPath.split(" -> ").at(-1)
          : rawPath;
        return repoAbsolute(root, renamedPath.replace(/^"|"$/gu, ""));
      }),
  );
}

function findCodeownersPath(root) {
  for (const rel of [
    "CODEOWNERS",
    ".github/CODEOWNERS",
    "docs/CODEOWNERS",
  ]) {
    const candidate = path.join(root, rel);
    if (fs.existsSync(candidate)) return candidate;
  }
  return null;
}

function codeownersIncludesPattern(text, pattern) {
  return text
    .split(/\r?\n/u)
    .map((line) => line.trim())
    .filter((line) => line !== "" && !line.startsWith("#"))
    .some((line) => line.split(/\s+/u)[0] === pattern);
}

function dependencySections(manifest) {
  return [
    ["dependencies", manifest.dependencies],
    ["devDependencies", manifest.devDependencies],
    ["optionalDependencies", manifest.optionalDependencies],
    ["peerDependencies", manifest.peerDependencies],
  ].filter(([, value]) => value && typeof value === "object");
}

function isDeterministicDependencyVersion(value) {
  const version = String(value ?? "").trim();
  return (
    /^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$/u.test(version) ||
    /^npm:[@A-Za-z0-9._/-]+@\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$/u.test(version)
  );
}

function isPinnedActionReference(actionRef) {
  const ref = String(actionRef ?? "").split("@").at(1);
  if (!ref) return false;
  if (/^[a-f0-9]{40}$/iu.test(ref)) return true;
  if (/^v\d+(?:\.\d+){0,2}$/u.test(ref)) return true;
  return false;
}

function isSuspiciousDependencyName(name) {
  const normalized = String(name ?? "").toLowerCase();
  return /(?:ocentra|openai|effect|typescript|eslint|vitest|playwright|duckdb)[_-](?:js|lib|safe|new|next)$/u.test(normalized);
}

function isBoundedNodeEngine(value) {
  const engine = String(value ?? "").trim();
  return (
    /^>=\d+(?:\.\d+)?(?:\.\d+)?\s+<\d+(?:\.\d+)?(?:\.\d+)?$/u.test(engine) ||
    /^\d+\.\d+\.\d+$/u.test(engine)
  );
}

function lineForJsonKey(filePath, key) {
  if (!fs.existsSync(filePath)) return 1;
  const lines = fs.readFileSync(filePath, "utf8").split(/\r?\n/u);
  const pattern = new RegExp(`"${escapeRegExp(key)}"\\s*:`, "u");
  const index = lines.findIndex((line) => pattern.test(line));
  return index === -1 ? 1 : index + 1;
}

function runSbomCheck(root, args) {
  const findings = [];
  const outputRoot = repoAbsolute(root, args.output ?? "target/security");
  if (args.dryRun) return [];
  fs.mkdirSync(outputRoot, { recursive: true });

  if (fs.existsSync(path.join(root, "package.json"))) {
    const npmSbom = spawnInRoot(root, "npm", [
      "sbom",
      "--sbom-format=cyclonedx",
    ]);
    if (npmSbom.status !== 0) {
      findings.push(
        finding(
          root,
          path.join(root, "package.json"),
          1,
          "NPM-1.12",
          "npm SBOM generation failed",
          compactProcessOutput(npmSbom),
        ),
      );
      findings.push(
        finding(
          root,
          path.join(root, "package.json"),
          1,
          "SBOM-1.1",
          "npm SBOM generation failed",
          compactProcessOutput(npmSbom),
        ),
      );
    } else
      fs.writeFileSync(
        path.join(outputRoot, "npm-sbom.cdx.json"),
        npmSbom.stdout,
        "utf8",
      );
  }

  if (fs.existsSync(path.join(root, "Cargo.toml"))) {
    const cargoMetadata = spawnInRoot(root, "cargo", [
      "metadata",
      "--format-version=1",
      "--locked",
    ]);
    if (cargoMetadata.status !== 0)
      findings.push(
        finding(
          root,
          path.join(root, "Cargo.toml"),
          1,
          "SBOM-1.1",
          "cargo metadata generation failed",
          compactProcessOutput(cargoMetadata),
        ),
      );
    else
      fs.writeFileSync(
        path.join(outputRoot, "cargo-metadata.json"),
        cargoMetadata.stdout,
        "utf8",
      );
  }

  return findings;
}

function collectAiRuleIndexFindings(root, config) {
  const findings = [];
  const agentsPath = path.join(root, "AGENTS.md");
  const rulesRoot = path.join(root, ".ocentra-ai", "rules");
  if (!fs.existsSync(agentsPath) || !fs.existsSync(rulesRoot)) return findings;

  const ruleFiles = fs
    .readdirSync(rulesRoot)
    .filter((entry) => entry.endsWith(".md") || entry.endsWith(".mdc"))
    .map((entry) => path.join(rulesRoot, entry));
  const indexFile =
    ruleFiles.find((file) => /rules|index/iu.test(path.basename(file))) ??
    ruleFiles[0];
  if (!indexFile) return findings;

  const agentsText = fs.readFileSync(agentsPath, "utf8");
  const indexText = fs.readFileSync(indexFile, "utf8");
  const indexRel = normalizeRel(root, indexFile);
  if (
    !agentsText.includes(indexRel) &&
    !agentsText.includes(indexRel.replaceAll("/", "\\"))
  ) {
    findings.push(
      finding(
        root,
        agentsPath,
        1,
        "AI-1.1",
        `AGENTS.md must reference ${indexRel}`,
        null,
      ),
    );
  }

  for (const ruleFile of ruleFiles) {
    const rel = normalizeRel(root, ruleFile);
    const lineCount = countLines(fs.readFileSync(ruleFile, "utf8"));
    if (
      ruleFile !== indexFile &&
      !indexText.includes(normalizeRel(rulesRoot, ruleFile))
    ) {
      findings.push(
        finding(
          root,
          ruleFile,
          1,
          "AI-1.1",
          `${rel} is not linked from ${indexRel}`,
          null,
        ),
      );
    }
    const maxLines = config.agentRuleMaxLines ?? 220;
    if (lineCount > maxLines) {
      findings.push(
        finding(
          root,
          ruleFile,
          maxLines + 1,
          "AI-1.1",
          `${rel} has ${lineCount} lines; split rule files above ${maxLines}`,
          null,
        ),
      );
    }
  }
  return findings;
}

function collectRuleCoverageFindings(root, _config, args = {}) {
  const packRoot = resolvePackRoot(root, args);
  const registryPath = path.join(packRoot, "rules", "rules.json");
  const findings = [];
  if (!fs.existsSync(registryPath)) {
    findings.push(
      finding(
        root,
        root,
        1,
        "ENF-1.1",
        `rule registry is missing: ${normalizeRel(root, registryPath)}`,
        null,
      ),
    );
    return findings;
  }

  const registry = JSON.parse(fs.readFileSync(registryPath, "utf8"));
  const rules = Array.isArray(registry.rules) ? registry.rules : [];
  const registryIds = new Set();
  const duplicateIds = new Set();
  for (const rule of rules) {
    const id = String(rule.id ?? "").toUpperCase();
    if (registryIds.has(id)) duplicateIds.add(id);
    registryIds.add(id);
  }
  for (const id of [...duplicateIds].sort()) {
    findings.push(
      finding(root, registryPath, 1, "ENF-1.6", `duplicate rule ID ${id}`, null),
    );
  }

  const fixtureEvidence = collectFixtureEvidence(packRoot);
  for (const rule of rules) {
    collectRegistryRuleMetadataFindings(root, packRoot, registryPath, rule, findings);
    collectRegistryDocFindings(root, packRoot, rule, findings);
    if (
      rule.validator !== "review" &&
      (rule.requiresFailFixture || rule.requiresPassFixture) &&
      !fixtureEvidence.has(rule.id)
    ) {
      findings.push(
        finding(
          root,
          registryPath,
          1,
          "ENF-1.4",
          `${rule.id} requires fixture/test evidence but no test references that rule ID`,
          null,
        ),
      );
    }
  }

  const docRuleIds = collectRoutedDocRuleIds(packRoot);
  for (const id of docRuleIds) {
    if (!registryIds.has(id)) {
      findings.push(
        finding(
          root,
          path.join(packRoot, "rules"),
          1,
          "ENF-1.1",
          `${id} is mentioned in routed rule docs but missing from rules/rules.json`,
          null,
        ),
      );
    }
  }

  const scannerRuleIds = collectScannerRuleIds(packRoot);
  for (const id of scannerRuleIds) {
    if (!registryIds.has(id)) {
      findings.push(
        finding(
          root,
          packRoot,
          1,
          "ENF-1.3",
          `${id} is emitted or referenced by scanner/check source but missing from rules/rules.json`,
          null,
        ),
      );
    }
  }

  collectRuleIdLockFindings(root, packRoot, registryPath, registryIds, findings);
  collectMetadataDriftFindings(root, registryPath, rules, findings);
  collectDeterministicOrderingFindings(root, registryPath, rules, findings);
  collectValidatorNetworkFindings(root, packRoot, findings);
  collectEnforcerBypassFindings(root, packRoot, findings);

  return findings;
}

const HARNESS_CONTRACT_SPECS = [
  ["HAR-2.1", "src/harness.mjs", [/\brunId\b/u, /\bcommand\b/u, /\bcwd\b/u, /\bstartedAt\b/u, /\bendedAt\b/u, /\bexitCode\b/u]],
  ["HAR-2.2", "src/harness.mjs", [/\bmaxArtifactBytes\b/u, /\bredactSecrets\b/u]],
  ["HAR-2.3", "src/harness.mjs", [/\bsortDiagnostics\b/u, /localeCompare/u]],
  ["HAR-2.4", "src/harness.mjs", [/\bparserDiagnostic\b/u, /HAR-2\.4/u]],
  ["HAR-2.5", "src/harness.mjs", [/\bcompiler-message\b/u, /\brustMessageToDiagnostic\b/u]],
  ["HAR-2.6", "src/harness.mjs", [/\bfilePath\b/u, /\bmessages\b/u, /eslint/u]],
  ["HAR-2.7", "src/harness.mjs", [/\bgeneralDiagnostics\b/u, /pyright/u, /ruff|mypy|pytest/u]],
  ["HAR-2.8", "src/harness.mjs", [/\bparsed\.runs\b/u, /\bSARIF result\b/u]],
  ["HAR-2.9", "src/harness.mjs", [/\bexport function lastFailure\b/u, /\brunDiagnostics\b/u]],
  ["HAR-2.10", "src/harness.mjs", [/\breadArtifact\b/u, /\bisInsideRoot\b/u]],
  ["HAR-2.11", "src/harness.mjs", [/\bpinned\b/u, /entry\.pinned === true/u]],
  ["HAR-2.12", "src/harness.mjs", [/\bok: exitCode === 0/u, /status: exitCode === 0 \? 'passed' : 'failed'/u]],
  ["HAR-2.13", "schemas/json/run-report.schema.json", [/"properties"/u]],
  ["HAR-2.14", "src/harness.mjs", [/\bredactSecrets\b/u, /\[REDACTED\]/u]],
  ["HAR-2.15", "src/harness.mjs", [/shell: false/u]],
];

const PROOF_CONTRACT_SPECS = [
  ["PROOF-1.1", "src/proof.mjs", [/\bprReady\b/u, /\bNo proof run found\b/u]],
  ["PROOF-1.2", "src/proof.mjs", [/\bgitState\b/u, /\bfiles\b/u, /\bprofile\b/u]],
  ["PROOF-1.3", "src/proof.mjs", [/manual-required/u, /manual-artifact/u]],
  ["PROOF-1.4", "src/proof.mjs", [/missing|required artifacts|failedArtifacts/u, /\bbyteLength\b/u]],
  ["PROOF-1.5", "src/proof.mjs", [/\bsha256\b/u, /hash-match|importedHashes|legacyHashes/u]],
  ["PROOF-1.6", "src/proof.mjs", [/\bdirty-worktree\b/u, /\ballowDirty\b/u]],
  ["PROOF-1.7", "src/proof.mjs", [/waived|unavailable|manual-required/u]],
  ["PROOF-1.8", "src/proof.mjs", [/command\.length === 0/u, /\bNo executable command\b/u]],
  ["PROOF-1.9", "src/proof.mjs", [/\bcommand:\s*\[/u, /shell: false/u]],
  ["PROOF-1.10", "proof/proofs.json", [/"docs"/u]],
  ["PROOF-1.11", "src/proof.mjs", [/\bcapabilities\b/u, /\bcapability\b/u]],
  ["PROOF-1.12", "src/proof.mjs", [/android-device|ios-device|manual-required/u]],
  ["PROOF-1.13", "src/proof.mjs", [/claimsProved/u, /claimsNotProved/u]],
  ["PROOF-1.14", "src/proof.mjs", [/diagnosticLimit/u, /slice\(0/u]],
  ["PROOF-1.15", "src/proof.mjs", [/\bredactSecrets\b/u, /\[REDACTED\]/u]],
];

const MCP_CONTRACT_SPECS = [
  ["MCP-1.1", "mcp/rust-rules-mcp.mjs", [/ocentra_enforcer_scan/u, /ocentra_enforcer_check/u]],
  ["MCP-1.2", "mcp/rust-rules-mcp.mjs", [/decodeScanToolArguments/u, /decodeCheckToolArguments/u, /decodeCoordinationToolArguments/u]],
  ["MCP-1.3", "mcp/rust-rules-mcp.mjs", [/additionalProperties:\s*false/u]],
  ["MCP-1.4", "mcp/rust-rules-mcp.mjs", [/summaryOnly/u, /includeScope/u]],
  ["MCP-1.5", "mcp/rust-rules-mcp.mjs", [/diagnosticLimit/u, /Math\.trunc\(args\.diagnosticLimit\)/u]],
  ["MCP-1.6", "mcp/rust-rules-mcp.mjs", [/shouldBlockStaleMcpTool/u, /COORDINATION_WRITE_TOOLS/u]],
  ["MCP-1.7", "mcp/rust-rules-mcp.mjs", [/ocentra_enforcer_mcp_status/u, /buildMcpFingerprint/u]],
  ["MCP-1.8", "mcp/rust-rules-mcp.mjs", [/ocentra_enforcer_explain/u, /runCli\("explain"/u]],
  ["MCP-1.9", "mcp/rust-rules-mcp.mjs", [/ocentra_enforcer_route/u, /buildRouteReport/u]],
  ["MCP-1.10", "mcp/rust-rules-mcp.mjs", [/runCli\(decoded\.cargo \? "cargo" : "scan"/u, /read-only|scan/u]],
  ["MCP-1.11", "mcp/rust-rules-mcp.mjs", [/ocentra_enforcer_coordination_claim/u, /ocentra_enforcer_coordination_release/u]],
  ["MCP-1.12", "mcp/rust-rules-mcp.mjs", [/function toolError/u, /JSON\.stringify\(body/u]],
];

const SCANNER_CONTRACT_SPECS = [
  ["SCAN-1.1", "src/source-policy-scanners.mjs", [/maskJavaScriptLine/u]],
  ["SCAN-1.2", "src/source-policy-scanners.mjs", [/maskJavaScriptLine/u, /\/\/|\/\*/u]],
  ["SCAN-1.3", "src/generic-scanners.mjs", [/ts-ignore|noqa|type:\s*ignore/u]],
  ["SCAN-1.4", "src/checks.mjs", [/split\(/u, /\\r\?\\n/u]],
  ["SCAN-1.5", "src/path-utils.mjs", [/toPosix/u, /normalizeRel/u]],
  ["SCAN-1.6", "src/path-utils.mjs", [/repoAbsolute/u, /path\.resolve/u]],
  ["SCAN-1.7", "src/path-utils.mjs", [/path\.isAbsolute/u, /path\.resolve/u]],
  ["SCAN-1.8", "src/path-utils.mjs", [/lstatSync/u, /isSymbolicLink/u]],
  ["SCAN-1.9", "src/path-utils.mjs", [/isSymbolicLink/u]],
  ["SCAN-1.10", "scripts/rust-rules.mjs", [/sortFindings/u, /compareFindings/u]],
  ["SCAN-1.11", "src/checks.mjs", [/maxArtifactBytes|64 \* 1024 \* 1024|maxBuffer/u]],
  ["SCAN-1.12", "src/generic-scanners.mjs", [/binary|readFileSync/u]],
  ["SCAN-1.13", "src/checks.mjs", [/try\s*\{/u, /catch/u]],
  ["SCAN-1.14", "src/routing.mjs", [/routeFamilyKeysForFile/u, /return \[\]/u]],
  ["SCAN-1.15", "scripts/rust-rules.mjs", [/--base/u, /--head/u]],
  ["SCAN-1.16", "src/checks.mjs", [/scopeEntries/u, /--files/u]],
  ["SCAN-1.17", "src/checks.mjs", [/mode: "all"|workspace/u]],
  ["SCAN-1.18", "scripts/rust-rules.mjs", [/Cargo\.toml/u, /package\.json/u]],
  ["SCAN-1.19", "scripts/rust-rules.mjs", [/scope/u, /files/u]],
  ["SCAN-1.20", "scripts/rust-rules.mjs", [/ignoreDirs/u, /ignoreFileGlobs/u]],
  ["SCAN-2.1", "scripts/rust-rules.mjs", [/cargo/u, /metadata/u]],
  ["SCAN-2.2", "scripts/rust-rules.mjs", [/scanRustFile/u, /signature|struct|enum/u]],
  ["SCAN-2.3", "src/harness.mjs", [/clippy|cargo/u, /compiler-message/u]],
  ["SCAN-2.4", "src/harness.mjs", [/rustdoc|cargo/u, /warning/u]],
  ["SCAN-2.5", "src/harness.mjs", [/eslint/u, /tsc/u]],
  ["SCAN-2.6", "src/generic-scanners.mjs", [/ruff/u, /output-format\\s\+json/u]],
  ["SCAN-2.7", "src/generic-scanners.mjs", [/pyright/u, /mypy/u]],
  ["SCAN-2.8", "src/harness.mjs", [/parsed\.runs/u, /SARIF/u]],
  ["SCAN-2.9", "src/generic-scanners.mjs", [/RegExp|test\(/u]],
  ["SCAN-2.10", "src/harness.mjs", [/dedupeDiagnostics/u, /fingerprint/u]],
];

function collectHarnessContractFindings(root, args = {}) {
  return collectRequiredPatternFindings(root, resolvePackRoot(root, args), HARNESS_CONTRACT_SPECS);
}

function collectProofContractFindings(root, args = {}) {
  return collectRequiredPatternFindings(root, resolvePackRoot(root, args), PROOF_CONTRACT_SPECS);
}

function collectMcpContractFindings(root, args = {}) {
  return collectRequiredPatternFindings(root, resolvePackRoot(root, args), MCP_CONTRACT_SPECS);
}

function collectScannerContractFindings(root, args = {}) {
  return collectRequiredPatternFindings(root, resolvePackRoot(root, args), SCANNER_CONTRACT_SPECS);
}

function collectRequiredPatternFindings(root, packRoot, specs) {
  const findings = [];
  for (const [ruleId, relFile, patterns] of specs) {
    const file = path.join(packRoot, relFile);
    const text = fs.existsSync(file) ? fs.readFileSync(file, "utf8") : "";
    const missing = patterns.filter((pattern) => !pattern.test(text));
    if (missing.length > 0) {
      findings.push(
        finding(
          root,
          file,
          1,
          ruleId,
          `${relFile} is missing contract marker(s): ${missing.map(String).join(", ")}`,
          null,
        ),
      );
    }
  }
  return findings;
}

const VALIDATOR_NETWORK_SCAN_FILES = [
  "src/checks.mjs",
  "src/generic-scanners.mjs",
  "src/source-policy-scanners.mjs",
  "src/rust-scanner.mjs",
  "src/policy.mjs",
  "scripts/rust-rules.mjs",
  "mcp/rust-rules-mcp.mjs",
];

const NETWORK_ACCESS_PATTERN =
  /\bfetch\s*\(|\bXMLHttpRequest\b|from\s+["']node:(?:http|https|net|dns)["']|import\s*\(\s*["']node:(?:http|https|net|dns)["']\s*\)|require\s*\(\s*["'](?:http|https|net|dns|node:http|node:https|node:net|node:dns)["']\s*\)/u;

function collectValidatorNetworkFindings(root, packRoot, findings) {
  for (const rel of VALIDATOR_NETWORK_SCAN_FILES) {
    const file = path.join(packRoot, rel);
    if (!fs.existsSync(file)) continue;
    const lines = fs.readFileSync(file, "utf8").split(/\r?\n/u);
    lines.forEach((line, index) => {
      if (NETWORK_ACCESS_PATTERN.test(line) && !line.includes("network-capability:allow")) {
        findings.push(
          finding(
            root,
            file,
            index + 1,
            "ENF-1.11",
            `${rel} uses network-capable API without an explicit network-capability declaration`,
            line,
          ),
        );
      }
    });
  }
}

const POLICY_CRITICAL_BYPASS_PATTERN =
  /\b(?:TODO|FIXME|HACK|TEMPORARY|TEMP|BYPASS|DISABLE_THIS_CHECK|SKIP_ENFORCER)\b/iu;

function collectEnforcerBypassFindings(root, packRoot, findings) {
  const dirs = ["src", "scripts", "mcp"];
  for (const dir of dirs) {
    const abs = path.join(packRoot, dir);
    if (!fs.existsSync(abs)) continue;
    for (const file of collectSourceFiles(abs, [".mjs", ".js", ".json", ".md"])) {
      if (file.endsWith(path.join("rules", "rules.json"))) continue;
      const lines = fs.readFileSync(file, "utf8").split(/\r?\n/u);
      lines.forEach((line, index) => {
        if (isPolicyCriticalBypassLine(line)) {
          findings.push(
            finding(
              root,
              file,
              index + 1,
              "ENF-1.13",
              `${normalizeRel(packRoot, file)} contains policy-critical temporary/bypass marker`,
              line,
            ),
          );
        }
      });
    }
  }
}

function isPolicyCriticalBypassLine(line) {
  const trimmed = line.trim();
  if (!POLICY_CRITICAL_BYPASS_PATTERN.test(trimmed)) return false;
  if (/\/\\b|POLICY_CRITICAL_BYPASS_PATTERN/u.test(trimmed)) return false;
  if (/^\/\/|^\/\*|^\*/u.test(trimmed)) return true;
  if (/\b(?:DISABLE_THIS_CHECK|SKIP_ENFORCER)\b/iu.test(trimmed)) return true;
  if (/\b(?:TODO|FIXME|HACK)\b/u.test(trimmed) && !/[`"']|\/\\b|pattern\s*:/u.test(trimmed)) return true;
  return false;
}

function collectRuleIdLockFindings(root, packRoot, registryPath, registryIds, findings) {
  const lockPath = path.join(packRoot, "rules", "rule-id-lock.json");
  if (!fs.existsSync(lockPath)) {
    findings.push(
      finding(root, registryPath, 1, "ENF-1.5", "rules/rule-id-lock.json is missing", null),
    );
    return;
  }
  let lock;
  try {
    lock = JSON.parse(fs.readFileSync(lockPath, "utf8"));
  } catch (error) {
    findings.push(
      finding(
        root,
        lockPath,
        1,
        "ENF-1.5",
        `rule ID lock file is not valid JSON: ${error instanceof Error ? error.message : String(error)}`,
        null,
      ),
    );
    return;
  }
  const lockedIds = Array.isArray(lock.ruleIds) ? lock.ruleIds.map(String) : [];
  if (lockedIds.length === 0) {
    findings.push(
      finding(root, lockPath, 1, "ENF-1.5", "rule ID lock file has no ruleIds array", null),
    );
    return;
  }
  const sorted = [...lockedIds].sort((a, b) => a.localeCompare(b));
  if (JSON.stringify(lockedIds) !== JSON.stringify(sorted)) {
    findings.push(
      finding(root, lockPath, 1, "ENF-1.9", "rule ID lock file must be sorted deterministically", null),
    );
  }
  for (const id of lockedIds) {
    if (!registryIds.has(id)) {
      findings.push(
        finding(root, lockPath, 1, "ENF-1.5", `locked rule ID ${id} is missing from rules/rules.json`, null),
      );
    }
  }
}

function collectMetadataDriftFindings(root, registryPath, rules, findings) {
  const localMetadata = new Map([
    ...Object.entries(CHECK_RULES),
    ...Object.entries(GENERIC_RULES),
  ]);
  for (const rule of rules) {
    const local = localMetadata.get(rule.id);
    if (!local) continue;
    for (const field of ["title", "snippet"]) {
      if (String(rule[field] ?? "") !== String(local[field] ?? "")) {
        findings.push(
          finding(
            root,
            registryPath,
            1,
            "ENF-1.7",
            `${rule.id} ${field} differs between rules/rules.json and validator metadata`,
            null,
          ),
        );
      }
    }
  }
}

function collectDeterministicOrderingFindings(root, registryPath, rules, findings) {
  const ids = rules.map((rule) => String(rule.id ?? ""));
  const sortedIds = [...ids].sort((a, b) => a.localeCompare(b));
  if (JSON.stringify(ids) !== JSON.stringify(sortedIds)) {
    findings.push(
      finding(root, registryPath, 1, "ENF-1.9", "rules/rules.json rule IDs must be sorted deterministically", null),
    );
  }
}

function collectDocsCompletenessFindings(root, args = {}) {
  const packRoot = resolvePackRoot(root, args);
  const findings = [];
  const rules = loadRegistryRules(packRoot);
  const registryPath = path.join(packRoot, "rules", "rules.json");
  const requiredHeadings = ["Covered Rules", "Fails", "Passes", "Fix Recipe", "Validator"];
  for (const file of collectSourceFiles(path.join(packRoot, "rules"), [".md"])) {
    if (path.basename(file) === "INDEX.md") continue;
    const text = fs.readFileSync(file, "utf8");
    const anchors = markdownAnchors(text);
    const missing = requiredHeadings.filter(
      (heading) => !anchors.has(markdownAnchor(heading)),
    );
    if (missing.length > 0) {
      findings.push(
        finding(
          root,
          file,
          1,
          "DOCENF-1.1",
          `${normalizeRel(packRoot, file)} is missing rule doc sections: ${missing.join(", ")}`,
          null,
        ),
      );
    }
    const rel = normalizeRel(packRoot, file);
    const docRules = rules.filter((rule) => String(rule.doc ?? "").split("#")[0] === rel);
    const sourceRules = docRules.filter((rule) =>
      ["rust", "typescript", "python"].includes(String(rule.language ?? ""))
      && ["source", "domain", "imports-modules"].includes(String(rule.family ?? "")),
    );
    if (sourceRules.length > 0 && !hasFailAndPassCodeBlocks(text)) {
      findings.push(
        finding(
          root,
          file,
          1,
          "DOCENF-1.2",
          `${rel} covers source rules but lacks both fail and pass fenced code examples`,
          null,
        ),
      );
    }
    const immutableRules = docRules.filter((rule) => rule.lockLevel === "immutable");
    if (immutableRules.length > 0 && /\bshould\b/iu.test(text)) {
      findings.push(
        finding(
          root,
          file,
          firstMatchingLine(text, /\bshould\b/iu),
          "DOCENF-1.6",
          `${rel} documents immutable rules with advisory "should" language`,
          null,
        ),
      );
    }
    if (/\brust-rules\b/iu.test(text) && !/compatibility alias/iu.test(text)) {
      findings.push(
        finding(
          root,
          file,
          firstMatchingLine(text, /\brust-rules\b/iu),
          "DOCENF-1.7",
          `${rel} refers to rust-rules without saying it is a compatibility alias`,
          null,
        ),
      );
    }
    findings.push(...collectTaggedCodeBlockFindings(root, file, text));
    if (/\b(?:rust-only|rust only|typescript\/python later|python\/typescript later)\b/iu.test(text)) {
      findings.push(
        finding(
          root,
          file,
          firstMatchingLine(text, /\b(?:rust-only|rust only|typescript\/python later|python\/typescript later)\b/iu),
          "DOCENF-1.8",
          `${rel} contains stale single-language positioning despite multi-language rules`,
          null,
        ),
      );
    }
    const advisoryRules = docRules.filter((rule) => rule.lockLevel === "advisory");
    if (advisoryRules.length > 0 && !/\b(?:promote|profile|failOn|severity|warning|error)\b/iu.test(text)) {
      findings.push(
        finding(
          root,
          file,
          1,
          "DOCENF-1.9",
          `${rel} covers advisory rules but does not explain profile promotion or severity handling`,
          null,
        ),
      );
    }
    const reviewOrProofRules = docRules.filter((rule) =>
      ["review", "proof"].includes(String(rule.validator ?? ""))
      || String(rule.family ?? "").includes("proof"),
    );
    if (reviewOrProofRules.length > 0 && !/\b(?:proof|checklist|review evidence|evidence)\b/iu.test(text)) {
      findings.push(
        finding(
          root,
          file,
          1,
          "DOCENF-1.10",
          `${rel} covers review/proof rules but does not name the expected evidence`,
          null,
        ),
      );
    }
  }
  for (const rule of rules) {
    const [, anchor = ""] = String(rule.doc ?? "").split("#");
    if (anchor && anchor !== markdownAnchor(anchor)) {
      findings.push(
        finding(
          root,
          registryPath,
          1,
          "DOCENF-1.5",
          `${rule.id} uses unstable doc anchor #${anchor}; use #${markdownAnchor(anchor)}`,
          rule.doc,
        ),
      );
    }
    if (String(rule.snippet ?? "").length > 240) {
      findings.push(
        finding(
          root,
          registryPath,
          1,
          "DOCENF-1.4",
          `${rule.id} snippet is longer than 240 characters`,
          rule.snippet,
        ),
      );
    }
  }
  return findings;
}

function collectTaggedCodeBlockFindings(root, file, markdown) {
  const findings = [];
  const blockPattern = /```([A-Za-z0-9_-]+)\s*\n([\s\S]*?)```/gu;
  for (const match of markdown.matchAll(blockPattern)) {
    const language = String(match[1] ?? "").toLowerCase();
    const code = String(match[2] ?? "");
    const line = lineNumberAt(markdown, match.index ?? 0);
    if (language === "json") {
      try {
        JSON.parse(code);
      } catch (error) {
        findings.push(
          finding(
            root,
            file,
            line,
            "DOCENF-1.3",
            `JSON code block is not parseable: ${error instanceof Error ? error.message : String(error)}`,
            null,
          ),
        );
      }
    }
    if (["js", "javascript", "ts", "typescript", "tsx", "rust", "rs", "python", "py"].includes(language) && !delimitersBalanced(code)) {
      findings.push(
        finding(
          root,
          file,
          line,
          "DOCENF-1.3",
          `${language} code block has unbalanced delimiters`,
          null,
        ),
      );
    }
  }
  return findings;
}

function delimitersBalanced(code) {
  const pairs = new Map([["}", "{"], [")", "("], ["]", "["]]);
  const stack = [];
  let quote = null;
  for (let index = 0; index < code.length; index += 1) {
    const char = code[index];
    const previous = code[index - 1];
    if (quote) {
      if (char === quote && previous !== "\\") quote = null;
      continue;
    }
    if (char === "\"" || char === "'" || char === "`") {
      quote = char;
      continue;
    }
    if (["{", "(", "["].includes(char)) stack.push(char);
    if (pairs.has(char) && stack.pop() !== pairs.get(char)) return false;
  }
  return stack.length === 0 && quote === null;
}

function hasFailAndPassCodeBlocks(markdown) {
  const failSection = markdownSection(markdown, "Fails");
  const passSection = markdownSection(markdown, "Passes");
  const codeBlock = /```(?:rust|rs|typescript|ts|tsx|python|py|js|javascript)?\s*[\s\S]*?```/iu;
  return (codeBlock.test(failSection) && codeBlock.test(passSection))
    || (/Fails:\s*\n\s*```[\s\S]*?```/iu.test(markdown)
      && /Passes:\s*\n\s*```[\s\S]*?```/iu.test(markdown));
}

function markdownSection(markdown, heading) {
  const lines = markdown.split(/\r?\n/u);
  const start = lines.findIndex((line) => new RegExp(`^##+\\s+${escapeRegExp(heading)}\\s*$`, "iu").test(line));
  if (start < 0) return "";
  const end = lines.findIndex((line, index) => index > start && /^##+\s+/u.test(line));
  return lines.slice(start + 1, end < 0 ? lines.length : end).join("\n");
}

function firstMatchingLine(text, pattern) {
  const lines = text.split(/\r?\n/u);
  const index = lines.findIndex((line) => pattern.test(line));
  return index < 0 ? 1 : index + 1;
}

const KNOWN_CONFIG_KEYS = new Set([
  "schemaVersion",
  "profileName",
  "failOn",
  "failFast",
  "enforceWorkspaceFiles",
  "requireCargoDeny",
  "requireCargoAudit",
  "runCargoDoc",
  "cargoOnFileScope",
  "cargoOnDiffScope",
  "cargoTestThreads",
  "allowUnsafeCode",
  "allowBuildRs",
  "allowGitDependencies",
  "allowPathDependencies",
  "publicReexportPolicy",
  "ignoreDirs",
  "ignoreFileGlobs",
  "rustRoots",
  "crateRootGlobs",
  "testFileGlobs",
  "rawTypeBoundaryGlobs",
  "boundaryOwnerNote",
  "facadeFileGlobs",
  "rawStringOwnerGlobs",
  "domainPrimitiveOwnerGlobs",
  "enforceRuntimeStringLiterals",
  "runtimeStringOwnerGlobs",
  "runtimeStringLineAllowPatterns",
  "enforceSerializedPublicDomainPrimitives",
  "serializedDomainOwnerGlobs",
  "blockedProtocolDependencies",
  "runtimeCrates",
  "testOnlyCrates",
  "allowedGitDependencies",
  "allowedExternalLicenses",
  "sourceShapePolicies",
  "sourceShapeOverrides",
  "importBoundaryPolicies",
  "architecturePolicyChecks",
  "singleSourceRequiredMirrorRoots",
  "strictEmptyTestTrees",
  "generatedArtifactsMode",
  "generatedArtifactsTracked",
  "agentRuleMaxLines",
  "maxActiveWaivers",
  "maxWaiverDays",
  "configChangeRequiresSelfCheck",
  "policyIntegrityChecked",
  "languages",
  "rules",
  "waivers",
  "tools",
  "harness",
]);

const BOUNDARY_CONFIG_KEYS = new Set([
  "rawTypeBoundaryGlobs",
  "facadeFileGlobs",
  "rawStringOwnerGlobs",
  "domainPrimitiveOwnerGlobs",
  "runtimeStringOwnerGlobs",
  "runtimeStringLineAllowPatterns",
  "serializedDomainOwnerGlobs",
]);

function collectConfigLockdownFindings(root, config) {
  const packRoot = resolvePackRoot(root);
  const rules = loadRegistryRules(packRoot);
  const registrySeverityMap = buildRegistrySeverityMap(rules);
  const registryPolicyMap = buildRegistryPolicyMap(rules);
  const findings = [];
  const configPath = existingConfigPath(root) ?? root;
  const rawConfig = readRawConfigObject(configPath);
  const rawFailOn = Array.isArray(config.rawFailOn) ? config.rawFailOn : config.failOn;
  for (const key of Object.keys(rawConfig ?? {})) {
    if (!KNOWN_CONFIG_KEYS.has(key)) {
      findings.push(
        finding(root, configPath, 1, "CFG-1.9", `unknown config key ${key}`, key),
      );
    }
  }
  if (rawConfig && (!rawConfig.schemaVersion || !rawConfig.profileName)) {
    findings.push(
      finding(root, configPath, 1, "CFG-1.10", "config must declare schemaVersion and profileName for unambiguous layering", null),
    );
  }
  const knownProfiles = new Set(["strict", "default", "ocentra-enforcer", "ocentra-parent"]);
  if (config.profileName && !knownProfiles.has(String(config.profileName))) {
    findings.push(
      finding(root, configPath, 1, "CFG-1.11", `unknown profileName ${config.profileName}`, String(config.profileName)),
    );
  }
  if (config.configChangeRequiresSelfCheck && config.policyIntegrityChecked !== true) {
    findings.push(
      finding(root, configPath, 1, "CFG-1.12", "config change requires policyIntegrityChecked=true after policy-integrity passes", null),
    );
  }
  if (isStrictProfile(config) && !normalizeFailOn(rawFailOn, { enforceError: false }).includes("error")) {
    findings.push(
      finding(
        root,
        configPath,
        1,
        "CFG-1.1",
        'strict profiles must keep "error" in failOn',
        null,
      ),
    );
  }

  for (const [ruleId, override] of Object.entries(config.rules ?? {})) {
    const policy = policyForRule(ruleId, config, registrySeverityMap, registryPolicyMap);
    const rule = registryPolicyMap.get(ruleId);
    if (!rule) {
      findings.push(
        finding(
          root,
          configPath,
          1,
          "ENF-1.3",
          `${ruleId} is configured but not registered`,
          null,
        ),
      );
      continue;
    }
    if (override.enabled === false && policy.disableBlocked) {
      findings.push(
        finding(
          root,
          configPath,
          1,
          "CFG-1.2",
          `${ruleId} is immutable and cannot be disabled`,
          null,
        ),
      );
    }
    if (override.severity && policy.downgradeBlocked) {
      findings.push(
        finding(
          root,
          configPath,
          1,
          "CFG-1.3",
          `${ruleId} is immutable and cannot be downgraded from ${rule.severity} to ${override.severity}`,
          null,
        ),
      );
    }
    if (override.enabled === false && !policy.disableBlocked && !hasOverrideWaiverMetadata(override)) {
      findings.push(
        finding(
          root,
          configPath,
          1,
          "CFG-1.8",
          `${ruleId} disable lacks waiverId, owner, issue, reason, scope, expires, and remediation`,
          null,
        ),
      );
    }
  }

  if (config.allowUnsafeCode && !hasWaiverFor(config, "CFG-1.4")) {
    findings.push(
      finding(root, configPath, 1, "CFG-1.4", "allowUnsafeCode=true requires a narrow waiver", null),
    );
  }
  if (config.publicReexportPolicy === "allow" && isStrictProfile(config) && !hasWaiverFor(config, "CFG-1.5")) {
    findings.push(
      finding(root, configPath, 1, "CFG-1.5", 'publicReexportPolicy="allow" is forbidden in strict profiles', null),
    );
  }
  for (const [field, value] of [
    ["allowBuildRs", config.allowBuildRs],
    ["allowGitDependencies", config.allowGitDependencies],
    ["allowPathDependencies", config.allowPathDependencies],
  ]) {
    if (value && !hasWaiverFor(config, "CFG-1.6")) {
      findings.push(
        finding(root, configPath, 1, "CFG-1.6", `${field}=true requires a narrow waiver`, null),
      );
    }
  }
  for (const [field, value] of Object.entries(rawConfig ?? {})) {
    if (!BOUNDARY_CONFIG_KEYS.has(field)) continue;
    if (Array.isArray(value) && value.length > 0 && !String(rawConfig.boundaryOwnerNote ?? "").trim()) {
      findings.push(
        finding(root, configPath, 1, "CFG-1.7", `${field} changes require boundaryOwnerNote`, field),
      );
    }
  }
  for (const [field, values] of [
    ["sourceShapeOverrides", config.sourceShapeOverrides],
    ["importBoundaryPolicies", config.importBoundaryPolicies],
  ]) {
    for (const entry of values ?? []) {
      const hasGlob = Boolean(entry.glob || (Array.isArray(entry.globs) && entry.globs.length > 0));
      if (hasGlob && !String(entry.note ?? "").trim()) {
        findings.push(
          finding(root, configPath, 1, "CFG-1.7", `${field} glob entries require note`, JSON.stringify(entry)),
        );
      }
    }
  }

  return findings;
}

function collectWaiverPolicyFindings(root, config) {
  const packRoot = resolvePackRoot(root);
  const registryPolicyMap = buildRegistryPolicyMap(loadRegistryRules(packRoot));
  const findings = [];
  const configPath = existingConfigPath(root) ?? root;
  const today = startOfUtcDay(new Date());
  const activeWaivers = config.waivers ?? [];
  const maxActiveWaivers = Number.isFinite(config.maxActiveWaivers) ? config.maxActiveWaivers : null;
  const maxWaiverDays = Number.isFinite(config.maxWaiverDays) ? config.maxWaiverDays : 90;
  if (maxActiveWaivers !== null && activeWaivers.length > maxActiveWaivers) {
    findings.push(
      finding(
        root,
        configPath,
        1,
        "WAIVER-1.7",
        `active waiver count ${activeWaivers.length} exceeds budget ${maxActiveWaivers}`,
        null,
      ),
    );
  }
  for (const waiver of config.waivers ?? []) {
    const ruleId = String(waiver.ruleId ?? "").toUpperCase();
    const missing = [
      "ruleId",
      "waiverId",
      "owner",
      "issue",
      "reason",
      "expires",
      "remediation",
    ].filter((field) => !String(waiver[field] ?? "").trim());
    if (!Array.isArray(waiver.scope) || waiver.scope.length === 0) missing.push("scope");
    if (waiver.ciAllowed !== true && waiver.ciAllowed !== false) missing.push("ciAllowed");
    if (missing.length > 0) {
      findings.push(
        finding(
          root,
          configPath,
          1,
          "WAIVER-1.1",
          `${waiver.waiverId ?? ruleId} waiver is missing: ${missing.join(", ")}`,
          null,
        ),
      );
    }
    if ((waiver.scope ?? []).some((scope) => isBroadWaiverScope(scope))) {
      findings.push(
        finding(
          root,
          configPath,
          1,
          "WAIVER-1.2",
          `${waiver.waiverId} uses a broad waiver scope`,
          null,
        ),
      );
    }
    const expires = parseUtcDate(waiver.expires);
    if (!expires || expires < today) {
      findings.push(
        finding(root, configPath, 1, "WAIVER-1.3", `${waiver.waiverId} is expired or has an invalid expiry`, null),
      );
    } else if (daysBetweenUtc(today, expires) > maxWaiverDays) {
      findings.push(
        finding(
          root,
          configPath,
          1,
          "WAIVER-1.8",
          `${waiver.waiverId} expiry exceeds max waiver window of ${maxWaiverDays} days`,
          null,
        ),
      );
    }
    const rule = registryPolicyMap.get(ruleId);
    const capabilities = rulePolicyCapabilities(rule ?? { severity: "error" });
    if (rule && capabilities.lockLevel === "immutable" && !capabilities.waivable) {
      findings.push(
        finding(
          root,
          configPath,
          1,
          "WAIVER-1.4",
          `${waiver.waiverId} attempts to waive immutable ${ruleId}`,
          null,
        ),
      );
    }
    if (process.env.CI && waiver.ciAllowed !== true) {
      findings.push(
        finding(
          root,
          configPath,
          1,
          "WAIVER-1.5",
          `${waiver.waiverId} is not CI-allowed`,
          null,
        ),
      );
    }
    if (waiver.visible === false) {
      findings.push(
        finding(root, configPath, 1, "WAIVER-1.6", `${waiver.waiverId} is hidden from output`, null),
      );
    }
    if (/^(?:ai|codex|agent|llm)$/iu.test(String(waiver.owner ?? "").trim())) {
      findings.push(
        finding(root, configPath, 1, "WAIVER-1.9", `${waiver.waiverId} owner must be an accountable human or team`, null),
      );
    }
    if (!String(waiver.remediation ?? "").trim()) {
      findings.push(
        finding(root, configPath, 1, "WAIVER-1.10", `${waiver.waiverId} lacks a remediation plan`, null),
      );
    }
  }
  return findings;
}

function resolvePackRoot(root, args = {}) {
  if (args.packRoot) return path.resolve(args.packRoot);
  const rootRules = path.join(root, "rules", "rules.json");
  return fs.existsSync(rootRules) ? root : PACK_ROOT;
}

function loadRegistryRules(packRoot) {
  const registryPath = path.join(packRoot, "rules", "rules.json");
  if (!fs.existsSync(registryPath)) return [];
  const registry = JSON.parse(fs.readFileSync(registryPath, "utf8"));
  return Array.isArray(registry.rules) ? registry.rules : [];
}

function collectRegistryRuleMetadataFindings(root, packRoot, registryPath, rule, findings) {
  const required = [
    "id",
    "language",
    "family",
    "severity",
    "title",
    "snippet",
    "lockLevel",
    "canDisable",
    "canDowngrade",
    "requiresFailFixture",
    "requiresPassFixture",
    "appliesTo",
    "triggers",
    "validator",
    "doc",
  ];
  const missing = required.filter((field) => {
    const value = rule[field];
    if (Array.isArray(value)) return value.length === 0;
    return value === undefined || value === null || value === "";
  });
  if (missing.length > 0) {
    findings.push(
      finding(
        root,
        registryPath,
        1,
        "ENF-1.1",
        `${rule.id ?? "(unknown rule)"} registry metadata is missing: ${missing.join(", ")}`,
        null,
      ),
    );
  }
  if (String(rule.snippet ?? "").length > 240) {
    findings.push(
      finding(
        root,
        registryPath,
        1,
        "ENF-1.1",
        `${rule.id} snippet exceeds 240 characters`,
        null,
      ),
    );
  }
  if (rule.validator !== "review") {
    const capabilities = rulePolicyCapabilities(rule);
    if (
      capabilities.lockLevel === "immutable" &&
      (rule.canDisable !== false || rule.canDowngrade !== false)
    ) {
      findings.push(
        finding(
          root,
          registryPath,
          1,
          "CFG-1.2",
          `${rule.id} is immutable but registry permits disable/downgrade`,
          null,
        ),
      );
    }
  }
  void packRoot;
}

function collectRegistryDocFindings(root, packRoot, rule, findings) {
  const [docRel, anchor = ""] = String(rule.doc ?? "").split("#");
  const docPath = path.join(packRoot, docRel);
  if (!docRel || !fs.existsSync(docPath)) {
    findings.push(
      finding(
        root,
        docPath,
        1,
        "ENF-1.2",
        `${rule.id} doc file is missing: ${rule.doc}`,
        null,
      ),
    );
    return;
  }
  if (!anchor) {
    findings.push(
      finding(root, docPath, 1, "ENF-1.2", `${rule.id} doc is missing an anchor`, null),
    );
    return;
  }
  const anchors = markdownAnchors(fs.readFileSync(docPath, "utf8"));
  if (!anchors.has(anchor.toLowerCase())) {
    findings.push(
      finding(
        root,
        docPath,
        1,
        "ENF-1.2",
        `${rule.id} doc anchor #${anchor} is missing from ${docRel}`,
        null,
      ),
    );
  }
}

function markdownAnchors(text) {
  const anchors = new Set();
  for (const match of text.matchAll(/^#{1,6}\s+(.+)$/gmu)) {
    anchors.add(markdownAnchor(match[1]));
  }
  return anchors;
}

function markdownAnchor(heading) {
  return String(heading)
    .trim()
    .toLowerCase()
    .replace(/[`*_~[\]()]/gu, "")
    .replace(/[^\p{Letter}\p{Number}\s-]/gu, "")
    .replace(/\s+/gu, "-");
}

function collectFixtureEvidence(packRoot) {
  const evidence = new Set();
  for (const file of collectSourceFiles(path.join(packRoot, "tests"), [".mjs", ".js", ".ts", ".rs", ".py", ".json"])) {
    const text = fs.readFileSync(file, "utf8");
    for (const match of text.matchAll(RULE_ID_RE)) evidence.add(match[0]);
  }
  for (const file of collectSourceFiles(path.join(packRoot, "tests", "fixtures"), [".mjs", ".js", ".ts", ".rs", ".py", ".json", ".toml"])) {
    const text = fs.readFileSync(file, "utf8");
    for (const match of text.matchAll(RULE_ID_RE)) evidence.add(match[0]);
  }
  return evidence;
}

const RULE_ID_RE =
  /\b(?:RR|TS|PY|SEC|GEN|DOC|DOCENF|HAR|MCP|PROOF|SCAN|TEST|PORT|SRC|CONTRACT|DEP|NPM|CI|REPO|SBOM|AI|ENF|CFG|WAIVER|BOUND|ARCH)-[0-9]+\.[0-9]+\b/gu;

function collectRoutedDocRuleIds(packRoot) {
  const ids = new Set();
  for (const file of collectSourceFiles(path.join(packRoot, "rules"), [".md"])) {
    const text = fs.readFileSync(file, "utf8");
    for (const match of text.matchAll(RULE_ID_RE)) ids.add(match[0]);
  }
  return ids;
}

function collectScannerRuleIds(packRoot) {
  const ids = new Set();
  for (const folder of ["src", "scripts", "mcp"]) {
    for (const file of collectSourceFiles(path.join(packRoot, folder), [".mjs", ".js"])) {
      const text = fs.readFileSync(file, "utf8");
      for (const match of text.matchAll(RULE_ID_RE)) ids.add(match[0]);
    }
  }
  return ids;
}

function collectSourceFiles(start, extensions) {
  if (!fs.existsSync(start)) return [];
  const files = [];
  const stack = [start];
  const extensionSet = new Set(extensions);
  const seen = new Set();
  while (stack.length > 0) {
    const current = stack.pop();
    const stats = fs.lstatSync(current);
    if (stats.isSymbolicLink()) continue;
    const real = fs.realpathSync(current);
    if (seen.has(real)) continue;
    seen.add(real);
    if (stats.isDirectory()) {
      for (const entry of fs.readdirSync(current)) stack.push(path.join(current, entry));
      continue;
    }
    if (stats.isFile() && extensionSet.has(path.extname(current))) files.push(current);
  }
  return files.sort((a, b) => a.localeCompare(b));
}

function existingConfigPath(root) {
  for (const rel of ["ocentra-enforcer.config.json", "rust-rules.config.json"]) {
    const candidate = path.join(root, rel);
    if (fs.existsSync(candidate)) return candidate;
  }
  return null;
}

function readRawConfigObject(configPath) {
  if (!configPath || !fs.existsSync(configPath) || !fs.statSync(configPath).isFile()) {
    return null;
  }
  try {
    const parsed = JSON.parse(fs.readFileSync(configPath, "utf8"));
    return parsed && typeof parsed === "object" && !Array.isArray(parsed) ? parsed : null;
  } catch {
    return null;
  }
}

function hasOverrideWaiverMetadata(override) {
  return Boolean(
    override.waiverId &&
      override.owner &&
      override.issue &&
      override.reason &&
      Array.isArray(override.scope) &&
      override.scope.length > 0 &&
      override.expires &&
      override.remediation,
  );
}

function hasWaiverFor(config, ruleId) {
  return (config.waivers ?? []).some(
    (waiver) => String(waiver.ruleId ?? "").toUpperCase() === ruleId,
  );
}

function parseUtcDate(value) {
  const text = String(value ?? "");
  if (!/^\d{4}-\d{2}-\d{2}$/u.test(text)) return null;
  const date = new Date(`${text}T00:00:00.000Z`);
  return Number.isNaN(date.getTime()) ? null : date;
}

function startOfUtcDay(date) {
  return new Date(Date.UTC(date.getUTCFullYear(), date.getUTCMonth(), date.getUTCDate()));
}

function daysBetweenUtc(start, end) {
  return Math.floor((end.getTime() - start.getTime()) / 86_400_000);
}

function isBroadWaiverScope(scope) {
  const normalized = String(scope ?? "").replaceAll("\\", "/").trim();
  return (
    normalized === "" ||
    normalized === "." ||
    normalized === "/" ||
    normalized === "**" ||
    normalized === "**/*" ||
    normalized === "src/**" ||
    normalized === "crates/**" ||
    normalized === "packages/**" ||
    normalized === "apps/**" ||
    /^\*\*\/\*\.[A-Za-z0-9]+$/u.test(normalized)
  );
}

function buildReport({ root, config, checkName, findings, scope = null }) {
  return {
    ok: findings.length === 0,
    command: "check",
    check: checkName,
    root,
    profileName: config.profileName ?? "strict",
    violations: findings,
    warnings: [],
    findings,
    bySeverity: findings.length === 0 ? {} : { error: findings.length },
    scope: scope ? reportScope(root, scope, findings) : undefined,
  };
}

function collectPolicyFiles(root, config, policy, scope = { mode: "all" }) {
  const extensions = new Set(policy.extensions ?? []);
  const predicate = (file, rel) =>
    extensions.has(path.extname(file).toLowerCase()) &&
    isUnderRoots(rel, policy.roots ?? []);
  if (scope.mode === "all")
    return collectFiles(root, policy.roots ?? [], config, predicate);
  return collectFiles(
    root,
    scopeEntries(root, scope, config),
    config,
    predicate,
  );
}

function childDirs(dir) {
  if (!fs.existsSync(dir)) return [];
  return fs
    .readdirSync(dir, { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => path.join(dir, entry.name));
}

function hasFile(start, predicate) {
  if (!fs.existsSync(start)) return false;
  const stats = fs.statSync(start);
  if (stats.isDirectory())
    return fs
      .readdirSync(start)
      .some((entry) => hasFile(path.join(start, entry), predicate));
  return stats.isFile() && predicate(start);
}

function countMatches(lines, pattern) {
  return lines.reduce((count, line) => count + (pattern.test(line) ? 1 : 0), 0);
}

function maxBraceNestingDepth(lines) {
  let depth = 0;
  let maxDepth = 0;
  for (const rawLine of lines) {
    const line = String(rawLine ?? "").replace(/\/\/.*$/u, "");
    for (const char of line) {
      if (char === "{") {
        depth += 1;
        maxDepth = Math.max(maxDepth, depth);
      } else if (char === "}") {
        depth = Math.max(0, depth - 1);
      }
    }
  }
  return maxDepth;
}

function maxPythonIndentDepth(lines) {
  let maxDepth = 0;
  for (const line of lines) {
    if (/^\s*$/u.test(line)) continue;
    const spaces = leadingWhitespace(line).replace(/\t/gu, "    ").length;
    maxDepth = Math.max(maxDepth, Math.floor(spaces / 4));
  }
  return maxDepth;
}

function countLines(text) {
  return text.length === 0 ? 0 : text.split(/\r?\n/u).length;
}

function findBlockEnd(lines, start) {
  let depth = 0;
  let seenBody = false;
  for (let index = start; index < lines.length; index += 1) {
    for (const char of lines[index]) {
      if (char === "{") {
        seenBody = true;
        depth += 1;
      } else if (char === "}") {
        depth -= 1;
      }
    }
    if (seenBody && depth <= 0) return index;
  }
  return start;
}

function findPythonBlockEnd(lines, start) {
  const startIndent = leadingWhitespace(lines[start]).length;
  let last = start;
  for (let index = start + 1; index < lines.length; index += 1) {
    const line = lines[index];
    if (/^\s*$/u.test(line)) {
      last = index;
      continue;
    }
    const trimmed = line.trimStart();
    if (trimmed.startsWith("#")) {
      last = index;
      continue;
    }
    if (leadingWhitespace(line).length <= startIndent) return last;
    last = index;
  }
  return last;
}

function leadingWhitespace(line) {
  return /^\s*/u.exec(line)?.[0] ?? "";
}

function valueAtPath(source, jsonPath) {
  let value = source;
  for (const segment of jsonPath.split(".")) {
    if (value === null || typeof value !== "object" || !(segment in value)) {
      throw new Error(`${jsonPath} is missing`);
    }
    value = value[segment];
  }
  return value;
}

function valueFromSpec(root, ownerPath, valueSpec) {
  const sourceText = fs.readFileSync(repoAbsolute(root, ownerPath), "utf8");
  if ("jsonPath" in valueSpec)
    return valueAtPath(JSON.parse(sourceText), valueSpec.jsonPath);
  if ("sourceObjectPath" in valueSpec)
    return valueAtSourceObjectPath(
      sourceText,
      valueSpec.sourceObjectPath,
      ownerPath,
    );
  if ("rustConst" in valueSpec)
    return valueAtRustConst(sourceText, valueSpec.rustConst, ownerPath);
  if ("rustSerdeRename" in valueSpec)
    return valueAtRustSerdeRename(
      sourceText,
      valueSpec.rustSerdeRename,
      ownerPath,
    );
  throw new Error(
    `${ownerPath}: ${valueSpec.name} needs jsonPath, sourceObjectPath, rustConst, or rustSerdeRename`,
  );
}

function loadContract(root, rawContract) {
  const ownerPath = rawContract.ownerPath;
  const values = (rawContract.values ?? []).map((valueSpec) => {
    const text = valueFromSpec(root, ownerPath, valueSpec);
    if (typeof text !== "string" || text.length === 0) {
      throw new Error(
        `${ownerPath}: ${valueSpec.name} must be a non-empty string`,
      );
    }
    return {
      name: valueSpec.name,
      text,
      pattern: createLiteralMatchPattern(text),
    };
  });
  const valueByName = new Map(values.map((value) => [value.name, value.text]));
  const mirrorPaths = [];
  for (const mirror of rawContract.mirrors ?? []) {
    mirrorPaths.push(mirror.path);
    for (const mirrorValueSpec of mirror.values ?? []) {
      const ownerText = valueByName.get(mirrorValueSpec.name);
      if (ownerText === undefined)
        throw new Error(
          `${mirror.path}: ${mirrorValueSpec.name} does not match an owner value name`,
        );
      const mirrorText = valueFromSpec(root, mirror.path, mirrorValueSpec);
      if (mirrorText !== ownerText) {
        throw new Error(
          `${mirror.path}: ${rawContract.name}.${mirrorValueSpec.name} ${mirrorText} does not match ${ownerPath} ${ownerText}`,
        );
      }
    }
  }
  return {
    ...rawContract,
    allowedPaths: new Set(
      [ownerPath, ...mirrorPaths, ...(rawContract.allowedPaths ?? [])].map(
        (entry) => entry.replaceAll("\\", "/"),
      ),
    ),
    scanRoots: rawContract.scanRoots ?? [],
    values,
  };
}

function collectContractScanFiles(root, contract, config) {
  return collectFiles(
    root,
    contract.scanRoots,
    config,
    (file, rel) =>
      sourceContractExtension(file) &&
      !contract.allowedPaths.has(rel) &&
      !isNonBlockingContractPath(rel),
  ).map((file) => normalizeRel(root, file));
}

function enforceRequiredMirrorCoverage(
  root,
  configPath,
  config,
  scopedFiles,
  findings,
) {
  for (const rootPath of config.requiredMirrorRoots ??
    config.singleSourceRequiredMirrorRoots ??
    []) {
    const coveredPaths = collectCoveredContractPaths(config, rootPath);
    const candidates =
      scopedFiles === null
        ? collectFiles(
            root,
            [rootPath],
            {},
            (file) => path.extname(file) === ".rs",
          ).map((file) => normalizeRel(root, file))
        : scopedFiles.filter(
            (filePath) =>
              filePath.startsWith(`${rootPath}/`) &&
              path.extname(filePath) === ".rs",
          );
    for (const filePath of candidates) {
      if (coveredPaths.has(filePath)) continue;
      findings.push(
        finding(
          root,
          repoAbsolute(root, filePath),
          1,
          "CONTRACT-1.1",
          `missing single-source manifest coverage; add it as a mirror/allowed path in ${normalizeRel(root, configPath)}`,
          null,
        ),
      );
    }
  }
}

function collectCoveredContractPaths(config, rootPath) {
  const covered = new Set();
  for (const contract of config.contracts ?? []) {
    if (contract.ownerPath?.startsWith(`${rootPath}/`))
      covered.add(contract.ownerPath);
    for (const mirror of contract.mirrors ?? []) {
      if (mirror.path?.startsWith(`${rootPath}/`)) covered.add(mirror.path);
    }
    for (const allowedPath of contract.allowedPaths ?? []) {
      if (allowedPath?.startsWith(`${rootPath}/`)) covered.add(allowedPath);
    }
  }
  return covered;
}

function valueAtSourceObjectPath(source, sourceObjectPath, ownerPath) {
  const lastDotIndex = sourceObjectPath.lastIndexOf(".");
  if (lastDotIndex <= 0 || lastDotIndex === sourceObjectPath.length - 1) {
    throw new Error(
      `${ownerPath}: ${sourceObjectPath} must be formatted as ObjectName.PropertyName or ObjectName.PropertyName[index]`,
    );
  }
  const objectName = sourceObjectPath.slice(0, lastDotIndex);
  const propertyPath = sourceObjectPath.slice(lastDotIndex + 1);
  const arrayIndexMatch =
    /^(?<propertyName>[A-Za-z0-9_]+)\[(?<index>\d+)\]$/u.exec(propertyPath);
  const propertyName = arrayIndexMatch?.groups?.propertyName ?? propertyPath;
  const objectPattern = new RegExp(
    `(?:export\\s+)?const\\s+${escapeRegExp(objectName)}\\s*=\\s*\\{([\\s\\S]*?)\\}\\s*(?:as\\s+const)?`,
    "u",
  );
  const kindGroupPattern = new RegExp(
    `(?:export\\s+)?const\\s+${escapeRegExp(objectName)}\\s*=\\s*defineLiteralKindGroup\\(\\s*\\{([\\s\\S]*?)\\}\\s*(?:as\\s+const)?\\s*\\)`,
    "u",
  );
  const objectBody =
    objectPattern.exec(source)?.[1] ?? kindGroupPattern.exec(source)?.[1];
  if (objectBody === undefined)
    throw new Error(`${ownerPath}: ${objectName} constant object is missing`);
  const directStringMatch = new RegExp(
    `\\b${escapeRegExp(propertyName)}\\s*:\\s*(['"\`])([^'"\`]+)\\1`,
    "u",
  ).exec(objectBody);
  if (directStringMatch !== null) return directStringMatch[2];
  const parsedStringMatch = new RegExp(
    `\\b${escapeRegExp(propertyName)}\\s*:\\s*[A-Za-z0-9_$.]+\\.parse\\(\\s*(['"\`])([^'"\`]+)\\1\\s*\\)`,
    "u",
  ).exec(objectBody);
  if (parsedStringMatch !== null) return parsedStringMatch[2];
  if (arrayIndexMatch !== null) {
    const arrayMatch = new RegExp(
      `\\b${escapeRegExp(propertyName)}\\s*:\\s*\\[([\\s\\S]*?)\\]`,
      "u",
    ).exec(objectBody);
    if (arrayMatch === null)
      throw new Error(
        `${ownerPath}: ${sourceObjectPath} array literal is missing`,
      );
    const stringMatches = [...arrayMatch[1].matchAll(/(['"`])([^'"`]+)\1/gu)];
    const index = Number.parseInt(arrayIndexMatch.groups.index, 10);
    if (index < stringMatches.length) return stringMatches[index][2];
    throw new Error(`${ownerPath}: ${sourceObjectPath} array entry is missing`);
  }
  throw new Error(
    `${ownerPath}: ${sourceObjectPath} string literal is missing`,
  );
}

function valueAtRustConst(source, rustConst, ownerPath) {
  const constMatch = new RegExp(
    `(?:pub\\s+)?const\\s+${escapeRegExp(rustConst)}\\s*:\\s*&str\\s*=\\s*"([^"]+)"\\s*;`,
    "u",
  ).exec(source);
  if (constMatch === null)
    throw new Error(`${ownerPath}: ${rustConst} string const is missing`);
  return constMatch[1];
}

function valueAtRustSerdeRename(source, rustSerdeRename, ownerPath) {
  const segments = rustSerdeRename.split("::");
  if (
    segments.length !== 2 ||
    segments.some((segment) => segment.length === 0)
  ) {
    throw new Error(
      `${ownerPath}: ${rustSerdeRename} must be formatted as EnumName::VariantName`,
    );
  }
  const [enumName, variantName] = segments;
  const enumMatch = new RegExp(
    `enum\\s+${escapeRegExp(enumName)}\\s*\\{([\\s\\S]*?)\\n\\}`,
    "u",
  ).exec(source);
  if (enumMatch === null)
    throw new Error(`${ownerPath}: ${enumName} enum is missing`);
  const variantMatch = new RegExp(
    `#\\[serde\\(rename\\s*=\\s*"([^"]+)"\\)\\]\\s*${escapeRegExp(variantName)}\\b`,
    "u",
  ).exec(enumMatch[1]);
  if (variantMatch === null)
    throw new Error(`${ownerPath}: ${rustSerdeRename} serde rename is missing`);
  return variantMatch[1];
}

function createLiteralMatchPattern(value) {
  return new RegExp(
    `(?<![A-Za-z0-9@._/-])${escapeRegExp(value)}(?![A-Za-z0-9@._/-])`,
    "u",
  );
}

function escapeRegExp(value) {
  return String(value).replace(/[.*+?^${}()|[\]\\]/gu, "\\$&");
}

function sourceContractExtension(filePath) {
  return /\.(?:rs|ts|tsx|mjs|cjs|js|json|md|ya?ml)$/u.test(filePath);
}

function isNonBlockingContractPath(rel) {
  return /^(?:docs(?:\/|$)|scripts\/test(?:\/|$))|.*(?:^|\/)tests?\/|.*(?:^|\/)[^/]*_tests?\.rs$|.*(?:^|\/)[^/]*\.(?:test|spec)\.(?:ts|tsx|js|jsx|mjs|cjs)$/u.test(
    rel,
  );
}

function scopeEntries(root, scope, config = {}) {
  if (scope.mode === "files") return scope.files ?? [];
  if (scope.mode === "diff") return diffFiles(root, scope.base, scope.head);
  if (scope.mode === "crate")
    return [
      scope.crateRoot ?? crateRootForName(root, config, scope.crateName),
    ].filter(Boolean);
  return [];
}

function scopeFilesByExtensions(root, scope, config, extensions) {
  return collectFiles(root, scopeEntries(root, scope, config), config, (file) =>
    extensions.has(path.extname(file).toLowerCase()),
  );
}

function scopeRelativeFiles(root, scope, config = {}) {
  return collectFiles(
    root,
    scopeEntries(root, scope, config),
    config,
    () => true,
  ).map((file) => normalizeRel(root, file));
}

function scopedProjectRoots(root, config, scope) {
  if (scope.mode === "all") return null;
  const rels = scopeRelativeFiles(root, scope, config);
  const roots = new Set();
  for (const rel of rels) {
    const segments = rel.split("/");
    if (
      (segments[0] === "packages" ||
        segments[0] === "apps" ||
        segments[0] === "crates") &&
      segments[1]
    ) {
      roots.add(`${segments[0]}/${segments[1]}`);
    }
  }
  return roots;
}

function trackedScopeFiles(root, scope) {
  const files =
    scope.mode === "all"
      ? gitNameOnly(root, ["ls-files"])
      : scopeRelativeFiles(root, scope, {});
  if (scope.mode === "all") return files;
  const tracked = new Set(gitNameOnly(root, ["ls-files"]));
  return files.filter((rel) => tracked.has(rel));
}

function stagedFiles(root) {
  return gitNameOnly(root, [
    "diff",
    "--cached",
    "--name-only",
    "--diff-filter=ACMR",
  ]);
}

function diffFiles(root, base, head) {
  if (!base || !head) return [];
  return gitNameOnly(root, [
    "diff",
    "--name-only",
    "--diff-filter=ACMR",
    base,
    head,
  ]);
}

function gitNameOnly(root, args) {
  const result = spawnSync("git", args, {
    cwd: root,
    encoding: "utf8",
    shell: false,
  });
  if ((result.status ?? 1) !== 0) return [];
  return result.stdout
    .split(/\r?\n/u)
    .map((line) => line.trim())
    .filter(Boolean)
    .map((line) => line.replaceAll("\\", "/"));
}

function crateRootForName(root, config, crateName) {
  if (!crateName) return null;
  for (const manifest of collectFiles(
    root,
    config.crateRootGlobs ?? ["crates/*", "tools/*", "."],
    config,
    (file) => path.basename(file) === "Cargo.toml",
  )) {
    const text = fs.readFileSync(manifest, "utf8");
    if (
      new RegExp(`^\\s*name\\s*=\\s*"${escapeRegExp(crateName)}"`, "mu").test(
        text,
      )
    )
      return path.dirname(manifest);
  }
  return null;
}

function isUnderRoots(rel, roots) {
  if (!Array.isArray(roots) || roots.length === 0) return true;
  return roots.some(
    (root) => rel === root || rel.startsWith(`${root.replaceAll("\\", "/")}/`),
  );
}

function importSpecifier(line) {
  return (
    /(?:^\s*import(?:\s+type)?(?:[\s\w*{},]*\s+from\s*)?|^\s*export(?:\s+type)?\s+[\s\w*{},]*\s+from\s*)['"]([^'"]+)['"]/u.exec(
      line,
    )?.[1] ?? null
  );
}

function isGeneratedArtifactPath(rel) {
  return (
    /^(?:output|test-results|playwright-report)\//u.test(rel) ||
    /(?:^|\/)(?:dist|build|coverage)\//u.test(rel)
  );
}

function reportScope(root, scope, findings) {
  const files =
    scope.mode === "all"
      ? uniqueSorted(findings.map((entry) => entry.file))
      : scopeRelativeFiles(root, scope, {});
  return {
    mode: scope.mode === "all" ? "workspace" : scope.mode,
    files,
    crateName: scope.crateName ?? undefined,
    base: scope.base ?? undefined,
    head: scope.head ?? undefined,
  };
}

function resolveContractConfigPath(root, explicitConfigPath) {
  const candidates = [
    explicitConfigPath ? repoAbsolute(root, explicitConfigPath) : null,
    path.join(root, "ocentra-enforcer.single-source-contracts.json"),
    path.join(root, "scripts", "check-single-source-contracts.json"),
  ].filter(Boolean);
  return candidates.find((candidate) => fs.existsSync(candidate)) ?? null;
}

function spawnInRoot(root, command, args) {
  return spawnSync(command, args, {
    cwd: root,
    encoding: "utf8",
    shell: process.platform === "win32",
    maxBuffer: 64 * 1024 * 1024,
  });
}

function compactProcessOutput(result) {
  return [result.stdout, result.stderr]
    .filter(Boolean)
    .join("\n")
    .split(/\r?\n/u)
    .filter(Boolean)
    .slice(0, 20)
    .join("\n");
}

function isIgnored(file, config) {
  const rel = String(file ?? "").replaceAll("\\", "/");
  const ignoreDirs = config.ignoreDirs ?? [];
  return rel.split("/").some((part) => ignoreDirs.includes(part));
}

function finding(root, file, line, ruleId, detail, source) {
  const rule = CHECK_RULES[ruleId];
  return {
    ruleId,
    severity: "error",
    title: rule.title,
    detail,
    file: normalizeRel(root, file),
    line,
    snippet: rule.snippet,
    source: source == null ? null : String(source).trim(),
  };
}

function genericFinding(root, file, line, ruleId, detail, source) {
  const rule = GENERIC_RULES[ruleId] ?? CHECK_RULES[ruleId];
  return {
    ruleId,
    severity: "error",
    title: rule.title,
    detail,
    file: normalizeRel(root, file),
    line,
    snippet: rule.snippet,
    source: source == null ? null : String(source).trim(),
  };
}
