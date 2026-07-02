import fs from "node:fs";
import path from "node:path";
import { matchesAnyGlob, normalizeRel, repoAbsolute } from "../src/path-utils.mjs";
import { runGenericScan } from "../src/generic-scanners.mjs";
import { scanAdditionalTypeScriptFile } from "../src/source-policy-scanners.mjs";
import {
  collectContractScanFiles,
  enforceRequiredMirrorCoverage,
  finding,
  genericFinding,
  importSpecifier,
  isGeneratedArtifactPath,
  isNonBlockingContractPath,
  isUnderRoots,
  loadContract,
  resolveContractConfigPath,
  scopeFilesByExtensions,
  scopeRelativeFiles,
  stagedFiles,
  trackedScopeFiles,
} from "./check-source-core-helpers.mjs";

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

function collectNoZodSourceFindings(root, config, scope = { mode: "all" }) {
  const files = scopeFilesByExtensions(
    root,
    scope,
    config,
    new Set([".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs"]),
  );
  const findings = [];
  for (const file of files) {
    for (const violation of scanAdditionalTypeScriptFile(root, file)) {
      if (violation.ruleId === "TS-1.2") {
        findings.push(violation);
      }
    }
  }
  return findings;
}

function collectNoNakedDomainStringsFindings(root, config, scope = { mode: "all" }) {
  const report = runGenericScan({
    root,
    scope,
    config,
    languages: ["rust", "typescript", "python", "common"],
  });
  const allowedRuleIds = new Set(["RR-6.1", "RR-6.5", "RR-18.16", "TS-1.3", "PY-1.3"]);
  return (report.violations ?? []).filter((entry) => allowedRuleIds.has(entry.ruleId));
}

function collectWeakAssertionsFindings(root, config, scope = { mode: "all" }) {
  const report = runGenericScan({
    root,
    scope,
    config,
    languages: ["typescript", "python", "common"],
  });
  return (report.violations ?? []).filter((entry) => entry.ruleId === "TEST-1.2");
}

function collectSkippedFocusedTestFindings(root, config, scope = { mode: "all" }) {
  const report = runGenericScan({
    root,
    scope,
    config,
    languages: ["rust", "typescript", "python", "common"],
  });
  const allowedRuleIds = new Set(["TS-3.1", "PY-2.1", "TEST-1.3"]);
  return (report.violations ?? []).filter((entry) => allowedRuleIds.has(entry.ruleId));
}

function collectValidationBypassFindings(root, config, scope = { mode: "all" }) {
  const report = runGenericScan({
    root,
    scope,
    config,
    languages: ["rust", "typescript", "python", "common"],
  });
  const allowedRuleIds = new Set(["RR-2.1", "RR-2.2", "TS-2.1", "PY-1.1", "PY-1.2"]);
  return (report.violations ?? []).filter((entry) => allowedRuleIds.has(entry.ruleId));
}

function collectPlaceholderImplementationFindings(root, config, scope = { mode: "all" }) {
  const report = runGenericScan({
    root,
    scope,
    config,
    languages: ["rust", "typescript", "python", "common"],
  });
  return (report.violations ?? []).filter((entry) => entry.ruleId === "SRC-1.2");
}

function collectReexportFindings(root, config, scope = { mode: "all" }) {
  const report = runGenericScan({
    root,
    scope,
    config,
    languages: ["rust", "typescript", "common"],
  });
  const allowedRuleIds = new Set(["RR-7.2", "RR-7.3", "TS-1.1"]);
  return (report.violations ?? []).filter((entry) => allowedRuleIds.has(entry.ruleId));
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
  const genericReport = runGenericScan({
    root,
    scope,
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

export {
  collectGeneratedArtifactFindings,
  collectImportBoundaryFindings,
  collectNoNakedDomainStringsFindings,
  collectNoZodSourceFindings,
  collectPlaceholderImplementationFindings,
  collectReexportFindings,
  collectSecretFindings,
  collectSingleSourceContractFindings,
  collectSkippedFocusedTestFindings,
  collectValidationBypassFindings,
  collectWeakAssertionsFindings,
};
