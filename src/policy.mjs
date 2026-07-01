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

export function applyWaivers(findings, config, registryRules = [], options = {}) {
  const registryPolicyMap = buildRegistryPolicyMap(registryRules);
  const waivers = Array.isArray(config.waivers) ? config.waivers : [];
  const active = [];
  const waived = [];
  for (const finding of findings) {
    const waiver = waivers.find((candidate) =>
      waiverAppliesToFinding(candidate, finding, registryPolicyMap, options),
    );
    if (!waiver) {
      active.push(finding);
      continue;
    }
    waived.push({
      ...finding,
      status: "waived",
      waiverId: waiver.waiverId,
      waiverOwner: waiver.owner,
      waiverIssue: waiver.issue,
      waiverExpires: waiver.expires,
      waiverReason: waiver.reason,
    });
  }
  return { active, waived };
}

function waiverAppliesToFinding(waiver, finding, registryPolicyMap, options) {
  if (!waiver || typeof waiver !== "object") return false;
  const ruleId = String(waiver.ruleId ?? "").toUpperCase();
  if (ruleId !== String(finding.ruleId ?? "").toUpperCase()) return false;
  if (!waiver.waiverId || !waiver.owner || !waiver.issue || !waiver.reason) {
    return false;
  }
  if (waiver.visible === false) return false;
  if (/\b(?:ai|agent|codex|llm)\b/iu.test(String(waiver.owner))) return false;
  if (isWaiverExpired(waiver.expires, options.now)) return false;
  if (options.ci === true && waiver.ciAllowed !== true) return false;
  const capabilities = rulePolicyCapabilities(registryPolicyMap.get(ruleId) ?? {});
  if (!capabilities.waivable) return false;
  const scopes = Array.isArray(waiver.scope) ? waiver.scope : [];
  return scopes.some((scope) => waiverScopeMatches(scope, finding.file ?? ""));
}

function isWaiverExpired(expires, now = new Date()) {
  if (!expires) return true;
  const parsed = new Date(`${expires}T23:59:59.999Z`);
  if (Number.isNaN(parsed.getTime())) return true;
  return parsed.getTime() < now.getTime();
}

function waiverScopeMatches(scope, file) {
  const normalizedScope = normalizePathForPolicy(scope);
  const normalizedFile = normalizePathForPolicy(file);
  if (!normalizedScope || isBroadWaiverScope(normalizedScope)) return false;
  if (normalizedScope === normalizedFile) return true;
  if (normalizedScope.endsWith("/**")) {
    const prefix = normalizedScope.slice(0, -3);
    return normalizedFile === prefix || normalizedFile.startsWith(`${prefix}/`);
  }
  const pattern = new RegExp(`^${globToRegex(normalizedScope)}$`, "u");
  return pattern.test(normalizedFile);
}

function normalizePathForPolicy(value) {
  return String(value ?? "").replaceAll("\\", "/").replace(/^\.\/+/u, "").trim();
}

function isBroadWaiverScope(scope) {
  return new Set(["", ".", "/", "**", "**/*", "src/**", "crates/**", "packages/**", "apps/**"]).has(scope);
}

function escapeRegex(value) {
  return String(value).replace(/[|\\{}()[\]^$+?.]/gu, "\\$&");
}

function globToRegex(value) {
  let output = "";
  for (let index = 0; index < value.length; index += 1) {
    const char = value[index];
    if (char === "*") {
      if (value[index + 1] === "*") {
        output += ".*";
        index += 1;
      } else {
        output += "[^/]*";
      }
      continue;
    }
    output += escapeRegex(char);
  }
  return output;
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
