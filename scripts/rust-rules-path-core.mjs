#!/usr/bin/env node
/*
 * Ocentra Enforcer path and scope helpers.
 */
import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { enrichFindingMetadata } from "../src/rule-registry.mjs";
import { RULES } from "../src/rule-metadata.mjs";

// boundaryOwnerNote: Enforcer-owned path helpers; edits require policy-integrity and self-scan validation.
const SCRIPT_PATH = fileURLToPath(import.meta.url);
const PACK_ROOT = path.resolve(path.join(path.dirname(SCRIPT_PATH), ".."));

function normalizeRel(root, filePath) {
  return path.relative(root, path.resolve(filePath)).split(path.sep).join("/");
}

function toPosix(value) {
  return value.split(path.sep).join("/");
}

function repoAbsolute(root, value) {
  return path.isAbsolute(value) ? value : path.join(root, value);
}

function globToRegExp(glob) {
  const special = /[.+^${}()|[\]\\]/g;
  let pattern = "";
  for (let i = 0; i < glob.length; i += 1) {
    const char = glob[i];
    if (char === "*") {
      if (glob[i + 1] === "*") {
        pattern += ".*";
        i += 1;
      } else {
        pattern += "[^/]*";
      }
    } else if (char === "?") {
      pattern += "[^/]";
    } else {
      pattern += char.replace(special, "\\$&");
    }
  }
  return new RegExp(`^${pattern}$`, "u");
}

const globCache = new Map();

function matchesGlob(relPath, glob) {
  if (!globCache.has(glob)) {
    globCache.set(glob, globToRegExp(glob));
  }
  return globCache.get(glob).test(relPath);
}

function matchesAnyGlob(relPath, globs) {
  return globs.some((glob) => matchesGlob(relPath, glob));
}

function isIgnoredPath(relPath, config) {
  return (
    relPath.split("/").some((segment) => config.ignoreDirs.includes(segment)) ||
    matchesAnyGlob(relPath, config.ignoreFileGlobs)
  );
}

function isRustFile(filePath) {
  return path.extname(filePath).toLowerCase() === ".rs";
}

function lineNumberAt(text, index) {
  let line = 1;
  for (let i = 0; i < index; i += 1) {
    if (text.charCodeAt(i) === 10) line += 1;
  }
  return line;
}

function runGit(root, args, label) {
  const result = spawnSync("git", args, {
    cwd: root,
    encoding: "utf8",
    shell: false,
  });
  if (result.error) throw result.error;
  if ((result.status ?? 1) !== 0) {
    throw new Error(
      `${label}: ${result.stderr?.trim() || "git command failed"}`,
    );
  }
  return result.stdout.trim();
}

function walkFiles(root, start, config, collect) {
  if (!fs.existsSync(start)) return;
  const stats = fs.statSync(start);
  const rel = normalizeRel(root, start);
  if (isIgnoredPath(rel, config)) return;
  if (stats.isDirectory()) {
    for (const entry of fs.readdirSync(start, { withFileTypes: true })) {
      walkFiles(root, path.join(start, entry.name), config, collect);
    }
    return;
  }
  if (stats.isFile()) collect(start);
}

function collectAllRustFiles(root, config) {
  const starts = config.rustRoots
    .map((entry) => repoAbsolute(root, entry))
    .filter((entry) => fs.existsSync(entry));
  if (starts.length === 0) starts.push(root);
  const files = [];
  for (const start of starts) {
    walkFiles(root, start, config, (file) => {
      if (isRustFile(file)) files.push(path.resolve(file));
    });
  }
  return uniqueSorted(files);
}

function collectExplicitRustFiles(root, config, entries) {
  const files = [];
  for (const entry of entries) {
    walkFiles(root, repoAbsolute(root, entry), config, (file) => {
      if (isRustFile(file)) files.push(path.resolve(file));
    });
  }
  return uniqueSorted(files);
}

function collectDiffRustFiles(root, config, base, head) {
  const output = runGit(
    root,
    [
      "diff",
      "--name-only",
      "--diff-filter=ACMR",
      base,
      head,
      "--",
      ...config.rustRoots,
    ],
    "failed to list diff files",
  );
  if (output === "") return [];
  return uniqueSorted(
    output
      .split(/\r?\n/u)
      .map((entry) => repoAbsolute(root, entry))
      .filter(
        (entry) =>
          fs.existsSync(entry) &&
          isRustFile(entry) &&
          !isIgnoredPath(normalizeRel(root, entry), config),
      ),
  );
}

function findCargoManifests(root, config) {
  const manifests = [];
  walkFiles(root, root, config, (file) => {
    if (path.basename(file) === "Cargo.toml")
      manifests.push(path.resolve(file));
  });
  return uniqueSorted(manifests);
}

function packageNameFromManifest(manifestPath) {
  const text = fs.readFileSync(manifestPath, "utf8");
  const packageBlock = text.match(
    /(?:^|\n)\s*\[package\]([\s\S]*?)(?:\n\s*\[|$)/u,
  );
  if (!packageBlock) return null;
  const nameMatch = packageBlock[1].match(/(?:^|\n)\s*name\s*=\s*"([^"]+)"/u);
  return nameMatch?.[1] ?? null;
}

