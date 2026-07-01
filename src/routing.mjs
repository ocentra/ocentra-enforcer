import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { decodeEnforcerConfig, decodeRuleRegistry } from '../schemas/effect/enforcer-schemas.mjs';
import { buildRegistryPolicyMap, buildRegistrySeverityMap, normalizeFailOn, normalizeRuleOverrides, normalizeToolPolicies, policyForRule } from './policy.mjs';

const DEFAULT_PACK_ROOT = path.resolve(path.join(path.dirname(fileURLToPath(import.meta.url)), '..'));

export function loadRuleRegistry(packRoot = DEFAULT_PACK_ROOT) {
  return decodeRuleRegistry(JSON.parse(fs.readFileSync(path.join(packRoot, 'rules', 'rules.json'), 'utf8')));
}

export function routeRules(args = {}, packRoot = DEFAULT_PACK_ROOT) {
  const root = path.resolve(args.root ?? process.cwd());
  const registry = loadRuleRegistry(packRoot);
  const config = loadRouteConfig(root, args, packRoot);
  const registrySeverityMap = buildRegistrySeverityMap(registry.rules);
  const registryPolicyMap = buildRegistryPolicyMap(registry.rules);
  const profileName = config.profileName ?? resolveProfileName(root, args, packRoot);
  const explicitRuleId = args.ruleId?.toUpperCase() ?? null;
  const familyKeys = explicitRuleId ? [] : routeFamilyKeys(args);
  const routedRules = registry.rules.map((rule) => {
    const lockedPolicy = policyForRule(rule.id, config, registrySeverityMap, registryPolicyMap);
    return {
      ...rule,
      enabled: lockedPolicy.enabled,
      severity: lockedPolicy.severity ?? rule.severity,
    };
  });
  const rules = explicitRuleId
    ? routedRules.filter((rule) => rule.id === explicitRuleId)
    : routedRules.filter((rule) => rule.enabled && matchesFamilyKeys(rule, familyKeys));
  const docs = uniqueSorted(rules.map((rule) => rule.doc));

  return {
    ok: true,
    productName: registry.productName,
    profileName,
    index: 'rules/INDEX.md',
    scope: describeRouteScope(args),
    docs,
    rules: rules.map((rule) => ({
      id: rule.id,
      language: rule.language,
      family: rule.family,
      severity: rule.severity,
      enabled: rule.enabled,
      doc: rule.doc,
      validator: rule.validator,
    })),
  };
}

export function routeFamilyKeys(args = {}) {
  if (args.scope === 'workspace') {
    return new Set(['rust:*', 'typescript:*', 'python:*', 'common:*']);
  }
  if (args.scope === 'crate') {
    return new Set(['rust:source', 'rust:domain', 'rust:imports-modules', 'rust:async-runtime', 'rust:toolchain-cargo', 'rust:dependencies']);
  }

  const files = Array.isArray(args.files) ? args.files : [];
  const families = new Set();
  for (const file of files) {
    for (const family of routeFamilyKeysForFile(file)) families.add(family);
  }
  return families;
}

export function routeFamilyKeysForFile(file) {
  const normalized = file.split(/[\\/]+/u).pop() ?? file;
  const lower = normalized.toLowerCase();
  const rel = file.split(/[\\/]+/u).join('/');
  const isTestPath = /(?:^|\/)(?:test|tests|__tests__)\/|(?:\.test|\.spec)\.[cm]?[jt]sx?$/iu.test(rel);

  if (lower.endsWith('.rs')) {
    const families = [
      'rust:source',
      'rust:domain',
      'rust:imports-modules',
      'rust:async-runtime',
      'common:source',
      'common:security',
      'common:documentation',
    ];
    if (/(?:^|\/)(?:test|tests)\/|(?:_test|_tests)\.rs$/iu.test(rel)) families.push('common:tests');
    return families;
  }
  if (normalized === 'Cargo.toml') return ['rust:toolchain-cargo', 'rust:dependencies', 'common:security'];
  if (normalized === 'Cargo.lock' || normalized === 'deny.toml') return ['rust:dependencies', 'common:security'];
  if (normalized === 'rust-toolchain.toml' || normalized === 'clippy.toml' || normalized === 'rustfmt.toml') return ['rust:toolchain-cargo'];

  if (/\.[cm]?[jt]sx?$/iu.test(lower)) {
    const families = ['typescript:source', 'common:source', 'common:security', 'common:generated-artifacts', 'common:documentation'];
    if (isTestPath) families.push('typescript:tests');
    if (isTestPath) families.push('common:tests');
    if (rel.startsWith('scripts/')) families.push('common:portability');
    if (rel.startsWith('src/') || rel.startsWith('scripts/') || rel.startsWith('mcp/')) families.push('common:harness');
    return families;
  }
  if (
    ['package.json', 'package-lock.json', 'pnpm-lock.yaml', 'yarn.lock', 'tsconfig.json'].includes(lower) ||
    /^tsconfig\..*\.json$/iu.test(lower) ||
    /^eslint\.config\./iu.test(lower) ||
    /^(?:vitest|jest|playwright)\.config\./iu.test(lower)
  ) {
    const families = ['typescript:toolchain', 'common:security'];
    if (lower === 'package.json') families.push('typescript:source');
    return families;
  }
  if (
    ['ocentra-enforcer.config.json', 'rust-rules.config.json', 'rules.json'].includes(lower) ||
    rel.startsWith('rules/') ||
    rel.startsWith('schemas/') ||
    rel.startsWith('src/') ||
    rel.startsWith('scripts/') ||
    rel.startsWith('mcp/')
  ) {
    return ['common:harness', 'common:documentation', 'common:security'];
  }

  if (lower.endsWith('.py')) {
    const families = ['python:source', 'common:source', 'common:security', 'common:generated-artifacts', 'common:documentation'];
    if (/(?:^|\/)(?:test|tests)\/|(?:^|\/)test_[^/]+\.py$|_test\.py$/iu.test(rel)) families.push('python:tests');
    if (/(?:^|\/)(?:test|tests)\/|(?:^|\/)test_[^/]+\.py$|_test\.py$/iu.test(rel)) families.push('common:tests');
    return families;
  }
  if (['pyproject.toml', 'requirements.txt', 'pytest.ini', 'mypy.ini', 'ruff.toml', 'uv.lock', 'poetry.lock'].includes(lower)) {
    return ['python:toolchain', 'common:security'];
  }

  return [];
}

