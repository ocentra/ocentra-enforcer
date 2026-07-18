import { escapeRegExp } from "./rust-rules-path-core.mjs";

/** Collects inherent implementation bodies for a Rust type. */
export function inherentImplBodies(source, typeName) {
  const bodies = [];
  const pattern = new RegExp(
    `^(?<indent>[\\t ]*)impl(?:\\s*<[^>{}]+>)?\\s+${escapeRegExp(typeName)}(?:\\s*<[^>{}]+>)?\\s*\\{(?<body>[\\s\\S]*?)^\\k<indent>\\}`,
    "gmu",
  );
  for (const match of source.matchAll(pattern)) bodies.push(match.groups?.body ?? "");
  return bodies;
}

/** Checks whether a Rust type has a fallible trait constructor. */
export function hasFallibleTraitConstructor(source, typeName) {
  return new RegExp(
    `\\bimpl(?:\\s*<[^>{}]+>)?\\s+TryFrom\\s*<[^>]+>\\s+for\\s+${escapeRegExp(typeName)}\\b`,
    "u",
  ).test(source);
}

/** Checks whether a newtype exposes only a closed construction surface. */
export function hasClosedConstructionSurface({ source, typeName, hasInvariant, declarationContext, rawFieldIsPublic, inherentBodies }) {
  if (!hasInvariant || rawFieldIsPublic || /\bDeserialize\b/u.test(declarationContext)) return false;
  const inboundRaw = new RegExp(
    `\\bimpl(?:\\s*<[^>{}]+>)?\\s+From\\s*<\\s*(?:String|&\\s*str|str|u8|u16|u32|u64|usize|i8|i16|i32|i64|isize|bool)\\s*>\\s+for\\s+${escapeRegExp(typeName)}\\b`,
    "u",
  ).test(source);
  const rawInherentConstructor = inherentBodies.some((body) =>
    /\bfn\s+[A-Za-z_][A-Za-z0-9_]*(?:\s*<[^>{}]+>)?\s*\([^)]*\b(?:String|str|bool|u8|u16|u32|u64|usize|i8|i16|i32|i64|isize)\b[^)]*\)\s*->\s*Self\b/u.test(body));
  return !inboundRaw && !rawInherentConstructor;
}

/** Checks whether a newtype constructor validates all possible inner values. */
export function hasAllValuesValidFromConstructor({ source, masked, typeName, inner, hasInvariant, declarationContext }) {
  if (!hasInvariant || /(?:Id|ID|Key|Ref)$/u.test(typeName)) return false;
  const numericOrBoolean = /^(?:u8|u16|u32|u64|usize|i8|i16|i32|i64|isize|bool)$/u.test(inner);
  const documentedEmptyValidString = inner === "String"
    && /\b(?:empty-valid|empty (?:text|value|string) is valid|empty values are valid)\b/iu.test(declarationContext);
  if (!numericOrBoolean && !documentedEmptyValidString) return false;
  return new RegExp(
    `\\bimpl(?:\\s*<[^>{}]+>)?\\s+From\\s*<\\s*${escapeRegExp(inner)}\\s*>\\s+for\\s+${escapeRegExp(typeName)}\\b`,
    "u",
  ).test(masked || source);
}
