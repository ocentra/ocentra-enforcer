import fs from "node:fs";
import { repoAbsolute } from "../src/path-utils.mjs";
import {
  collectContractScanFiles,
  enforceRequiredMirrorCoverage,
  finding,
  isNonBlockingContractPath,
  loadContract,
  missingRequiredContractPaths,
  recordMissingContractPaths,
  resolveContractConfigPath,
  scopeRelativeFiles,
} from "./check-source-core-helpers.mjs";

export function collectSingleSourceContractFindings(
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
    const missingPaths = missingRequiredContractPaths(root, rawContract);
    if (missingPaths.length > 0) {
      if (
        scopedFiles !== null &&
        missingPaths.every((missingPath) => !scopedFiles.includes(missingPath))
      ) {
        continue;
      }
      recordMissingContractPaths(root, rawContract, missingPaths, findings);
      continue;
    }

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
