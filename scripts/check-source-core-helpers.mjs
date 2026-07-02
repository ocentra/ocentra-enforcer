import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import {
  collectFiles,
  lineNumberAt,
  matchesAnyGlob,
  normalizeRel,
  repoAbsolute,
  uniqueSorted,
} from "../src/path-utils.mjs";
import { GENERIC_RULES } from "../src/generic-scanners.mjs";
import { enrichFindingMetadata } from "../src/rule-registry.mjs";
import { CHECK_RULES } from "../src/check-metadata.mjs";

const RULE_ID_RE = /\b[A-Z]{2,}-\d+(?:\.\d+)*\b/gu;
const PACK_ROOT = path.resolve(path.join(path.dirname(fileURLToPath(import.meta.url)), ".."));

function resolvePackRoot(root) {
  return path.resolve(root ?? process.cwd());
}

function loadRegistryRules(packRoot) {
  const registryPath = path.join(packRoot, "rules", "rules.json");
  return JSON.parse(fs.readFileSync(registryPath, "utf8")).rules ?? [];
}

function collectRegistryRuleMetadataFindings(root, packRoot, registryPath, rule, findings) {
  const rel = normalizeRel(packRoot, registryPath);
  const text = fs.readFileSync(registryPath, "utf8");
  if (!rule.validator) {
    findings.push(
      finding(
        root,
        registryPath,
        1,
        "ENF-1.1",
        `${rule.id} registry entry is missing a validator`,
        null,
      ),
    );
  }
  if (!rule.title) {
    findings.push(
      finding(
        root,
        registryPath,
        1,
        "ENF-1.1",
        `${rule.id} registry entry is missing a title`,
        null,
      ),
    );
  }
  if (!text.includes(`"${rule.id}"`)) {
    findings.push(
      finding(
        root,
        registryPath,
        1,
        "ENF-1.1",
        `${rule.id} registry metadata is not present in the registry file`,
        null,
      ),
    );
  }
  if (!rel) return;
}

function collectRegistryDocFindings(root, packRoot, rule, findings) {
  const docPath = path.join(packRoot, rule.doc);
  if (!fs.existsSync(docPath)) {
    findings.push(
      finding(root, docPath, 1, "ENF-1.2", `${rule.id} doc is missing`, null),
    );
    return;
  }
  const text = fs.readFileSync(docPath, "utf8");
  if (!markdownAnchors(text).some((anchor) => anchor === rule.id.toLowerCase())) {
    findings.push(
      finding(root, docPath, 1, "ENF-1.2", `${rule.id} doc is missing an anchor`, null),
    );
  }
}

function markdownAnchors(text) {
  const anchors = new Set();
  for (const line of String(text).split(/\r?\n/u)) {
    const match = /^#{1,6}\s+(.+?)\s*$/u.exec(line);
    if (match) anchors.add(markdownAnchor(match[1]));
  }
  return [...anchors];
}

function markdownAnchor(heading) {
  return String(heading)
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/gu, "-")
    .replace(/^-+|-+$/gu, "");
}

function collectFixtureEvidence(packRoot) {
  const evidence = new Map();
  const fixtureRoot = path.join(packRoot, "docs", "expectations");
  for (const file of collectSourceFiles(fixtureRoot, [".mjs", ".js", ".ts", ".tsx", ".rs", ".py", ".json", ".toml", ".txt", ".md", ".yml", ".yaml"])) {
    const rel = normalizeRel(packRoot, file);
    const text = fs.readFileSync(file, "utf8");
    for (const match of text.matchAll(RULE_ID_RE)) {
      ensureFixtureEvidenceEntry(evidence, match[0]).push(rel);
    }
  }
  for (const file of collectSourceFiles(path.join(packRoot, "tests"), [".mjs", ".js", ".ts", ".rs", ".py", ".json"])) {
    const rel = normalizeRel(packRoot, file);
    const text = fs.readFileSync(file, "utf8");
    for (const match of text.matchAll(RULE_ID_RE)) {
      ensureFixtureEvidenceEntry(evidence, match[0]).push(rel);
    }
  }
  for (const file of collectSourceFiles(path.join(packRoot, "rules"), [".md"])) {
    const rel = normalizeRel(packRoot, file);
    const text = fs.readFileSync(file, "utf8");
    for (const match of text.matchAll(RULE_ID_RE)) {
      ensureFixtureEvidenceEntry(evidence, match[0]).push(rel);
    }
  }
  for (const folder of ["src", "scripts", "mcp"]) {
    for (const file of collectSourceFiles(path.join(packRoot, folder), [".mjs", ".js"])) {
      const rel = normalizeRel(packRoot, file);
      const text = fs.readFileSync(file, "utf8");
      for (const match of text.matchAll(RULE_ID_RE)) {
        ensureFixtureEvidenceEntry(evidence, match[0]).push(rel);
      }
    }
  }
  return evidence;
}

