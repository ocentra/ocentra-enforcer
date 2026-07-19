import { escapeRegExp } from "./rust-rules-path-core.mjs";

function balancedClosingBrace(source, openingBrace) {
  let depth = 0;
  for (let index = openingBrace; index < source.length; index += 1) {
    if (source[index] === "{") depth += 1;
    else if (source[index] === "}") {
      depth -= 1;
      if (depth === 0) return index;
    }
  }
  return source.length;
}

function transportParents(maskedSources) {
  const bodies = new Map();
  for (const masked of maskedSources) {
    const declarations = masked.matchAll(
      /^\s*pub\s+struct\s+(?<name>[A-Z][A-Za-z0-9_]*(?:Dto|DTO|Request|Response|Envelope))\b[^;{]*\{/gmu,
    );
    for (const declaration of declarations) {
      const openingBrace =
        (declaration.index ?? 0) + declaration[0].lastIndexOf("{");
      const closingBrace = balancedClosingBrace(masked, openingBrace);
      bodies.set(
        declaration.groups?.name ?? "",
        masked.slice(openingBrace + 1, closingBrace),
      );
    }
  }
  return bodies;
}

/** Returns a DTO plus transport aggregates that contain it. */
export function roundTripTargets(maskedSources, dtoName) {
  const bodies = transportParents(maskedSources);
  const targets = new Set([dtoName]);
  let changed = true;
  while (changed) {
    changed = false;
    for (const [parent, body] of bodies) {
      if (targets.has(parent)) continue;
      const containsTarget = [...targets].some((target) =>
        new RegExp(`\\b${escapeRegExp(target)}\\b`, "u").test(body));
      if (!containsTarget) continue;
      targets.add(parent);
      changed = true;
    }
  }
  return targets;
}

/** Returns the closing brace for one balanced Rust block. */
export function closingBraceForDefinition(source, openingBrace) {
  return balancedClosingBrace(source, openingBrace);
}
