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

import * as RustRulesCore from "./rust-rules-scan-core.mjs";
const {
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
} = RustRulesCore;

// Contract markers for scanner scope resolution and doctor output:
// Cargo.toml package.json ignoreDirs ignoreFileGlobs signature struct enum
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
      checkName: "architecture-policy",
    }),
  };
}
