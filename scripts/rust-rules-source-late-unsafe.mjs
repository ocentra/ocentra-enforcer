import { addViolation, escapeRegExp, lineNumberAtIndex } from "./rust-rules-path-core.mjs";
import {
  hasAllValuesValidFromConstructor,
  hasClosedConstructionSurface,
  hasFallibleTraitConstructor,
  inherentImplBodies,
} from "./rust-rules-source-late-newtypes.mjs";

/** Applies unsafe-code evidence rules to a Rust source scan context. */
export function applyUnsafeEvidenceRules({
  source,
  masked,
  originalLines,
  root,
  filePath,
  violations,
}) {
  const maskedLines = masked.split(/\r?\n/u);
  const unsafeLine = maskedLines.findIndex((line) => /\bunsafe\b/u.test(line));
  if (unsafeLine < 0) return;
  if (!/\bMIRI-PROOF:/u.test(source)) {
    addViolation(violations, root, filePath, unsafeLine + 1, "RR-3.30", "unsafe source lacks MIRI-PROOF evidence.", originalLines[unsafeLine]);
    addViolation(violations, root, filePath, unsafeLine + 1, "RR-12.30", "unsafe module lacks MIRI-PROOF evidence.", originalLines[unsafeLine]);
  }
  if (!/\bGEIGER-PROOF:/u.test(source)) {
    addViolation(violations, root, filePath, unsafeLine + 1, "RR-3.31", "unsafe source lacks GEIGER-PROOF evidence.", originalLines[unsafeLine]);
  }
}

/** Applies newtype-constructor rules to a Rust source scan context. */
export function applyNewtypeConstructorRules({
  source,
  masked,
  originalLines,
  root,
  filePath,
  violations,
  isBoundary,
  isConfigurationBoundary,
}) {
  if (isConfigurationBoundary) return;

  function hasValidatedInherentConstructor(typeName, hasInvariant, zeroValidInvariant) {
    return inherentImplBodies(source, typeName).some((body) => {
      const constructors = body.matchAll(
        /\b(?:pub(?:\([^)]*\))?\s+)?(?:const\s+)?fn\s+(?<name>new|try_new|parse|from_[A-Za-z0-9_]+|try_from_[A-Za-z0-9_]+|normalized|clamped)(?:\s*<[^>{}]+>)?\s*\((?<params>[^)]*)\)\s*->\s*(?<return>[^{;]+)(?<body>\{[\s\S]*?\n[\t ]*\})?/gu,
      );
      for (const constructor of constructors) {
        const returnType = constructor.groups?.return ?? "";
        if (/\b(?:Result|Option)\s*<\s*Self\b/u.test(returnType)) return true;
        if (!hasInvariant || !/^Self\b/u.test(returnType.trim())) continue;
        const params = constructor.groups?.params ?? "";
        const bodyText = constructor.groups?.body ?? "";
        if (!/\b(?:String|str|bool|u8|u16|u32|u64|usize|i8|i16|i32|i64|isize)\b/u.test(params)) {
          return true;
        }
        if (
          /\b(?:NonZero|clamp|checked_|saturating_)\b|\.len\s*\(|\.count\s*\(|&\s*0b[01]+/u.test(bodyText)
        ) {
          return true;
        }
        if (zeroValidInvariant || isBoundary) return true;
      }
      return false;
    });
  }

  for (const match of source.matchAll(/pub\s+struct\s+(?<name>[A-Z][A-Za-z0-9_]*)\s*\(\s*(?<visibility>pub\s+)?(?<inner>String|&\s*str|str|u8|u16|u32|u64|usize|i8|i16|i32|i64|isize|bool)[^)]*\)\s*;/gu)) {
    const typeName = match.groups?.name ?? "";
    const inner = (match.groups?.inner ?? "").replace(/\s+/gu, "");
    const lineNo = lineNumberAtIndex(source, match.index ?? 0);
    const contextStart = Math.max(0, lineNo - 9);
    const declarationContext = originalLines.slice(contextStart, lineNo).join("\n");
    const hasInvariant = /\bBRAND-INVARIANT:/u.test(declarationContext);
    const zeroValidInvariant =
      inner === "bool" ||
      /\b(?:non-negative|zero-inclusive|zero-based|exact (?:number|count|length)|cardinality|signed|count)\b/iu.test(
        declarationContext,
      );
    const hasBooleanConstructor = [inner === "bool", hasInvariant].every(Boolean);
    const hasConstructor = [
      hasValidatedInherentConstructor(typeName, hasInvariant, zeroValidInvariant),
      hasFallibleTraitConstructor(source, typeName),
      hasAllValuesValidFromConstructor({ source, masked, typeName, inner, hasInvariant, declarationContext }),
      hasBooleanConstructor,
      hasClosedConstructionSurface({
        source,
        typeName,
        hasInvariant,
        declarationContext,
        rawFieldIsPublic: Boolean(match.groups?.visibility),
        inherentBodies: inherentImplBodies(source, typeName),
      }),
    ].some(Boolean);
    if (!hasConstructor) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-6.44",
        `newtype ${typeName} lacks a validated constructor or documented zero-valid From conversion.`,
        originalLines[lineNo - 1] ?? null,
      );
    }
  }
}
