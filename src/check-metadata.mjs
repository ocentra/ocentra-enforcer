function ruleMetadataEntries(rows) {
  return Object.fromEntries(
    rows.map(([id, title, snippet]) => [id, { title, snippet }]),
  );
}

const CHECK_RULES_VALUE = Object.freeze({
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
    snippet: "Pin workflow actions by full commit SHA; do not use floating tags, branches, main, master, or latest.",
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
  "CI-1.21": {
    title: "Subprocess JSON capture must be CI-safe",
    snippet: "Use file-backed capture or an explicit large maxBuffer when parsing child-process stdout/stderr as JSON.",
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
});

export const CHECK_RULES = CHECK_RULES_VALUE;

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

export { DEFAULT_ALLOWED_LICENSES };

export const CHECK_ALIASES = new Map([
  ["check-source-shape", "source-shape"],
  ["check-required-tests", "required-tests"],
  ["check-single-source-contracts", "single-source-contracts"],
  ["check-ai-rule-index", "ai-rule-index"],
  ["check-dependency-policy", "dependency-policy"],
  ["write-sbom", "sbom"],
  ["rust-string-boundaries", "no-naked-domain-strings"],
]);

const SCANNER_BACKED_CHECKS_VALUE = Object.freeze({
  "no-zod-source": {
    languages: ["typescript", "common"],
    ruleIds: ["TS-1.2"],
  },
  "no-test-doubles": {
    languages: ["rust", "typescript", "python", "common"],
    ruleIds: ["TEST-1.1", "TS-8.8"],
  },
  "cross-platform-script-commands": {
    languages: ["rust", "typescript", "python", "common"],
    ruleIds: ["PORT-1.1"],
  },
  "no-naked-domain-strings": {
    languages: ["rust", "typescript", "python", "common"],
    ruleIds: ["RR-6.1", "RR-6.5", "RR-18.16", "TS-1.3", "PY-1.3"],
  },
  "weak-assertions": {
    languages: ["typescript", "python", "common"],
    ruleIds: ["TEST-1.2"],
  },
  "skipped-focused-tests": {
    languages: ["rust", "typescript", "python", "common"],
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
    languages: ["rust", "typescript", "common"],
    ruleIds: ["RR-7.2", "RR-7.3", "TS-1.1"],
  },
});

export const SCANNER_BACKED_CHECKS = SCANNER_BACKED_CHECKS_VALUE;
