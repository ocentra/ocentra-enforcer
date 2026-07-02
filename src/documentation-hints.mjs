function scanTypeScriptDocumentationHints(addViolation, root, filePath, lines) {
  const violations = [];
  lines.forEach((line, idx) => {
    if (
      !/^\s*export\s+(?:async\s+)?(?:function|class|interface|type|const|let|var|enum)\s+[A-Za-z_$][\w$]*/u.test(
        line,
      )
    )
      return;
    if (hasLeadingDocComment(lines, idx, "/**")) return;
    addViolation(
      violations,
      root,
      filePath,
      idx + 1,
      "DOC-1.1",
      "Exported TypeScript/JavaScript API has no leading JSDoc comment.",
      line,
    );
  });
  return violations;
}

function scanPythonDocumentationHints(addViolation, root, filePath, lines) {
  const violations = [];
  lines.forEach((line, idx) => {
    if (!/^(?:async\s+)?def\s+[A-Za-z_]\w*\s*\(|^class\s+[A-Za-z_]\w*/u.test(line)) return;
    if (hasPythonDocstringAfter(lines, idx)) return;
    addViolation(
      violations,
      root,
      filePath,
      idx + 1,
      "DOC-1.1",
      "Top-level Python function/class has no docstring.",
      line,
    );
  });
  return violations;
}

function scanRustDocumentationHints(addViolation, root, filePath, lines) {
  const violations = [];
  lines.forEach((line, idx) => {
    if (
      !/^\s*pub(?:\([^)]*\)|\s+)?\s*(?:async\s+)?(?:fn|struct|enum|trait)\s+[A-Za-z_]\w*/u.test(
        line,
      )
    )
      return;
    if (
      hasLeadingDocComment(lines, idx, "///") ||
      hasLeadingDocComment(lines, idx, "#[doc")
    )
      return;
    addViolation(
      violations,
      root,
      filePath,
      idx + 1,
      "DOC-1.1",
      "Public Rust API has no leading rustdoc comment.",
      line,
    );
  });
  return violations;
}

function hasLeadingDocComment(lines, index, marker) {
  for (let cursor = index - 1; cursor >= 0; cursor -= 1) {
    const line = lines[cursor]?.trim() ?? "";
    if (line === "") continue;
    return line.startsWith(marker) || line.endsWith("*/");
  }
  return false;
}

function hasPythonDocstringAfter(lines, index) {
  for (let cursor = index + 1; cursor < lines.length; cursor += 1) {
    const line = lines[cursor]?.trim() ?? "";
    if (line === "") continue;
    return line.startsWith('"""') || line.startsWith("'''");
  }
  return false;
}

function isPythonTestPath(rel) {
  return /(?:^|\/)(?:test|tests)\/|(?:^|\/)test_[^/]+\.py$|_test\.py$/iu.test(
    rel,
  );
}

export {
  hasLeadingDocComment,
  hasPythonDocstringAfter,
  isPythonTestPath,
  scanPythonDocumentationHints,
  scanRustDocumentationHints,
  scanTypeScriptDocumentationHints,
};
