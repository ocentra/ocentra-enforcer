import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { collectFiles, normalizeRel, repoAbsolute, uniqueSorted } from './path-utils.mjs';
import {
  SOURCE_POLICY_RULES,
  scanAdditionalCommonFile,
  scanAdditionalTypeScriptFile,
} from './source-policy-scanners.mjs';

export const GENERIC_RULES = {
  'TS-1.1': {
    title: 'TypeScript/JavaScript re-exports are forbidden',
    snippet: 'Import from the owning module at the call site; do not add barrel exports or re-export shims.',
  },
  'TS-2.1': {
    title: 'TypeScript/JavaScript suppression comments are forbidden',
    snippet: 'Fix the type/lint issue or add a project policy exception rather than suppressing validation.',
  },
  'TS-3.1': {
    title: 'Skipped/focused JavaScript tests are forbidden',
    snippet: 'Remove .skip/.only/.todo and keep every checked-in test executable.',
  },
  'TS-5.1': {
    title: 'TypeScript compiler checks must pass',
    snippet: 'Run tsc --noEmit through the Enforcer harness and fix compiler diagnostics.',
  },
  'TS-5.2': {
    title: 'ESLint JSON diagnostics must pass',
    snippet: 'Run ESLint with --format json through the Enforcer harness and fix lint diagnostics.',
  },
  'PY-1.1': {
    title: 'Python lint suppression comments are forbidden',
    snippet: 'Fix the Ruff/Pylint issue or move the exception into reviewed policy.',
  },
  'PY-1.2': {
    title: 'Python type-ignore comments are forbidden',
    snippet: 'Fix the type issue or model the dynamic boundary explicitly.',
  },
  'PY-2.1': {
    title: 'Skipped/focused Python tests are forbidden',
    snippet: 'Remove skip/focus markers or move the exception into reviewed test policy.',
  },
  'PY-3.1': {
    title: 'Ruff diagnostics must pass',
    snippet: 'Run Ruff with JSON output through the Enforcer harness and fix diagnostics.',
  },
  'PY-3.2': {
    title: 'Python type-check diagnostics must pass',
    snippet: 'Run Pyright or mypy through the Enforcer harness and fix diagnostics.',
  },
  'SEC-1.1': {
    title: 'Inline secrets are forbidden',
    snippet: 'Move credentials into a secret manager or local ignored environment file.',
  },
  'GEN-1.1': {
    title: 'Generated artifacts must not be committed as source',
    snippet: 'Generate artifacts in CI/build output, not in tracked source folders.',
  },
  'TEST-1.2': {
    title: 'Weak assertions are forbidden',
    snippet: 'Assert concrete behavior and values instead of existence, truthiness, or broad matcher placeholders.',
  },
  'TEST-1.3': {
    title: 'Hidden, focused, or ignored tests are forbidden',
    snippet: 'Remove focused, skipped, todo, ignored, or fixme markers before claiming test coverage.',
  },
  'SRC-1.2': {
    title: 'Placeholder implementation markers are forbidden',
    snippet: 'Replace TODO, placeholder, not-implemented, debug-print, and temporary code with real behavior before landing.',
  },
  'DOC-1.1': {
    title: 'Public API documentation is recommended',
    snippet: 'Add a short rustdoc/JSDoc/docstring for exported or public API, or disable/downgrade this advisory in project policy.',
  },
  'HAR-1.1': {
    title: 'Harnessed command failed',
    snippet: 'Read compact diagnostics first, then inspect bounded raw artifacts only if needed.',
  },
  ...SOURCE_POLICY_RULES,
};

