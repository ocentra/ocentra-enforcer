import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";
import {
  collectFiles,
  normalizeRel,
} from "./path-utils.mjs";
import { GENERIC_RULES, runGenericScan } from "./generic-scanners.mjs";
import { scanAdditionalTypeScriptFile } from "./source-policy-scanners.mjs";
import {
  applyRulePolicy,
  applyWaivers,
  isSeverityDowngrade,
  splitFindings,
} from "./policy.mjs";
import {
  collectConfigLockdownFindings,
  collectWaiverPolicyFindings,
} from "./check-policy.mjs";
import { collectDocsCompletenessFindings } from "./check-docs.mjs";
import {
  collectCiIntegrityFindings,
  collectDependencyPolicyFindings,
  collectMutationRiskFindings,
  collectPackageDeterminismFindings,
  collectRepoGovernanceFindings,
  runSbomCheck,
} from "./check-governance.mjs";
import {
  enrichFindingMetadata,
  enrichFindingsMetadata,
  registryRules,
} from "./rule-registry.mjs";
import {
  CHECK_ALIASES,
  CHECK_RULES,
  SCANNER_BACKED_CHECKS,
} from "./check-metadata.mjs";
import {
  collectSourceShapeFindings,
  applySourceShapeOverrides,
  inspectTypeScriptShape,
  inspectPythonShape,
  inspectRustShape,
  collectRequiredTestFindings,
  collectInlineSourceTestFindings,
  isInlineTestSourceCandidate,
  inlineTestPatternForFile,
  collectStrictEmptyTestTreeFindings,
  collectEmptyPlaceholderTrees,
  collectSingleSourceContractFindings,
  collectGeneratedArtifactFindings,
  collectNoZodSourceFindings,
  collectNoNakedDomainStringsFindings,
  collectWeakAssertionsFindings,
  collectSkippedFocusedTestFindings,
  collectValidationBypassFindings,
  collectPlaceholderImplementationFindings,
  collectReexportFindings,
  collectSecretFindings,
  collectImportBoundaryFindings,
  resolvePackRoot,
  loadRegistryRules,
  collectRegistryRuleMetadataFindings,
  collectRegistryDocFindings,
  markdownAnchors,
  markdownAnchor,
  collectFixtureEvidence,
  ensureFixtureEvidenceEntry,
  ruleIdFromFixturePath,
  collectRoutedDocRuleIds,
  collectScannerRuleIds,
  collectSourceFiles,
  buildReport,
  collectPolicyFiles,
  childDirs,
  hasFile,
  countMatches,
  maxBraceNestingDepth,
  maxPythonIndentDepth,
  countLines,
  findBlockEnd,
  findPythonBlockEnd,
  leadingWhitespace,
  valueAtPath,
  valueFromSpec,
  loadContract,
  collectContractScanFiles,
  enforceRequiredMirrorCoverage,
  collectCoveredContractPaths,
  valueAtSourceObjectPath,
  valueAtRustConst,
  valueAtRustSerdeRename,
  createLiteralMatchPattern,
  escapeRegExp,
  sourceContractExtension,
  isNonBlockingContractPath,
  scopeEntries,
  scopeFilesByExtensions,
  scopeRelativeFiles,
  scopedProjectRoots,
  trackedScopeFiles,
  stagedFiles,
  gitNameOnly,
  crateRootForName,
  isUnderRoots,
  importSpecifier,
  isGeneratedArtifactPath,
  reportScope,
  resolveContractConfigPath,
  resolveCommand,
  isIgnored,
  finding,
  genericFinding,
} from "../scripts/check-source-core.mjs";

const PACK_ROOT = path.resolve(path.join(path.dirname(fileURLToPath(import.meta.url)), ".."));

