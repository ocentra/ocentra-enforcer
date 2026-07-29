import { isGeneratedArtifactPath } from "./check-source-core-helpers.mjs";

const NAKED_DOMAIN_STRING_RULE_IDS = new Set([
  "RR-6.1",
  "RR-6.5",
  "RR-18.16",
  "TS-1.3",
  "PY-1.3",
]);
const GENERATED_MIRROR_PATTERN =
  /(?:^|[\\/])generated-[^\\/]+\.(?:ts|tsx|js|jsx|mjs|cjs)$/u;

function isNakedDomainStringFinding(entry) {
  if (!NAKED_DOMAIN_STRING_RULE_IDS.has(entry.ruleId)) return false;
  const file = String(entry.file ?? "");
  return !isGeneratedArtifactPath(file) && !isGeneratedMirrorFile(file);
}

function isGeneratedMirrorFile(file) {
  return (
    GENERATED_MIRROR_PATTERN.test(file) ||
    file.includes("/generated/") ||
    file.includes("\\generated\\")
  );
}

export { isNakedDomainStringFinding };
