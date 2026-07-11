import fs from "node:fs";
import path from "node:path";
import process from "node:process";

import { normalizeWaiverToday } from "./packaged-waiver-date.mjs";
import {
  normalizeExactWaiverPath,
  validatePackagedWaiver,
} from "./packaged-waiver-record.mjs";

export function loadPackagedWaiverRegistry(registryPath, registryRules, options = {}) {
  let document;
  try {
    document = JSON.parse(fs.readFileSync(registryPath, "utf8"));
  } catch (error) {
    throw new Error(`Cannot load packaged waiver registry ${registryPath}: ${error.message}`);
  }
  if (!document || typeof document !== "object" || !Array.isArray(document.waivers)) {
    throw new Error(`Packaged waiver registry ${registryPath} must contain a waivers array.`);
  }

  const rulesById = new Map(registryRules.map((rule) => [String(rule.id).toUpperCase(), rule]));
  const today = normalizeWaiverToday(options.today);
  const seen = new Set();
  return document.waivers.map((raw, index) => {
    const waiver = validatePackagedWaiver(raw, index, rulesById, today);
    const key = `${waiver.path}:${waiver.ruleId}`;
    if (seen.has(key)) {
      throw new Error(`Packaged waiver registry has duplicate exact scope ${key}.`);
    }
    seen.add(key);
    return waiver;
  });
}

export function upsertPackagedWaiverRegistry(registryPath, registryRules, candidate, options = {}) {
  const today = normalizeWaiverToday(options.today);
  const existing = fs.existsSync(registryPath)
    ? loadPackagedWaiverRegistry(registryPath, registryRules, { today })
    : [];
  const rulesById = new Map(registryRules.map((rule) => [String(rule.id).toUpperCase(), rule]));
  const waiver = validatePackagedWaiver(candidate, "requested", rulesById, today);
  const retained = existing.filter((item) => item.path !== waiver.path || item.ruleId !== waiver.ruleId);
  const document = { waivers: [...retained, waiver].sort((left, right) => `${left.path}:${left.ruleId}`.localeCompare(`${right.path}:${right.ruleId}`)) };
  const parent = path.dirname(registryPath);
  fs.mkdirSync(parent, { recursive: true });
  const temporary = `${registryPath}.${process.pid}.${Date.now()}.tmp`;
  fs.writeFileSync(temporary, `${JSON.stringify(document, null, 2)}\n`, "utf8");
  fs.renameSync(temporary, registryPath);
  return waiver;
}

export function applyPackagedWaivers(findings, waivers, options = {}) {
  const today = normalizeWaiverToday(options.today);
  const waiverIdPrefix = options.waiverIdPrefix ?? "PACKAGED-WAIVER";
  const waiverSource = options.waiverSource ?? "packaged-registry";
  const active = [];
  const waived = [];
  for (const finding of findings) {
    if (!finding.file) {
      active.push(finding);
      continue;
    }
    let path;
    try {
      path = normalizeExactWaiverPath(finding.file, "finding file");
    } catch {
      // Aggregate scanner findings cannot be matched by an exact-file waiver.
      active.push(finding);
      continue;
    }
    const ruleId = String(finding.ruleId ?? "").toUpperCase();
    const waiver = waivers.find((candidate) =>
      candidate.path === path
      && candidate.ruleId === ruleId
      && (!candidate.expires || candidate.expires >= today),
    );
    if (!waiver) {
      active.push(finding);
      continue;
    }
    waived.push({
      ...finding,
      status: "waived",
      waiverId: `${waiverIdPrefix}:${waiver.ruleId}:${waiver.path}`,
      waiverOwner: waiver.owner,
      waiverExpires: waiver.expires,
      waiverReason: waiver.reason,
      waiverSource,
    });
  }
  return { active, waived };
}
