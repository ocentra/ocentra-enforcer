import {
  countMatches,
  findBlockEnd,
  findPythonBlockEnd,
  finding,
  maxBraceNestingDepth,
  maxPythonIndentDepth,
} from "./check-source-core-helpers.mjs";

export function inspectTypeScriptShape(root, file, text, policy) {
  const findings = [];
  const lines = text.split(/\r?\n/u);
  const maxNestingDepth = maxBraceNestingDepth(lines);
  const branchCount = countMatches(lines, /\b(?:if|else\s+if|for|while|switch|case|catch)\b|\?\s*[^:]+:/u);
  const classCount = countMatches(lines, /^\s*(?:export\s+)?class\s+[A-Za-z_$]/u);
  const exportCount = countMatches(
    lines,
    /^\s*export\s+(?:class|function|const|let|var|type|interface|enum|default|\{|\*)/u,
  );
  const functionStarts = [];

  if (maxNestingDepth > (policy.maxNestingDepth ?? 4)) {
    findings.push(finding(root, file, 1, "SRC-2.6", `file nesting depth is ${maxNestingDepth}; maximum is ${policy.maxNestingDepth ?? 4}`, null));
  }
  if (branchCount > (policy.maxBranches ?? 12)) {
    findings.push(finding(root, file, 1, "SRC-2.7", `file has ${branchCount} branch points; maximum is ${policy.maxBranches ?? 12}`, null));
  }

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
    findings.push(finding(root, file, 1, "SRC-1.1", `file has ${classCount} classes; maximum is ${policy.maxClasses}`, null));
    findings.push(finding(root, file, 1, "SRC-2.5", `file has ${classCount} classes; maximum is ${policy.maxClasses}`, null));
  }
  if (exportCount > policy.maxExports) {
    findings.push(finding(root, file, 1, "SRC-1.1", `file has ${exportCount} exports; maximum is ${policy.maxExports}`, null));
    findings.push(finding(root, file, 1, "SRC-2.3", `file has ${exportCount} exports; maximum is ${policy.maxExports}`, null));
  }
  for (const start of functionStarts) {
    const end = findBlockEnd(lines, start);
    const span = end - start + 1;
    if (span > policy.maxFunctionLines) {
      findings.push(finding(root, file, start + 1, "SRC-1.1", `function has ${span} lines; maximum is ${policy.maxFunctionLines}`, lines[start]));
      findings.push(finding(root, file, start + 1, "SRC-2.2", `function has ${span} lines; maximum is ${policy.maxFunctionLines}`, lines[start]));
    }
  }

  return findings;
}

export function inspectPythonShape(root, file, text, policy) {
  const findings = [];
  const lines = text.split(/\r?\n/u);
  const maxNestingDepth = maxPythonIndentDepth(lines);
  const branchCount = countMatches(lines, /^\s*(?:if|elif|for|while|try|except|with|match|case)\b/u);
  const classStarts = [];
  const functionStarts = [];

  if (maxNestingDepth > (policy.maxNestingDepth ?? 4)) {
    findings.push(finding(root, file, 1, "SRC-2.6", `file nesting depth is ${maxNestingDepth}; maximum is ${policy.maxNestingDepth ?? 4}`, null));
  }
  if (branchCount > (policy.maxBranches ?? 12)) {
    findings.push(finding(root, file, 1, "SRC-2.7", `file has ${branchCount} branch points; maximum is ${policy.maxBranches ?? 12}`, null));
  }

  lines.forEach((line, index) => {
    if (/^\s*class\s+[A-Za-z_]\w*/u.test(line)) classStarts.push(index);
    if (/^\s*(?:async\s+)?def\s+[A-Za-z_]\w*\s*\(/u.test(line)) functionStarts.push(index);
  });

  if (classStarts.length > policy.maxClasses) {
    findings.push(finding(root, file, 1, "SRC-1.1", `file has ${classStarts.length} classes; maximum is ${policy.maxClasses}`, null));
    findings.push(finding(root, file, 1, "SRC-2.5", `file has ${classStarts.length} classes; maximum is ${policy.maxClasses}`, null));
  }
  if (functionStarts.length > policy.maxFunctions) {
    findings.push(finding(root, file, 1, "SRC-1.1", `file has ${functionStarts.length} functions; maximum is ${policy.maxFunctions}`, null));
  }
  for (const start of functionStarts) {
    const end = findPythonBlockEnd(lines, start);
    const span = end - start + 1;
    if (span > policy.maxFunctionLines) {
      findings.push(finding(root, file, start + 1, "SRC-1.1", `function has ${span} lines; maximum is ${policy.maxFunctionLines}`, lines[start]));
      findings.push(finding(root, file, start + 1, "SRC-2.2", `function has ${span} lines; maximum is ${policy.maxFunctionLines}`, lines[start]));
    }
  }

  return findings;
}

export function inspectRustShape(root, file, text, policy) {
  const findings = [];
  const lines = text.split(/\r?\n/u);
  const maxNestingDepth = maxBraceNestingDepth(lines);
  const branchCount = countMatches(lines, /\b(?:if|else\s+if|for|while|loop|match)\b|=>/u);
  const functionStarts = [];
  let typeCount = 0;

  if (maxNestingDepth > (policy.maxNestingDepth ?? 4)) {
    findings.push(finding(root, file, 1, "SRC-2.6", `file nesting depth is ${maxNestingDepth}; maximum is ${policy.maxNestingDepth ?? 4}`, null));
  }
  if (branchCount > (policy.maxBranches ?? 12)) {
    findings.push(finding(root, file, 1, "SRC-2.7", `file has ${branchCount} branch points; maximum is ${policy.maxBranches ?? 12}`, null));
  }

  lines.forEach((line, index) => {
    if (/^\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+\w+/u.test(line)) functionStarts.push(index);
    if (/^\s*(?:pub\s+)?(?:struct|enum)\s+\w+/u.test(line)) typeCount += 1;
  });

  if (functionStarts.length > policy.maxFunctions) {
    findings.push(finding(root, file, 1, "SRC-1.1", `file has ${functionStarts.length} functions; maximum is ${policy.maxFunctions}`, null));
  }
  if (typeCount > policy.maxTypes) {
    findings.push(finding(root, file, 1, "SRC-1.1", `file has ${typeCount} structs/enums; maximum is ${policy.maxTypes}`, null));
    findings.push(finding(root, file, 1, "SRC-2.4", `file has ${typeCount} structs/enums; maximum is ${policy.maxTypes}`, null));
  }
  for (const start of functionStarts) {
    const end = findBlockEnd(lines, start);
    const span = end - start + 1;
    if (span > policy.maxFunctionLines) {
      findings.push(finding(root, file, start + 1, "SRC-1.1", `function has ${span} lines; maximum is ${policy.maxFunctionLines}`, lines[start]));
      findings.push(finding(root, file, start + 1, "SRC-2.2", `function has ${span} lines; maximum is ${policy.maxFunctionLines}`, lines[start]));
    }
  }

  return findings;
}
