import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { lineNumberAt, normalizeRel, repoAbsolute } from "./path-utils.mjs";
import {
  buildRegistryPolicyMap,
  buildRegistrySeverityMap,
  isSeverityDowngrade,
  isStrictProfile,
  normalizeFailOn,
  policyForRule,
  rulePolicyCapabilities,
} from "./policy.mjs";
import { registryRules as loadRegistryRules } from "./rule-registry.mjs";

const KNOWN_CONFIG_KEYS = new Set([
  "schemaVersion",
  "profileName",
  "failOn",
  "failFast",
  "enforceWorkspaceFiles",
  "requireCargoDeny",
  "requireCargoAudit",
  "runCargoDoc",
  "cargoOnFileScope",
  "cargoOnDiffScope",
  "cargoTestThreads",
  "allowUnsafeCode",
  "allowBuildRs",
  "allowGitDependencies",
  "allowPathDependencies",
  "publicReexportPolicy",
  "ignoreDirs",
  "ignoreFileGlobs",
  "rustRoots",
  "crateRootGlobs",
  "testFileGlobs",
  "rawTypeBoundaryGlobs",
  "boundaryOwnerNote",
  "facadeFileGlobs",
  "rawStringOwnerGlobs",
  "domainPrimitiveOwnerGlobs",
  "enforceRuntimeStringLiterals",
  "runtimeStringOwnerGlobs",
  "runtimeStringLineAllowPatterns",
  "enforceSerializedPublicDomainPrimitives",
  "serializedDomainOwnerGlobs",
  "blockedProtocolDependencies",
  "runtimeCrates",
  "testOnlyCrates",
  "allowedGitDependencies",
  "allowedExternalLicenses",
  "sourceShapePolicies",
  "sourceShapeOverrides",
  "importBoundaryPolicies",
  "architecturePolicyChecks",
  "singleSourceRequiredMirrorRoots",
  "strictEmptyTestTrees",
  "generatedArtifactsMode",
  "generatedArtifactsTracked",
  "agentRuleMaxLines",
  "maxActiveWaivers",
  "maxWaiverDays",
  "configChangeRequiresSelfCheck",
  "policyIntegrityChecked",
  "languages",
  "rules",
  "waivers",
  "tools",
  "harness",
]);

const BOUNDARY_CONFIG_KEYS = new Set([
  "rawTypeBoundaryGlobs",
  "facadeFileGlobs",
  "rawStringOwnerGlobs",
  "domainPrimitiveOwnerGlobs",
  "runtimeStringOwnerGlobs",
  "runtimeStringLineAllowPatterns",
  "serializedDomainOwnerGlobs",
]);

function finding(root, file, line, ruleId, detail, source) {
  return {
    root,
    file: repoAbsolute(root, file),
    rel: normalizeRel(root, file),
    line,
    ruleId,
    detail,
  };
}

function resolvePackRoot(root) {
  const rootRules = path.join(root, "rules", "rules.json");
  return fs.existsSync(rootRules)
    ? root
    : path.resolve(path.join(path.dirname(fileURLToPath(import.meta.url)), ".."));
}

function existingConfigPath(root) {
  for (const rel of ["ocentra-enforcer.config.json", "rust-rules.config.json"]) {
    const candidate = path.join(root, rel);
    if (fs.existsSync(candidate)) return candidate;
  }
  return null;
}

function readRawConfigObject(configPath) {
  if (!configPath || !fs.existsSync(configPath) || !fs.statSync(configPath).isFile()) {
    return null;
  }
  try {
    const parsed = JSON.parse(fs.readFileSync(configPath, "utf8"));
    return parsed && typeof parsed === "object" && !Array.isArray(parsed) ? parsed : null;
  } catch {
    return null;
  }
}

function hasOverrideWaiverMetadata(override) {
  return Boolean(
    override.waiverId &&
      override.owner &&
      override.issue &&
      override.reason &&
      Array.isArray(override.scope) &&
      override.scope.length > 0 &&
      override.expires &&
      override.remediation,
  );
}

function hasWaiverFor(config, ruleId) {
  return (config.waivers ?? []).some(
    (waiver) => String(waiver.ruleId ?? "").toUpperCase() === ruleId,
  );
}

function parseUtcDate(value) {
  const text = String(value ?? "");
  if (!/^\d{4}-\d{2}-\d{2}$/u.test(text)) return null;
  const date = new Date(`${text}T00:00:00.000Z`);
  return Number.isNaN(date.getTime()) ? null : date;
}

function startOfUtcDay(date) {
  return new Date(Date.UTC(date.getUTCFullYear(), date.getUTCMonth(), date.getUTCDate()));
}

function daysBetweenUtc(start, end) {
  return Math.floor((startOfUtcDay(end) - startOfUtcDay(start)) / (24 * 60 * 60 * 1000));
}

