const ERROR_NAME_RE = /Error$/;

export function extractImports(text) {
  const imports = [];
  const namedRe = /import\s+(?:type\s+)?\{([^}]+)\}\s+from\s+["']([^"']+)["']/g;
  const defaultRe = /import\s+(?:type\s+)?(\w+)\s+from\s+["']([^"']+)["']/g;
  let m;
  while ((m = namedRe.exec(text))) {
    const names = m[1].split(",").map((s) => s.trim().split(/\s+as\s+/)[0].trim()).filter(Boolean);
    imports.push({ names, source: m[2] });
  }
  while ((m = defaultRe.exec(text))) {
    imports.push({ names: [m[1]], source: m[2] });
  }
  return imports;
}

export function classifyBinding(name, text) {
  const callRe = new RegExp(`\\b${name}\\.\\w+\\s*\\(`);
  const hasCall = callRe.test(text);
  if (!hasCall) return "none";
  if (ERROR_NAME_RE.test(name)) {
    const nonInstanceofCallRe = new RegExp(`(?<!instanceof\\s+)\\b${name}\\.\\w+\\s*\\(`);
    if (!nonInstanceofCallRe.test(text)) return "info";
  }
  return "hard";
}
