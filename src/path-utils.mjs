import fs from 'node:fs';
import path from 'node:path';

// PUBLIC-API-BUDGET-JUSTIFICATION: path utilities are shared by scanners, harness, MCP, proof, and coordination adapters.
/** Lists directory names excluded from ordinary repository scans. */
export const DEFAULT_IGNORE_DIRS = [
  '.git',
  '.hub',
  '.turbo',
  '.wrangler',
  '.enforce',
  '.tmp',
  '.venv',
  '.ruff_cache',
  '.mypy_cache',
  '.pytest_cache',
  'target',
  'node_modules',
  '.next',
  'dist',
  'build',
  'coverage',
  'output',
  'vendor',
  'External',
];

const globCache = new Map();

/** Returns values de-duplicated case-insensitively and sorted for stable output. */
export function uniqueSorted(values) {
  return [...new Map(values.map((value) => [String(value).toLowerCase(), value])).values()]
    .sort((a, b) => a.localeCompare(b));
}

/** Converts a filesystem path into POSIX separator form. */
export function toPosix(value) {
  return String(value).split(path.sep).join('/');
}

/** Resolves a file path as a POSIX relative path from a repository root. */
export function normalizeRel(root, filePath) {
  return toPosix(path.relative(root, path.resolve(filePath)));
}

/** Resolves a repository-relative or absolute path to an absolute path. */
export function repoAbsolute(root, value) {
  return path.isAbsolute(value) ? path.resolve(value) : path.resolve(path.join(root, value));
}

/** Converts the supported path-glob syntax into a regular expression. */
export function globToRegExp(glob) {
  const special = /[.+^${}()|[\]\\]/g;
  let pattern = '';
  for (let i = 0; i < glob.length; i += 1) {
    const char = glob[i];
    if (char === '*') {
      if (glob[i + 1] === '*') {
        pattern += '.*';
        i += 1;
      } else {
        pattern += '[^/]*';
      }
    } else if (char === '?') {
      pattern += '[^/]';
    } else {
      pattern += char.replace(special, '\\$&');
    }
  }
  return new RegExp(`^${pattern}$`, 'u');
}

/** Determines whether a relative path matches a cached path glob. */
export function matchesGlob(relPath, glob) {
  if (!globCache.has(glob)) globCache.set(glob, globToRegExp(glob));
  return globCache.get(glob).test(relPath);
}

/** Determines whether a relative path matches any supplied path glob. */
export function matchesAnyGlob(relPath, globs = []) {
  return globs.some((glob) => matchesGlob(relPath, glob));
}

function hasGeneratedDirectorySegment(relPath, prefix, finalSegmentIsDirectory) {
  const segments = relPath.split('/').filter(Boolean);
  const directorySegments = finalSegmentIsDirectory
    ? segments
    : segments.slice(0, -1);
  return directorySegments.some((segment) => segment.startsWith(prefix));
}

/** Determines whether a path is excluded by directory or file-glob policy. */
export function isIgnoredPath(relPath, config = {}, finalSegmentIsDirectory = false) {
  const ignoreDirs = config.ignoreDirs ?? DEFAULT_IGNORE_DIRS;
  const ignoreFileGlobs = config.ignoreFileGlobs ?? [];
  return (
    relPath.split('/').some((segment) => ignoreDirs.includes(segment)) ||
    (ignoreDirs.includes('target') &&
      hasGeneratedDirectorySegment(relPath, 'target-', finalSegmentIsDirectory)) ||
    (ignoreDirs.includes('.tmp') &&
      hasGeneratedDirectorySegment(relPath, '.tmp-', finalSegmentIsDirectory)) ||
    matchesAnyGlob(relPath, ignoreFileGlobs)
  );
}

/** Walks eligible files below a start path and invokes a collector for each. */
export function walkFiles(root, start, config, collect, forcedPrefix = null) {
  if (!fs.existsSync(start)) return;
  const stats = fs.lstatSync(start);
  if (stats.isSymbolicLink()) return;
  const rel = normalizeRel(root, start);
  const scopedRel =
    forcedPrefix != null &&
    (rel === forcedPrefix || rel.startsWith(`${forcedPrefix}/`))
      ? rel.slice(forcedPrefix.length).replace(/^\//u, '')
      : rel;
  if (
    scopedRel !== '' &&
    isIgnoredPath(scopedRel, config, stats.isDirectory())
  ) return;
  if (stats.isDirectory()) {
    for (const entry of fs.readdirSync(start, { withFileTypes: true })) {
      walkFiles(root, path.join(start, entry.name), config, collect, forcedPrefix);
    }
    return;
  }
  if (stats.isFile()) collect(path.resolve(start));
}

/** Collects eligible repository files that satisfy a caller predicate. */
export function collectFiles(root, entries, config, predicate, forceEntries = true) {
  const starts = entries.length > 0 ? entries.map((entry) => repoAbsolute(root, entry)) : [root];
  const files = [];
  for (const start of starts) {
    const forcedPrefix =
      forceEntries && entries.length > 0 ? normalizeRel(root, start) : null;
    walkFiles(root, start, config, (file) => {
      if (predicate(file, normalizeRel(root, file))) files.push(file);
    }, forcedPrefix);
  }
  return uniqueSorted(files);
}

/** Calculates the one-based line number containing a character index. */
export function lineNumberAt(text, index) {
  let line = 1;
  for (let i = 0; i < index; i += 1) {
    if (text.charCodeAt(i) === 10) line += 1;
  }
  return line;
}