function ensureFixtureEvidenceEntry(evidence, ruleId) {
  if (!evidence.has(ruleId)) evidence.set(ruleId, []);
  return evidence.get(ruleId);
}

function ruleIdFromFixturePath(rel) {
  const match = RULE_ID_RE.exec(rel);
  RULE_ID_RE.lastIndex = 0;
  return match?.[0] ?? null;
}

function collectRoutedDocRuleIds(packRoot) {
  const docRoot = path.join(packRoot, "rules");
  return collectSourceFiles(docRoot, [".md"]).flatMap((file) => {
    const text = fs.readFileSync(file, "utf8");
    return [...text.matchAll(RULE_ID_RE)].map((match) => match[0]);
  });
}

function collectScannerRuleIds(packRoot) {
  const ruleFiles = collectSourceFiles(path.join(packRoot, "src"), [".mjs", ".js"]);
  const ids = new Set();
  for (const file of ruleFiles) {
    const text = fs.readFileSync(file, "utf8");
    for (const match of text.matchAll(RULE_ID_RE)) ids.add(match[0]);
  }
  return [...ids].sort();
}

function collectSourceFiles(start, extensions) {
  if (!fs.existsSync(start)) return [];
  return collectFiles(start, [start], {}, (file) =>
    extensions.length === 0 || extensions.includes(path.extname(file)),
  );
}

function existingConfigPath(root) {
  for (const candidate of [
    path.join(root, "ocentra-enforcer.config.json"),
    path.join(root, ".ocentra-enforcer.json"),
    path.join(root, ".ocentra", "config.json"),
  ]) {
    if (fs.existsSync(candidate)) return candidate;
  }
  return null;
}

function readRawConfigObject(configPath) {
  return JSON.parse(fs.readFileSync(configPath, "utf8"));
}

function hasOverrideWaiverMetadata(override) {
  return Boolean(
    override &&
      typeof override === "object" &&
      (override.note || override.expiresAt || override.waiverId || override.justification),
  );
}

function hasWaiverFor(config, ruleId) {
  return (config.waivers ?? []).some((waiver) => {
    const ruleIds = Array.isArray(waiver.ruleIds) ? waiver.ruleIds : [];
    return ruleIds.includes(ruleId);
  });
}

function parseUtcDate(value) {
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? null : date;
}

function startOfUtcDay(date) {
  return new Date(Date.UTC(date.getUTCFullYear(), date.getUTCMonth(), date.getUTCDate()));
}

function daysBetweenUtc(start, end) {
  return Math.floor((startOfUtcDay(end) - startOfUtcDay(start)) / (24 * 60 * 60 * 1000));
}

function isBroadWaiverScope(scope) {
  return !scope || scope === "all" || scope === "*" || scope === "**";
}

function buildReport({ root, config, checkName, findings, scope = null }) {
  const active = findings.filter((entry) => !isIgnored(entry.file, config));
  return {
    ok: active.length === 0,
    root,
    check: checkName,
    scope: scope ? reportScope(root, scope, active) : undefined,
    findings: active,
  };
}

function collectPolicyFiles(root, config, policy, scope = { mode: "all" }) {
  return collectFiles(
    root,
    scopeEntries(root, scope, config),
    config,
    (file) => isUnderRoots(normalizeRel(root, file), policy.roots ?? []),
  );
}

