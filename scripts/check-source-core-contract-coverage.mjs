import path from "node:path";

import { collectFiles, normalizeRel, repoAbsolute } from "../src/path-utils.mjs";
import { finding } from "./check-source-core-helpers.mjs";

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
    if (contract.ownerPath?.startsWith(`${rootPath}/`)) {
      covered.add(contract.ownerPath);
    }
    for (const mirror of contract.mirrors ?? []) {
      if (mirror.path?.startsWith(`${rootPath}/`)) covered.add(mirror.path);
    }
    for (const allowedPath of contract.allowedPaths ?? []) {
      if (allowedPath?.startsWith(`${rootPath}/`)) covered.add(allowedPath);
    }
  }
  return covered;
}

export { collectCoveredContractPaths, enforceRequiredMirrorCoverage };