export function describeRouteScope(args = {}) {
  if (args.ruleId) return { mode: 'rule', ruleId: args.ruleId.toUpperCase() };
  if (args.scope === 'crate') return { mode: 'crate', crateName: args.crateName ?? null };
  if (args.scope === 'diff') return { mode: 'diff', base: args.base ?? null, head: args.head ?? null, files: args.files ?? [] };
  if (args.scope === 'workspace') return { mode: 'workspace' };
  return { mode: 'files', files: args.files ?? [] };
}

function ruleFamilyKey(rule) {
  return `${rule.language}:${rule.family}`;
}

function matchesFamilyKeys(rule, familyKeys) {
  return familyKeys.has(ruleFamilyKey(rule)) || familyKeys.has(`${rule.language}:*`);
}

function loadRouteConfig(root, args, packRoot) {
  const routeDefaultConfig = {
    profileName: args.profile ?? 'strict',
    failOn: ['error'],
    rules: {},
    tools: {},
  };
  let parsed = {};
  const candidate = routeConfigCandidate(root, args, packRoot);
  if (candidate && fs.existsSync(candidate)) parsed = JSON.parse(fs.readFileSync(candidate, 'utf8'));
  const decoded = decodeEnforcerConfig({ ...routeDefaultConfig, ...parsed });
  return {
    ...decoded,
    failOn: normalizeFailOn(decoded.failOn),
    rules: normalizeRuleOverrides(decoded.rules),
    tools: normalizeToolPolicies(decoded.tools),
  };
}

function routeConfigCandidate(root, args, packRoot) {
  if (args.configPath) return path.isAbsolute(args.configPath) ? args.configPath : path.join(root, args.configPath);
  if (args.profile) return profileConfigPath(args.profile, packRoot);
  const branded = path.join(root, 'ocentra-enforcer.config.json');
  if (fs.existsSync(branded)) return branded;
  const legacy = path.join(root, 'rust-rules.config.json');
  if (fs.existsSync(legacy)) return legacy;
  return profileConfigPath('strict', packRoot);
}

function resolveProfileName(root, args, packRoot) {
  if (args.configPath) {
    const configPath = path.isAbsolute(args.configPath) ? args.configPath : path.join(root, args.configPath);
    if (!fs.existsSync(configPath)) return 'custom';
    const parsed = JSON.parse(fs.readFileSync(configPath, 'utf8'));
    return parsed.profileName ?? 'custom';
  }
  if (args.profile) {
    const profilePath = profileConfigPath(args.profile, packRoot);
    const parsed = JSON.parse(fs.readFileSync(profilePath, 'utf8'));
    return parsed.profileName ?? args.profile;
  }
  const targetConfig = routeConfigCandidate(root, args, packRoot);
  if (fs.existsSync(targetConfig)) {
    const parsed = JSON.parse(fs.readFileSync(targetConfig, 'utf8'));
    return parsed.profileName ?? 'strict';
  }
  return 'strict';
}

function profileConfigPath(profile, packRoot) {
  if (!/^[A-Za-z0-9][A-Za-z0-9_.-]*$/u.test(profile)) throw new Error(`Invalid profile name: ${profile}`);
  const profilePath = path.join(packRoot, 'profiles', `${profile}.json`);
  if (!fs.existsSync(profilePath)) throw new Error(`Unknown Ocentra Enforcer profile "${profile}". Expected ${profilePath}.`);
  return profilePath;
}

function uniqueSorted(values) {
  return [...new Set(values)].sort((a, b) => a.localeCompare(b));
}