function childDirs(dir) {
  if (!fs.existsSync(dir)) return [];
  return fs
    .readdirSync(dir, { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => path.join(dir, entry.name));
}

function hasFile(start, predicate) {
  if (!fs.existsSync(start)) return false;
  if (fs.statSync(start).isFile()) return predicate(start);
  return fs
    .readdirSync(start, { withFileTypes: true })
    .some((entry) => hasFile(path.join(start, entry.name), predicate));
}

function countMatches(lines, pattern) {
  return lines.reduce((count, line) => count + (pattern.test(line) ? 1 : 0), 0);
}

function maxBraceNestingDepth(lines) {
  let depth = 0;
  let max = 0;
  for (const line of lines) {
    for (const ch of line) {
      if (ch === "{") max = Math.max(max, ++depth);
      else if (ch === "}") depth = Math.max(0, depth - 1);
    }
  }
  return max;
}

function maxPythonIndentDepth(lines) {
  let max = 0;
  for (const line of lines) {
    if (!String(line).trim()) continue;
    const spaces = leadingWhitespace(line).replace(/\t/gu, "    ").length;
    max = Math.max(max, Math.floor(spaces / 4));
  }
  return max;
}

function countLines(text) {
  return String(text).split(/\r?\n/u).length;
}

function findBlockEnd(lines, start) {
  let depth = 0;
  let sawOpeningBrace = false;
  for (let index = start; index < lines.length; index += 1) {
    const line = lines[index];
    for (const ch of line) {
      if (ch === "{") {
        depth += 1;
        sawOpeningBrace = true;
        continue;
      }
      if (ch === "}") {
        depth -= 1;
        if (sawOpeningBrace && depth === 0) return index + 1;
      }
    }
  }
  return lines.length;
}

function findPythonBlockEnd(lines, start) {
  const startIndent = leadingWhitespace(lines[start]).length;
  let last = start + 1;
  for (let index = start + 1; index < lines.length; index += 1) {
    const line = lines[index];
    if (!String(line).trim()) {
      last = index + 1;
      continue;
    }
    if (leadingWhitespace(line).length <= startIndent) return last;
    last = index + 1;
  }
  return lines.length;
}

function leadingWhitespace(line) {
  return String(line).match(/^\s*/u)?.[0] ?? "";
}

function valueAtPath(source, jsonPath) {
  let value = source;
  for (const segment of jsonPath.split(".")) {
    if (value === null || typeof value !== "object" || !(segment in value)) {
      throw new Error(`${jsonPath} is missing`);
    }
    value = value[segment];
  }
  return value;
}

function valueFromSpec(root, ownerPath, valueSpec) {
  const sourceText = fs.readFileSync(repoAbsolute(root, ownerPath), "utf8");
  if ("jsonPath" in valueSpec)
    return valueAtPath(JSON.parse(sourceText), valueSpec.jsonPath);
  if ("sourceObjectPath" in valueSpec)
    return valueAtSourceObjectPath(
      sourceText,
      valueSpec.sourceObjectPath,
      ownerPath,
    );
  if ("rustConst" in valueSpec)
    return valueAtRustConst(sourceText, valueSpec.rustConst, ownerPath);
  if ("rustSerdeRename" in valueSpec)
    return valueAtRustSerdeRename(
      sourceText,
      valueSpec.rustSerdeRename,
      ownerPath,
    );
  throw new Error(
    `${ownerPath}: ${valueSpec.name} needs jsonPath, sourceObjectPath, rustConst, or rustSerdeRename`,
  );
}

function loadContract(root, rawContract) {
  const contract =
    typeof rawContract === "string"
      ? JSON.parse(fs.readFileSync(path.join(root, rawContract), "utf8"))
      : rawContract;
  const ownerPath = contract.ownerPath;
  const values = (contract.values ?? []).map((valueSpec) => {
    const text = valueFromSpec(root, ownerPath, valueSpec);
    if (typeof text !== "string" || text.length === 0) {
      throw new Error(
        `${ownerPath}: ${valueSpec.name} must be a non-empty string`,
      );
    }
    return {
      name: valueSpec.name,
      text,
      pattern: createLiteralMatchPattern(text),
    };
  });
  const valueByName = new Map(values.map((value) => [value.name, value.text]));
  const mirrorPaths = [];
  for (const mirror of contract.mirrors ?? []) {
    mirrorPaths.push(mirror.path);
    for (const mirrorValueSpec of mirror.values ?? []) {
      const ownerText = valueByName.get(mirrorValueSpec.name);
      if (ownerText === undefined)
        throw new Error(
          `${mirror.path}: ${mirrorValueSpec.name} does not match an owner value name`,
        );
      const mirrorText = valueFromSpec(root, mirror.path, mirrorValueSpec);
      if (mirrorText !== ownerText) {
        throw new Error(
          `${mirror.path}: ${contract.name}.${mirrorValueSpec.name} ${mirrorText} does not match ${ownerPath} ${ownerText}`,
        );
      }
    }
  }
  return {
    ...contract,
    allowedPaths: new Set(
      [ownerPath, ...mirrorPaths, ...(contract.allowedPaths ?? [])].map((entry) =>
        entry.replaceAll("\\", "/"),
      ),
    ),
    scanRoots: contract.scanRoots ?? [],
    values,
  };
}

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
    if (contract.ownerPath?.startsWith(`${rootPath}/`))
      covered.add(contract.ownerPath);
    for (const mirror of contract.mirrors ?? []) {
      if (mirror.path?.startsWith(`${rootPath}/`)) covered.add(mirror.path);
    }
    for (const allowedPath of contract.allowedPaths ?? []) {
      if (allowedPath?.startsWith(`${rootPath}/`)) covered.add(allowedPath);
    }
  }
  return covered;
}

