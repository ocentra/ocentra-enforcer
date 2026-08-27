import {
  addViolation,
  COPIED_BLOCK_RE,
  DOMAIN_PATH_RE,
  LAYER_IMPORTS,
  firstDuplicateFunctionName,
  firstMatchingLine,
  hasLargeRepeatedBlock,
  isEnforcerToolingPath,
} from "./generic-scanner-shared.mjs";

function addSourceOwnershipViolation(violations, root, filePath, line, ruleId, detail, source) {
  addViolation(violations, root, filePath, line, ruleId, detail, source);
}

/** Scans structural domain ownership and import-layer rules for one file. */
export function scanDomainStructureRules(violations, root, filePath, rel, lines, text, importText) {
  const domainFile = DOMAIN_PATH_RE.test(rel);

  if (domainFile && /(?:\/boundary|\/boundaries|\/transport|\/codec|\/decoder|\/adapter|\/adapters)/iu.test(importText)) {
    addSourceOwnershipViolation(violations, root, filePath, firstMatchingLine(lines, /(?:^\s*import\b|^\s*export\b.*\bfrom\b|^\s*(?:const|let|var)\s+\w+\s*=\s*require\(|^\s*use\s+)/u), "BOUND-1.4", "domain file imports boundary/adapter module.", rel);
  }
  if (!isEnforcerToolingPath(rel) && (COPIED_BLOCK_RE.test(text) || hasLargeRepeatedBlock(lines))) {
    addSourceOwnershipViolation(violations, root, filePath, firstMatchingLine(lines, COPIED_BLOCK_RE), "SRC-2.11", "copied or repeated source block found.", rel);
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
}
