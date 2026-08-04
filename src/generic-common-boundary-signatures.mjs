import {
  isDomainBoundaryReturnType,
  isUntypedBoundaryError,
  isUntypedBoundaryType,
  rawBoundaryTypePattern,
} from "./generic-common-boundary-type-semantics.mjs";
import {
  boundaryErrorType,
  boundarySuccessType,
} from "./generic-common-boundary-type-syntax.mjs";

const CONVERSION_NAME_RE = /^(?:convert|decode|deserialize|fromRaw|into_?domain|parse|toDomain|try_from|validate)/iu;

function functionSignatures(text) {
  const signatures = [];
  const declared = /\b(?:(?<visibility>export|pub(?:\([^)]*\))?)\s+)?(?:async\s+)?(?<kind>function|def|fn)\s+(?<name>[A-Za-z_]\w*)\s*\((?<parameters>[^)]*)\)\s*(?::|->)\s*(?<returnType>[^\n{;]+)/gu;
  for (const match of text.matchAll(declared)) {
    signatures.push({
      name: match.groups?.name ?? "",
      parameters: match.groups?.parameters ?? "",
      returnType: (match.groups?.returnType ?? "").trim(),
      // `pub(crate)` and `pub(super)` are crate-internal implementation seams,
      // not an exported API.  Treating them as public caused the boundary rule
      // to report DTOs which cannot escape the crate.
      isPublic: /^(?:export|pub)$/u.test(match.groups?.visibility ?? "")
        || (match.groups?.kind === "def" && !String(match.groups?.name ?? "").startsWith("_")),
    });
  }
  return signatures;
}

function validTryFromTarget(text, rawTypes) {
  const rawPattern = rawBoundaryTypePattern(rawTypes);
  for (const match of text.matchAll(/\bimpl\s+TryFrom\s*<\s*(?<raw>[^>]+)>\s+for\s+(?<target>[A-Za-z_]\w*)/gu)) {
    if (rawPattern.test(match.groups?.raw ?? "") && isDomainBoundaryReturnType(match.groups?.target ?? "", rawTypes)) return true;
  }
  return false;
}

function validFromTarget(text, rawTypes) {
  const rawPattern = rawBoundaryTypePattern(rawTypes);
  for (const match of text.matchAll(/\bimpl\s+From\s*<\s*(?<raw>[^>]+)>\s+for\s+(?<target>[A-Za-z_]\w*)/gu)) {
    if (rawPattern.test(match.groups?.raw ?? "") && isDomainBoundaryReturnType(match.groups?.target ?? "", rawTypes)) return true;
  }
  return false;
}

function removeRustAndDocComments(text) {
  return text
    .replace(/\/\/[^\n\r]*/gu, "")
    .replace(/\/\*[\s\S]*?\*\//gu, "");
}

/** Extracts return-signature facts needed for boundary type analysis. */
export function analyzeBoundarySignatures(text, rawTypes) {
  const rawPattern = rawBoundaryTypePattern(rawTypes);
  const signatures = functionSignatures(text);
  const conversions = signatures.filter((signature) => CONVERSION_NAME_RE.test(signature.name));
  const hasDomainConversion = validTryFromTarget(text, rawTypes)
    || validFromTarget(text, rawTypes)
    || conversions.some((signature) => isDomainBoundaryReturnType(signature.returnType, rawTypes));
  const publicRawReturn = rawTypes.size > 0 && signatures.find((signature) =>
    signature.isPublic && rawPattern.test(signature.returnType)
      && !hasDocumentedWireOutput(text, signature.returnType, rawTypes));
  const untypedConversion = rawTypes.size > 0 && conversions.find((signature) => {
    // A boundary helper can parse a local representation, but only an
    // exported conversion that accepts a declared raw DTO can leak an
    // untyped transport error across the API boundary.
    if (!signature.isPublic || !rawPattern.test(signature.parameters)) return false;
    // Validators commonly return `Result<(), TypedError>` after inspecting an
    // already-typed value.  They are not raw-to-domain conversions, so a unit
    // success value is correct and must not be treated as an untyped boundary.
    if (boundarySuccessType(signature.returnType).trim() === "()") return false;
    const typedError = boundaryErrorType(signature.returnType);
    return isUntypedBoundaryType(boundarySuccessType(signature.returnType))
      || (typedError !== null && isUntypedBoundaryError(typedError));
  });
  const untypedTryFromError = /\btype\s+Error\s*=\s*(?:String|&?str|anyhow::Error|Box\s*<\s*dyn\s+(?:std::error::)?Error\s*>)\s*;/u.test(text);
  return {
    hasDomainConversion,
    // Only exported APIs are boundary ingress. Trait conversion methods and
    // crate-private persistence/serialization helpers may accept a DTO while
    // they are still inside the boundary module; treating those as public
    // ingress created false BOUND-1.2 findings in typed code.
    hasRawInput: rawTypes.size > 0 && signatures.some((signature) => signature.isPublic && (
      rawPattern.test(signature.parameters)
        || (CONVERSION_NAME_RE.test(signature.name) && rawPattern.test(signature.returnType))
    )),
    publicRawReturn,
    untypedConversion,
    untypedTryFromError,
  };
}

function hasDocumentedWireOutput(text, returnType, rawTypes) {
  if (!/\bROUNDTRIP-TEST:/u.test(text)) return false;
  const rawPattern = rawBoundaryTypePattern(rawTypes);
  const output = boundarySuccessType(returnType);
  if (!rawPattern.test(output)) return false;
  return /#\[derive\([^\]]*Serialize[^\]]*\)\][\s\S]{0,240}\b(?:pub(?:\([^)]*\))?\s+)?(?:struct|enum)\s+/u.test(text);
}

/** Reports whether a domain signature exposes a boundary implementation type. */
export function domainSignatureLeaksBoundaryType(text) {
  const signature = /\b(?:export\s+)?(?:async\s+)?(?:function|const|def|fn)\s+\w+[^\n{;]*(?:Dto|DTO|Payload|Raw[A-Z]|Request\b)/u;
  return signature.test(removeRustAndDocComments(text));
}