function isBroadWaiverScope(scope) {
  const normalized = String(scope ?? "").replaceAll("\\", "/").trim();
  return (
    normalized === "" ||
    normalized === "." ||
    normalized === "/" ||
    normalized === "**" ||
    normalized === "**/*" ||
    normalized === "src/**" ||
    normalized === "crates/**" ||
    normalized === "packages/**" ||
    normalized === "apps/**" ||
    /^\*\*\/\*\.[A-Za-z0-9]+$/u.test(normalized)
  );
}

export function collectConfigLockdownFindings(root, config) {
  const packRoot = resolvePackRoot(root);
  const rules = loadRegistryRules(packRoot);
  const registrySeverityMap = buildRegistrySeverityMap(rules);
  const registryPolicyMap = buildRegistryPolicyMap(rules);
  const findings = [];
  const configPath = existingConfigPath(root) ?? root;
  const rawConfig = readRawConfigObject(configPath);
  const rawFailOn = Array.isArray(config.rawFailOn) ? config.rawFailOn : config.failOn;
  for (const key of Object.keys(rawConfig ?? {})) {
    if (!KNOWN_CONFIG_KEYS.has(key)) {
      findings.push(
        finding(root, configPath, 1, "CFG-1.9", `unknown config key ${key}`, key),
      );
    }
  }
  if (rawConfig && (!rawConfig.schemaVersion || !rawConfig.profileName)) {
    findings.push(
      finding(root, configPath, 1, "CFG-1.10", "config must declare schemaVersion and profileName for unambiguous layering", null),
    );
  }
  const knownProfiles = new Set(["strict", "default", "ocentra-enforcer", "ocentra-parent"]);
  if (config.profileName && !knownProfiles.has(String(config.profileName))) {
    findings.push(
      finding(root, configPath, 1, "CFG-1.11", `unknown profileName ${config.profileName}`, String(config.profileName)),
    );
  }
  if (config.configChangeRequiresSelfCheck && config.policyIntegrityChecked !== true) {
    findings.push(
      finding(root, configPath, 1, "CFG-1.12", "config change requires policyIntegrityChecked=true after policy-integrity passes", null),
    );
  }
  if (isStrictProfile(config) && !normalizeFailOn(rawFailOn, { enforceError: false }).includes("error")) {
    findings.push(
      finding(
        root,
        configPath,
        1,
        "CFG-1.1",
        'strict profiles must keep "error" in failOn',
        null,
      ),
    );
  }

  for (const [ruleId, override] of Object.entries(config.rules ?? {})) {
    const policy = policyForRule(ruleId, config, registrySeverityMap, registryPolicyMap);
    const rule = registryPolicyMap.get(ruleId);
    if (!rule) {
      findings.push(
        finding(
          root,
          configPath,
          1,
          "ENF-1.3",
          `${ruleId} is configured but not registered`,
          null,
        ),
      );
      continue;
    }
    if (override.enabled === false && policy.disableBlocked) {
      findings.push(
        finding(
          root,
          configPath,
          1,
          "CFG-1.2",
          `${ruleId} is immutable and cannot be disabled`,
          null,
        ),
      );
    }
    if (override.severity && policy.downgradeBlocked) {
      findings.push(
        finding(
          root,
          configPath,
          1,
          "CFG-1.3",
          `${ruleId} is immutable and cannot be downgraded from ${rule.severity} to ${override.severity}`,
          null,
        ),
      );
    }
    if (override.enabled === false && !policy.disableBlocked && !hasOverrideWaiverMetadata(override)) {
      findings.push(
        finding(
          root,
          configPath,
          1,
          "CFG-1.8",
          `${ruleId} disable lacks waiverId, owner, issue, reason, scope, expires, and remediation`,
          null,
        ),
      );
    }
  }

  if (config.allowUnsafeCode && !hasWaiverFor(config, "CFG-1.4")) {
    findings.push(
      finding(root, configPath, 1, "CFG-1.4", "allowUnsafeCode=true requires a narrow waiver", null),
    );
  }
  if (config.publicReexportPolicy === "allow" && isStrictProfile(config) && !hasWaiverFor(config, "CFG-1.5")) {
    findings.push(
      finding(root, configPath, 1, "CFG-1.5", 'publicReexportPolicy="allow" is forbidden in strict profiles', null),
    );
  }
  for (const [field, value] of [
    ["allowBuildRs", config.allowBuildRs],
    ["allowGitDependencies", config.allowGitDependencies],
    ["allowPathDependencies", config.allowPathDependencies],
  ]) {
    if (value && !hasWaiverFor(config, "CFG-1.6")) {
      findings.push(
        finding(root, configPath, 1, "CFG-1.6", `${field}=true requires a narrow waiver`, null),
      );
    }
  }
  for (const [field, value] of Object.entries(rawConfig ?? {})) {
    if (!BOUNDARY_CONFIG_KEYS.has(field)) continue;
    if (Array.isArray(value) && value.length > 0 && !String(rawConfig.boundaryOwnerNote ?? "").trim()) {
      findings.push(
        finding(root, configPath, 1, "CFG-1.7", `${field} changes require boundaryOwnerNote`, field),
      );
    }
  }
  for (const [field, values] of [
    ["sourceShapeOverrides", config.sourceShapeOverrides],
    ["importBoundaryPolicies", config.importBoundaryPolicies],
  ]) {
    for (const entry of values ?? []) {
      const hasGlob = Boolean(entry.glob || (Array.isArray(entry.globs) && entry.globs.length > 0));
      if (hasGlob && !String(entry.note ?? "").trim()) {
        findings.push(
          finding(root, configPath, 1, "CFG-1.7", `${field} glob entries require note`, JSON.stringify(entry)),
        );
      }
    }
  }

  return findings;
}

