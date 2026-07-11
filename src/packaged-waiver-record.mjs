import { rulePolicyCapabilities } from "./policy.mjs";
import { normalizeWaiverDate } from "./packaged-waiver-date.mjs";

export function validatePackagedWaiver(raw, index, rulesById, today) {
  if (!raw || typeof raw !== "object" || Array.isArray(raw)) {
    throw new Error(`Packaged waiver ${index} must be an object.`);
  }
  rejectUnsupportedFields(raw, index);
  const waiver = normalizePackagedWaiver(raw, index);
  rejectUnknownOrNonWaivableRule(waiver, index, rulesById);
  rejectExpiredWaiver(waiver, index, today);
  return waiver;
}

export function normalizeExactWaiverPath(value, label) {
  const path = String(value ?? "").trim().replaceAll("\\", "/").replace(/^\.\/+/u, "");
  if (isInvalidWaiverPath(path)) {
    throw new Error(`${label} must be one narrow repository-relative file path.`);
  }
  return path;
}

function rejectUnsupportedFields(raw, index) {
  const allowed = new Set(["path", "ruleId", "owner", "reason", "expires"]);
  for (const key of Object.keys(raw)) {
    if (!allowed.has(key)) {
      throw new Error(`Packaged waiver ${index} contains unsupported field ${key}.`);
    }
  }
}

function normalizePackagedWaiver(raw, index) {
  const owner = String(raw.owner ?? "").trim();
  const reason = String(raw.reason ?? "").trim();
  if (!owner) throw new Error(`Packaged waiver ${index} has an empty owner.`);
  if (!reason) throw new Error(`Packaged waiver ${index} has an empty reason.`);
  return {
    path: normalizeExactWaiverPath(raw.path, `waiver ${index} path`),
    ruleId: String(raw.ruleId ?? "").trim().toUpperCase(),
    owner,
    reason,
    expires: raw.expires == null ? null : normalizeWaiverDate(raw.expires, `waiver ${index} expiry`),
  };
}

function rejectUnknownOrNonWaivableRule(waiver, index, rulesById) {
  const rule = rulesById.get(waiver.ruleId);
  if (!waiver.ruleId || !rule) {
    throw new Error(`Packaged waiver ${index} references unknown rule ${waiver.ruleId || "<empty>"}.`);
  }
  if (!rulePolicyCapabilities(rule).waivable) {
    throw new Error(`Packaged waiver ${index} references non-waivable rule ${waiver.ruleId}.`);
  }
}

function rejectExpiredWaiver(waiver, index, today) {
  if (waiver.expires && waiver.expires < today) {
    throw new Error(`Packaged waiver ${index} expired on ${waiver.expires}.`);
  }
}

function isInvalidWaiverPath(path) {
  return !path
    || path.startsWith("/")
    || path.includes(":")
    || path.includes("*")
    || path.split("/").some((segment) => !segment || segment === "." || segment === "..");
}
