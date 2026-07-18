const DATA_FETCH_PRIMITIVE_RE = /\b(useQuery|useMutation|useSWR|useInfiniteQuery|createQuery)\s*\(/;
const ERROR_NAME_RE = /Error$/;
const PRESENTATION_EXT_RE = /\.(tsx?|jsx?|vue)$/i;
const TEST_FILE_RE = /\.(test|spec)\.[jt]sx?$/i;
const HOOK_FILE_RE = /(^|\/)hooks\/|(^|\/)use[A-Z][^/]*\.(ts|tsx|js|jsx)$/;

/** Determines whether a relative path belongs to a presentation surface. */
export function isPresentationFile(relPath, presentationDirSegments) {
  if (!PRESENTATION_EXT_RE.test(relPath)) return false;
  if (TEST_FILE_RE.test(relPath)) return false;
  if (HOOK_FILE_RE.test(relPath)) return false;
  const segments = relPath.split("/");
  return presentationDirSegments.some((segment) => segments.includes(segment));
}

function extractImports(text) {
  const imports = [];
  const namedRe = /import\s+(?:type\s+)?\{([^}]+)\}\s+from\s+["']([^"']+)["']/g;
  const defaultRe = /import\s+(?:type\s+)?(\w+)\s+from\s+["']([^"']+)["']/g;
  let match;
  while ((match = namedRe.exec(text))) {
    const names = match[1].split(",").map((name) => name.trim().split(/\s+as\s+/)[0].trim()).filter(Boolean);
    imports.push({ names, source: match[2] });
  }
  while ((match = defaultRe.exec(text))) imports.push({ names: [match[1]], source: match[2] });
  return imports;
}

function classifyBinding(name, text) {
  if (!new RegExp(`\\b${name}\\.\\w+\\s*\\(`).test(text)) return "none";
  if (ERROR_NAME_RE.test(name)) {
    const nonInstanceofCallRe = new RegExp(`(?<!instanceof\\s+)\\b${name}\\.\\w+\\s*\\(`);
    if (!nonInstanceofCallRe.test(text)) return "info";
  }
  return "hard";
}

/** Scans a presentation file for prohibited business-logic coupling. */
export function scanUiPresentationFile(relPath, text, businessLogicPatterns, eventSourcePatterns) {
  const findings = [];
  const hasDataFetchPrimitive = DATA_FETCH_PRIMITIVE_RE.test(text);
  for (const imported of extractImports(text)) {
    if (businessLogicPatterns.some((pattern) => pattern.test(imported.source))) {
      const bindingFindings = imported.names.map((name) => ({
        file: relPath,
        kind: "business-logic-import",
        severity: classifyBinding(name, text),
        source: imported.source,
        binding: name,
        hasDataFetchPrimitive,
      })).filter((finding) => finding.severity !== "none");
      findings.push(...bindingFindings);
    }
    if (eventSourcePatterns.some((pattern) => pattern.test(imported.source))) {
      findings.push({ file: relPath, kind: "event-source-import", severity: "hard", source: imported.source, binding: imported.names.join(", ") });
    }
  }
  return findings;
}
