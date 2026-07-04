const LOCKFILE_NAMES = new Set([
  "cargo.lock",
  "package-lock.json",
  "pnpm-lock.yaml",
  "yarn.lock",
  "uv.lock",
  "poetry.lock",
]);

export function protectedSingletonGroup(path) {
  const normalized = normalizeCoordinationPath(path);
  const basename = normalized.split("/").at(-1) ?? normalized;
  return (
    lockfileGroup(basename) ??
    releaseGroup(normalized, basename) ??
    migrationGroup(normalized) ??
    generatedGroup(normalized) ??
    ciGroup(normalized)
  );
}

function lockfileGroup(basename) {
  return LOCKFILE_NAMES.has(basename) ? `lockfile:${basename}` : null;
}

function releaseGroup(normalized, basename) {
  if (/^(changelog|changes|release-notes)(\.md)?$/u.test(basename)) {
    return `release:${basename}`;
  }
  return /^(version|VERSION)$/u.test(normalized)
    ? `release:${basename.toLowerCase()}`
    : null;
}

function migrationGroup(normalized) {
  return normalized.includes("/migrations/") || normalized.startsWith("migrations/")
    ? `migrations:${normalized}`
    : null;
}

function generatedGroup(normalized) {
  const generatedPath = normalized.includes("/generated/") || normalized.startsWith("generated/");
  const generatedContract = normalized.includes("generated") && /schema|contract|dto|bridge/u.test(normalized);
  return generatedPath || generatedContract ? `generated:${normalized}` : null;
}

function ciGroup(normalized) {
  return normalized.startsWith(".github/workflows/") ? `ci:${normalized}` : null;
}

function normalizeCoordinationPath(value) {
  return String(value ?? "")
    .trim()
    .replace(/\\/gu, "/")
    .replace(/\/+/gu, "/")
    .replace(/^\.\//u, "")
    .toLowerCase();
}
