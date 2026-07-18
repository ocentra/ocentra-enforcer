import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";
import {
  collectFiles,
  lineNumberAt,
  normalizeRel,
  repoAbsolute,
  uniqueSorted,
} from "../src/path-utils.mjs";
import { GENERIC_RULES, runGenericScan } from "../src/generic-scanners.mjs";
import { scanAdditionalTypeScriptFile } from "../src/source-policy-scanners.mjs";
import {
  applyRulePolicy,
  applyWaivers,
  buildRegistryPolicyMap,
  buildRegistrySeverityMap,
  isSeverityDowngrade,
  isStrictProfile,
  normalizeFailOn,
  policyForRule,
  rulePolicyCapabilities,
  splitFindings,
} from "../src/policy.mjs";
import {
  enrichFindingMetadata,
  enrichFindingsMetadata,
  registryRules,
} from "../src/rule-registry.mjs";
import {
  CHECK_ALIASES,
  CHECK_RULES,
  DEFAULT_ALLOWED_LICENSES,
  SCANNER_BACKED_CHECKS,
} from "../src/check-metadata.mjs";
import { countMatches } from "./check-source-core-helpers.mjs";
import {
  applySourceShapeOverrides,
  collectSourceShapeFindings,
  inspectPythonShape,
  inspectRustShape,
  inspectTypeScriptShape,
} from "./check-source-core-source-shape.mjs";
import {
  collectEmptyPlaceholderTrees,
  collectInlineSourceTestFindings,
  collectRequiredTestFindings,
  collectStrictEmptyTestTreeFindings,
  inlineTestPatternForFile,
  isInlineTestSourceCandidate,
} from "./check-source-core-tests.mjs";
import {
  collectGeneratedArtifactFindings,
  collectImportBoundaryFindings,
  collectNoNakedDomainStringsFindings,
  collectNoZodSourceFindings,
  collectPlaceholderImplementationFindings,
  collectReexportFindings,
  collectSecretFindings,
  collectSingleSourceContractFindings,
  collectSkippedFocusedTestFindings,
  collectValidationBypassFindings,
  collectWeakAssertionsFindings,
} from "./check-source-core-checks.mjs";
import {
  collectContractScanFiles,
  collectCoveredContractPaths,
  enforceRequiredMirrorCoverage,
  isNonBlockingContractPath,
  loadContract,
  resolveContractConfigPath,
  sourceContractExtension,
} from "./check-source-core-contracts.mjs";
import {
  createLiteralMatchPattern,
  escapeRegExp,
  valueAtPath,
  valueAtRustConst,
  valueAtRustSerdeRename,
  valueAtSourceObjectPath,
  valueFromSpec,
} from "./check-source-core-contract-values.mjs";

const PACK_ROOT = path.resolve(path.join(path.dirname(fileURLToPath(import.meta.url)), ".."));

function resolvePackRoot(root, args = {}) {
  if (args.packRoot) return path.resolve(args.packRoot);
  const rootRules = path.join(root, "rules", "rules.json");
  return fs.existsSync(rootRules) ? root : PACK_ROOT;
}

function loadRegistryRules(packRoot) {
  const registryPath = path.join(packRoot, "rules", "rules.json");
  if (!fs.existsSync(registryPath)) return [];
  const registry = JSON.parse(fs.readFileSync(registryPath, "utf8"));
  return Array.isArray(registry.rules) ? registry.rules : [];
}

