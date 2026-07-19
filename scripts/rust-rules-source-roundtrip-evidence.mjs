import { escapeRegExp, maskRustCode } from "./rust-rules-path-core.mjs";
import { withProofEvidenceIndex } from "./rust-rules-source-test-evidence-cache.mjs";
import { collectTestFunctions } from "./rust-rules-source-test-structure-helpers.mjs";
import {
  roundTripsTargetDataflow,
  targetProjectionValues,
} from "./rust-rules-source-roundtrip-dataflow.mjs";
import {
  closingBraceForDefinition,
  roundTripTargets,
} from "./rust-rules-source-roundtrip-graph.mjs";

function roundTripHelperNames(maskedSources) {
  const helpers = new Set();
  for (const masked of maskedSources) {
    const definitions = masked.matchAll(
      /\bfn\s+(?<name>[A-Za-z_][A-Za-z0-9_]*)\s*<\s*(?<type>[A-Za-z_][A-Za-z0-9_]*)\b[^;{}>]*>[^;{]*\{/gu,
    );
    for (const definitionMatch of definitions) {
      const openingBrace =
        (definitionMatch.index ?? 0) + definitionMatch[0].lastIndexOf("{");
      const closingBrace = closingBraceForDefinition(masked, openingBrace);
      const definition = masked.slice(
        definitionMatch.index ?? 0,
        closingBrace + 1,
      );
      const typeParameter = definitionMatch.groups?.type ?? "";
      if (!/\bSerialize\b/u.test(definition)) continue;
      if (!/\bDeserialize(?:Owned)?\b/u.test(definition)) continue;
      if (!roundTripsTargetDataflow(definition, typeParameter)) continue;
      helpers.add(definitionMatch.groups?.name ?? "");
    }
  }
  helpers.delete("");
  return helpers;
}

function callsRoundTripHelper(testBody, target, helperNames) {
  const escapedTarget = escapeRegExp(target);
  const targetValues = targetProjectionValues(testBody, target);
  for (const helperName of helperNames) {
    const escapedHelper = escapeRegExp(helperName);
    const invocation = new RegExp(
      `\\b${escapedHelper}\\s*::\\s*<\\s*${escapedTarget}\\s*>\\s*\\(`,
      "u",
    );
    if (invocation.test(testBody)) return true;
    const directProjectionInvocation = new RegExp(
      `\\b${escapedHelper}\\s*\\(\\s*&?\\s*${escapedTarget}\\b`,
      "u",
    );
    if (directProjectionInvocation.test(testBody)) return true;
    for (const value of targetValues) {
      const escapedValue = escapeRegExp(value);
      const inferredInvocation = new RegExp(
        `\\b${escapedHelper}\\s*\\(\\s*&?\\s*${escapedValue}\\b\\s*\\)`,
        "u",
      );
      if (inferredInvocation.test(testBody)) return true;
    }
  }
  return false;
}

function testRoundTripsTarget(testBody, target, helperNames) {
  if (callsRoundTripHelper(testBody, target, helperNames)) return true;
  return roundTripsTargetDataflow(testBody, target);
}

function roundTripEvidenceIndex(candidates) {
  const indexedCandidates = candidates.map((candidate) => ({
    ...candidate,
    masked: maskRustCode(candidate.source),
  }));
  const maskedSources = indexedCandidates.map((candidate) => candidate.masked);
  return {
    candidates: indexedCandidates.map((candidate) => ({
      ...candidate,
      testFunctions: collectTestFunctions(candidate.source, candidate.masked),
    })),
    helperNames: roundTripHelperNames(maskedSources),
    resultByDtoName: new Map(),
    targetsByDtoName: new Map(),
  };
}

/** Checks crate-local executable tests for target-specific round-trip behavior. */
export function hasRoundTripEvidence(root, filePath, source, dtoName) {
  const index = withProofEvidenceIndex(
    root,
    filePath,
    source,
    roundTripEvidenceIndex,
  );
  if (index.resultByDtoName.has(dtoName)) {
    return index.resultByDtoName.get(dtoName);
  }
  let targets = index.targetsByDtoName.get(dtoName);
  if (!targets) {
    targets = roundTripTargets(
      index.candidates.map((candidate) => candidate.masked),
      dtoName,
    );
    index.targetsByDtoName.set(dtoName, targets);
  }
  const result = index.candidates.some((candidate) =>
    candidate.testFunctions.some(
      (testFunction) => {
        const body = candidate.masked.slice(
          testFunction.bodyStart,
          testFunction.bodyEnd,
        );
        return [...targets].some((target) =>
          testRoundTripsTarget(body, target, index.helperNames));
      },
    ));
  index.resultByDtoName.set(dtoName, result);
  return result;
}
