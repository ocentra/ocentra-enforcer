import fs from "node:fs";
import path from "node:path";

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

/** Returns cargo-audit advisory IDs governed by deny.toml. */
export function cargoAuditIgnoredAdvisories(root) {
  const denyPath = path.join(root, "deny.toml");
  if (!fs.existsSync(denyPath)) return [];
  const advisories = tomlSection(
    fs.readFileSync(denyPath, "utf8"),
    "advisories",
  );
  const ignore =
    advisories.match(/^\s*ignore\s*=\s*\[(?<items>[\s\S]*?)\]/mu)?.groups
      ?.items ?? "";
  return [...ignore.matchAll(/"(?<id>RUSTSEC-\d{4}-\d{4})"/gu)].map(
    (match) => match.groups.id,
  );
}