export function collectWaiverPolicyFindings(root, config) {
  const packRoot = resolvePackRoot(root);
  const registryPolicyMap = buildRegistryPolicyMap(loadRegistryRules(packRoot));
  const findings = [];
  const configPath = existingConfigPath(root) ?? root;
  const today = startOfUtcDay(new Date());
  const activeWaivers = config.waivers ?? [];
  const maxActiveWaivers = Number.isFinite(config.maxActiveWaivers) ? config.maxActiveWaivers : null;
  const maxWaiverDays = Number.isFinite(config.maxWaiverDays) ? config.maxWaiverDays : 90;
  if (maxActiveWaivers !== null && activeWaivers.length > maxActiveWaivers) {
    findings.push(
      finding(
        root,
        configPath,
        1,
        "WAIVER-1.7",
        `active waiver count ${activeWaivers.length} exceeds budget ${maxActiveWaivers}`,
        null,
      ),
    );
  }
  for (const waiver of config.waivers ?? []) {
    const ruleId = String(waiver.ruleId ?? "").toUpperCase();
    const missing = [
      "ruleId",
      "waiverId",
      "owner",
      "issue",
      "reason",
      "expires",
      "remediation",
    ].filter((field) => !String(waiver[field] ?? "").trim());
    if (!Array.isArray(waiver.scope) || waiver.scope.length === 0) missing.push("scope");
    if (waiver.ciAllowed !== true && waiver.ciAllowed !== false) missing.push("ciAllowed");
    if (missing.length > 0) {
      findings.push(
        finding(
          root,
          configPath,
          1,
          "WAIVER-1.1",
          `${waiver.waiverId ?? ruleId} waiver is missing: ${missing.join(", ")}`,
          null,
        ),
      );
    }
    if ((waiver.scope ?? []).some((scope) => isBroadWaiverScope(scope))) {
      findings.push(
        finding(
          root,
          configPath,
          1,
          "WAIVER-1.2",
          `${waiver.waiverId} uses a broad waiver scope`,
          null,
        ),
      );
    }
    const expires = parseUtcDate(waiver.expires);
    if (!expires || expires < today) {
      findings.push(
        finding(root, configPath, 1, "WAIVER-1.3", `${waiver.waiverId} is expired or has an invalid expiry`, null),
      );
    } else if (daysBetweenUtc(today, expires) > maxWaiverDays) {
      findings.push(
        finding(
          root,
          configPath,
          1,
          "WAIVER-1.8",
          `${waiver.waiverId} expiry exceeds max waiver window of ${maxWaiverDays} days`,
          null,
        ),
      );
    }
    const rule = registryPolicyMap.get(ruleId);
    const capabilities = rulePolicyCapabilities(rule ?? { severity: "error" });
    if (rule && capabilities.lockLevel === "immutable" && !capabilities.waivable) {
      findings.push(
        finding(
          root,
          configPath,
          1,
          "WAIVER-1.4",
          `${waiver.waiverId} attempts to waive immutable ${ruleId}`,
          null,
        ),
      );
    }
    if (process.env.CI && waiver.ciAllowed !== true) {
      findings.push(
        finding(
          root,
          configPath,
          1,
          "WAIVER-1.5",
          `${waiver.waiverId} is not CI-allowed`,
          null,
        ),
      );
    }
    if (waiver.visible === false) {
      findings.push(
        finding(root, configPath, 1, "WAIVER-1.6", `${waiver.waiverId} is hidden from output`, null),
      );
    }
    if (/^(?:ai|codex|agent|llm)$/iu.test(String(waiver.owner ?? "").trim())) {
      findings.push(
        finding(root, configPath, 1, "WAIVER-1.9", `${waiver.waiverId} owner must be an accountable human or team`, null),
      );
    }
    if (!String(waiver.remediation ?? "").trim()) {
      findings.push(
        finding(root, configPath, 1, "WAIVER-1.10", `${waiver.waiverId} lacks a remediation plan`, null),
      );
    }
  }
  return findings;
}