const TS_EXTENSIONS = new Set(['.ts', '.tsx', '.js', '.jsx', '.mjs', '.cjs', '.mts', '.cts']);
const PY_EXTENSIONS = new Set(['.py']);
const SECRET_RE = /\b(?:api[_-]?key|secret|token|password|private[_-]?key)\b\s*[:=]\s*["'][A-Za-z0-9_./+=:@-]{16,}["']/iu;
const weakAssertionPatterns = [
  { pattern: /\.toBeDefined\s*\(/u, detail: 'toBeDefined() is too weak.' },
  { pattern: /\.toBeTruthy\s*\(/u, detail: 'toBeTruthy() is too weak.' },
  { pattern: /\.toBeFalsy\s*\(/u, detail: 'toBeFalsy() is too weak.' },
  { pattern: /\.not\.toThrow\s*\(/u, detail: 'not.toThrow() is too weak.' },
  { pattern: /\.toMatchObject\s*\(\s*\{\s*\}\s*\)/u, detail: 'empty toMatchObject({}) is too weak.' },
  { pattern: /expect\.anything\s*\(\s*\)/u, detail: 'expect.anything() is too weak.' },
  { pattern: /expect\.any\s*\(\s*(?:String|Number)\s*\)/u, detail: 'expect.any(String|Number) is too weak.' },
  { pattern: /assert!\(\s*[\w.]+\s*\.is_some\(\)\s*\)/u, detail: 'assert!(value.is_some()) is too weak.' },
  { pattern: /assert!\(\s*[\w.]+\s*\.is_ok\(\)\s*\)/u, detail: 'assert!(result.is_ok()) is too weak.' },
  { pattern: /assert!\(\s*[\w.]+\s*\.is_err\(\)\s*\)/u, detail: 'assert!(result.is_err()) is too weak.' },
  { pattern: /assert!\(\s*[^)]+\.len\(\)\s*>\s*0\s*\)/u, detail: 'length > 0 assertion is too weak.' },
  { pattern: /assert!\(\s*!\s*[^)]+\.is_empty\(\)\s*\)/u, detail: '!is_empty() assertion is too weak.' },
  { pattern: /assert!\(\s*[^)]+\.contains\([^)]+\)\s*\)/u, detail: 'contains() assertion is too weak without checking meaning.' },
];
const word = (...parts) => parts.join('');
const placeholderCommentPatterns = [
  { pattern: /\bTODO\b/u, detail: 'TODO marker found in production source.' },
  { pattern: /\bFIXME\b/u, detail: 'FIXME marker found in production source.' },
  { pattern: /\bTBD\b/u, detail: 'TBD marker found in production source.' },
  { pattern: new RegExp(String.raw`\b${word('place', 'holder')}\b`, 'iu'), detail: 'placeholder marker found in production source.' },
  { pattern: new RegExp(String.raw`\b${word('st', 'ub')}\b`, 'iu'), detail: 'stub marker found in production source.' },
  { pattern: new RegExp(String.raw`\b${word('fa', 'ke')}\b`, 'iu'), detail: 'fake marker found in production source.' },
  { pattern: /\btemporary\b/iu, detail: 'temporary marker found in production source.' },
  { pattern: /\bfor now\b/iu, detail: 'for now marker found in production source.' },
  { pattern: /\bscaffold[- ]only\b/iu, detail: 'scaffold-only marker found in production source.' },
];
const placeholderDirectPatterns = [
  { pattern: /throw new Error\(\s*['"`]not implemented['"`]\s*\)/iu, detail: 'not implemented throw found.' },
  { pattern: /raise\s+NotImplementedError\b/u, detail: 'NotImplementedError found.' },
  { pattern: /return\s+null\s+as\s+any/u, detail: 'return null as any found.' },
  { pattern: /return\s+\{\s*\}\s+as\s+any/u, detail: 'return {} as any found.' },
  { pattern: /\btodo!\s*\(\s*\)/u, detail: 'todo!() found.' },
  { pattern: /\bunimplemented!\s*\(\s*\)/u, detail: 'unimplemented!() found.' },
  { pattern: /panic!\(\s*['"`]not implemented['"`]\s*\)/iu, detail: 'not implemented panic found.' },
  { pattern: /\bdbg!\s*\(/u, detail: 'dbg!() found.' },
  { pattern: /\bprintln!\s*\(/u, detail: 'println!() found.' },
  { pattern: /\beprintln!\s*\(/u, detail: 'eprintln!() found.' },
  { pattern: /\bunreachable!\s*\(\s*\)/u, detail: 'bare unreachable!() found.' },
];

export function runGenericScan({ root, scope, config, languages = [] }) {
  const activeLanguages = new Set(languages);
  const files = collectGenericScopeFiles(root, scope, config, activeLanguages);
  const violations = [];
  for (const filePath of files) {
    const ext = path.extname(filePath).toLowerCase();
    if (activeLanguages.has('typescript') && TS_EXTENSIONS.has(ext)) {
      violations.push(...scanTypeScriptFile(root, filePath));
    }
    if (activeLanguages.has('python') && PY_EXTENSIONS.has(ext)) {
      violations.push(...scanPythonFile(root, filePath));
    }
    if (activeLanguages.has('common')) {
      violations.push(...scanCommonFile(root, filePath));
    }
    if (config.failFast && violations.length > 0) break;
  }
  return {
    files: files.map((file) => normalizeRel(root, file)),
    violations,
  };
}

export function collectGenericScopeFiles(root, scope, config, activeLanguages) {
  const entries = scope.mode === 'files' ? scope.files ?? [] : scope.mode === 'crate' && scope.crateRoot ? [scope.crateRoot] : [];
  if (scope.mode === 'diff') {
    const output = runGitDiff(root, scope.base, scope.head);
    if (output === '') return [];
    return uniqueSorted(
      output
        .split(/\r?\n/u)
        .map((entry) => repoAbsolute(root, entry))
        .filter((file) => fs.existsSync(file) && isGenericFile(file, activeLanguages))
    );
  }
  return collectFiles(root, entries, config, (file) => isGenericFile(file, activeLanguages));
}

export function scanTypeScriptFile(root, filePath) {
  const rel = normalizeRel(root, filePath);
  const lines = fs.readFileSync(filePath, 'utf8').split(/\r?\n/u);
  const violations = [];
  lines.forEach((line, idx) => {
    const lineNo = idx + 1;
    const comment = jsStyleCommentText(line);
    if (/^\s*export\s+(?:\*\s+from|\*\s+as\s+[A-Za-z_$][\w$]*\s+from|(?:type\s+)?\{[^}]*\}\s+from)/u.test(line)) {
      addViolation(violations, root, filePath, lineNo, 'TS-1.1', 'Barrel-style re-export found.', line);
    }
    if (/(?:\b(?:eslint-disable|biome-ignore|oxlint-disable|prettier-ignore)\b|@ts-(?:ignore|expect-error|nocheck)\b)/u.test(comment)) {
      addViolation(violations, root, filePath, lineNo, 'TS-2.1', 'TypeScript/JavaScript validation suppression found.', line);
    }
    if (isTestPath(rel) && /\b(?:describe|it|test)\s*\.\s*(?:skip|only|todo)\s*\(/u.test(line)) {
      addViolation(violations, root, filePath, lineNo, 'TS-3.1', 'Skipped or focused test found.', line);
    }
    if (isTestPath(rel) && /\btest\s*\.\s*(?:fixme|skip|only)\s*\(/u.test(line)) {
      addViolation(violations, root, filePath, lineNo, 'TS-3.1', 'Playwright skipped or focused test found.', line);
    }
    if (isTestPath(rel) && /\bexpect\s*\(\s*(?:true|false|null|undefined)\s*\)\s*\.\s*(?:toBe|toEqual)\s*\(/u.test(line)) {
      addViolation(violations, root, filePath, lineNo, 'TEST-1.2', 'literal truth assertion is too weak.', line);
    }
  });
  violations.push(...scanAdditionalTypeScriptFile(root, filePath));
  return violations;
}

export function scanPythonFile(root, filePath) {
  const rel = normalizeRel(root, filePath);
  const lines = fs.readFileSync(filePath, 'utf8').split(/\r?\n/u);
  const violations = [];
  lines.forEach((line, idx) => {
    const lineNo = idx + 1;
    const comment = hashCommentText(line);
    if (/#\s*noqa\b|\bpylint:\s*disable\b/iu.test(comment)) {
      addViolation(violations, root, filePath, lineNo, 'PY-1.1', 'Python lint suppression found.', line);
    }
    if (/#\s*type:\s*ignore\b/iu.test(comment)) {
      addViolation(violations, root, filePath, lineNo, 'PY-1.2', 'Python type-ignore suppression found.', line);
    }
    if (isPythonTestPath(rel) && /@pytest\.mark\.(?:skip|skipif|xfail|focus)|pytest\.skip\s*\(|unittest\.skip/u.test(line)) {
      addViolation(violations, root, filePath, lineNo, 'PY-2.1', 'Skipped or focused Python test found.', line);
    }
  });
  return violations;
}

export function scanCommonFile(root, filePath) {
  const rel = normalizeRel(root, filePath);
  const lines = fs.readFileSync(filePath, 'utf8').split(/\r?\n/u);
  const ext = path.extname(filePath).toLowerCase();
  const violations = [];
  lines.forEach((line, idx) => {
    const lineNo = idx + 1;
    const comment = sourceCommentText(ext, line);
    if (SECRET_RE.test(line)) {
      addViolation(violations, root, filePath, lineNo, 'SEC-1.1', 'Inline secret-like assignment found.', redact(line));
    }
    if (/@generated|<auto-generated>|Generated by/iu.test(comment) && !/\.d\.ts$/iu.test(rel)) {
      addViolation(violations, root, filePath, lineNo, 'GEN-1.1', 'Generated artifact marker found in tracked source scope.', line);
    }
    if (isTestPath(rel) || isPythonTestPath(rel)) {
      for (const rule of weakAssertionPatterns) {
        if (rule.pattern.test(line)) {
          addViolation(violations, root, filePath, lineNo, 'TEST-1.2', rule.detail, line);
        }
      }
    }
    if (ext === '.rs' && isTestPath(rel) && /#\s*\[\s*ignore\s*\]/u.test(line)) {
      addViolation(violations, root, filePath, lineNo, 'TEST-1.3', 'Rust #[ignore] test found.', line);
    }
    if (isProductionSourcePath(rel, ext)) {
      for (const rule of placeholderDirectPatterns) {
        if (rule.pattern.test(line)) {
          addViolation(violations, root, filePath, lineNo, 'SRC-1.2', rule.detail, line);
        }
      }
      if (comment !== '') {
        for (const rule of placeholderCommentPatterns) {
          if (rule.pattern.test(comment)) {
            addViolation(violations, root, filePath, lineNo, 'SRC-1.2', rule.detail, line);
          }
        }
      }
    }
  });
  violations.push(...scanAdditionalCommonFile(root, filePath, lines));
  violations.push(...scanDocumentationHints(root, filePath, rel, lines));
  return violations;
}

function scanDocumentationHints(root, filePath, rel, lines) {
  if (isTestPath(rel) || isPythonTestPath(rel) || /\.d\.ts$/iu.test(rel)) return [];
  const ext = path.extname(filePath).toLowerCase();
  if (TS_EXTENSIONS.has(ext)) return scanTypeScriptDocumentationHints(root, filePath, lines);
  if (PY_EXTENSIONS.has(ext)) return scanPythonDocumentationHints(root, filePath, lines);
  if (ext === '.rs') return scanRustDocumentationHints(root, filePath, lines);
  return [];
}

function scanTypeScriptDocumentationHints(root, filePath, lines) {
  const violations = [];
  lines.forEach((line, idx) => {
    if (!/^\s*export\s+(?:async\s+)?(?:function|class|interface|type|const|let|var|enum)\s+[A-Za-z_$][\w$]*/u.test(line)) return;
    if (hasLeadingDocComment(lines, idx, '/**')) return;
    addViolation(violations, root, filePath, idx + 1, 'DOC-1.1', 'Exported TypeScript/JavaScript API has no leading JSDoc comment.', line);
  });
  return violations;
}

function scanPythonDocumentationHints(root, filePath, lines) {
  const violations = [];
  lines.forEach((line, idx) => {
    if (!/^(?:async\s+)?def\s+[A-Za-z_]\w*\s*\(|^class\s+[A-Za-z_]\w*/u.test(line)) return;
    if (hasPythonDocstringAfter(lines, idx)) return;
    addViolation(violations, root, filePath, idx + 1, 'DOC-1.1', 'Top-level Python function/class has no docstring.', line);
  });
  return violations;
}

function scanRustDocumentationHints(root, filePath, lines) {
  const violations = [];
  lines.forEach((line, idx) => {
    if (!/^\s*pub(?:\([^)]*\)|\s+)?\s*(?:async\s+)?(?:fn|struct|enum|trait)\s+[A-Za-z_]\w*/u.test(line)) return;
    if (hasLeadingDocComment(lines, idx, '///') || hasLeadingDocComment(lines, idx, '#[doc')) return;
    addViolation(violations, root, filePath, idx + 1, 'DOC-1.1', 'Public Rust API has no leading rustdoc comment.', line);
  });
  return violations;
}

function hasLeadingDocComment(lines, index, marker) {
  for (let cursor = index - 1; cursor >= 0; cursor -= 1) {
    const line = lines[cursor]?.trim() ?? '';
    if (line === '') continue;
    return line.startsWith(marker) || line.endsWith('*/');
  }
  return false;
}

function hasPythonDocstringAfter(lines, index) {
  for (let cursor = index + 1; cursor < lines.length; cursor += 1) {
    const line = lines[cursor]?.trim() ?? '';
    if (line === '') continue;
    return line.startsWith('"""') || line.startsWith("'''");
  }
  return false;
}

function sourceCommentText(ext, line) {
  if (TS_EXTENSIONS.has(ext) || ext === '.rs') return jsStyleCommentText(line);
  if (PY_EXTENSIONS.has(ext)) return hashCommentText(line);
  return line;
}

function jsStyleCommentText(line) {
  const single = line.indexOf('//');
  const block = line.indexOf('/*');
  const indexes = [single, block].filter((index) => index >= 0);
  if (indexes.length === 0) return '';
  return line.slice(Math.min(...indexes));
}

function hashCommentText(line) {
  const index = line.indexOf('#');
  return index >= 0 ? line.slice(index) : '';
}

function addViolation(violations, root, filePath, line, ruleId, detail, sourceLine = null) {
  const rule = GENERIC_RULES[ruleId] ?? { title: 'Unknown rule', snippet: '' };
  violations.push({
    ruleId,
    title: rule.title,
    detail,
    file: normalizeRel(root, filePath),
    line,
    snippet: rule.snippet,
    source: sourceLine?.trim() ?? null,
  });
}

function isGenericFile(filePath, activeLanguages) {
  const ext = path.extname(filePath).toLowerCase();
  if (activeLanguages.has('typescript') && TS_EXTENSIONS.has(ext)) return true;
  if (activeLanguages.has('python') && PY_EXTENSIONS.has(ext)) return true;
  if (
    activeLanguages.has('common') &&
    (TS_EXTENSIONS.has(ext) ||
      PY_EXTENSIONS.has(ext) ||
      ext === '.rs' ||
      isPolicyFile(filePath) ||
      isSensitiveOrGeneratedPath(filePath))
  ) {
    return true;
  }
  return false;
}

function isPolicyFile(filePath) {
  const name = path.basename(filePath).toLowerCase();
  return ['package.json', 'pyproject.toml', 'requirements.txt', 'cargo.toml', 'deny.toml', '.env', '.env.local'].includes(name);
}

function isSensitiveOrGeneratedPath(filePath) {
  const normalized = filePath.split(path.sep).join('/');
  const name = path.basename(normalized).toLowerCase();
  return (
    /(?:^|\/)(?:output|test-results|playwright-report)\//iu.test(normalized) ||
    name === 'google-services.json' ||
    name === 'googleservice-info.plist' ||
    name === 'id_rsa' ||
    name === 'id_rsa.pub' ||
    /^\.env(?:\..+)?$/iu.test(name) ||
    /\.(?:pem|p12|pfx|key)$/iu.test(name)
  );
}

function isTestPath(rel) {
  return /(?:^|\/)(?:test|tests|__tests__)\/|(?:\.test|\.spec)\.[cm]?[jt]sx?$/iu.test(rel);
}

function isPythonTestPath(rel) {
  return /(?:^|\/)(?:test|tests)\/|(?:^|\/)test_[^/]+\.py$|_test\.py$/iu.test(rel);
}

function isProductionSourcePath(rel, ext) {
  if (!(TS_EXTENSIONS.has(ext) || PY_EXTENSIONS.has(ext) || ext === '.rs')) return false;
  if (isTestPath(rel) || isPythonTestPath(rel)) return false;
  if (/(?:^|\/)(?:build\.rs|fixtures?|__fixtures__)\//iu.test(rel)) return false;
  return /^(?:src|apps|packages|crates|tools|scripts)\//u.test(rel);
}

function runGitDiff(root, base, head) {
  if (!base || !head) return '';
  const result = spawnSync('git', ['diff', '--name-only', '--diff-filter=ACMR', base, head], {
    cwd: root,
    encoding: 'utf8',
    shell: false,
  });
  if ((result.status ?? 1) !== 0) throw new Error(result.stderr?.trim() || 'failed to list diff files');
  return result.stdout.trim();
}

function redact(value) {
  return value.replace(/(["'])[A-Za-z0-9_./+=:@-]{8,}\1/gu, '$1[REDACTED]$1');
}
