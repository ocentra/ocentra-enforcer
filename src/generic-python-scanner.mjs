import fs from "node:fs";
import { normalizeRel } from "./path-utils.mjs";
import {
  addViolation,
  pythonBadModuleNamePattern,
  isPythonTestPath,
  scanPythonTestBlocks,
} from "./generic-scanner-shared.mjs";
import {
  scanPythonDomainModelRules,
  scanPythonFunctionRules,
  scanPythonSafetyRules,
  scanPythonSuppressions,
  scanPythonTestRules,
} from "./generic-python-scanner-rules.mjs";

export function scanPythonFile(root, filePath) {
  const rel = normalizeRel(root, filePath);
  const lines = fs.readFileSync(filePath, "utf8").split(/\r?\n/u);
  const violations = [];
  const isTestPath = isPythonTestPath(rel);
  if (pythonBadModuleNamePattern.test(rel)) {
    addViolation(
      violations,
      root,
      filePath,
      1,
      "PY-4.31",
      "dumping-ground Python module name found.",
      rel,
    );
  }
  lines.forEach((line, idx) => {
    const lineNo = idx + 1;
    const context = {
      root,
      filePath,
      rel,
      lines,
      violations,
      index: idx,
      isTestPath,
      priorWindow: lines.slice(Math.max(0, idx - 12), idx + 1).join("\n"),
    };
    scanPythonSuppressions(context, line, lineNo);
    scanPythonDomainModelRules(context, line, lineNo);
    scanPythonFunctionRules(context, line, lineNo);
    scanPythonSafetyRules(context, line, lineNo);
    scanPythonTestRules(context, line, lineNo);
  });
  if (isTestPath) {
    violations.push(...scanPythonTestBlocks(root, filePath, lines));
  }
  return violations;
}
