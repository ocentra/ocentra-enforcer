import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import {
  collectFiles,
  matchesAnyGlob,
  normalizeRel,
  repoAbsolute,
  uniqueSorted,
} from "./path-utils.mjs";
import { GENERIC_RULES, runGenericScan } from "./generic-scanners.mjs";

export const CHECK_RULES = {
  "SRC-1.1": {
    title: "Source files must stay within shape limits",
    snippet:
      "Split oversized files, long functions, and dumping-ground modules before adding behavior.",
  },
  "TEST-2.1": {
    title: "Source workspaces must have test scaffolds",
    snippet: "Add package/crate tests before treating source work as complete.",
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
  const classCount = countMatches(
    lines,
    /^\s*(?:export\s+)?class\s+[A-Za-z_$]/u,
  );
  const exportCount = countMatches(
    lines,
    /^\s*export\s+(?:class|function|const|let|var|type|interface|enum|default|\{|\*)/u,
  );
  const functionStarts = [];

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
    }
  }

  return findings;
}

function inspectPythonShape(root, file, text, policy) {
  const findings = [];
  const lines = text.split(/\r?\n/u);
  const classStarts = [];
  const functionStarts = [];

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
    }
  }

  return findings;
}

function inspectRustShape(root, file, text, policy) {
  const findings = [];
  const lines = text.split(/\r?\n/u);
  const functionStarts = [];
  let typeCount = 0;

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
    const hasInlineTestModule = hasFile(
      path.join(dir, "src"),
      (file) =>
        file.endsWith(".rs") &&
        fs.readFileSync(file, "utf8").includes("#[cfg(test)]"),
    );
    const hasIntegrationTest = hasFile(path.join(dir, "tests"), (file) =>
      file.endsWith(".rs"),
    );
    if (!hasInlineTestModule && !hasIntegrationTest) {
      findings.push(
        finding(
          root,
          cargoPath,
          1,
          "TEST-2.1",
          `${normalizeRel(root, dir)} is missing Rust unit or integration tests`,
          null,
        ),
      );
    }
    collectStrictEmptyTestTreeFindings(
      root,
      dir,
      strictEmptyTestTrees,
      findings,
    );
  }

  return findings.filter((entry) => !isIgnored(entry.file, config));
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
    if (npmSbom.status !== 0)
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
    else
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
