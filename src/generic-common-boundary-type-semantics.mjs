import { boundarySuccessType } from "./generic-common-boundary-type-syntax.mjs";

const RAW_TYPE_FALLBACK_RE = /\b(?:Raw[A-Z]\w*|[A-Z]\w*(?:Dto|DTO|Payload|Body|Request))\b/u;
const UNTYPED_TYPE_RE = /^(?:&?str|string|String|bool|boolean|number|void|unknown|any|dict|object|serde_json::Value|Value|\(\))$/u;

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/gu, "\\$&");
}

/** Builds the pattern used to recognize declared raw boundary types. */
export function rawBoundaryTypePattern(rawTypes) {
  if (rawTypes.size === 0) return RAW_TYPE_FALLBACK_RE;
  return new RegExp(`\\b(?:${[...rawTypes].map(escapeRegExp).join("|")})\\b`, "u");
}

/** Determines whether a return type exposes a raw boundary value. */
export function isDomainBoundaryReturnType(returnType, rawTypes) {
  const candidate = boundarySuccessType(returnType)
    .replace(/^&(?:'\w+\s+)?/u, "")
    .trim();
  if (!candidate || /^[{([]/u.test(candidate)) return false;
  if (UNTYPED_TYPE_RE.test(candidate) || rawBoundaryTypePattern(rawTypes).test(candidate)) return false;
  return /\b(?:Self|[A-Z][A-Za-z0-9_]*)\b/u.test(candidate);
}

/** Determines whether a boundary value type is absent or untyped. */
export function isUntypedBoundaryType(typeText) {
  return UNTYPED_TYPE_RE.test(String(typeText ?? "").trim());
}

/** Determines whether a boundary error type is absent or primitive. */
export function isUntypedBoundaryError(typeText) {
  const normalized = String(typeText ?? "").replace(/\s+/gu, "").replace(/^&(?:'\w+)?/u, "");
  return UNTYPED_TYPE_RE.test(normalized)
    || /^(?:anyhow::Error|Box<dyn(?:std::error::)?Error>|dyn(?:std::error::)?Error)$/u.test(normalized);
}
