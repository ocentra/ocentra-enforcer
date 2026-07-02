import fs from "node:fs";
import path from "node:path";
import { collectFiles } from "../src/path-utils.mjs";
import {
  childDirs,
  finding,
  hasFile,
  scopedProjectRoots,
} from "./check-source-core-helpers.mjs";

function collectRequiredTestFindings(
  root,
  config,
  scope = { mode: "all" },
  args = {},
) {
  const findings = [];
  const scopedRoots = scopedProjectRoots(root, config, scope);
  const strictEmptyTestTrees =
    args.strictEmptyTestTrees === true || config.strictEmptyTestTrees === true;
  for (const workspaceRoot of ["packages", "apps"]) {
    for (const dir of childDirs(path.join(root, workspaceRoot))) {
      if (hasScopedRoots(scopedRoots) && !scopedRoots.has(normalizeProjectRoot(root, dir))) {
        continue;
      }
      const packageJsonPath = path.join(dir, "package.json");
      const srcPath = path.join(dir, "src");
      if (!fs.existsSync(packageJsonPath) || !fs.existsSync(srcPath)) continue;
      const manifest = JSON.parse(fs.readFileSync(packageJsonPath, "utf8"));
      const hasTests = hasFile(path.join(dir, "tests"), (file) =>
        /\.(?:test|spec)\.[cm]?tsx?$/u.test(file),
      );
      if (!hasTests) {
        findings.push(
          finding(
            root,
            packageJsonPath,
            1,
            "TEST-2.1",
            `${manifest.name ?? normalizeProjectRoot(root, dir)} is missing tests/*.test.ts`,
            null,
          ),
        );
      }
      collectInlineSourceTestFindings(root, srcPath, config, findings);
      collectStrictEmptyTestTreeFindings(
        root,
        dir,
        strictEmptyTestTrees,
        findings,
      );
    }
  }

  for (const dir of childDirs(path.join(root, "crates"))) {
    if (hasScopedRoots(scopedRoots) && !scopedRoots.has(normalizeProjectRoot(root, dir))) {
      continue;
    }
    const cargoPath = path.join(dir, "Cargo.toml");
    if (!fs.existsSync(cargoPath)) continue;
    const hasIntegrationTest = hasFile(path.join(dir, "tests"), (file) =>
      file.endsWith(".rs"),
    );
    if (!hasIntegrationTest) {
      findings.push(
        finding(
          root,
          cargoPath,
          1,
          "TEST-2.1",
          `${normalizeProjectRoot(root, dir)} is missing organized Rust tests under tests/`,
          null,
        ),
      );
    }
    collectInlineSourceTestFindings(root, path.join(dir, "src"), config, findings);
    collectStrictEmptyTestTreeFindings(
      root,
      dir,
      strictEmptyTestTrees,
      findings,
    );
  }

  return findings.filter((entry) => !isIgnored(entry.file, config));
}

function collectInlineSourceTestFindings(root, srcPath, config, findings) {
  if (!fs.existsSync(srcPath)) return;
  const files = collectFiles(
    root,
    [srcPath],
    config,
    (file) => isInlineTestSourceCandidate(file),
  );
  for (const file of files) {
    const text = fs.readFileSync(file, "utf8");
    const lines = text.split(/\r?\n/u);
    const pattern = inlineTestPatternForFile(file);
    for (const [index, line] of lines.entries()) {
      if (!pattern.test(line)) continue;
      findings.push(
        finding(
          root,
          file,
          index + 1,
          "TEST-2.2",
          `${normalizeProjectRoot(root, file)} contains inline test code; move it under an organized test root`,
          line,
        ),
      );
      break;
    }
  }
}

function isInlineTestSourceCandidate(file) {
  return /\.(?:rs|[cm]?[jt]sx?|py)$/u.test(file);
}

function inlineTestPatternForFile(file) {
  if (file.endsWith(".rs")) return /^\s*#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]/u;
  if (file.endsWith(".py")) return /^\s*def\s+test_[A-Za-z0-9_]+\s*\(/u;
  return /^\s*(?:describe|it|test)(?:\s*\.\s*(?:skip|only|todo|concurrent))?\s*\(/u;
}

function collectStrictEmptyTestTreeFindings(root, projectDir, strictMode, findings) {
  if (!strictMode) return;
  const testsPath = path.join(projectDir, "tests");
  if (!fs.existsSync(testsPath)) return;
  collectEmptyPlaceholderTrees(root, testsPath, findings);
}

function collectEmptyPlaceholderTrees(root, treePath, findings) {
  const stack = [treePath];
  let hasRealFile = false;
  let reportedCount = 0;
  while (stack.length > 0) {
    const current = stack.pop();
    const entries = fs.readdirSync(current, { withFileTypes: true });
    const files = entries.filter((entry) => entry.isFile());
    const dirs = entries.filter((entry) => entry.isDirectory());
    const realFiles = files.filter((entry) => entry.name !== ".gitkeep");
    if (realFiles.length > 0) hasRealFile = true;
    if (dirs.length === 0 && files.length > 0 && realFiles.length === 0) {
      reportedCount += 1;
      const placeholderCount = files.length;
      findings.push(
        finding(
          root,
          current,
          1,
          "TEST-2.1",
          `${path.relative(root, current).replaceAll("\\", "/")}: empty test/proof category tree contains only ${placeholderCount} .gitkeep placeholder file${placeholderCount === 1 ? "" : "s"}`,
          null,
        ),
      );
    }
    if (dirs.length === 0 && files.length === 0) {
      reportedCount += 1;
      findings.push(
        finding(
          root,
          current,
          1,
          "TEST-2.1",
          `${path.relative(root, current).replaceAll("\\", "/")}: empty test/proof category tree has no files`,
          null,
        ),
      );
    }
    for (const dir of dirs) stack.push(path.join(current, dir.name));
  }
  if (!hasRealFile && reportedCount === 0) {
    const detail = fs.readdirSync(treePath).length === 0
      ? `${path.relative(root, treePath).replaceAll("\\", "/")}: empty test/proof category tree has no files`
      : `${path.relative(root, treePath).replaceAll("\\", "/")}: empty test/proof category tree has no real test files`;
    findings.push(finding(root, treePath, 1, "TEST-2.1", detail, null));
  }
  return { hasRealFile, reportedCount };
}

function hasScopedRoots(scopedRoots) {
  return scopedRoots instanceof Set && scopedRoots.size > 0;
}

function isIgnored(file, config) {
  const rel = String(file ?? "").replaceAll("\\", "/");
  const ignoreDirs = config.ignoreDirs ?? [];
  return rel.split("/").some((part) => ignoreDirs.includes(part));
}

function normalizeProjectRoot(root, target) {
  return path.relative(root, target).replaceAll("\\", "/");
}

export {
  collectEmptyPlaceholderTrees,
  collectInlineSourceTestFindings,
  collectRequiredTestFindings,
  collectStrictEmptyTestTreeFindings,
  inlineTestPatternForFile,
  isInlineTestSourceCandidate,
};
