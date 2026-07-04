import { findBlockEnd, finding } from "./check-source-core-helpers.mjs";
import { maskTypeScriptMetricLines } from "./check-source-shape-typescript-templates.mjs";

function collectTypeScriptFunctionStarts(lines) {
  const starts = [];
  lines.forEach((line, index) => {
    if (isTypeScriptFunctionStart(line)) starts.push(index);
  });
  return starts;
}

function maxTypeScriptBlockNestingDepth(lines) {
  const stack = [];
  let codeDepth = 0;
  let maxDepth = 0;
  for (const line of lines) {
    for (const [index, char] of [...line].entries()) {
      if (char === "{") {
        const kind = isTypeScriptCodeBlockOpening(line, index) ? "code" : "data";
        stack.push(kind);
        if (kind !== "code") continue;
        maxDepth = Math.max(maxDepth, ++codeDepth);
      } else if (char === "}") {
        const kind = stack.pop();
        if (kind === "code") codeDepth = Math.max(0, codeDepth - 1);
      }
    }
  }
  return maxDepth;
}

function maskTypeScriptMetricLine(line) {
  return String(line ?? "")
    .replace(/\/\/.*$/u, "")
    .replace(/'(?:[^'\\]|\\.)*'/gu, "''")
    .replace(/"(?:[^"\\]|\\.)*"/gu, '""')
    .replace(/`(?:[^`\\]|\\.)*`/gu, "``")
    .replace(/\/(?:[^\/\\\n]|\\.)+\/[dgimsuvy]*/gu, "//");
}

function isTypeScriptCodeBlockOpening(line, braceIndex) {
  const prefix = line.slice(0, braceIndex).trim();
  return (
    /=>\s*$/u.test(prefix) ||
    /^(?:export\s+)?(?:async\s+)?function\b/u.test(prefix) ||
    /^(?:export\s+)?class\b/u.test(prefix) ||
    /^(?:if|else|for|while|switch|case|catch|try|finally)\b/u.test(prefix) ||
    /^(?:async\s+)?[A-Za-z_$][\w$]*\s*\([^)]*\)\s*$/u.test(prefix)
  );
}

function isTypeScriptFunctionStart(line) {
  return (
    /^\s*(?:export\s+)?(?:async\s+)?function\s+[A-Za-z_$][\w$]*\s*\(/u.test(
      line,
    ) ||
    /^\s*(?:export\s+)?(?:const|let|var)\s+[A-Za-z_$][\w$]*\s*=\s*(?:async\s*)?\([^)]*\)\s*=>\s*\{/u.test(
      line,
    ) ||
    /^\s*(?:async\s+)?(?!if\b|for\b|while\b|switch\b|catch\b|else\b|return\b)[A-Za-z_$][\w$]*\s*\([^)]*\)\s*\{/u.test(
      line,
    )
  );
}

function addFileBudgetFinding(
  findings,
  root,
  file,
  ruleId,
  actual,
  maximum,
  prefix,
  suffix = null,
) {
  if (actual <= maximum) return;
  const detail = suffix
    ? `${prefix} ${actual} ${suffix}; maximum is ${maximum}`
    : `${prefix} ${actual}; maximum is ${maximum}`;
  findings.push(finding(root, file, 1, ruleId, detail, null));
}

function addDuplicatedShapeFinding(
  findings,
  root,
  file,
  actual,
  maximum,
  noun,
  secondaryRuleId,
) {
  if (actual <= maximum) return;
  const detail = `file has ${actual} ${noun}; maximum is ${maximum}`;
  findings.push(finding(root, file, 1, "SRC-1.1", detail, null));
  findings.push(finding(root, file, 1, secondaryRuleId, detail, null));
}

function addFunctionBudgetFindings(
  findings,
  root,
  file,
  lines,
  functionStarts,
  maxFunctionLines,
) {
  for (const start of functionStarts) {
    const end = findBlockEnd(lines, start);
    const span = end - start + 1;
    if (span <= maxFunctionLines) continue;
    const detail = `function has ${span} lines; maximum is ${maxFunctionLines}`;
    findings.push(
      finding(root, file, start + 1, "SRC-1.1", detail, lines[start]),
    );
    findings.push(
      finding(root, file, start + 1, "SRC-2.2", detail, lines[start]),
    );
  }
}

export {
  addDuplicatedShapeFinding,
  addFileBudgetFinding,
  addFunctionBudgetFindings,
  collectTypeScriptFunctionStarts,
  maskTypeScriptMetricLine,
  maxTypeScriptBlockNestingDepth,
};
