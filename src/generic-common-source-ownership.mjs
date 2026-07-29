import {
  addViolation,
  BOUNDARY_PATH_RE,
  COPIED_BLOCK_RE,
  DOMAIN_PATH_RE,
  FACADE_PATH_RE,
  GENERATED_PATH_RE,
  LAYER_IMPORTS,
  countTextMatches,
  firstDuplicateFunctionName,
  firstMatchingLine,
  hasLargeRepeatedBlock,
  importsOwnModule,
  isCoordinationVendorToolingPath,
  isEnforcerToolingPath,
  isImportLikeLine,
  isTestPath,
} from "./generic-scanner-shared.mjs";
import { scanBoundaryRules } from "./generic-common-boundary-ownership.mjs";
import { domainSignatureLeaksBoundaryType } from "./generic-common-boundary-signatures.mjs";

function addSourceOwnershipViolation(violations, root, filePath, line, ruleId, detail, source) {
  addViolation(violations, root, filePath, line, ruleId, detail, source);
}

function rustImportTargetsDifferentQualifiedModule(rel, importText) {
  if (!rel.endsWith(".rs")) return false;
  const sourceSegments = rel
    .replace(/^(?:.*?\/)?src\//u, "")
    .replace(/(?:\/mod)?\.rs$/u, "")
    .split("/")
    .filter(Boolean);
  const basename = sourceSegments.at(-1);
  if (!basename) return false;

  const imports = importText.matchAll(
    /\buse\s+(?<root>crate|self|super)::(?<path>[A-Za-z_][A-Za-z0-9_:]*)/gu,
  );
  for (const match of imports) {
    const importedSegments = (match.groups?.path ?? "").split("::").filter(Boolean);
    const basenameIndex = importedSegments.lastIndexOf(basename);
    if (basenameIndex < 0) continue;
    const importedModuleSegments = importedSegments.slice(0, basenameIndex + 1);
    let resolved;
    if (match.groups?.root === "crate") {
      resolved = importedModuleSegments;
    } else if (match.groups?.root === "super") {
      resolved = [...sourceSegments.slice(0, -1), ...importedModuleSegments];
    } else {
      resolved = [...sourceSegments, ...importedModuleSegments];
    }
    if (resolved.join("::") !== sourceSegments.join("::")) return true;
  }
  return false;
}

function scanDomainAndArchitectureRules(violations, root, filePath, rel, lines, text, importText) {
  const domainFile = DOMAIN_PATH_RE.test(rel);
  const generatedFile = GENERATED_PATH_RE.test(rel);
  const facadeFile = FACADE_PATH_RE.test(rel);
  const enforcerToolingFile = isEnforcerToolingPath(rel);

  if (domainFile && /(?:\/boundary|\/boundaries|\/transport|\/codec|\/decoder|\/adapter|\/adapters)/iu.test(importText)) {
    addSourceOwnershipViolation(violations, root, filePath, firstMatchingLine(lines, /(?:^\s*import\b|^\s*export\b.*\bfrom\b|^\s*(?:const|let|var)\s+\w+\s*=\s*require\(|^\s*use\s+)/u), "BOUND-1.4", "domain file imports boundary/adapter module.", rel);
  }
  const copiedBlockClaim = [...text.matchAll(new RegExp(COPIED_BLOCK_RE.source, "giu"))]
    .some((match) => !/\b(?:no code is|not|never)[\s\S]{0,80}$/iu.test(text.slice(Math.max(0, (match.index ?? 0) - 80), match.index)));
  if (!enforcerToolingFile && (copiedBlockClaim || hasLargeRepeatedBlock(lines))) {
    addSourceOwnershipViolation(violations, root, filePath, firstMatchingLine(lines, COPIED_BLOCK_RE), "SRC-2.11", "copied or repeated source block found.", rel);
  }
  if (domainFile && !BOUNDARY_PATH_RE.test(rel) && domainSignatureLeaksBoundaryType(text)) {
    addSourceOwnershipViolation(violations, root, filePath, firstMatchingLine(lines, /(?:Dto|DTO|Payload|Raw|Request)/u), "BOUND-1.9", "boundary DTO leaks into public/domain signature.", rel);
  }
  const duplicateFunction = firstDuplicateFunctionName(lines);
  if (duplicateFunction) {
    addSourceOwnershipViolation(violations, root, filePath, duplicateFunction.line, "SRC-2.12", `duplicate function name ${duplicateFunction.name} found.`, duplicateFunction.source);
  }

  const importedLayers = new Set();
  for (const line of lines) {
    for (const [layer, pattern] of LAYER_IMPORTS) {
      if (pattern.test(line)) importedLayers.add(layer);
    }
  }
  if (importedLayers.size >= 3) {
    addSourceOwnershipViolation(violations, root, filePath, 1, "SRC-2.13", `mixed responsibility imports found: ${[...importedLayers].sort().join(", ")}`, rel);
  }
  if (/(?:^|\/)internal(?:\/|$)/iu.test(rel) && /\b(?:export\s+|pub\s+)/u.test(text)) {
    addSourceOwnershipViolation(violations, root, filePath, firstMatchingLine(lines, /\b(?:export\s+|pub\s+)/u), "SRC-2.14", "internal module exposes public API.", rel);
  }
  if (/(?:^|\/)(?:domain|core|model|models)(?:\/|$)/iu.test(rel) && /from\s+["'][^"']*(?:\/apps?|\/ui|\/components|\/adapters?|\/infra|\/platform)[^"']*["']|use\s+crate::(?:app|ui|adapter|infra|platform)::/iu.test(text)) {
    addSourceOwnershipViolation(violations, root, filePath, firstMatchingLine(lines, /(?:\/apps?|\/ui|\/components|\/adapters?|\/infra|\/platform|crate::(?:app|ui|adapter|infra|platform)::)/iu), "SRC-2.15", "domain/core module imports higher-level app, UI, adapter, or infra dependency.", rel);
  }
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

/** Scans source ownership declarations for policy violations. */
export function scanSourceOwnershipPolicy(root, filePath, rel, lines) {
  const violations = [];
  const text = lines.join("\n");
  const importText = lines.filter(isImportLikeLine).join("\n");
  scanBoundaryRules(violations, root, filePath, rel, lines, text);
  scanDomainAndArchitectureRules(violations, root, filePath, rel, lines, text, importText);
  return violations;
}
