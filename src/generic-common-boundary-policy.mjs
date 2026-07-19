const BOUNDARY_SCOPE_FIELDS = [
  "rawTypeBoundaryGlobs",
  "facadeFileGlobs",
  "rawStringOwnerGlobs",
  "domainPrimitiveOwnerGlobs",
  "serializedDomainOwnerGlobs",
  "runtimeStringOwnerGlobs",
];

const GENERIC_SCOPE_SEGMENTS = new Set([
  "apps",
  "crates",
  "packages",
  "src",
  "source",
]);

function isGlobOnlySegment(segment) {
  return segment === "*" || segment === "**" || /^\*\.[A-Za-z0-9]+$/u.test(segment);
}

function hasNamedOwnerSegment(pattern) {
  const normalized = String(pattern ?? "")
    .trim()
    .replaceAll("\\", "/")
    .replace(/^\.\//u, "")
    .replace(/\/+$/u, "");
  if (!normalized) return false;
  return normalized.split("/").some((segment) =>
    !GENERIC_SCOPE_SEGMENTS.has(segment) && !isGlobOnlySegment(segment));
}

function parseBoundaryConfig(rel, text) {
  if (!rel.toLowerCase().endsWith(".json")) return null;
  if (!BOUNDARY_SCOPE_FIELDS.some((field) => text.includes(`"${field}"`))) return null;
  try {
    const parsed = JSON.parse(text);
    return parsed && typeof parsed === "object" && !Array.isArray(parsed) ? parsed : null;
  } catch {
    return null;
  }
}

/** Returns the first structurally broad boundary scope, if one exists. */
export function broadBoundaryScope(config) {
  for (const field of BOUNDARY_SCOPE_FIELDS) {
    const patterns = config[field];
    if (!Array.isArray(patterns)) continue;
    for (const pattern of patterns) {
      if (typeof pattern === "string" && !hasNamedOwnerSegment(pattern)) {
        return { field, pattern };
      }
    }
  }
  return null;
}

/** Validates boundary ownership configuration and records policy violations. */
export function scanBoundaryPolicyConfiguration(
  violations,
  root,
  filePath,
  rel,
  text,
  record,
) {
  const config = parseBoundaryConfig(rel, text);
  if (!config) return;
  const broadScope = broadBoundaryScope(config);
  if (!broadScope) return;
  record(
    violations,
    root,
    filePath,
    1,
    "BOUND-1.7",
    `${broadScope.field} uses a catch-all boundary scope without a named owner segment.`,
    broadScope.pattern,
  );
}
