export function dependencySections(manifest) {
  return [
    ["dependencies", manifest.dependencies],
    ["devDependencies", manifest.devDependencies],
    ["optionalDependencies", manifest.optionalDependencies],
    ["peerDependencies", manifest.peerDependencies],
  ].filter(([, value]) => value && typeof value === "object");
}

export function isDeterministicDependencyVersion(value) {
  const version = String(value ?? "").trim();
  return (
    /^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$/u.test(version) ||
    /^npm:[@A-Za-z0-9._/-]+@\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$/u.test(version)
  );
}

export function isSuspiciousDependencyName(name) {
  const normalized = String(name ?? "").toLowerCase();
  return /(?:ocentra|openai|effect|typescript|eslint|vitest|playwright|duckdb)[_-](?:js|lib|safe|new|next)$/u.test(normalized);
}

export function isBoundedNodeEngine(value) {
  const engine = String(value ?? "").trim();
  return (
    /^>=\d+(?:\.\d+)?(?:\.\d+)?\s+<\d+(?:\.\d+)?(?:\.\d+)?$/u.test(engine) ||
    /^\d+\.\d+\.\d+$/u.test(engine)
  );
}
