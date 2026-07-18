import { escapeRegExp, maskRustCode } from "./rust-rules-path-core.mjs";
import { balancedBodyAt } from "./rust-rules-source-test-evidence-ranges-balanced.mjs";
import { DECODE_OPERATION } from "./rust-rules-source-test-evidence-roundtrip-codecs.mjs";

function inherentImplDescriptors(masked) {
  const descriptors = [];
  const declaration = /\bimpl(?:\s*<[^>{}]*>)?\s+(?<target>[A-Z][A-Za-z0-9_]*)(?:\s*<[^>{}]*>)?\s*\{/gu;
  for (const match of masked.matchAll(declaration)) {
    const targetName = match.groups?.target ?? "";
    const openingBrace = (match.index ?? 0) + match[0].lastIndexOf("{");
    const body = balancedBodyAt(masked, openingBrace);
    if (body) descriptors.push({ targetName, body });
  }
  return descriptors;
}

function defaultImplDescriptors(masked) {
  const descriptors = [];
  const declaration = /\bimpl(?:\s*<[^>{}]*>)?\s+Default\s+for\s+(?<target>[A-Z][A-Za-z0-9_]*)(?:\s*<[^>{}]*>)?\s*\{/gu;
  for (const match of masked.matchAll(declaration)) {
    const openingBrace = (match.index ?? 0) + match[0].lastIndexOf("{");
    const body = balancedBodyAt(masked, openingBrace);
    if (body) descriptors.push({ targetName: match.groups?.target ?? "", body });
  }
  return descriptors;
}

/** Describes associated constructors only when their signature returns the target type. */
export function roundTripFactoryDescriptors(source) {
  const masked = maskRustCode(source);
  return [...inherentImplDescriptors(masked), ...defaultImplDescriptors(masked)].flatMap((descriptor) => {
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
export function roundTripDecoderDescriptors(source) {
  const masked = maskRustCode(source);
  const descriptors = [];
  const declaration = /\bfn\s+(?<name>(?:decode|from_(?:json|wire))[A-Za-z0-9_]*)\s*(?:<[^>{}]*>)?\s*\([^)]*\)(?<tail>[^{;]*)\{/gu;
  for (const match of masked.matchAll(declaration)) {
    const openingBrace = (match.index ?? 0) + match[0].lastIndexOf("{");
    const body = balancedBodyAt(masked, openingBrace);
    if (!body || !DECODE_OPERATION.test(body)) continue;
    const returned = /->\s*(?:(?:[A-Za-z_][A-Za-z0-9_]*::)*(?:Result|Option)\s*<\s*)?(?:[A-Za-z_][A-Za-z0-9_]*::)*(?<target>[A-Z][A-Za-z0-9_]*(?:Dto|DTO|Request|Response|Envelope))\b/u.exec(match.groups?.tail ?? "");
    if (returned?.groups?.target) {
      descriptors.push({ name: match.groups?.name ?? "", targetName: returned.groups.target });
    }
  }
  return descriptors;
}

/** Describes free functions whose signatures return a concrete DTO value. */
export function roundTripValueProducerDescriptors(source) {
  const masked = maskRustCode(source);
  const descriptors = [];
  const declaration = /\bfn\s+(?<name>[A-Za-z_][A-Za-z0-9_]*)\s*(?:<[^>{}]*>)?\s*\([^)]*\)(?<tail>[^{;]*)\{/gu;
  for (const match of masked.matchAll(declaration)) {
    const returned = /->\s*(?:(?:[A-Za-z_][A-Za-z0-9_]*::)*(?:Result|Option)\s*<\s*)?(?:[A-Za-z_][A-Za-z0-9_]*::)*(?<target>[A-Z][A-Za-z0-9_]*(?:Dto|DTO|Request|Response|Envelope))\b/u.exec(match.groups?.tail ?? "");
    if (returned?.groups?.target) {
      descriptors.push({ name: match.groups?.name ?? "", targetName: returned.groups.target });
    }
  }
  return descriptors;
}
