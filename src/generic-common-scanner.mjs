import fs from "node:fs";
import path from "node:path";
import { normalizeRel } from "./path-utils.mjs";
import { scanAdditionalCommonFile } from "./source-policy-scanners.mjs";
import {
  buildCommonScanContext,
  scanCommonFilePrelude,
  scanCommonLineRules,
} from "./generic-common-line-rules.mjs";
import { scanSourceOwnershipPolicy } from "./generic-common-source-ownership.mjs";
import {
  scanPythonDocumentationHints,
  scanRustDocumentationHints,
  scanTypeScriptDocumentationHints,
} from "./documentation-hints.mjs";
import {
  addViolation,
  PY_EXTENSIONS,
  TS_EXTENSIONS,
  hasOwnershipFile,
  isProductionSourcePath,
  isPythonTestPath,
  isTestPath,
} from "./generic-scanner-shared.mjs";

export function scanCommonFile(root, filePath) {
  const rel = normalizeRel(root, filePath);
  const lines = fs.readFileSync(filePath, "utf8").split(/\r?\n/u);
  const text = lines.join("\n");
  const ext = path.extname(filePath).toLowerCase();
  const violations = [];
  const context = buildCommonScanContext(root, filePath, rel, ext, lines, text, violations, hasOwnershipFile);
  scanCommonFilePrelude(context);
  lines.forEach((line, idx) => {
    scanCommonLineRules(context, line, idx);
  });
  if (context.isProductionSource) {
    violations.push(...scanSourceOwnershipPolicy(root, filePath, rel, lines));
  }
  if (path.basename(rel).toLowerCase() === "pyproject.toml") {
    violations.push(...scanPythonToolchainPolicy(root, filePath, lines));
  }
  if (path.basename(rel).toLowerCase() === "requirements.txt") {
    violations.push(...scanPythonRequirementsPolicy(root, filePath, lines));
  }
  violations.push(...scanAdditionalCommonFile(root, filePath, lines));
  violations.push(...scanDocumentationHints(root, filePath, rel, lines));
  return violations;
}

function scanPythonToolchainPolicy(root, filePath, lines) {
  const violations = [];
  const text = lines.join("\n");
  const dir = path.dirname(filePath);
  if (!/\[tool\.ruff(?:\.|])/.test(text)) {
    addViolation(violations, root, filePath, 1, "PY-5.2", "pyproject.toml is missing Ruff configuration.", "pyproject.toml");
  }
  const hasPyright = /\[tool\.pyright]/.test(text);
  const hasMypy = /\[tool\.mypy]/.test(text);
  if (!hasPyright && !hasMypy) {
    addViolation(violations, root, filePath, 1, "PY-5.3", "pyproject.toml is missing Pyright or mypy configuration.", "pyproject.toml");
  }
  if ((!hasPyright && !hasMypy) || (hasPyright && !/typeCheckingMode\s*=\s*["']strict["']/.test(text)) || (hasMypy && !/strict\s*=\s*true/.test(text))) {
    addViolation(violations, root, filePath, 1, "PY-5.4", "Python type checker strict mode is not enabled.", "pyproject.toml");
  }
  if (!["uv.lock", "poetry.lock", "pdm.lock"].some((name) => fs.existsSync(path.join(dir, name)))) {
    addViolation(violations, root, filePath, 1, "PY-5.7", "Python lockfile is missing.", "pyproject.toml");
  }
  lines.forEach((line, index) => {
    if (/git\+|@\s*git(?:hub|lab)?\.com|https:\/\/github\.com/iu.test(line)) {
      addViolation(violations, root, filePath, index + 1, "PY-5.9", "Python git dependency found.", line);
    }
    if (/(?:path|file)\s*=\s*["'][^"']+["']|(?:\.\.\/|\.\/)/u.test(line)) {
      addViolation(violations, root, filePath, index + 1, "PY-5.10", "Python local path dependency found.", line);
    }
  });
  return violations;
}

function scanPythonRequirementsPolicy(root, filePath, lines) {
  const violations = [];
  if (!fs.existsSync(path.join(path.dirname(filePath), "pyproject.toml"))) {
    addViolation(violations, root, filePath, 1, "PY-5.1", "requirements.txt has no pyproject.toml owner.", "requirements.txt");
  }
  lines.forEach((line, index) => {
    const trimmed = line.trim();
    if (trimmed === "" || trimmed.startsWith("#")) return;
    if (/git\+|https:\/\/github\.com/iu.test(trimmed)) {
      addViolation(violations, root, filePath, index + 1, "PY-5.9", "Python git dependency found.", line);
      return;
    }
    if (/(?:^|-e\s+)(?:\.{1,2}\/|file:)/u.test(trimmed)) {
      addViolation(violations, root, filePath, index + 1, "PY-5.10", "Python local path dependency found.", line);
      return;
    }
    if (!/==[^=\s]+/.test(trimmed) && !/--hash=/.test(trimmed)) {
      addViolation(violations, root, filePath, index + 1, "PY-5.8", "unpinned Python requirement found.", line);
    }
  });
  return violations;
}

function scanDocumentationHints(root, filePath, rel, lines) {
  if (isTestPath(rel) || isPythonTestPath(rel) || /\.d\.ts$/iu.test(rel))
    return [];
  const ext = path.extname(filePath).toLowerCase();
  if (TS_EXTENSIONS.has(ext))
    return scanTypeScriptDocumentationHints(addViolation, root, filePath, lines);
  if (PY_EXTENSIONS.has(ext))
    return scanPythonDocumentationHints(addViolation, root, filePath, lines);
  if (ext === ".rs") return scanRustDocumentationHints(addViolation, root, filePath, lines);
  return [];
}
