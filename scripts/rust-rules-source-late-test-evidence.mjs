import fs from "node:fs";
import path from "node:path";
import { addViolation, firstLineMatching, escapeRegExp } from "./rust-rules-path-core.mjs";

const cargoEvidenceCache = new Map();

function crateRootForEvidence(root, filePath) {
  let current = path.dirname(filePath);
  while (current.toLowerCase().startsWith(path.resolve(root).toLowerCase())) {
    if (fs.existsSync(path.join(current, "Cargo.toml"))) return current;
    const parent = path.dirname(current);
    if (parent === current) return null;
    current = parent;
  }
  return null;
}

function collectEvidenceTree(directory, files) {
  if (!fs.existsSync(directory)) return;
  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    const entryPath = path.join(directory, entry.name);
    if (entry.isDirectory()) collectEvidenceTree(entryPath, files);
    else if (entry.isFile() && entry.name.endsWith(".rs")) {
      files.push({ path: entryPath, source: fs.readFileSync(entryPath, "utf8") });
    }
  }
}

export function clearProofEvidenceCache() {
  cargoEvidenceCache.clear();
}

function cargoEvidenceSources(root, filePath, source) {
  const crateRoot = crateRootForEvidence(root, filePath);
  if (!crateRoot) return [{ path: filePath, source }];
  const cacheKey = path.resolve(crateRoot).toLowerCase();
  let cached = cargoEvidenceCache.get(cacheKey);
  if (!cached) {
    const files = [];
    // The production file is supplied directly; only dedicated test/fuzz trees
    // are cross-file evidence, so this stays bounded per crate and is reset
    // at the beginning of every top-level scan.
    collectEvidenceTree(path.join(crateRoot, "tests"), files);
    collectEvidenceTree(path.join(crateRoot, "fuzz"), files);
    cached = { files };
    cargoEvidenceCache.set(cacheKey, cached);
  }
  return [{ path: filePath, source }, ...cached.files.filter((candidate) => candidate.path !== filePath)];
}

function hasTargetEvidence(root, filePath, source, targetName, evidencePattern) {
  const target = new RegExp(`\\b${escapeRegExp(targetName)}\\b`, "u");
  return cargoEvidenceSources(root, filePath, source).some(
    (candidate) => target.test(candidate.source) && evidencePattern.test(candidate.source),
  );
}

function evidenceLineForTarget(originalLines, targetName) {
  return firstLineMatching(originalLines, new RegExp(`^\\s*(?:pub\\s+)?(?:async\\s+)?fn\\s+${escapeRegExp(targetName)}\\s*\\(`, "u"));
}

