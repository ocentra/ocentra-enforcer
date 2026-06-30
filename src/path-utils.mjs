import fs from 'node:fs';
import path from 'node:path';

export const DEFAULT_IGNORE_DIRS = [
  '.git',
  '.hub',
  '.turbo',
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

export function uniqueSorted(values) {
  return [...new Set(values)].sort((a, b) => a.localeCompare(b));
}

export function toPosix(value) {
  return String(value).split(path.sep).join('/');
}

export function normalizeRel(root, filePath) {
  return toPosix(path.relative(root, path.resolve(filePath)));
}

export function repoAbsolute(root, value) {
  return path.isAbsolute(value) ? path.resolve(value) : path.resolve(path.join(root, value));
}

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

export function matchesGlob(relPath, glob) {
  if (!globCache.has(glob)) globCache.set(glob, globToRegExp(glob));
  return globCache.get(glob).test(relPath);
}

export function matchesAnyGlob(relPath, globs = []) {
  return globs.some((glob) => matchesGlob(relPath, glob));
}

export function isIgnoredPath(relPath, config = {}) {
  const ignoreDirs = config.ignoreDirs ?? DEFAULT_IGNORE_DIRS;
  const ignoreFileGlobs = config.ignoreFileGlobs ?? [];
  return relPath.split('/').some((segment) => ignoreDirs.includes(segment)) || matchesAnyGlob(relPath, ignoreFileGlobs);
}

export function walkFiles(root, start, config, collect) {
  if (!fs.existsSync(start)) return;
  const stats = fs.statSync(start);
  const rel = normalizeRel(root, start);
  if (rel !== '' && isIgnoredPath(rel, config)) return;
  if (stats.isDirectory()) {
    for (const entry of fs.readdirSync(start, { withFileTypes: true })) {
      walkFiles(root, path.join(start, entry.name), config, collect);
    }
    return;
  }
  if (stats.isFile()) collect(path.resolve(start));
}

export function collectFiles(root, entries, config, predicate) {
  const starts = entries.length > 0 ? entries.map((entry) => repoAbsolute(root, entry)) : [root];
  const files = [];
  for (const start of starts) {
    walkFiles(root, start, config, (file) => {
      if (predicate(file, normalizeRel(root, file))) files.push(file);
    });
  }
  return uniqueSorted(files);
}

export function lineNumberAt(text, index) {
  let line = 1;
  for (let i = 0; i < index; i += 1) {
    if (text.charCodeAt(i) === 10) line += 1;
  }
  return line;
}