function collectRegistryRuleMetadataFindings(root, packRoot, registryPath, rule, findings) {
  const required = [
    "id",
    "language",
    "family",
    "severity",
    "title",
    "snippet",
    "lockLevel",
    "canDisable",
    "canDowngrade",
    "requiresFailFixture",
    "requiresPassFixture",
    "appliesTo",
    "triggers",
    "validator",
    "doc",
  ];
  const missing = required.filter((field) => {
    const value = rule[field];
    if (Array.isArray(value)) return value.length === 0;
    return value === undefined || value === null || value === "";
  });
  if (missing.length > 0) {
    findings.push(
      finding(
        root,
        registryPath,
        1,
        "ENF-1.1",
        `${rule.id ?? "(unknown rule)"} registry metadata is missing: ${missing.join(", ")}`,
        null,
      ),
    );
  }
  if (String(rule.snippet ?? "").length > 240) {
    findings.push(
      finding(
        root,
        registryPath,
        1,
        "ENF-1.1",
        `${rule.id} snippet exceeds 240 characters`,
        null,
      ),
    );
  }
  if (rule.validator !== "review") {
    const capabilities = rulePolicyCapabilities(rule);
    if (
      capabilities.lockLevel === "immutable" &&
      (rule.canDisable !== false || rule.canDowngrade !== false)
    ) {
      findings.push(
        finding(
          root,
          registryPath,
          1,
          "CFG-1.2",
          `${rule.id} is immutable but registry permits disable/downgrade`,
          null,
        ),
      );
    }
  }
  void packRoot;
}

function collectRegistryDocFindings(root, packRoot, rule, findings) {
  const [docRel, anchor = ""] = String(rule.doc ?? "").split("#");
  const docPath = path.join(packRoot, docRel);
  if (!docRel || !fs.existsSync(docPath)) {
    findings.push(
      finding(
        root,
        docPath,
        1,
        "ENF-1.2",
        `${rule.id} doc file is missing: ${rule.doc}`,
        null,
      ),
    );
    return;
  }
  if (!anchor) {
    findings.push(
      finding(root, docPath, 1, "ENF-1.2", `${rule.id} doc is missing an anchor`, null),
    );
    return;
  }
  const anchors = markdownAnchors(fs.readFileSync(docPath, "utf8"));
  if (!anchors.has(anchor.toLowerCase())) {
    findings.push(
      finding(
        root,
        docPath,
        1,
        "ENF-1.2",
        `${rule.id} doc anchor #${anchor} is missing from ${docRel}`,
        null,
      ),
    );
  }
}

