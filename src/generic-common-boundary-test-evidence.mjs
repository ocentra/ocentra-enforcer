import fs from "node:fs";
import path from "node:path";

const crateTestTextCache = new Map();
const NEGATIVE_EVIDENCE_RE = /(?:^|[^A-Za-z0-9])(?:invalid|malformed|corrupt|truncated|negative|reject(?:ed|ion|s)?|fails?_closed|unsupported)(?:$|[^A-Za-z0-9])/imu;

function rustTestText(root, rel) {
  const crateMatch = rel.match(/^(?<crate>.+?)\/src\//u);
  if (!crateMatch?.groups?.crate) return "";
  const testRoot = path.join(root, crateMatch.groups.crate, "tests");
  const cacheKey = path.resolve(testRoot);
  if (crateTestTextCache.has(cacheKey)) return crateTestTextCache.get(cacheKey);
  const chunks = [];
  const visit = (directory) => {
    if (!fs.existsSync(directory)) return;
    for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
      const absolute = path.join(directory, entry.name);
      if (entry.isDirectory()) visit(absolute);
      else if (entry.isFile() && entry.name.endsWith(".rs")) chunks.push(fs.readFileSync(absolute, "utf8"));
    }
  };
  visit(testRoot);
  const testText = chunks.join("\n");
  crateTestTextCache.set(cacheKey, testText);
  return testText;
}

/** Find negative test evidence that names this boundary's raw type or source subject. */
export function hasNegativeBoundaryEvidence(root, rel, text, rawTypes) {
  if (NEGATIVE_EVIDENCE_RE.test(text)) return true;
  const tests = rustTestText(root, rel);
  if (!NEGATIVE_EVIDENCE_RE.test(tests)) return false;
  const stem = (rel.split("/").at(-1) ?? "").replace(/\.rs$/u, "");
  const subjectTerms = [...rawTypes, ...stem.split("_").filter((term) => term.length >= 4)];
  return subjectTerms.some((term) => new RegExp(`(?:^|[^A-Za-z0-9])${term.replace(/[.*+?^${}()|[\]\\]/gu, "\\$&")}(?:$|[^A-Za-z0-9])`, "imu").test(tests));
}
