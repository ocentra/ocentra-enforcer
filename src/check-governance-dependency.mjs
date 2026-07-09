import fs from "node:fs";
import path from "node:path";
import { DEFAULT_ALLOWED_LICENSES } from "./check-metadata.mjs";
import { finding, compactProcessOutput, spawnInRoot } from "../scripts/check-source-core-helpers.mjs";

export function collectDependencyPolicyFindings(root, config) {
  const findings = [];
  const packageLockPath = path.join(root, "package-lock.json");
  if (fs.existsSync(packageLockPath)) {
    findings.push(...collectNpmDependencyPolicyFindings(root, packageLockPath, config));
  }
  const cargoLockPath = path.join(root, "Cargo.lock");
  if (fs.existsSync(cargoLockPath)) {
    findings.push(...collectCargoDependencyPolicyFindings(root, cargoLockPath));
  }
  return findings;
}

function collectNpmDependencyPolicyFindings(root, packageLockPath, config) {
  const findings = [];
  const audit = spawnInRoot(root, "npm", ["audit", "--audit-level=high", "--json"]);
  if (audit.status !== 0) {
    findings.push(
      finding(root, packageLockPath, 1, "NPM-1.9", "npm audit reported high-or-higher vulnerabilities", compactProcessOutput(audit)),
      finding(root, packageLockPath, 1, "DEP-1.1", "npm audit reported high-or-higher vulnerabilities", compactProcessOutput(audit)),
    );
  }
  const lock = JSON.parse(fs.readFileSync(packageLockPath, "utf8"));
  const allowed = new Set(config.allowedExternalLicenses ?? [...DEFAULT_ALLOWED_LICENSES]);
  for (const [lockPath, packageEntry] of Object.entries(lock.packages ?? {})) {
    if (!lockPath.includes("node_modules")) continue;
    const packageName = lockPath.split("node_modules/").at(-1);
    if (packageName?.startsWith("@ocentra-parent/") || packageName?.startsWith("@ocentra/")) continue;
    const license = packageEntry.license;
    if (typeof license !== "string" || !allowed.has(license)) {
      findings.push(
        finding(root, packageLockPath, 1, "NPM-1.10", `${lockPath}: ${license ?? "MISSING"}`, null),
        finding(root, packageLockPath, 1, "DEP-1.2", `${lockPath}: ${license ?? "MISSING"}`, null),
      );
    }
  }
  return findings;
}

function collectCargoDependencyPolicyFindings(root, cargoLockPath) {
  const findings = [];
  const cargoAuditArgs = ["audit", "--deny", "warnings"];
  for (const advisoryId of cargoAuditIgnoredAdvisories(root)) {
    cargoAuditArgs.push("--ignore", advisoryId);
  }
  const cargoAudit = spawnInRoot(root, "cargo", cargoAuditArgs);
  if (cargoAudit.error?.code === "ENOENT") {
    findings.push(
      finding(root, cargoLockPath, 1, "DEP-1.1", "cargo audit is not installed", "Install cargo-audit or disable this check in project policy."),
    );
  } else if (cargoAudit.status !== 0) {
    findings.push(
      finding(root, cargoLockPath, 1, "DEP-1.1", "cargo audit reported advisories", compactProcessOutput(cargoAudit)),
    );
  }
  return findings;
}

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