function collectCrateRustFiles(root, config, crateName) {
  if (!crateName) throw new Error("--crate requires a package name.");
  const manifest = findCargoManifests(root, config).find(
    (candidate) => packageNameFromManifest(candidate) === crateName,
  );
  if (!manifest)
    throw new Error(
      `No Cargo package named "${crateName}" was found under ${root}.`,
    );
  const crateRoot = path.dirname(manifest);
  return {
    crateName,
    crateRoot,
    manifest,
    files: collectExplicitRustFiles(root, config, [crateRoot]),
  };
}

function resolveScope(root, config, scope) {
  if (scope.mode === "files") {
    return {
      ...scope,
      files: collectExplicitRustFiles(root, config, scope.files),
    };
  }
  if (scope.mode === "diff") {
    return {
      ...scope,
      files: collectDiffRustFiles(root, config, scope.base, scope.head),
    };
  }
  if (scope.mode === "crate") {
    return {
      ...scope,
      ...collectCrateRustFiles(root, config, scope.crateName),
    };
  }
  return { mode: "all", files: collectAllRustFiles(root, config) };
}

function uniqueSorted(files) {
  return [...new Set(files)].sort((a, b) => a.localeCompare(b));
}

function maskRustCode(source) {
  let out = "";
  let state = "code";
  let blockDepth = 0;
  let rawHashes = "";
  const pushMask = (ch) => {
    out += ch === "\n" ? "\n" : " ";
  };

  for (let i = 0; i < source.length; i += 1) {
    const ch = source[i];
    const next = source[i + 1];

    if (state === "code") {
      const rawMatch = source.slice(i).match(/^(?:b|c|br)?r(#+)?"/u);
      if (rawMatch) {
        state = "rawString";
        rawHashes = rawMatch[1] ?? "";
        for (let j = 0; j < rawMatch[0].length; j += 1) pushMask(source[i + j]);
        i += rawMatch[0].length - 1;
        continue;
      }
      if (ch === "/" && next === "/") {
        state = "lineComment";
        pushMask(ch);
        continue;
      }
      if (ch === "/" && next === "*") {
        state = "blockComment";
        blockDepth = 1;
        pushMask(ch);
        continue;
      }
      if (ch === '"' || (ch === "b" && next === '"')) {
        state = "string";
        pushMask(ch);
        continue;
      }
      if (ch === "'") {
        if (/^'[A-Za-z_][A-Za-z0-9_]*\b/u.test(source.slice(i))) {
          out += ch;
          continue;
        }
        state = "char";
        pushMask(ch);
        continue;
      }
      out += ch;
      continue;
    }

    if (state === "lineComment") {
      pushMask(ch);
      if (ch === "\n") state = "code";
      continue;
    }

    if (state === "blockComment") {
      pushMask(ch);
      if (ch === "/" && next === "*") {
        blockDepth += 1;
        pushMask(next);
        i += 1;
      } else if (ch === "*" && next === "/") {
        blockDepth -= 1;
        pushMask(next);
        i += 1;
        if (blockDepth === 0) state = "code";
      }
      continue;
    }

    if (state === "string" || state === "char") {
      pushMask(ch);
      if (ch === "\\") {
        if (i + 1 < source.length) {
          pushMask(source[i + 1]);
          i += 1;
        }
      } else if (
        (state === "string" && ch === '"') ||
        (state === "char" && ch === "'")
      ) {
        state = "code";
      }
      continue;
    }

    if (state === "rawString") {
      pushMask(ch);
      if (
        ch === '"' &&
        source.slice(i + 1, i + 1 + rawHashes.length) === rawHashes
      ) {
        for (let j = 0; j < rawHashes.length; j += 1)
          pushMask(source[i + 1 + j]);
        i += rawHashes.length;
        state = "code";
      }
    }
  }

  return out;
}

function contextHas(lines, index, token, distance = 4) {
  const start = Math.max(0, index - distance);
  return lines
    .slice(start, index + 1)
    .join("\n")
    .includes(token);
}

function firstLineMatching(lines, pattern) {
  const index = lines.findIndex((line) => pattern.test(line));
  return index < 0 ? 1 : index + 1;
}

function escapeRegExp(value) {
  return String(value).replace(/[.*+?^${}()|[\]\\]/gu, "\\$&");
}

function lineNumberAtIndex(source, index) {
  return source.slice(0, index).split(/\r?\n/u).length;
}

function addViolation(
  violations,
  root,
  filePath,
  line,
  ruleId,
  detail,
  sourceLine = null,
) {
  violations.push({
    ...enrichFindingMetadata({
      ruleId,
      detail,
      file: filePath === "." ? "." : normalizeRel(root, filePath),
      line,
      source: sourceLine?.trim() ?? null,
    }, PACK_ROOT, RULES),
  });
}

export {
  PACK_ROOT,
  RULES,
  normalizeRel,
  toPosix,
  repoAbsolute,
  globToRegExp,
  matchesGlob,
  matchesAnyGlob,
  isIgnoredPath,
  isRustFile,
  lineNumberAt,
  runGit,
  walkFiles,
  collectAllRustFiles,
  collectExplicitRustFiles,
  collectDiffRustFiles,
  findCargoManifests,
  packageNameFromManifest,
  collectCrateRustFiles,
  resolveScope,
  uniqueSorted,
  maskRustCode,
  contextHas,
  firstLineMatching,
  escapeRegExp,
  lineNumberAtIndex,
  addViolation,
};
