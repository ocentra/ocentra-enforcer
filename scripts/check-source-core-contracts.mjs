import fs from "node:fs";
import path from "node:path";

import { collectFiles, normalizeRel, repoAbsolute } from "../src/path-utils.mjs";
import { collectCoveredContractPaths, enforceRequiredMirrorCoverage } from "./check-source-core-contract-coverage.mjs";
import { escapeRegExp } from "./check-source-core-contract-rust-values.mjs";
import { loadContract } from "./check-source-core-contract-load.mjs";

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

function sourceContractExtension(filePath) {
  return /\.(?:rs|ts|tsx|mjs|cjs|js|json|md|ya?ml)$/u.test(filePath);
}

function isNonBlockingContractPath(rel) {
  return /^(?:docs(?:\/|$)|scripts\/test(?:\/|$))|.*(?:^|\/)tests?\/|.*(?:^|\/)[^/]*_tests?\.rs$|.*(?:^|\/)[^/]*\.(?:test|spec)\.(?:ts|tsx|js|jsx|mjs|cjs)$/u.test(
    rel,
  );
}

function resolveContractConfigPath(root, explicitConfigPath) {
  const candidates = [
    explicitConfigPath ? repoAbsolute(root, explicitConfigPath) : null,
    path.join(root, "ocentra-enforcer.single-source-contracts.json"),
    path.join(root, "scripts", "check-single-source-contracts.json"),
  ].filter(Boolean);
  return candidates.find((candidate) => fs.existsSync(candidate)) ?? null;
}

export {
  collectContractScanFiles,
  collectCoveredContractPaths,
  enforceRequiredMirrorCoverage,
  escapeRegExp,
  isNonBlockingContractPath,
  loadContract,
  resolveContractConfigPath,
  sourceContractExtension,
};
