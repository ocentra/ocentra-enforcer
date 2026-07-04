import { countMatches } from "./check-source-core-helpers.mjs";
import {
  addDuplicatedShapeFinding,
  addFileBudgetFinding,
  addFunctionBudgetFindings,
  collectTypeScriptFunctionStarts,
  maskTypeScriptMetricLine,
  maxTypeScriptBlockNestingDepth,
} from "./check-source-shape-typescript-shared.mjs";
import { maskTypeScriptMetricLines } from "./check-source-shape-typescript-templates.mjs";

export function inspectTypeScriptShape(root, file, text, policy) {
  const findings = [];
  const lines = text.split(/\r?\n/u);
  const metricLines = maskTypeScriptMetricLines(lines, maskTypeScriptMetricLine);
  const maxNestingDepth = maxTypeScriptBlockNestingDepth(metricLines);
  const branchCount = countMatches(
    metricLines,
    /\b(?:if|else\s+if|for|while|switch|case|catch)\b|\?\s*[^:]+:/u,
  );
  const classCount = countMatches(
    metricLines,
    /^\s*(?:export\s+)?class\s+[A-Za-z_$]/u,
  );
  const exportCount = countMatches(
    metricLines,
    /^\s*export\s+(?:class|function|const|let|var|type|interface|enum|default|\{|\*)/u,
  );
  const functionStarts = collectTypeScriptFunctionStarts(metricLines);

  addFileBudgetFinding(
    findings,
    root,
    file,
    "SRC-2.6",
    maxNestingDepth,
    policy.maxNestingDepth ?? 4,
    "file nesting depth is",
  );
  addFileBudgetFinding(
    findings,
    root,
    file,
    "SRC-2.7",
    branchCount,
    policy.maxBranches ?? 12,
    "file has",
    "branch points",
  );
  addDuplicatedShapeFinding(
    findings,
    root,
    file,
    classCount,
    policy.maxClasses,
    "classes",
    "SRC-2.5",
  );
  addDuplicatedShapeFinding(
    findings,
    root,
    file,
    exportCount,
    policy.maxExports,
    "exports",
    "SRC-2.3",
  );
  addFunctionBudgetFindings(
    findings,
    root,
    file,
    lines,
    functionStarts,
    policy.maxFunctionLines,
  );

  return findings;
}
