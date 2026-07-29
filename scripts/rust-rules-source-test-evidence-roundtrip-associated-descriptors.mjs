import { escapeRegExp, maskRustCode } from "./rust-rules-path-core.mjs";
import { balancedBodyAt } from "./rust-rules-source-test-evidence-ranges-balanced.mjs";
import { DECODE_OPERATION } from "./rust-rules-source-test-evidence-roundtrip-codecs.mjs";
import { roundTripImplDescriptors } from "./rust-rules-source-test-evidence-roundtrip-impl-descriptors.mjs";
import { descriptorFromMatch } from "./rust-rules-source-test-evidence-roundtrip-return-descriptors.mjs";

/** Describes associated constructors only when their signature returns the target type. */
export function roundTripFactoryDescriptors(source, maskedSource = null) {
  const masked = typeof maskedSource === "string" ? maskedSource : maskRustCode(source);
  return roundTripImplDescriptors(masked).flatMap((descriptor) => {
    const target = escapeRegExp(descriptor.targetName);
    const declaration = new RegExp(
      `\\b(?:pub(?:\\([^)]*\\))?\\s+)?(?:const\\s+)?fn\\s+(?<method>[A-Za-z_][A-Za-z0-9_]*)(?:\\s*<[^>{}]*>)?\\s*\\([^)]*\\)\\s*->\\s*(?:Self|${target})\\b`,
      "gu",
    );
    return [...descriptor.body.matchAll(declaration)].map((match) => ({
      targetName: descriptor.targetName,
      method: match.groups?.method ?? "",
    }));
  });
}

/** Describes named decode wrappers only when they return a DTO and call a real codec. */
export function roundTripDecoderDescriptors(source, maskedSource = null) {
  const masked = typeof maskedSource === "string" ? maskedSource : maskRustCode(source);
  const descriptors = [];
  const declaration = /\bfn\s+(?<name>(?:decode|from_(?:json|wire))[A-Za-z0-9_]*)\s*(?:<[^>{}]*>)?\s*\([^)]*\)(?<tail>[^{;]*)\{/gu;
  for (const match of masked.matchAll(declaration)) {
    const openingBrace = (match.index ?? 0) + match[0].lastIndexOf("{");
    const body = balancedBodyAt(masked, openingBrace);
    if (!body || !DECODE_OPERATION.test(body)) continue;
    const descriptor = descriptorFromMatch(match);
    if (descriptor) descriptors.push(descriptor);
  }
  return descriptors;
}

/** Describes free functions whose signatures return a concrete DTO value. */
export function roundTripValueProducerDescriptors(source, maskedSource = null) {
  const masked = typeof maskedSource === "string" ? maskedSource : maskRustCode(source);
  const descriptors = [];
  const declaration = /\bfn\s+(?<name>[A-Za-z_][A-Za-z0-9_]*)\s*(?:<[^>{}]*>)?\s*\([^)]*\)(?<tail>[^{;]*)\{/gu;
  for (const match of masked.matchAll(declaration)) {
    const descriptor = descriptorFromMatch(match);
    if (descriptor) descriptors.push(descriptor);
  }
  return descriptors;
}
