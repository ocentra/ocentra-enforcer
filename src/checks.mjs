import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { collectFiles, normalizeRel, repoAbsolute } from './path-utils.mjs';

export const CHECK_RULES = {
  'SRC-1.1': {
    title: 'Source files must stay within shape limits',
    snippet: 'Split oversized files, long functions, and dumping-ground modules before adding behavior.',
  },
  'TEST-2.1': {
    title: 'Source workspaces must have test scaffolds',
    snippet: 'Add package/crate tests before treating source work as complete.',
  },
  'CONTRACT-1.1': {
    title: 'Single-source contract values must not be copied',
    snippet: 'Import or derive values from the owner contract instead of duplicating literals.',
  },
  'DEP-1.1': {
    title: 'Dependency security audit must pass',
    snippet: 'Fix high npm audit findings or cargo audit advisories instead of suppressing them.',
  },
  'DEP-1.2': {
    title: 'External npm package licenses must match policy',
    snippet: 'Use approved licenses or add a reviewed project policy exception.',
  },
  'SBOM-1.1': {
    title: 'SBOM generation must complete',
    snippet: 'Generate package metadata artifacts without committing generated output to source.',
  },
  'AI-1.1': {
    title: 'Agent rule docs must be indexed',
    snippet: 'Keep AGENTS and rule docs routed through a small index instead of broad rulebook loading.',
  },
};

const DEFAULT_ALLOWED_LICENSES = new Set([
  '0BSD',
  'Apache-2.0 OR MIT',
  'Apache-2.0',
  'BSD-2-Clause',
  'BSD-3-Clause',
  'BlueOak-1.0.0',
  'CC-BY-4.0',
  'ISC',
  'MIT',
  'MPL-2.0',
  'Python-2.0',
]);

const CHECK_ALIASES = new Map([
  ['check-source-shape', 'source-shape'],
  ['check-required-tests', 'required-tests'],
  ['check-single-source-contracts', 'single-source-contracts'],
  ['check-ai-rule-index', 'ai-rule-index'],
  ['check-dependency-policy', 'dependency-policy'],
  ['write-sbom', 'sbom'],
]);

export const SCANNER_BACKED_CHECKS = {
  'no-zod-source': {
    languages: ['typescript', 'common'],
    ruleIds: ['TS-1.2'],
  },
  'no-naked-domain-strings': {
    languages: ['typescript', 'common'],
    ruleIds: ['TS-1.3'],
  },
  'no-test-doubles': {
    languages: ['typescript', 'python', 'common'],
    ruleIds: ['TEST-1.1'],
  },
  'weak-assertions': {
    languages: ['typescript', 'python', 'common'],
    ruleIds: ['TEST-1.2'],
  },
  'skipped-focused-tests': {
    languages: ['typescript', 'python', 'common'],
    ruleIds: ['TS-3.1', 'PY-2.1', 'TEST-1.3'],
  },
  'validation-bypass': {
    languages: ['rust', 'typescript', 'python', 'common'],
    ruleIds: ['RR-2.1', 'RR-2.2', 'TS-2.1', 'PY-1.1', 'PY-1.2'],
  },
  'placeholder-implementation': {
    languages: ['rust', 'typescript', 'python', 'common'],
    ruleIds: ['RR-4.2', 'RR-4.3', 'SRC-1.2'],
  },
  reexports: {
    languages: ['rust', 'typescript'],
    ruleIds: ['RR-7.2', 'RR-7.3', 'TS-1.1'],
  },
  'cross-platform-script-commands': {
    languages: ['common'],
    ruleIds: ['PORT-1.1'],
  },
  'generated-artifacts': {
    languages: ['common'],
    ruleIds: ['GEN-1.1', 'GEN-1.2'],
  },
  secrets: {
    languages: ['common'],
    ruleIds: ['SEC-1.1', 'SEC-1.2'],
  },
  'rust-string-boundaries': {
    languages: ['rust'],
    ruleIds: ['RR-18.16'],
  },
};

export function normalizeCheckName(value) {
  const normalized = String(value ?? '').trim().replace(/^check-/u, '');
  return CHECK_ALIASES.get(normalized) ?? normalized;
}

