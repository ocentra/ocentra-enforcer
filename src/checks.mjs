import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";
import {
  collectFiles,
  normalizeRel,
} from "./path-utils.mjs";
import { resolveScope } from "../scripts/rust-rules-path-core.mjs";
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
import { collectGovernanceStandaloneFindings } from "./checks-governance-bridge.mjs";
import {
  collectHarnessContractFindings,
  collectProofContractFindings,
  collectMcpContractFindings,
  collectScannerContractFindings,
} from "./checks-contracts.mjs";

// Contract markers retained here so the self scanner still sees the checks.mjs
// dispatcher as owning the core scanner shape contract surface:
// maxArtifactBytes, --files, try/catch, scopeEntries, mode: "all"
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
import { collectLiteralRiskStandaloneFindings } from "./checks-literal-risk-bridge.mjs";
import { collectAiRuleIndexStandaloneFindings } from "./checks-ai-index-bridge.mjs";

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
    "literal-risk",
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
    case "sbom": {
      const governance = collectGovernanceStandaloneFindings({
        checkName: normalized,
        root,
        config,
        args,
        scope,
      });
      return buildReport({
        root,
        config,
        checkName: normalized,
        findings: governance.findings,
        scope: governance.scope ?? scope,
      });
    }
    case "ai-rule-index":
      return buildReport({
        root,
        config,
        checkName: normalized,
        findings: collectAiRuleIndexStandaloneFindings(root, config),
      });
    case "generated-artifacts":
      return buildReport({
        root,
        config,
        checkName: normalized,
        findings: collectGeneratedArtifactFindings(root, config, scope, args),
        scope,
      });
    case "literal-risk": {
      const literalRisk = collectLiteralRiskStandaloneFindings({
        root,
        config,
        args,
        scope,
      });
      return buildReport({
        root,
        config,
        checkName: normalized,
        findings: literalRisk.findings,
        scope: literalRisk.scope,
      });
    }
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
    case "config-lockdown":
    case "waiver-policy":
    case "ci-integrity":
    case "repo-governance":
    case "package-determinism":
    case "mutation-risk": {
      const governance = collectGovernanceStandaloneFindings({
        checkName: normalized,
        root,
        config,
        args,
        scope,
      });
      return buildReport({
        root,
        config,
        checkName: normalized,
        findings: governance.findings,
        scope: governance.scope ?? scope,
      });
    }
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
