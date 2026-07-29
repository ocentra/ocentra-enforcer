import fs from "node:fs";
import path from "node:path";
import { finding, compactProcessOutput, spawnInRoot } from "../scripts/check-source-core-helpers.mjs";

/** Evaluate cargo-audit results against the repository advisory policy. */
export function collectCargoDependencyPolicyFindings(root, cargoLockPath) {
  const findings = [];
  const cargoAuditArgs = ["audit", "--deny", "warnings"];
  for (const advisoryId of cargoAuditIgnoredAdvisories(root)) cargoAuditArgs.push("--ignore", advisoryId);
  const cargoAudit = spawnInRoot(root, "cargo", cargoAuditArgs);
  if (cargoAudit.error?.code === "ENOENT") {
    findings.push(finding(root, cargoLockPath, 1, "DEP-1.1", "cargo audit is not installed", "Install cargo-audit or disable this check in project policy."));
  } else if (cargoAudit.status !== 0) {
    findings.push(finding(root, cargoLockPath, 1, "DEP-1.1", "cargo audit reported advisories", compactProcessOutput(cargoAudit)));
  }
  return findings;
}

/** Read cargo-audit advisory IDs ignored by the canonical deny policy. */
export function cargoAuditIgnoredAdvisories(root) {
  const denyPath = path.join(root, "deny.toml");
  if (!fs.existsSync(denyPath)) return [];
  const advisories = tomlSection(fs.readFileSync(denyPath, "utf8"), "advisories");
  const ignore = advisories.match(/^\s*ignore\s*=\s*\[(?<items>[\s\S]*?)\]/mu)?.groups?.items ?? "";
  return [...ignore.matchAll(/"(?<id>RUSTSEC-\d{4}-\d{4})"/gu)].map((match) => match.groups.id);
}

function tomlSection(text, sectionName) {
  const lines = text.split(/\r?\n/u);
  const sectionHeader = `[${sectionName}]`;
  const start = lines.findIndex((line) => line.trim() === sectionHeader);
  if (start === -1) return "";
  const body = [];
  for (const line of lines.slice(start + 1)) {
    if (/^\s*\[[^\]]+\]\s*$/u.test(line)) break;
    body.push(line);
  }
  return body.join("\n");
}