export function listStandaloneChecks() {
  return [
    ...Object.keys(SCANNER_BACKED_CHECKS),
    'source-shape',
    'required-tests',
    'single-source-contracts',
    'dependency-policy',
    'sbom',
    'ai-rule-index',
  ];
}

export function runStandaloneCheck({ checkName, root, config = {}, args = {} }) {
  const normalized = normalizeCheckName(checkName);
  switch (normalized) {
    case 'source-shape':
      return buildReport({ root, config, checkName: normalized, findings: collectSourceShapeFindings(root, config) });
    case 'required-tests':
      return buildReport({ root, config, checkName: normalized, findings: collectRequiredTestFindings(root, config) });
    case 'single-source-contracts':
      return buildReport({ root, config, checkName: normalized, findings: collectSingleSourceContractFindings(root, args.checkConfigPath) });
    case 'dependency-policy':
      return buildReport({ root, config, checkName: normalized, findings: collectDependencyPolicyFindings(root, config) });
    case 'sbom':
      return buildReport({ root, config, checkName: normalized, findings: runSbomCheck(root, args) });
    case 'ai-rule-index':
      return buildReport({ root, config, checkName: normalized, findings: collectAiRuleIndexFindings(root, config) });
    default:
      throw new Error(`Unknown standalone check: ${checkName}`);
  }
}

function collectSourceShapeFindings(root, config) {
  const policies = config.sourceShapePolicies ?? [
    {
      roots: ['apps'],
      extensions: ['.ts', '.tsx'],
      kind: 'typescript',
      maxClasses: 1,
      maxExports: 35,
      maxFunctionLines: 80,
      maxLines: 1000,
    },
    {
      roots: ['packages'],
      extensions: ['.ts', '.tsx'],
      kind: 'typescript',
      maxClasses: 1,
      maxExports: 45,
      maxFunctionLines: 80,
      maxLines: 1000,
    },
    {
      roots: ['crates'],
      extensions: ['.rs'],
      kind: 'rust',
      maxFunctionLines: 80,
      maxFunctions: 18,
      maxLines: 1000,
      maxTypes: 24,
    },
  ];

  const findings = [];
  for (const policy of policies) {
    for (const file of collectPolicyFiles(root, config, policy)) {
      const rel = normalizeRel(root, file);
      const text = fs.readFileSync(file, 'utf8');
      if (policy.kind === 'rust') findings.push(...inspectRustShape(root, file, text, policy));
      else findings.push(...inspectTypeScriptShape(root, file, text, policy));
      const lines = countLines(text);
      if (lines > policy.maxLines) {
        findings.push(finding(root, file, policy.maxLines + 1, 'SRC-1.1', `file has ${lines} lines; maximum is ${policy.maxLines}`, null));
      }
    }
  }
  return findings;
}

