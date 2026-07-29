import { balancedBodyAt } from "./rust-rules-source-test-evidence-ranges-balanced.mjs";

function descriptorsFor(masked, declaration, targetName) {
  const descriptors = [];
  for (const match of masked.matchAll(declaration)) {
    const openingBrace = (match.index ?? 0) + match[0].lastIndexOf("{");
    const body = balancedBodyAt(masked, openingBrace);
    if (body) descriptors.push({ targetName: targetName(match), body });
  }
  return descriptors;
}

export function roundTripImplDescriptors(masked) {
  const inherent = /\bimpl(?:\s*<[^>{}]*>)?\s+(?<target>[A-Z][A-Za-z0-9_]*)(?:\s*<[^>{}]*>)?\s*\{/gu;
  const defaulted = /\bimpl(?:\s*<[^>{}]*>)?\s+Default\s+for\s+(?<target>[A-Z][A-Za-z0-9_]*)(?:\s*<[^>{}]*>)?\s*\{/gu;
  const name = (match) => match.groups?.target ?? "";
  return [...descriptorsFor(masked, inherent, name), ...descriptorsFor(masked, defaulted, name)];
}
