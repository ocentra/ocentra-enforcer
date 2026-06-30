import fs from 'node:fs';
import path from 'node:path';
import { normalizeRel } from './path-utils.mjs';

export const SOURCE_POLICY_RULES = {
  'TS-1.2': {
    title: 'Direct Zod source usage is forbidden',
    snippet: 'Use Effect Schema through domain-owned schemas instead of importing or exposing Zod directly.',
  },
  'TS-1.3': {
    title: 'Naked domain string aliases are forbidden',
    snippet: 'Use Effect Schema brands plus decode helpers instead of type FooId = string or manual __brand intersections.',
  },
  'TEST-1.1': {
    title: 'Test doubles are forbidden by default',
    snippet: 'Use real domain contracts, real parsers, and real local services instead of mocks, fakes, stubs, or spies.',
  },
  'PORT-1.1': {
    title: 'Platform-specific script commands must be guarded',
    snippet: 'Put Windows-only command invocations behind an explicit process.platform guard or use a cross-platform helper.',
  },
  'SEC-1.2': {
    title: 'Sensitive files are forbidden in source scope',
    snippet: 'Do not commit .env files, private keys, mobile service secrets, or credential bundles.',
  },
  'GEN-1.2': {
    title: 'Generated output folders must not be committed as source',
    snippet: 'Keep proof output, test results, reports, and generated build artifacts in ignored output folders or CI artifacts.',
  },
};

