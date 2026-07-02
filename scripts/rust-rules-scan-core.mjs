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
  normalizeCheckName,
  runStandaloneCheck,
} from "../src/checks.mjs";
import {
  CHECK_RULES,
  SCANNER_BACKED_CHECKS,
} from "../src/check-metadata.mjs";
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
import {
  DEFAULT_ARCHITECTURE_POLICY_CHECKS,
  DEFAULT_CONFIG,
  MERGED_ARRAY_CONFIG_KEYS,
  RULES,
  VERIFY_MODE_CHECKS,
} from "../src/rule-metadata.mjs";

import * as RustRulesPathCore from "./rust-rules-path-core.mjs";
import * as RustRulesEngine from "./rust-rules-scan-engine.mjs";
const {
  normalizeRel,
  toPosix,
  repoAbsolute,
  globToRegExp,
  matchesGlob,
  matchesAnyGlob,
  isIgnoredPath,
  isRustFile,
  lineNumberAt,
  runGit,
  walkFiles,
  collectAllRustFiles,
  collectExplicitRustFiles,
  collectDiffRustFiles,
  findCargoManifests,
  packageNameFromManifest,
  collectCrateRustFiles,
  resolveScope,
  uniqueSorted,
  maskRustCode,
  contextHas,
  firstLineMatching,
  escapeRegExp,
  lineNumberAtIndex,
  addViolation,
} = RustRulesPathCore;
const {
  scanRustFile,
  scanWorkspaceFiles,
  manifestPathsForScope,
  nearestCargoManifest,
  scanCargoManifest,
  dependencyNameFromManifestLine,
  dependencyRequirementFromManifestLine,
  workspacePackageNamesFromManifests,
  loadCargoMetadata,
  scanCargoMetadata,
  runScanner,
  commandExists,
  runCommand,
  shouldRunCargoForScope,
  cargoPackageArgs,
  configuredCargoCommand,
  strongestEnabledSeverity,
  runCargoGates,
  collectFunctionSignatures,
  functionName,
  functionParams,
  normalizedNameTokens,
  isSuspiciousSerializedFieldName,
  braceDelta,
  isTestFile,
  isRawTypeBoundary,
  isBoundaryModulePath,
  isRawStringOwner,
  isDomainPrimitiveOwner,
  isRuntimeStringOwner,
  isSerializedDomainOwner,
  hasStringLiteral,
} = RustRulesEngine;


// boundaryOwnerNote: Enforcer-owned boundary glob handling; edits require policy-integrity and self-scan validation.
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
        explicitFiles.push(...parseFileList(tokens[fileIndex]));
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
      explicitFiles.push(...parseFileList(arg));
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

function parseFileList(value) {
  return String(value ?? "")
    .split(/[,\r\n]/u)
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

export {
  usage,
  defaultArgs,
  parseArgs,
  loadConfig,
  resolveConfigCandidate,
  readProfileConfig,
  mergeConfigLayers,
  resolveDefaultConfigPath,
  normalizeConfig,
  parseAdapterList,
  normalizeVerifyMode,
  createInitReport,
  createCodexInstallReport,
  applyCodexInstallReport,
  createCodexUninstallCliReport,
  createCodexDoctorReport,
  expandAdapters,
  targetUsesHusky,
  buildInitWrites,
  initWrite,
  mcpConfigTemplate,
  applyInitReport,
  printInitReport,
  printCodexInstallReport,
  printCodexUninstallReport,
  printCodexDoctorReport,
  normalizeRel,
  toPosix,
  repoAbsolute,
  globToRegExp,
  matchesGlob,
  matchesAnyGlob,
  isIgnoredPath,
  isRustFile,
  lineNumberAt,
  runGit,
  walkFiles,
  collectAllRustFiles,
  collectExplicitRustFiles,
  collectDiffRustFiles,
  findCargoManifests,
  packageNameFromManifest,
  collectCrateRustFiles,
  resolveScope,
  uniqueSorted,
  maskRustCode,
  contextHas,
  firstLineMatching,
  escapeRegExp,
  lineNumberAtIndex,
  addViolation,
  applyPolicyAndWaivers,
  policyPreflightFindings,
  collectFunctionSignatures,
  functionName,
  functionParams,
  normalizedNameTokens,
  isSuspiciousSerializedFieldName,
  braceDelta,
  isTestFile,
  isRawTypeBoundary,
  isBoundaryModulePath,
  isRawStringOwner,
  isDomainPrimitiveOwner,
  isRuntimeStringOwner,
  isSerializedDomainOwner,
  hasStringLiteral,
  scanRustFile,
  scanWorkspaceFiles,
  manifestPathsForScope,
  nearestCargoManifest,
  scanCargoManifest,
  dependencyNameFromManifestLine,
  dependencyRequirementFromManifestLine,
  workspacePackageNamesFromManifests,
  loadCargoMetadata,
  scanCargoMetadata,
  runScanner,
  commandExists,
  runCommand,
  shouldRunCargoForScope,
  cargoPackageArgs,
  configuredCargoCommand,
  strongestEnabledSeverity,
  runCargoGates,
};