function valueAtSourceObjectPath(source, sourceObjectPath, ownerPath) {
  const [objectName, propertyName] = sourceObjectPath.split(".");
  const objectPattern = new RegExp(
    `(?:export\\s+)?const\\s+${escapeRegExp(objectName)}\\s*=\\s*\\{([\\s\\S]*?)\\}\\s*(?:as\\s+const)?`,
    "mu",
  );
  const objectMatch = objectPattern.exec(source);
  if (!objectMatch) throw new Error(`Unable to find ${sourceObjectPath} in ${ownerPath}`);
  const propertyPattern = new RegExp(
    "\\b" +
      escapeRegExp(propertyName) +
      "\\s*:\\s*(['\"`])([^'\"`]+)\\1",
    "mu",
  );
  const propertyMatch = propertyPattern.exec(objectMatch[1]);
  return propertyMatch?.[2] ?? null;
}

function valueAtRustConst(source, rustConst, ownerPath) {
  const pattern = new RegExp(
    `(?:pub\\s+)?const\\s+${escapeRegExp(rustConst)}\\s*:\\s*&str\\s*=\\s*"([^"]+)"\\s*;`,
    "mu",
  );
  const match = pattern.exec(source);
  if (!match) throw new Error(`Unable to find ${rustConst} in ${ownerPath}`);
  return match[1];
}

function valueAtRustSerdeRename(source, rustSerdeRename, ownerPath) {
  const [enumName, variantName] = rustSerdeRename.split(".");
  const enumPattern = new RegExp(
    `enum\\s+${escapeRegExp(enumName)}\\s*\\{([\\s\\S]*?)\\n\\}`,
    "mu",
  );
  const enumMatch = enumPattern.exec(source);
  if (!enumMatch) throw new Error(`Unable to find ${rustSerdeRename} in ${ownerPath}`);
  const renamePattern = new RegExp(
    `#\\[serde\\(rename\\s*=\\s*"([^"]+)"\\)\\]\\s*${escapeRegExp(variantName)}\\b`,
    "mu",
  );
  const renameMatch = renamePattern.exec(enumMatch[1]);
  return renameMatch?.[1] ?? null;
}

function createLiteralMatchPattern(value) {
  return new RegExp(
    `(?<![A-Za-z0-9@._/-])${escapeRegExp(value)}(?![A-Za-z0-9@._/-])`,
    "u",
  );
}