function markdownAnchors(text) {
  const anchors = new Set();
  for (const match of text.matchAll(/^#{1,6}\s+(.+)$/gmu)) {
    anchors.add(markdownAnchor(match[1]));
  }
  return anchors;
}

function markdownAnchor(heading) {
  return String(heading)
    .trim()
    .toLowerCase()
    .replace(/[`*_~[\]()]/gu, "")
    .replace(/[^\p{Letter}\p{Number}\s-]/gu, "")
    .replace(/\s+/gu, "-");
}

function collectFixtureEvidence(packRoot) {
  const evidence = new Map();
  const fixtureRoot = path.join(packRoot, "tests", "fixtures", "enforcer");
  for (const file of collectSourceFiles(fixtureRoot, [".mjs", ".js", ".ts", ".tsx", ".rs", ".py", ".json", ".toml", ".tf", ".txt", ".md", ".yml", ".yaml"])) {
    const fixtureRel = normalizeRel(packRoot, file);
    const ruleId = ruleIdFromFixturePath(fixtureRel);
    if (!ruleId) continue;
    const entry = ensureFixtureEvidenceEntry(evidence, ruleId);
    if (/\.fail\./iu.test(fixtureRel)) entry.failFixtures.push(fixtureRel);
    if (/\.pass\./iu.test(fixtureRel)) entry.passFixtures.push(fixtureRel);
    entry.fixtureRefs.add(fixtureRel);
    entry.fixtureRefs.add(path.basename(fixtureRel));
    entry.fixtureRefs.add(fixtureRel.replace(/^tests\/fixtures\/enforcer\//u, ""));
  }
  for (const file of collectSourceFiles(path.join(packRoot, "tests"), [".mjs", ".js", ".ts", ".rs", ".py", ".json"])) {
    const rel = normalizeRel(packRoot, file);
    if (rel.startsWith("tests/fixtures/")) continue;
    const text = fs.readFileSync(file, "utf8");
    for (const match of text.matchAll(RULE_ID_RE)) {
      ensureFixtureEvidenceEntry(evidence, match[0].toUpperCase()).testReferences.add(rel);
    }
    for (const [ruleId, entry] of evidence.entries()) {
      if ([...entry.fixtureRefs].some((ref) => text.includes(ref))) {
        entry.testReferences.add(rel);
      }
    }
  }
  return evidence;
}

function ensureFixtureEvidenceEntry(evidence, ruleId) {
  if (!evidence.has(ruleId)) {
    evidence.set(ruleId, {
      failFixtures: [],
      passFixtures: [],
      fixtureRefs: new Set(),
      testReferences: new Set(),
    });
  }
  return evidence.get(ruleId);
}

function ruleIdFromFixturePath(rel) {
  const match = path.basename(rel).match(/^([a-z]+-\d+\.\d+)/iu);
  return match ? match[1].toUpperCase() : null;
}

const RULE_ID_RE =
  /\b(?:RR|TS|PY|SEC|GEN|DOC|DOCENF|HAR|MCP|PROOF|SCAN|TEST|PORT|SRC|CONTRACT|DEP|NPM|CI|REPO|SBOM|AI|ENF|CFG|WAIVER|BOUND|ARCH)-[0-9]+\.[0-9]+\b/gu;

function collectRoutedDocRuleIds(packRoot) {
  const ids = new Set();
  for (const file of collectSourceFiles(path.join(packRoot, "rules"), [".md"])) {
    const text = fs.readFileSync(file, "utf8");
    for (const match of text.matchAll(RULE_ID_RE)) ids.add(match[0]);
  }
  return ids;
}

function collectScannerRuleIds(packRoot) {
  const ids = new Set();
  for (const folder of ["src", "scripts", "mcp"]) {
    for (const file of collectSourceFiles(path.join(packRoot, folder), [".mjs", ".js"])) {
      const text = fs.readFileSync(file, "utf8");
      for (const match of text.matchAll(RULE_ID_RE)) ids.add(match[0]);
    }
  }
  return ids;
}

function collectSourceFiles(start, extensions) {
  if (!fs.existsSync(start)) return [];
  const files = [];
  const stack = [start];
  const extensionSet = new Set(extensions);
  const seen = new Set();
  while (stack.length > 0) {
    const current = stack.pop();
    const stats = fs.lstatSync(current);
    if (stats.isSymbolicLink()) continue;
    const real = fs.realpathSync(current);
    if (seen.has(real)) continue;
    seen.add(real);
    if (stats.isDirectory()) {
      for (const entry of fs.readdirSync(current)) stack.push(path.join(current, entry));
      continue;
    }
    if (stats.isFile() && extensionSet.has(path.extname(current))) files.push(current);
  }
  return files.sort((a, b) => a.localeCompare(b));
}

function existingConfigPath(root) {
  for (const rel of ["ocentra-enforcer.config.json", "rust-rules.config.json"]) {
    const candidate = path.join(root, rel);
    if (fs.existsSync(candidate)) return candidate;
  }
  return null;
}

function readRawConfigObject(configPath) {
  if (!configPath || !fs.existsSync(configPath) || !fs.statSync(configPath).isFile()) {
    return null;
  }
  try {
    const parsed = JSON.parse(fs.readFileSync(configPath, "utf8"));
    return parsed && typeof parsed === "object" && !Array.isArray(parsed) ? parsed : null;
  } catch {
    return null;
  }
}

function hasOverrideWaiverMetadata(override) {
  return Boolean(
    override.waiverId &&
      override.owner &&
      override.issue &&
      override.reason &&
      Array.isArray(override.scope) &&
      override.scope.length > 0 &&
      override.expires &&
      override.remediation,
  );
}

function hasWaiverFor(config, ruleId) {
  return (config.waivers ?? []).some(
    (waiver) => String(waiver.ruleId ?? "").toUpperCase() === ruleId,
  );
}

function parseUtcDate(value) {
  const text = String(value ?? "");
  if (!/^\d{4}-\d{2}-\d{2}$/u.test(text)) return null;
  const date = new Date(`${text}T00:00:00.000Z`);
  return Number.isNaN(date.getTime()) ? null : date;
}

function startOfUtcDay(date) {
  return new Date(Date.UTC(date.getUTCFullYear(), date.getUTCMonth(), date.getUTCDate()));
}

function daysBetweenUtc(start, end) {
  return Math.floor((end.getTime() - start.getTime()) / 86_400_000);
}

function isBroadWaiverScope(scope) {
  const normalized = String(scope ?? "").replaceAll("\\", "/").trim();
  return (
    normalized === "" ||
    normalized === "." ||
    normalized === "/" ||
    normalized === "**" ||
    normalized === "**/*" ||
    normalized === "src/**" ||
    normalized === "crates/**" ||
    normalized === "packages/**" ||
    normalized === "apps/**" ||
    /^\*\*\/\*\.[A-Za-z0-9]+$/u.test(normalized)
  );
}

function buildReport({ root, config, checkName, findings, scope = null }) {
  const fallback = { ...CHECK_RULES, ...GENERIC_RULES };
  const enriched = enrichFindingsMetadata(findings, PACK_ROOT, fallback);
  const policyFindings = applyRulePolicy(enriched, config, registryRules(PACK_ROOT));
  const { active, waived } = applyWaivers(
    policyFindings,
    config,
    registryRules(PACK_ROOT),
    { ci: process.env.CI === "true" },
  );
  const { violations, warnings, bySeverity } = splitFindings(active, config);
  return {
    ok: violations.length === 0,
    command: "check",
    check: checkName,
    root,
    profileName: config.profileName ?? "strict",
    violations,
    warnings,
    waived,
    findings: [...active, ...waived],
    bySeverity,
    scope: scope ? reportScope(root, scope, active) : undefined,
  };
}

function collectPolicyFiles(root, config, policy, scope = { mode: "all" }) {
  const extensions = new Set(policy.extensions ?? []);
  const predicate = (file, rel) =>
    extensions.has(path.extname(file).toLowerCase()) &&
    isUnderRoots(rel, policy.roots ?? []);
  if (isWorkspaceScope(scope))
    return collectFiles(root, policy.roots ?? [], config, predicate);
  return collectFiles(
    root,
    scopeEntries(root, scope, config),
    config,
    predicate,
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
  const stats = fs.statSync(start);
  if (stats.isDirectory())
    return fs
      .readdirSync(start)
      .some((entry) => hasFile(path.join(start, entry), predicate));
  return stats.isFile() && predicate(start);
}

function maxBraceNestingDepth(lines) {
  let depth = 0;
  let maxDepth = 0;
  for (const rawLine of lines) {
    const line = maskBraceSensitiveLine(rawLine);
    for (const char of line) {
      if (char === "{") {
        depth += 1;
        maxDepth = Math.max(maxDepth, depth);
      } else if (char === "}") {
        depth = Math.max(0, depth - 1);
      }
    }
  }
  return maxDepth;
}

function maskBraceSensitiveLine(line) {
  return String(line ?? "")
    .replace(/\/\/.*$/u, "")
    .replace(/'(?:[^'\\]|\\.)*'/gu, "''")
    .replace(/"(?:[^"\\]|\\.)*"/gu, '""')
    .replace(/`(?:[^`\\]|\\.)*`/gu, "``")
    .replace(/\/(?:[^\/\\\n]|\\.)+\/[dgimsuvy]*/gu, "//");
}

function maxPythonIndentDepth(lines) {
  let maxDepth = 0;
  for (const line of lines) {
    if (/^\s*$/u.test(line)) continue;
    const spaces = leadingWhitespace(line).replace(/\t/gu, "    ").length;
    maxDepth = Math.max(maxDepth, Math.floor(spaces / 4));
  }
  return maxDepth;
}

function countLines(text) {
  return text.length === 0 ? 0 : text.split(/\r?\n/u).length;
}

function findBlockEnd(lines, start) {
  let depth = 0;
  let seenBody = false;
  for (let index = start; index < lines.length; index += 1) {
    for (const char of lines[index]) {
      if (char === "{") {
        seenBody = true;
        depth += 1;
      } else if (char === "}") {
        depth -= 1;
      }
    }
    if (seenBody && depth <= 0) return index;
  }
  return start;
}

function findPythonBlockEnd(lines, start) {
  const startIndent = leadingWhitespace(lines[start]).length;
  let last = start;
  for (let index = start + 1; index < lines.length; index += 1) {
    const line = lines[index];
    if (/^\s*$/u.test(line)) {
      last = index;
      continue;
    }
    const trimmed = line.trimStart();
    if (trimmed.startsWith("#")) {
      last = index;
      continue;
    }
    if (leadingWhitespace(line).length <= startIndent) return last;
    last = index;
  }
  return last;
}

function leadingWhitespace(line) {
  return /^\s*/u.exec(line)?.[0] ?? "";
}

function isWorkspaceScope(scope = {}) {
  return scope.mode === "all" || scope.mode === "workspace";
}

function scopeEntries(root, scope, config = {}) {
  if (isWorkspaceScope(scope)) return [];
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
  if (isWorkspaceScope(scope)) return null;
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
  const files =
    isWorkspaceScope(scope)
      ? gitNameOnly(root, ["ls-files"])
      : scopeRelativeFiles(root, scope, {});
  if (isWorkspaceScope(scope)) return files;
  const tracked = new Set(gitNameOnly(root, ["ls-files"]));
  return files.filter((rel) => tracked.has(rel));
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
  return result.stdout
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
  if (!Array.isArray(roots) || roots.length === 0) return true;
  return roots.some(
    (root) => rel === root || rel.startsWith(`${root.replaceAll("\\", "/")}/`),
  );
}

function importSpecifier(line) {
  return (
    /(?:^\s*import(?:\s+type)?(?:[\s\w*{},]*\s+from\s*)?|^\s*export(?:\s+type)?\s+[\s\w*{},]*\s+from\s*)['"]([^'"]+)['"]/u.exec(
      line,
    )?.[1] ?? null
  );
}

function isGeneratedArtifactPath(rel) {
  const normalized = rel.replaceAll("\\\\", "/");
  if (/(?:^|\/)fixtures\//u.test(normalized)) return false;
  const directorySegments = normalized
    .split("/")
    .filter(Boolean)
    .slice(0, -1);
  return (
    /^(?:output|test-results|playwright-report)\//u.test(rel) ||
    /(?:^|\/)(?:dist|build|coverage)\//u.test(rel) ||
    directorySegments.some(
      (segment) =>
        segment === "target" ||
        segment.startsWith("target-") ||
        segment === ".tmp" ||
        segment.startsWith(".tmp-"),
    )
  );
}

function reportScope(root, scope, findings) {
  const files =
    scope.mode === "all"
      ? uniqueSorted(findings.map((entry) => entry.file))
      : scopeRelativeFiles(root, scope, {});
  return {
    mode: scope.mode === "all" ? "workspace" : scope.mode,
    files,
    crateName: scope.crateName ?? undefined,
    base: scope.base ?? undefined,
    head: scope.head ?? undefined,
  };
}

function spawnInRoot(root, command, args) {
  const resolved = resolveCommand(command);
  return spawnSync(resolved.command, [...resolved.args, ...args], {
    cwd: root,
    encoding: "utf8",
    shell: false,
    maxBuffer: 64 * 1024 * 1024,
  });
}

function resolveCommand(command) {
  if (process.platform !== "win32") return { command, args: [] };
  const nodeDir = path.dirname(process.execPath);
  const npmCli = path.join(nodeDir, "node_modules", "npm", "bin", "npm-cli.js");
  const npxCli = path.join(nodeDir, "node_modules", "npm", "bin", "npx-cli.js");
  if (command === "npm" && fs.existsSync(npmCli)) {
    return { command: process.execPath, args: [npmCli] };
  }
  if (command === "npx" && fs.existsSync(npxCli)) {
    return { command: process.execPath, args: [npxCli] };
  }
  const aliases = new Map([
    ["cargo", "cargo.exe"],
    ["git", "git.exe"],
    ["node", "node.exe"],
  ]);
  return { command: aliases.get(command) ?? command, args: [] };
}

function compactProcessOutput(result) {
  return [result.stdout, result.stderr]
    .filter(Boolean)
    .join("\n")
    .split(/\r?\n/u)
    .filter(Boolean)
    .slice(0, 20)
    .join("\n");
}

function isIgnored(file, config) {
  const rel = String(file ?? "").replaceAll("\\", "/");
  const ignoreDirs = config.ignoreDirs ?? [];
  return rel.split("/").some((part) => ignoreDirs.includes(part));
}

function finding(root, file, line, ruleId, detail, source) {
  return enrichFindingMetadata({
    ruleId,
    detail,
    file: normalizeRel(root, file),
    line,
    source: source == null ? null : String(source).trim(),
  }, PACK_ROOT, CHECK_RULES);
}

function genericFinding(root, file, line, ruleId, detail, source) {
  return enrichFindingMetadata({
    ruleId,
    detail,
    file: normalizeRel(root, file),
    line,
    source: source == null ? null : String(source).trim(),
  }, PACK_ROOT, { ...GENERIC_RULES, ...CHECK_RULES });
}



export { collectSourceShapeFindings, applySourceShapeOverrides, inspectTypeScriptShape, inspectPythonShape, inspectRustShape, collectRequiredTestFindings, collectInlineSourceTestFindings, isInlineTestSourceCandidate, inlineTestPatternForFile, collectStrictEmptyTestTreeFindings, collectEmptyPlaceholderTrees, collectSingleSourceContractFindings, collectGeneratedArtifactFindings, collectNoZodSourceFindings, collectNoNakedDomainStringsFindings, collectWeakAssertionsFindings, collectSkippedFocusedTestFindings, collectValidationBypassFindings, collectPlaceholderImplementationFindings, collectReexportFindings, collectSecretFindings, collectImportBoundaryFindings, resolvePackRoot, loadRegistryRules, collectRegistryRuleMetadataFindings, collectRegistryDocFindings, markdownAnchors, markdownAnchor, collectFixtureEvidence, ensureFixtureEvidenceEntry, ruleIdFromFixturePath, collectRoutedDocRuleIds, collectScannerRuleIds, collectSourceFiles, existingConfigPath, readRawConfigObject, hasOverrideWaiverMetadata, hasWaiverFor, parseUtcDate, startOfUtcDay, daysBetweenUtc, isBroadWaiverScope, buildReport, collectPolicyFiles, childDirs, hasFile, countMatches, maxBraceNestingDepth, maxPythonIndentDepth, countLines, findBlockEnd, findPythonBlockEnd, leadingWhitespace, valueAtPath, valueFromSpec, loadContract, collectContractScanFiles, enforceRequiredMirrorCoverage, collectCoveredContractPaths, valueAtSourceObjectPath, valueAtRustConst, valueAtRustSerdeRename, createLiteralMatchPattern, escapeRegExp, sourceContractExtension, isNonBlockingContractPath, scopeEntries, scopeFilesByExtensions, scopeRelativeFiles, scopedProjectRoots, trackedScopeFiles, stagedFiles, diffFiles, gitNameOnly, crateRootForName, isUnderRoots, importSpecifier, isGeneratedArtifactPath, reportScope, resolveContractConfigPath, spawnInRoot, resolveCommand, compactProcessOutput, isIgnored, finding, genericFinding, normalizeRel, lineNumberAt };
