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

const CONVERSION_NAME_RE = /^(?:convert|decode|deserialize|fromRaw|parse|toDomain|try_from|validate)/iu;

function functionSignatures(text) {
  const signatures = [];
  const declared = /\b(?:(?<visibility>export|pub(?:\([^)]*\))?)\s+)?(?:async\s+)?(?<kind>function|def|fn)\s+(?<name>[A-Za-z_]\w*)\s*\((?<parameters>[^)]*)\)\s*(?::|->)\s*(?<returnType>[^\n{;]+)/gu;
  for (const match of text.matchAll(declared)) {
    signatures.push({
      name: match.groups?.name ?? "",
      parameters: match.groups?.parameters ?? "",
      returnType: (match.groups?.returnType ?? "").trim(),
      isPublic: Boolean(match.groups?.visibility)
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

/** Extracts return-signature facts needed for boundary type analysis. */
export function analyzeBoundarySignatures(text, rawTypes) {
  const rawPattern = rawBoundaryTypePattern(rawTypes);
  const signatures = functionSignatures(text);
  const conversions = signatures.filter((signature) => CONVERSION_NAME_RE.test(signature.name));
  const hasDomainConversion = validTryFromTarget(text, rawTypes)
    || conversions.some((signature) => isDomainBoundaryReturnType(signature.returnType, rawTypes));
  const publicRawReturn = signatures.find((signature) =>
    signature.isPublic && rawPattern.test(signature.returnType));
  const untypedConversion = conversions.find((signature) => {
    const typedError = boundaryErrorType(signature.returnType);
    return isUntypedBoundaryType(boundarySuccessType(signature.returnType))
      || (typedError !== null && isUntypedBoundaryError(typedError));
  });
  const untypedTryFromError = /\btype\s+Error\s*=\s*(?:String|&?str|anyhow::Error|Box\s*<\s*dyn\s+(?:std::error::)?Error\s*>)\s*;/u.test(text);
  return {
    hasDomainConversion,
    publicRawReturn,
    untypedConversion,
    untypedTryFromError,
  };
}

/** Reports whether a domain signature exposes a boundary implementation type. */
export function domainSignatureLeaksBoundaryType(text) {
  const signature = /\b(?:export\s+)?(?:async\s+)?(?:function|const|def|fn)\s+\w+[^\n{;]*(?:Dto|DTO|Payload|Raw[A-Z]|Request\b)/u;
  return signature.test(text);
}