function escapeRegExp(value) {
  return String(value).replace(/[.*+?^${}()|[\]\\]/gu, "\\$&");
}

function sourceContractExtension(filePath) {
  return /\.(?:rs|ts|tsx|mjs|cjs|js|json|md|ya?ml)$/u.test(filePath);
}

function isNonBlockingContractPath(rel) {
  return /^(?:docs(?:\/|$)|scripts\/test(?:\/|$))|.*(?:^|\/)tests?\/|.*(?:^|\/)[^/]*_tests?\.rs$|.*(?:^|\/)[^/]*\.(?:test|spec)\.(?:ts|tsx|js|jsx|mjs|cjs)$/u.test(
    rel,
  );
}

function scopeEntries(root, scope, config = {}) {
  if (scope.mode === "files") return scope.files ?? [];
  if (scope.mode === "diff") return diffFiles(root, scope.base, scope.head);
  if (scope.mode === "crate")
    return [
      scope.crateRoot ?? crateRootForName(root, config, scope.crateName),
    ].filter(Boolean);
  return [];
}

function scopeFilesByExtensions(root, scope, config, extensions) {
  return collectFiles(root, scopeEntries(root, scope, config), config, (file) =>
    extensions.has(path.extname(file).toLowerCase()),
  );
}

function scopeRelativeFiles(root, scope, config = {}) {
  return collectFiles(
    root,
    scopeEntries(root, scope, config),
    config,
    () => true,
  ).map((file) => normalizeRel(root, file));
}

function scopedProjectRoots(root, config, scope) {
  if (scope.mode === "all") return null;
  const rels = scopeRelativeFiles(root, scope, config);
  const roots = new Set();
  for (const rel of rels) {
    const segments = rel.split("/");
    if (
      (segments[0] === "packages" ||
        segments[0] === "apps" ||
        segments[0] === "crates") &&
      segments[1]
    ) {
      roots.add(`${segments[0]}/${segments[1]}`);
    }
  }
  return roots;
}

function trackedScopeFiles(root, scope) {
  const rels =
    scope.mode === "all"
      ? gitNameOnly(root, ["ls-files"])
      : scopeRelativeFiles(root, scope, {});
  if (scope.mode === "all") return rels;
  const tracked = new Set(gitNameOnly(root, ["ls-files"]));
  return rels.filter((rel) => tracked.has(rel));
}

function stagedFiles(root) {
  return gitNameOnly(root, [
    "diff",
    "--cached",
    "--name-only",
    "--diff-filter=ACMR",
  ]);
}

function diffFiles(root, base, head) {
  if (!base || !head) return [];
  return gitNameOnly(root, [
    "diff",
    "--name-only",
    "--diff-filter=ACMR",
    base,
    head,
  ]);
}

function gitNameOnly(root, args) {
  const result = spawnSync("git", args, {
    cwd: root,
    encoding: "utf8",
    shell: false,
  });
  if ((result.status ?? 1) !== 0) return [];
  return String(result.stdout ?? "")
    .split(/\r?\n/u)
    .map((line) => line.trim())
    .filter(Boolean)
    .map((line) => line.replaceAll("\\", "/"));
}

function crateRootForName(root, config, crateName) {
  if (!crateName) return null;
  for (const manifest of collectFiles(
    root,
    config.crateRootGlobs ?? ["crates/*", "tools/*", "."],
    config,
    (file) => path.basename(file) === "Cargo.toml",
  )) {
    const text = fs.readFileSync(manifest, "utf8");
    if (
      new RegExp(`^\\s*name\\s*=\\s*"${escapeRegExp(crateName)}"`, "mu").test(
        text,
      )
    )
      return path.dirname(manifest);
  }
  return null;
}

function isUnderRoots(rel, roots) {
  if (!roots || roots.length === 0) return true;
  return roots.some((root) => rel.startsWith(`${root}/`) || rel === root);
}