function inspectTypeScriptShape(root, file, text, policy) {
  const findings = [];
  const lines = text.split(/\r?\n/u);
  const classCount = countMatches(lines, /^\s*(?:export\s+)?class\s+[A-Za-z_$]/u);
  const exportCount = countMatches(lines, /^\s*export\s+(?:class|function|const|let|var|type|interface|enum|default|\{|\*)/u);
  const functionStarts = [];

  lines.forEach((line, index) => {
    if (/^\s*(?:export\s+)?(?:async\s+)?function\s+[A-Za-z_$]|\)\s*=>\s*\{|\b(?:async\s+)?[A-Za-z_$][\w$]*\s*\([^)]*\)\s*\{/u.test(line)) {
      functionStarts.push(index);
    }
  });

  if (classCount > policy.maxClasses) {
    findings.push(finding(root, file, 1, 'SRC-1.1', `file has ${classCount} classes; maximum is ${policy.maxClasses}`, null));
  }
  if (exportCount > policy.maxExports) {
    findings.push(finding(root, file, 1, 'SRC-1.1', `file has ${exportCount} exports; maximum is ${policy.maxExports}`, null));
  }
  for (const start of functionStarts) {
    const end = findBlockEnd(lines, start);
    const span = end - start + 1;
    if (span > policy.maxFunctionLines) {
      findings.push(finding(root, file, start + 1, 'SRC-1.1', `function has ${span} lines; maximum is ${policy.maxFunctionLines}`, lines[start]));
    }
  }

  return findings;
}

function inspectRustShape(root, file, text, policy) {
  const findings = [];
  const lines = text.split(/\r?\n/u);
  const functionStarts = [];
  let typeCount = 0;

  lines.forEach((line, index) => {
    if (/^\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+\w+/u.test(line)) functionStarts.push(index);
    if (/^\s*(?:pub\s+)?(?:struct|enum)\s+\w+/u.test(line)) typeCount += 1;
  });

  if (functionStarts.length > policy.maxFunctions) {
    findings.push(finding(root, file, 1, 'SRC-1.1', `file has ${functionStarts.length} functions; maximum is ${policy.maxFunctions}`, null));
  }
  if (typeCount > policy.maxTypes) {
    findings.push(finding(root, file, 1, 'SRC-1.1', `file has ${typeCount} structs/enums; maximum is ${policy.maxTypes}`, null));
  }
  for (const start of functionStarts) {
    const end = findBlockEnd(lines, start);
    const span = end - start + 1;
    if (span > policy.maxFunctionLines) {
      findings.push(finding(root, file, start + 1, 'SRC-1.1', `function has ${span} lines; maximum is ${policy.maxFunctionLines}`, lines[start]));
    }
  }
  return findings;
}

function collectRequiredTestFindings(root, config) {
  const findings = [];
  for (const workspaceRoot of ['packages', 'apps']) {
    for (const dir of childDirs(path.join(root, workspaceRoot))) {
      const packageJsonPath = path.join(dir, 'package.json');
      const srcPath = path.join(dir, 'src');
      if (!fs.existsSync(packageJsonPath) || !fs.existsSync(srcPath)) continue;
      const manifest = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'));
      const hasTests = hasFile(path.join(dir, 'tests'), (file) => /\.(?:test|spec)\.[cm]?tsx?$/u.test(file));
      if (!hasTests) {
        findings.push(finding(root, packageJsonPath, 1, 'TEST-2.1', `${manifest.name ?? normalizeRel(root, dir)} is missing tests/*.test.ts`, null));
      }
    }
  }

  for (const dir of childDirs(path.join(root, 'crates'))) {
    const cargoPath = path.join(dir, 'Cargo.toml');
    if (!fs.existsSync(cargoPath)) continue;
    const hasInlineTestModule = hasFile(path.join(dir, 'src'), (file) => file.endsWith('.rs') && fs.readFileSync(file, 'utf8').includes('#[cfg(test)]'));
    const hasIntegrationTest = hasFile(path.join(dir, 'tests'), (file) => file.endsWith('.rs'));
    if (!hasInlineTestModule && !hasIntegrationTest) {
      findings.push(finding(root, cargoPath, 1, 'TEST-2.1', `${normalizeRel(root, dir)} is missing Rust unit or integration tests`, null));
    }
  }

  return findings.filter((entry) => !isIgnored(entry.file, config));
}

function collectSingleSourceContractFindings(root, explicitConfigPath) {
  const configPath = resolveContractConfigPath(root, explicitConfigPath);
  if (!configPath) return [];
  const config = JSON.parse(fs.readFileSync(configPath, 'utf8'));
  const findings = [];

  for (const rawContract of config.contracts ?? []) {
    const ownerPath = rawContract.ownerPath;
    const ownerFullPath = repoAbsolute(root, ownerPath);
    const source = JSON.parse(fs.readFileSync(ownerFullPath, 'utf8'));
    const values = rawContract.values.map(({ name, jsonPath }) => ({
      name,
      text: valueAtPath(source, jsonPath),
    }));
    const allowedPaths = new Set([ownerPath, ...(rawContract.allowedPaths ?? [])].map((entry) => entry.replaceAll('\\', '/')));
    for (const scanRoot of rawContract.scanRoots ?? []) {
      for (const file of collectFiles(root, [scanRoot], {}, (candidate) => /\.(?:rs|ts|tsx|mjs|cjs|js|json|md|ya?ml)$/u.test(candidate))) {
        const rel = normalizeRel(root, file);
        if (allowedPaths.has(rel)) continue;
        const text = fs.readFileSync(file, 'utf8');
        for (const value of values) {
          if (typeof value.text === 'string' && value.text.length > 0 && text.includes(value.text)) {
            findings.push(finding(root, file, 1, 'CONTRACT-1.1', `copied ${rawContract.name}.${value.name} ${value.text}; import or derive from ${ownerPath}`, null));
          }
        }
      }
    }
  }

  return findings;
}

function collectDependencyPolicyFindings(root, config) {
  const findings = [];
  const packageLockPath = path.join(root, 'package-lock.json');
  if (fs.existsSync(packageLockPath)) {
    const audit = spawnInRoot(root, 'npm', ['audit', '--audit-level=high', '--json']);
    if (audit.status !== 0) {
      findings.push(finding(root, packageLockPath, 1, 'DEP-1.1', 'npm audit reported high-or-higher vulnerabilities', compactProcessOutput(audit)));
    }
    const lock = JSON.parse(fs.readFileSync(packageLockPath, 'utf8'));
    const allowed = new Set(config.allowedExternalLicenses ?? [...DEFAULT_ALLOWED_LICENSES]);
    for (const [lockPath, packageEntry] of Object.entries(lock.packages ?? {})) {
      if (!lockPath.includes('node_modules')) continue;
      const packageName = lockPath.split('node_modules/').at(-1);
      if (packageName?.startsWith('@ocentra-parent/') || packageName?.startsWith('@ocentra/')) continue;
      const license = packageEntry.license;
      if (typeof license !== 'string' || !allowed.has(license)) {
        findings.push(finding(root, packageLockPath, 1, 'DEP-1.2', `${lockPath}: ${license ?? 'MISSING'}`, null));
      }
    }
  }

  if (fs.existsSync(path.join(root, 'Cargo.lock'))) {
    const cargoAudit = spawnInRoot(root, 'cargo', ['audit', '--deny', 'warnings']);
    if (cargoAudit.error?.code === 'ENOENT') {
      findings.push(finding(root, path.join(root, 'Cargo.lock'), 1, 'DEP-1.1', 'cargo audit is not installed', 'Install cargo-audit or disable this check in project policy.'));
    } else if (cargoAudit.status !== 0) {
      findings.push(finding(root, path.join(root, 'Cargo.lock'), 1, 'DEP-1.1', 'cargo audit reported advisories', compactProcessOutput(cargoAudit)));
    }
  }

  return findings;
}

function runSbomCheck(root, args) {
  const findings = [];
  const outputRoot = repoAbsolute(root, args.output ?? 'target/security');
  if (args.dryRun) return [];
  fs.mkdirSync(outputRoot, { recursive: true });

  if (fs.existsSync(path.join(root, 'package.json'))) {
    const npmSbom = spawnInRoot(root, 'npm', ['sbom', '--sbom-format=cyclonedx']);
    if (npmSbom.status !== 0) findings.push(finding(root, path.join(root, 'package.json'), 1, 'SBOM-1.1', 'npm SBOM generation failed', compactProcessOutput(npmSbom)));
    else fs.writeFileSync(path.join(outputRoot, 'npm-sbom.cdx.json'), npmSbom.stdout, 'utf8');
  }

  if (fs.existsSync(path.join(root, 'Cargo.toml'))) {
    const cargoMetadata = spawnInRoot(root, 'cargo', ['metadata', '--format-version=1', '--locked']);
    if (cargoMetadata.status !== 0) findings.push(finding(root, path.join(root, 'Cargo.toml'), 1, 'SBOM-1.1', 'cargo metadata generation failed', compactProcessOutput(cargoMetadata)));
    else fs.writeFileSync(path.join(outputRoot, 'cargo-metadata.json'), cargoMetadata.stdout, 'utf8');
  }

  return findings;
}

function collectAiRuleIndexFindings(root, config) {
  const findings = [];
  const agentsPath = path.join(root, 'AGENTS.md');
  const rulesRoot = path.join(root, '.ocentra-ai', 'rules');
  if (!fs.existsSync(agentsPath) || !fs.existsSync(rulesRoot)) return findings;

  const ruleFiles = fs
    .readdirSync(rulesRoot)
    .filter((entry) => entry.endsWith('.md') || entry.endsWith('.mdc'))
    .map((entry) => path.join(rulesRoot, entry));
  const indexFile = ruleFiles.find((file) => /rules|index/iu.test(path.basename(file))) ?? ruleFiles[0];
  if (!indexFile) return findings;

  const agentsText = fs.readFileSync(agentsPath, 'utf8');
  const indexText = fs.readFileSync(indexFile, 'utf8');
  const indexRel = normalizeRel(root, indexFile);
  if (!agentsText.includes(indexRel) && !agentsText.includes(indexRel.replaceAll('/', '\\'))) {
    findings.push(finding(root, agentsPath, 1, 'AI-1.1', `AGENTS.md must reference ${indexRel}`, null));
  }

  for (const ruleFile of ruleFiles) {
    const rel = normalizeRel(root, ruleFile);
    const lineCount = countLines(fs.readFileSync(ruleFile, 'utf8'));
    if (ruleFile !== indexFile && !indexText.includes(normalizeRel(rulesRoot, ruleFile))) {
      findings.push(finding(root, ruleFile, 1, 'AI-1.1', `${rel} is not linked from ${indexRel}`, null));
    }
    const maxLines = config.agentRuleMaxLines ?? 220;
    if (lineCount > maxLines) {
      findings.push(finding(root, ruleFile, maxLines + 1, 'AI-1.1', `${rel} has ${lineCount} lines; split rule files above ${maxLines}`, null));
    }
  }
  return findings;
}

function buildReport({ root, config, checkName, findings }) {
  return {
    ok: findings.length === 0,
    command: 'check',
    check: checkName,
    root,
    profileName: config.profileName ?? 'strict',
    violations: findings,
    warnings: [],
    findings,
    bySeverity: findings.length === 0 ? {} : { error: findings.length },
  };
}

function collectPolicyFiles(root, config, policy) {
  return collectFiles(
    root,
    policy.roots ?? [],
    config,
    (file) => (policy.extensions ?? []).includes(path.extname(file).toLowerCase())
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
  if (stats.isDirectory()) return fs.readdirSync(start).some((entry) => hasFile(path.join(start, entry), predicate));
  return stats.isFile() && predicate(start);
}

function countMatches(lines, pattern) {
  return lines.reduce((count, line) => count + (pattern.test(line) ? 1 : 0), 0);
}

function countLines(text) {
  return text.length === 0 ? 0 : text.split(/\r?\n/u).length;
}

function findBlockEnd(lines, start) {
  let depth = 0;
  let seenBody = false;
  for (let index = start; index < lines.length; index += 1) {
    for (const char of lines[index]) {
      if (char === '{') {
        seenBody = true;
        depth += 1;
      } else if (char === '}') {
        depth -= 1;
      }
    }
    if (seenBody && depth <= 0) return index;
  }
  return start;
}

function valueAtPath(source, jsonPath) {
  let value = source;
  for (const segment of jsonPath.split('.')) {
    if (value === null || typeof value !== 'object' || !(segment in value)) {
      throw new Error(`${jsonPath} is missing`);
    }
    value = value[segment];
  }
  return value;
}

function resolveContractConfigPath(root, explicitConfigPath) {
  const candidates = [
    explicitConfigPath ? repoAbsolute(root, explicitConfigPath) : null,
    path.join(root, 'ocentra-enforcer.single-source-contracts.json'),
    path.join(root, 'scripts', 'check-single-source-contracts.json'),
  ].filter(Boolean);
  return candidates.find((candidate) => fs.existsSync(candidate)) ?? null;
}

function spawnInRoot(root, command, args) {
  return spawnSync(command, args, {
    cwd: root,
    encoding: 'utf8',
    shell: process.platform === 'win32',
    maxBuffer: 64 * 1024 * 1024,
  });
}

function compactProcessOutput(result) {
  return [result.stdout, result.stderr]
    .filter(Boolean)
    .join('\n')
    .split(/\r?\n/u)
    .filter(Boolean)
    .slice(0, 20)
    .join('\n');
}

function isIgnored(file, config) {
  const rel = String(file ?? '').replaceAll('\\', '/');
  const ignoreDirs = config.ignoreDirs ?? [];
  return rel.split('/').some((part) => ignoreDirs.includes(part));
}

function finding(root, file, line, ruleId, detail, source) {
  const rule = CHECK_RULES[ruleId];
  return {
    ruleId,
    severity: 'error',
    title: rule.title,
    detail,
    file: normalizeRel(root, file),
    line,
    snippet: rule.snippet,
    source: source == null ? null : String(source).trim(),
  };
}
