import path from "node:path";
import { normalizeRel } from "./path-utils.mjs";
import {
  collectGenericScopeFiles,
  GENERIC_RULES,
  isTypeScriptConfigPath,
  PY_EXTENSIONS,
  TS_EXTENSIONS,
} from "./generic-scanner-shared.mjs";
import { scanCommonFile } from "./generic-common-scanner.mjs";
import { scanPythonFile } from "./generic-python-scanner.mjs";
import { scanTypeScriptFile } from "./generic-typescript-scanner.mjs";

export function runGenericScan({ root, scope, config, languages = [] }) {
  const activeLanguages = new Set(languages);
  const files = collectGenericScopeFiles(root, scope, config, activeLanguages);
  const violations = [];
  for (const filePath of files) {
    const ext = path.extname(filePath).toLowerCase();
    if (
      activeLanguages.has("typescript") &&
      (TS_EXTENSIONS.has(ext) || isTypeScriptConfigPath(filePath))
    ) {
      violations.push(...scanTypeScriptFile(root, filePath));
    }
    if (activeLanguages.has("python") && PY_EXTENSIONS.has(ext)) {
      violations.push(...scanPythonFile(root, filePath));
    }
    if (activeLanguages.has("common")) {
      violations.push(...scanCommonFile(root, filePath));
    }
    if (config.failFast && violations.length > 0) break;
  }
  return {
    files: files.map((file) => normalizeRel(root, file)),
    violations,
  };
}

export {
  collectGenericScopeFiles,
  GENERIC_RULES,
  scanCommonFile,
  scanPythonFile,
  scanTypeScriptFile,
};
