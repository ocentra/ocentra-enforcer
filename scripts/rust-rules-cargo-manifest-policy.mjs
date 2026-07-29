import path from "node:path";
import { normalizeRel, toPosix } from "./rust-rules-path-core.mjs";

/** Read one string-valued setting from the deny.toml advisories section. */
export function advisoryPolicyValue(denyText, key) {
  let section = "";
  for (const rawLine of denyText.split(/\r?\n/u)) {
    const line = rawLine.replace(/\s+#.*$/u, "").trim();
    if (line.length === 0) continue;
    const sectionMatch = /^\[([^\]]+)\]$/u.exec(line);
    if (sectionMatch) {
      section = sectionMatch[1];
      continue;
    }
    if (section !== "advisories") continue;
    const settingMatch = /^([A-Za-z0-9_-]+)\s*=\s*"([^"]+)"\s*$/u.exec(line);
    if (settingMatch?.[1] === key) return settingMatch[2];
  }
  return null;
}

/** Decide whether a build.rs path is globally or exactly approved by policy. */
export function isAllowedBuildScript(root, buildRs, config) {
  if (config.allowBuildRs) return true;
  const relativeBuildRs = normalizeRel(root, buildRs);
  return (config.allowedBuildRsPaths ?? []).some((candidate) => {
    const invalidCandidate =
      typeof candidate !== "string" ||
      candidate.length === 0 ||
      candidate.includes("\\") ||
      path.isAbsolute(candidate);
    if (invalidCandidate) return false;
    const normalizedCandidate = toPosix(candidate);
    const invalidSegment = normalizedCandidate
      .split("/")
      .some((segment) => segment === "" || segment === "." || segment === "..");
    if (normalizedCandidate.startsWith("/") || invalidSegment) return false;
    return normalizedCandidate === relativeBuildRs;
  });
}
