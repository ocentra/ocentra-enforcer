export const SEVERITIES = Object.freeze(['error', 'warning', 'info']);
// PUBLIC-API-BUDGET-JUSTIFICATION: policy helpers are the shared config contract for CLI, MCP, scanners, and tests.

const DEFAULT_FAIL_ON = Object.freeze(['error']);
const STRICT_PROFILE_NAMES = new Set(['strict', 'default']);

const SEVERITY_RANK = Object.freeze({
  info: 1,
  warning: 2,
  error: 3,
});

export function normalizeFailOn(value, options = {}) {
  const enforceError = options.enforceError !== false;
  const failOn = Array.isArray(value) ? value : DEFAULT_FAIL_ON;
  const normalized = [
    ...new Set(failOn.map((entry) => String(entry).trim()).filter(Boolean)),
  ];
  if (normalized.length === 0 && !enforceError) return [];
  if (normalized.length === 0) return [...DEFAULT_FAIL_ON];
  if (enforceError && !normalized.includes('error')) {
    return ['error', ...normalized];
  }
  return normalized;
}

export function normalizeRuleOverrides(value = {}) {
  const overrides = {};
  for (const [ruleId, override] of Object.entries(value ?? {})) {
    if (!override || typeof override !== 'object') continue;
    overrides[ruleId.toUpperCase()] = {
      enabled: override.enabled !== false,
      severity: override.severity ?? null,
      note: override.note ?? null,
      waiverId: override.waiverId ?? null,
      owner: override.owner ?? null,
      issue: override.issue ?? null,
      reason: override.reason ?? null,
      scope: Array.isArray(override.scope) ? override.scope : [],
      expires: override.expires ?? null,
      remediation: override.remediation ?? null,
      ciAllowed: override.ciAllowed ?? null,
      localAllowed: override.localAllowed ?? null,
    };
  }
  return overrides;
}

export function normalizeToolPolicies(value = {}) {
  const policies = {};
  for (const [toolId, policy] of Object.entries(value ?? {})) {
    if (!policy || typeof policy !== 'object') continue;
    policies[toolId] = {
      enabled: policy.enabled !== false,
      severity: policy.severity ?? null,
      note: policy.note ?? null,
    };
  }
  return policies;
}

export function buildRegistrySeverityMap(registryRules = []) {
  const map = new Map();
  for (const rule of registryRules) map.set(rule.id, rule.severity ?? 'error');
  return map;
}

export function buildRegistryPolicyMap(registryRules = []) {
  const map = new Map();
  for (const rule of registryRules) map.set(rule.id, rule);
  return map;
}

export function isStrictProfile(config = {}) {
  const profileName = String(config.profileName ?? 'strict').toLowerCase();
  return STRICT_PROFILE_NAMES.has(profileName) || profileName.includes('strict');
}

export function isSeverityDowngrade(fromSeverity, toSeverity) {
  return (SEVERITY_RANK[toSeverity] ?? 0) < (SEVERITY_RANK[fromSeverity] ?? 0);
}

export function rulePolicyCapabilities(rule = {}) {
  const lockLevel =
    rule.lockLevel ?? (rule.severity === 'error' ? 'immutable' : 'advisory');
  const advisory = lockLevel === 'advisory';
  const profileOverridable = lockLevel === 'profile-overridable';
  return {
    lockLevel,
    canDisable: rule.canDisable ?? (advisory || profileOverridable),
    canDowngrade: rule.canDowngrade ?? (advisory || profileOverridable),
    waivable: rule.waivable === true || lockLevel === 'waiver-required',
  };
}

export function policyForRule(
  ruleId,
  config,
  registrySeverityMap = new Map(),
  registryPolicyMap = new Map(),
) {
  const normalizedRuleId = String(ruleId).toUpperCase();
  const override = config.rules?.[normalizedRuleId] ?? null;
  const registryRule = registryPolicyMap.get(normalizedRuleId) ?? {};
  const registrySeverity =
    registryRule.severity ?? registrySeverityMap.get(normalizedRuleId) ?? 'error';
  const capabilities = rulePolicyCapabilities({
    ...registryRule,
    severity: registrySeverity,
  });
  const requestedSeverity = override?.severity ?? null;
  const downgradeBlocked =
    requestedSeverity &&
    isSeverityDowngrade(registrySeverity, requestedSeverity) &&
    !capabilities.canDowngrade;
  const disableBlocked =
    override?.enabled === false && !capabilities.canDisable;
  return {
    enabled: disableBlocked ? true : override?.enabled !== false,
    severity: downgradeBlocked ? null : requestedSeverity,
    defaultSeverity: registrySeverity,
    lockLevel: capabilities.lockLevel,
    canDisable: capabilities.canDisable,
    canDowngrade: capabilities.canDowngrade,
    disableBlocked,
    downgradeBlocked,
  };
}

export function applyRulePolicy(findings, config, registryRules = []) {
  const registrySeverityMap = buildRegistrySeverityMap(registryRules);
  const registryPolicyMap = buildRegistryPolicyMap(registryRules);
  const policyFindings = [];
  for (const finding of findings) {
    const policy = policyForRule(
      finding.ruleId,
      config,
      registrySeverityMap,
      registryPolicyMap,
    );
    if (!policy.enabled) continue;
    policyFindings.push({
      ...finding,
      severity: policy.severity ?? finding.severity ?? policy.defaultSeverity,
    });
  }
  return policyFindings;
}

export function splitFindings(findings, config) {
  const failOn = new Set(normalizeFailOn(config.failOn));
  const violations = [];
  const warnings = [];
  for (const finding of findings) {
    if (failOn.has(finding.severity)) violations.push(finding);
    else warnings.push(finding);
  }
  return {
    violations,
    warnings,
    bySeverity: summarizeBySeverity(findings),
  };
}

export function summarizeBySeverity(findings) {
  const summary = {};
  for (const finding of findings) {
    summary[finding.severity] = (summary[finding.severity] ?? 0) + 1;
  }
  return summary;
}

export function policyForTool(toolId, config, defaults = {}) {
  const policy = config.tools?.[toolId] ?? {};
  return {
    enabled: policy.enabled ?? defaults.enabled ?? true,
    severity: policy.severity ?? defaults.severity ?? 'error',
  };
}
