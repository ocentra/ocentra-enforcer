export const SEVERITIES = Object.freeze(['error', 'warning', 'info']);

const DEFAULT_FAIL_ON = Object.freeze(['error']);

export function normalizeFailOn(value) {
  const failOn = Array.isArray(value) && value.length > 0 ? value : DEFAULT_FAIL_ON;
  return [...new Set(failOn.map((entry) => String(entry).trim()).filter(Boolean))];
}

export function normalizeRuleOverrides(value = {}) {
  const overrides = {};
  for (const [ruleId, override] of Object.entries(value ?? {})) {
    if (!override || typeof override !== 'object') continue;
    overrides[ruleId.toUpperCase()] = {
      enabled: override.enabled !== false,
      severity: override.severity ?? null,
      note: override.note ?? null,
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

export function policyForRule(ruleId, config, registrySeverityMap = new Map()) {
  const normalizedRuleId = String(ruleId).toUpperCase();
  const override = config.rules?.[normalizedRuleId] ?? null;
  const registrySeverity = registrySeverityMap.get(normalizedRuleId) ?? 'error';
  return {
    enabled: override?.enabled !== false,
    severity: override?.severity ?? null,
    defaultSeverity: registrySeverity,
  };
}

export function applyRulePolicy(findings, config, registryRules = []) {
  const registrySeverityMap = buildRegistrySeverityMap(registryRules);
  const policyFindings = [];
  for (const finding of findings) {
    const policy = policyForRule(finding.ruleId, config, registrySeverityMap);
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