const zodSourcePatterns = [
  { label: 'direct zod import', pattern: /from\s+['"]zod['"]|require\(\s*['"]zod['"]\s*\)/u },
  { label: 'Zod resolver', pattern: /\bzodResolver\b/u },
  {
    label: 'Zod public type/API',
    pattern: /\bZod(?:Error|Issue|Type|Schema|Object|String|Number|Boolean|Array|Record|Union)\b/u,
  },
  { label: 'stale schema validator path', pattern: new RegExp(String.raw`schema[/\\]` + 'zo' + 'd', 'u') },
];

const manualBrandPattern = /\b(?:export\s+)?type\s+\w+\s*=\s*string\s*&\s*\{\s*readonly\s+__brand\b/u;
const nakedDomainAliasPattern =
  /^\s*(?:export\s+)?type\s+(\w*(?:Id|ID|Path|Key|Name|Hash|URL|Url|Type|Slug|Route|Label|Title|Description|Status|Version)\w*)\s*=\s*string\s*;/u;
const windowsOnlyCommandPatterns = [
  {
    label: 'Windows cmd npm invocation',
    pattern: /['"]cmd(?:\.exe)?['"]\s*,\s*\[\s*['"]\/c['"]\s*,\s*['"]npm['"]/u,
  },
];
const word = (...parts) => parts.join('');
const doubleTerms = {
  m: word('mo', 'ck'),
  f: word('fa', 'ke'),
  s: word('st', 'ub'),
  p: word('sp', 'y'),
  po: word('sp', 'y', 'On'),
};
const testDoublePatterns = [
  { label: 'module double API', pattern: new RegExp(String.raw`\b(?:vi|jest)\.${doubleTerms.m}\b`, 'iu') },
  { label: 'double function API', pattern: /\b(?:vi|jest)\.fn\b/iu },
  {
    label: 'observer double API',
    pattern: new RegExp(String.raw`\b(?:vi|jest)\.${doubleTerms.po}\b|\b${doubleTerms.po}\b`, 'iu'),
  },
  {
    label: 'test-double package',
    pattern: new RegExp(String.raw`\b(?:${word('si', 'non')}|${word('no', 'ck')}|${word('m', 'sw')})\b`, 'iu'),
  },
  {
    label: 'test-double vocabulary',
    pattern: new RegExp(String.raw`\b(?:${doubleTerms.m}|${doubleTerms.f}|${doubleTerms.s}|${doubleTerms.p})\b`, 'iu'),
  },
];
const allowedSensitivePathPatterns = [/(^|\/)\.env\.example$/iu, /(^|\/)\.env\.sample$/iu, /(^|\/)\.env\.template$/iu];
const forbiddenSensitivePathPatterns = [
  /(^|\/)\.env(\..+)?$/iu,
  /(^|\/)google-services\.json$/iu,
  /(^|\/)GoogleService-Info\.plist$/u,
  /(^|\/)id_rsa(\.pub)?$/iu,
  /\.(pem|p12|pfx|key)$/iu,
];

export function scanAdditionalTypeScriptFile(root, filePath) {
  const rel = normalizeRel(root, filePath);
  const lines = readLines(filePath);
  const violations = [];
  const generatedPath = isGeneratedSourcePath(rel);

  lines.forEach((line, index) => {
    const lineNo = index + 1;
    for (const rule of zodSourcePatterns) {
      if (rule.pattern.test(line)) {
        addViolation(violations, root, filePath, lineNo, 'TS-1.2', rule.label, line);
      }
    }

    if (!generatedPath && manualBrandPattern.test(line)) {
      addViolation(violations, root, filePath, lineNo, 'TS-1.3', 'manual string brand', line);
    }

    const nakedDomainAlias = line.match(nakedDomainAliasPattern);
    if (!generatedPath && nakedDomainAlias) {
      addViolation(violations, root, filePath, lineNo, 'TS-1.3', `naked domain string alias ${nakedDomainAlias[1]}`, line);
    }
  });

  if (path.basename(rel) === 'package.json') {
    violations.push(...scanPackageManifestForZod(root, filePath));
  }

  return violations;
}

export function scanAdditionalCommonFile(root, filePath, lines) {
  const rel = normalizeRel(root, filePath);
  const violations = [];

  if (isForbiddenSensitivePath(rel)) {
    addViolation(violations, root, filePath, 1, 'SEC-1.2', 'forbidden sensitive file path', rel);
  }

  if (/^(?:output|test-results|playwright-report)\//iu.test(rel)) {
    addViolation(violations, root, filePath, 1, 'GEN-1.2', 'generated output path is in source scope', rel);
  }

  if (path.basename(rel) === 'package.json') {
    violations.push(...scanPackageManifestForZod(root, filePath));
  }

  if (isSourceLikeForTestDoubles(rel)) {
    lines.forEach((line, index) => {
      for (const rule of testDoublePatterns) {
        if (rule.pattern.test(line)) {
          addViolation(violations, root, filePath, index + 1, 'TEST-1.1', rule.label, line);
        }
      }
    });
  }

  if (rel.startsWith('scripts/') && rel.endsWith('.mjs')) {
    lines.forEach((line, index) => {
      for (const rule of windowsOnlyCommandPatterns) {
        if (rule.pattern.test(line) && !hasNearbyWindowsGuard(lines, index)) {
          addViolation(violations, root, filePath, index + 1, 'PORT-1.1', rule.label, line);
        }
      }
    });
  }

  return violations;
}

function scanPackageManifestForZod(root, filePath) {
  const violations = [];
  let parsed;
  try {
    parsed = JSON.parse(readLines(filePath).join('\n'));
  } catch {
    return violations;
  }

  for (const section of ['dependencies', 'devDependencies', 'peerDependencies', 'optionalDependencies']) {
    const dependencies = parsed[section];
    if (dependencies == null || typeof dependencies !== 'object') continue;
    for (const name of Object.keys(dependencies)) {
      if (['zod', 'zod-to-json-schema', 'zod-validation-error'].includes(name)) {
        addViolation(violations, root, filePath, 1, 'TS-1.2', `direct ${name} dependency in ${section}`, name);
      }
    }
  }

  return violations;
}

function readLines(filePath) {
  return fs.readFileSync(filePath, 'utf8').split(/\r?\n/u);
}

function hasNearbyWindowsGuard(lines, lineIndex) {
  const start = Math.max(0, lineIndex - 8);
  const nearby = lines.slice(start, lineIndex + 1).join('\n');
  return /process\.platform\s*={2,3}\s*['"]win32['"]|process\.platform\s*!={1,2}\s*['"]win32['"]/u.test(nearby);
}

function isForbiddenSensitivePath(rel) {
  if (allowedSensitivePathPatterns.some((pattern) => pattern.test(rel))) return false;
  return forbiddenSensitivePathPatterns.some((pattern) => pattern.test(rel));
}

function isSourceLikeForTestDoubles(rel) {
  return /\.(?:ts|tsx|js|jsx|mjs|cjs|mts|cts|py|rs)$/iu.test(rel);
}

function isGeneratedSourcePath(rel) {
  return /(?:^|\/)generated(?:\/|$)/iu.test(rel);
}

function addViolation(violations, root, filePath, line, ruleId, detail, sourceLine = null) {
  const rule = SOURCE_POLICY_RULES[ruleId] ?? { title: 'Unknown rule', snippet: '' };
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