function ruleMetadataEntries(rows) {
  return Object.fromEntries(
    rows.map(([id, title, snippet]) => [id, { title, snippet }]),
  );
}

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
    case "no-zod-source":
      return buildReport({
        root,
        config,
        checkName: normalized,
        findings: collectNoZodSourceFindings(root, config, scope),
        scope,
      });
    case "no-naked-domain-strings":
      return buildReport({
        root,
        config,
        checkName: normalized,
        findings: collectNoNakedDomainStringsFindings(root, config, scope),
        scope,
      });
    case "weak-assertions":
      return buildReport({
        root,
        config,
        checkName: normalized,
        findings: collectWeakAssertionsFindings(root, config, scope),
        scope,
      });
    case "skipped-focused-tests":
      return buildReport({
        root,
        config,
        checkName: normalized,
        findings: collectSkippedFocusedTestFindings(root, config, scope),
        scope,
      });
    case "validation-bypass":
      return buildReport({
        root,
        config,
        checkName: normalized,
        findings: collectValidationBypassFindings(root, config, scope),
        scope,
      });
    case "placeholder-implementation":
      return buildReport({
        root,
        config,
        checkName: normalized,
        findings: collectPlaceholderImplementationFindings(root, config, scope),
        scope,
      });
    case "reexports":
      return buildReport({
        root,
        config,
        checkName: normalized,
        findings: collectReexportFindings(root, config, scope),
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
    const evidence = fixtureEvidence.get(String(rule.id ?? "").toUpperCase());
    if (rule.validator === "review") continue;
    const hasBehavioralEvidence = (evidence?.testReferences.size ?? 0) > 0;
    const hasFailEvidence = (evidence?.failFixtures.length ?? 0) > 0 || hasBehavioralEvidence;
    const hasPassEvidence = (evidence?.passFixtures.length ?? 0) > 0 || hasBehavioralEvidence;
    if (rule.requiresFailFixture && !hasFailEvidence) {
      findings.push(
        finding(
          root,
          registryPath,
          1,
          "ENF-1.4",
          `${rule.id} requires fail evidence but no .fail fixture or behavioral test reference is present under tests/fixtures/enforcer or tests/**`,
          null,
        ),
      );
    }
    if (rule.requiresPassFixture && !hasPassEvidence) {
      findings.push(
        finding(
          root,
          registryPath,
          1,
          "ENF-1.4",
          `${rule.id} requires pass evidence but no .pass fixture or behavioral test reference is present under tests/fixtures/enforcer or tests/**`,
          null,
        ),
      );
    }
    if (
      (rule.requiresFailFixture || rule.requiresPassFixture) &&
      evidence &&
      evidence.testReferences.size === 0
    ) {
      findings.push(
        finding(
          root,
          registryPath,
          1,
          "ENF-1.4",
          `${rule.id} has fixtures but no behavioral test references those fixtures or the rule ID`,
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
  ["HAR-2.10", "tests/enforcer-harness.test.mjs", [/Artifact path escapes harness root/u, /runs",\s*"artifact"/u]],
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
  ["PROOF-1.6", "tests/enforcer-proof.test.mjs", [/dirty-worktree/u, /allowDirty/u]],
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
  ["MCP-1.3", "tests/rust-rules-mcp.test.mjs", [/unexpected argument/u, /result\.isError/u]],
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
  ["SCAN-1.3", "src/generic-scanner-shared.mjs", [/ts-ignore|noqa|type:\s*ignore/u]],
  ["SCAN-1.4", "src/checks.mjs", [/split\(/u, /\\r\?\\n/u]],
  ["SCAN-1.5", "src/path-utils.mjs", [/toPosix/u, /normalizeRel/u]],
  ["SCAN-1.6", "src/path-utils.mjs", [/repoAbsolute/u, /path\.resolve/u]],
  ["SCAN-1.7", "src/path-utils.mjs", [/path\.isAbsolute/u, /path\.resolve/u]],
  ["SCAN-1.8", "src/path-utils.mjs", [/lstatSync/u, /isSymbolicLink/u]],
  ["SCAN-1.9", "src/path-utils.mjs", [/isSymbolicLink/u]],
  ["SCAN-1.10", "scripts/rust-rules.mjs", [/sortFindings/u, /compareFindings/u]],
  ["SCAN-1.11", "src/checks.mjs", [/maxArtifactBytes|64 \* 1024 \* 1024|maxBuffer/u]],
  ["SCAN-1.12", "src/generic-common-scanner.mjs", [/binary|readFileSync/u]],
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
  ["SCAN-2.6", "src/generic-common-scanner.mjs", [/ruff/u, /output-format\\s\+json/u]],
  ["SCAN-2.7", "src/generic-common-scanner.mjs", [/pyright/u, /mypy/u]],
  ["SCAN-2.8", "src/harness.mjs", [/parsed\.runs/u, /SARIF/u]],
  ["SCAN-2.9", "src/generic-scanner-shared.mjs", [/RegExp|test\(/u]],
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
  "src/generic-scanner-shared.mjs",
  "src/generic-common-scanner.mjs",
  "src/generic-python-scanner.mjs",
  "src/generic-typescript-scanner.mjs",
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
  const lockedIds = Array.isArray(lock.rules)
    ? lock.rules.map((entry) => String(entry?.id ?? "")).filter(Boolean)
    : Array.isArray(lock.ruleIds)
      ? lock.ruleIds.map(String)
      : [];
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
  const lockedSet = new Set(lockedIds);
  for (const id of [...registryIds].sort((a, b) => a.localeCompare(b))) {
    if (!lockedSet.has(id)) {
      findings.push(
        finding(root, lockPath, 1, "ENF-1.5", `registry rule ID ${id} is missing from rules/rule-id-lock.json`, null),
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