function importSpecifier(line) {
  return line.match(/from\s+["'`](.*?)["'`]/u)?.[1] ?? null;
}

function isGeneratedArtifactPath(rel) {
  return (
    /^(?:output|test-results|playwright-report)\//u.test(rel) ||
    /(?:^|\/)(?:dist|build|coverage)\//u.test(rel)
  );
}

function reportScope(root, scope, findings) {
  const files =
    scope.mode === "all"
      ? undefined
      : scope.mode === "files"
        ? (scope.files ?? []).map((file) => normalizeRel(root, repoAbsolute(root, file)))
        : scopeRelativeFiles(root, scope, {});
  return {
    mode: scope.mode,
    files,
    findingCount: findings.length,
  };
}

function resolveContractConfigPath(root, explicitConfigPath) {
  if (explicitConfigPath) return path.isAbsolute(explicitConfigPath) ? explicitConfigPath : path.join(root, explicitConfigPath);
  return existingConfigPath(root);
}

function spawnInRoot(root, command, args) {
  const invocation = resolveCommand(command, args);
  return spawnSync(invocation.command, invocation.args, {
    cwd: root,
    encoding: "utf8",
    shell: false,
  });
}

function resolveCommand(command, args) {
  if (
    process.platform === "win32" &&
    (command === "npm" || command === "npx" || command === "pnpm")
  ) {
    return {
      command: process.env.ComSpec || "cmd.exe",
      args: ["/d", "/s", "/c", command, ...args],
    };
  }
  return { command, args };
}

function compactProcessOutput(result) {
  return {
    status: result.status ?? 0,
    stdout: String(result.stdout ?? "").trim(),
    stderr: String(result.stderr ?? "").trim(),
  };
}

function isIgnored(file, config) {
  return (config.ignore ?? []).some((glob) => matchesAnyGlob(file, [glob]));
}

function finding(root, file, line, ruleId, detail, source) {
  return enrichFindingMetadata(
    {
      ruleId,
      detail,
      file: normalizeRel(root, file),
      line,
      source: source == null ? null : String(source).trim(),
    },
    PACK_ROOT,
    CHECK_RULES,
  );
}

function genericFinding(root, file, line, ruleId, detail, source) {
  return enrichFindingMetadata(
    {
      ruleId,
      detail,
      file: normalizeRel(root, file),
      line,
      source: source == null ? null : String(source).trim(),
    },
    PACK_ROOT,
    { ...GENERIC_RULES, ...CHECK_RULES },
  );
}

export {
  RULE_ID_RE,
  buildReport,
  childDirs,
  collectContractScanFiles,
  collectCoveredContractPaths,
  collectFixtureEvidence,
  collectPolicyFiles,
  collectRegistryDocFindings,
  collectRegistryRuleMetadataFindings,
  collectRoutedDocRuleIds,
  collectScannerRuleIds,
  collectSourceFiles,
  compactProcessOutput,
  countMatches,
  countLines,
  createLiteralMatchPattern,
  crateRootForName,
  daysBetweenUtc,
  diffFiles,
  enforceRequiredMirrorCoverage,
  escapeRegExp,
  existingConfigPath,
  findBlockEnd,
  findPythonBlockEnd,
  finding,
  genericFinding,
  gitNameOnly,
  hasFile,
  hasOverrideWaiverMetadata,
  hasWaiverFor,
  importSpecifier,
  isBroadWaiverScope,
  isGeneratedArtifactPath,
  isIgnored,
  isNonBlockingContractPath,
  isUnderRoots,
  leadingWhitespace,
  loadContract,
  loadRegistryRules,
  markdownAnchor,
  markdownAnchors,
  maxBraceNestingDepth,
  maxPythonIndentDepth,
  parseUtcDate,
  readRawConfigObject,
  reportScope,
  resolveCommand,
  resolveContractConfigPath,
  resolvePackRoot,
  scopedProjectRoots,
  scopeEntries,
  scopeFilesByExtensions,
  scopeRelativeFiles,
  sourceContractExtension,
  spawnInRoot,
  stagedFiles,
  startOfUtcDay,
  trackedScopeFiles,
  valueAtPath,
  valueAtRustConst,
  valueAtRustSerdeRename,
  valueAtSourceObjectPath,
  valueFromSpec,
};