function parserDefinitions(masked) {
  return [...masked.matchAll(/^\s*(?:pub\s+)?(?:async\s+)?fn\s+(?<name>try_new|parse[A-Za-z0-9_]*)\s*\(/gmu)];
}

function parserBody(masked, startIndex) {
  const tail = masked.slice(startIndex);
  const nextDefinition = /\n\s*(?:pub\s+)?(?:async\s+)?fn\s+[A-Za-z_][A-Za-z0-9_]*\s*\(/u.exec(tail.slice(1));
  return nextDefinition ? tail.slice(0, nextDefinition.index + 1) : tail;
}

function parserTargetRequiresFuzzEvidence(masked, parserTarget) {
  const name = parserTarget.groups?.name ?? "";
  const identifierTokens = name
    .replace(/([a-z0-9])([A-Z])/gu, "$1_$2")
    .toLowerCase()
    .split(/_+/u);
  if (identifierTokens.some((token) => ["binary", "packet", "frame", "network"].includes(token))) {
    return true;
  }

  const body = parserBody(masked, parserTarget.index ?? 0);
  const openingBrace = body.indexOf("{");
  const signature = openingBrace >= 0 ? body.slice(0, openingBrace) : body;
  if (/(?:&\s*)?\[\s*u8\s*\]|\bVec\s*<\s*u8\s*>|\bBytes\b/u.test(signature)) return true;
  return /\b(?:binary|packet|frame|network)\b/iu.test(body);
}

function hasPropertyEvidence(root, filePath, source, targetName) {
  const target = new RegExp(`\\b${escapeRegExp(targetName)}\\b`, "u");
  const crateRoot = crateRootForEvidence(root, filePath);
  const relativeTarget = path.relative(crateRoot ?? path.dirname(path.dirname(filePath)), filePath).replaceAll(path.sep, "/");
  const registered = new RegExp(`[\"']${escapeRegExp(`${relativeTarget}::${targetName}`)}[\"']\\s*=>`, "u");
  return cargoEvidenceSources(root, filePath, source).some((candidate) =>
    /\b(?:proptest|quickcheck)!\s*\{/u.test(candidate.source) && (target.test(candidate.source) || registered.test(candidate.source)));
}

export function applyProofEvidenceRules({ source, masked, originalLines, root, filePath, violations, isBoundary, isTestSource }) {
  if (!isBoundary) {
    for (const definition of parserDefinitions(masked)) {
      const name = definition.groups?.name ?? "parser";
      const lineNo = evidenceLineForTarget(originalLines, name);
      if (!hasTargetEvidence(root, filePath, source, name, /\b(?:invalid|reject|malformed|bad input)\b/iu)) {
        addViolation(violations, root, filePath, lineNo, "RR-12.16", `validated constructor/parser ${name} lacks invalid-input test evidence.`, originalLines[lineNo - 1] ?? null);
      }
      if (name.startsWith("parse") && !hasTargetEvidence(root, filePath, source, name, /\b(?:invalid|empty|oversized|malformed)\b/iu)) {
        addViolation(violations, root, filePath, lineNo, "RR-12.17", `parser ${name} lacks invalid/empty/oversized/malformed test evidence.`, originalLines[lineNo - 1] ?? null);
      }
    }
  }
  if (/\b(?:TryFrom|From)\s*<[^>]*(?:Dto|Request|Response|Envelope)[^>]*>/u.test(source) && !/\b(?:negative|invalid|reject)\b/iu.test(source)) {
    const lineNo = firstLineMatching(originalLines, /\b(?:TryFrom|From)\s*</u);
    addViolation(violations, root, filePath, lineNo, "RR-12.18", "DTO conversion lacks negative test evidence.", originalLines[lineNo - 1] ?? null);
  }
  if (/\b(?:BUGFIX|FIXES|bugfix|fixes)\b/u.test(source) && !/\bREGRESSION-TEST:/u.test(source)) {
    const lineNo = firstLineMatching(originalLines, /\b(?:BUGFIX|FIXES|bugfix|fixes)\b/u);
    addViolation(violations, root, filePath, lineNo, "RR-12.19", "bugfix marker lacks REGRESSION-TEST evidence.", originalLines[lineNo - 1] ?? null);
  }
  if (!isTestSource) {
    for (const propertyTarget of masked.matchAll(/^\s*pub\s+(?:async\s+)?fn\s+(?<name>(?:normalize|parse)[A-Za-z0-9_]*)\s*\(/gmu)) {
      const name = propertyTarget.groups?.name ?? "parser";
      if (!hasPropertyEvidence(root, filePath, source, name)) {
        const lineNo = evidenceLineForTarget(originalLines, name);
        addViolation(violations, root, filePath, lineNo, "RR-12.27", `normalizer/parser ${name} lacks property-test evidence.`, originalLines[lineNo - 1] ?? null);
      }
    }
  }
  for (const parserTarget of masked.matchAll(/^\s*pub\s+(?:async\s+)?fn\s+(?<name>parse[A-Za-z0-9_]*)\s*\(/gmu)) {
    const name = parserTarget.groups?.name ?? "parser";
    if (!parserTargetRequiresFuzzEvidence(masked, parserTarget)) continue;
    if (!hasTargetEvidence(root, filePath, source, name, /(?:\bfuzz(?:_|\b)|\bcargo fuzz\b|\bFUZZ-TARGET:)/iu)) {
      const lineNo = evidenceLineForTarget(originalLines, name);
      addViolation(violations, root, filePath, lineNo, "RR-12.28", `binary/network parser ${name} lacks fuzz target evidence.`, originalLines[lineNo - 1] ?? null);
    }
  }
  const hasAsyncLoop = /\basync\s+fn\b|\.await\b/u.test(masked) && /^\s*loop\s*\{/mu.test(masked);
  if ((/\b(?:tokio::spawn|select!|unbounded_channel)\b/u.test(masked) || hasAsyncLoop) && !/\b(?:shutdown|cancellation|CANCELLATION-TEST:|SHUTDOWN-TEST:)\b/iu.test(source)) {
    const lineNo = firstLineMatching(originalLines, /\b(?:tokio::spawn|select!|unbounded_channel)\b|^\s*loop\s*\{/mu);
    addViolation(violations, root, filePath, lineNo, "RR-12.29", "concurrency code lacks cancellation/shutdown test evidence.", originalLines[lineNo - 1] ?? null);
  }
}
