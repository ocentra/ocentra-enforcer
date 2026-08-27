import {
  addViolation,
  DOMAIN_PATH_RE,
  FACADE_PATH_RE,
  GENERATED_PATH_RE,
  countTextMatches,
  firstMatchingLine,
  importsOwnModule,
  isCoordinationVendorToolingPath,
  isEnforcerToolingPath,
  isTestPath,
} from "./generic-scanner-shared.mjs";

function addSourceOwnershipViolation(violations, root, filePath, line, ruleId, detail, source) {
  addViolation(violations, root, filePath, line, ruleId, detail, source);
}

/** Scans public API, generated-code, and higher-level architecture rules. */
export function scanDomainApiRules(violations, root, filePath, rel, lines, text, importText) {
  const domainFile = DOMAIN_PATH_RE.test(rel);
  const generatedFile = GENERATED_PATH_RE.test(rel);
  const facadeFile = FACADE_PATH_RE.test(rel);
  const enforcerToolingFile = isEnforcerToolingPath(rel);

  if (domainFile) {
    for (const [ruleId, pattern, detail] of [
      ["ARCH-1.1", /(?:\/infra|\/platform|node:fs|node:child_process|std::fs|std::process)/iu, "domain imports infrastructure dependency."],
      ["ARCH-1.2", /(?:\/ui|\/components|\/views|react|tsx?["'])/iu, "domain imports UI dependency."],
      ["ARCH-1.3", /(?:\/db|\/database|\/repo|prisma|typeorm|sqlx|diesel)/iu, "domain imports database dependency."],
      ["ARCH-1.4", /(?:\/http|\/api|\/server|axios|fetch|reqwest|hyper)/iu, "domain imports HTTP dependency."],
      ["ARCH-1.5", /(?:\/adapter|\/adapters)/iu, "domain imports adapter dependency."],
    ]) {
      if (pattern.test(importText)) {
        addSourceOwnershipViolation(violations, root, filePath, firstMatchingLine(lines, /(?:^\s*import\b|^\s*export\b.*\bfrom\b|^\s*(?:const|let|var)\s+\w+\s*=\s*require\(|^\s*use\s+)/u), ruleId, detail, rel);
      }
    }
  }
  if (generatedFile && /(?:\/domain\/internal|\/internal|private|unstable)/iu.test(importText)) {
    addSourceOwnershipViolation(violations, root, filePath, firstMatchingLine(lines, /(?:^\s*import\b|^\s*export\b.*\bfrom\b|^\s*(?:const|let|var)\s+\w+\s*=\s*require\(|^\s*use\s+)/u), "ARCH-1.6", "generated code depends on domain/internal module.", rel);
  }
  if (!isTestPath(rel) && /(?:\/test-support|\/tests?\/helpers|__tests__|vitest|pytest|unittest)/iu.test(importText)) {
    addSourceOwnershipViolation(violations, root, filePath, firstMatchingLine(lines, /(?:^\s*import\b|^\s*export\b.*\bfrom\b|^\s*(?:const|let|var)\s+\w+\s*=\s*require\(|^\s*use\s+)/u), "ARCH-1.7", "production source imports test support.", rel);
  }
  if (!isCoordinationVendorToolingPath(rel) && /(?:^|\/)(?:main|cli|bin)\.(?:ts|tsx|js|mjs|rs|py)$/iu.test(rel) && /(?:\/domain|\/core|\/infra|\/db)/iu.test(importText) && !/(?:\/app|\/application|\/boundary)/iu.test(importText)) {
    addSourceOwnershipViolation(violations, root, filePath, 1, "ARCH-1.8", "CLI/main imports outside application boundary.", rel);
  }
  if (!enforcerToolingFile && (/(?:circular import|cycle detected|imports itself)/iu.test(importText) || importsOwnModule(rel, importText))) {
    addSourceOwnershipViolation(violations, root, filePath, 1, "ARCH-1.9", "circular import marker or self-import found.", rel);
  }
  const exportCount = countTextMatches(text, /^\s*export\s+(?:class|function|const|let|var|type|interface|enum|default|\{|\*)/gmu);
  if (exportCount > 10 && !/\bPUBLIC-API-BUDGET-JUSTIFICATION:/u.test(text)) {
    addSourceOwnershipViolation(violations, root, filePath, 1, "ARCH-1.11", `public export count ${exportCount} exceeds budget 10.`, rel);
  }
  if (facadeFile && /^\s*export\s+(?:\*|\{[^}]+\}\s+from)/mu.test(text) && !/\b(?:facadeProfile|publicFacadeAllowed|stable-api)\b/u.test(text)) {
    addSourceOwnershipViolation(violations, root, filePath, 1, "ARCH-1.12", "barrel/facade export lacks explicit profile marker.", rel);
  }
  if (facadeFile && /\b(?:internal|unstable|experimental|private)\b/iu.test(text)) {
    addSourceOwnershipViolation(violations, root, filePath, firstMatchingLine(lines, /\b(?:internal|unstable|experimental|private)\b/iu), "ARCH-1.13", "public facade exports unstable/internal API.", rel);
  }
  if (/\bexport\s+(?:type|interface|class|function|const)[\s\S]{0,120}\b(?:Internal|internal|Private|private|Raw[A-Z]\w+)/u.test(text)) {
    addSourceOwnershipViolation(violations, root, filePath, firstMatchingLine(lines, /\bexport\s+(?:type|interface|class|function|const)/u), "ARCH-1.14", "public API leaks internal/raw type.", rel);
  }
}
