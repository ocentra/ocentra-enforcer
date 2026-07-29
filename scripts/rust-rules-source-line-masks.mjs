import { contextHas } from "./rust-rules-path-core.mjs";
import { braceDelta } from "./rust-rules-source-helpers.mjs";
import { isTransportRecordName } from "./rust-rules-source-classification.mjs";

/** Determines whether a line begins a Rust test function signature. */
export function isTestFunctionSignature(lines, index) {
  const attribute = /^\s*#\[(?:test|tokio::test|async_std::test)\b/u;
  const functionStart = /^\s*(?:pub\s+)?(?:async\s+)?fn\b/u;
  const marker = lines.slice(Math.max(0, index - 4), index).reverse()
    .find((line) => attribute.test(line) || functionStart.test(line));
  return attribute.test(marker ?? "");
}

/** Builds a line mask for transport-record declarations. */
export function transportRecordLineMask(lines) {
  const transportLines = Array(lines.length).fill(false);
  let pendingSerdeDerive = false;
  let pendingStart = null;
  let transportDepth = null;
  for (let index = 0; index < lines.length; index += 1) {
    const line = lines[index];
    if (transportDepth !== null) {
      transportLines[index] = true;
      transportDepth += braceDelta(line);
      if (transportDepth <= 0) transportDepth = null;
      continue;
    }
    if (/^\s*#\[derive\([^#\]]*\b(?:Serialize|Deserialize)\b[^#\]]*\)\]/u.test(line)) {
      pendingSerdeDerive = true;
      pendingStart ??= index;
      continue;
    }
    if ((/^\s*#\[/u.test(line) || /^\s*$/u.test(line)) && pendingStart !== null) continue;
    const declaration = line.match(/^\s*(?:pub(?:\([^)]*\))?\s+)?(?:struct|enum)\s+(?<name>[A-Z][A-Za-z0-9_]*)/u);
    if (declaration?.groups?.name) {
      if (isTransportRecordName(declaration.groups.name, pendingSerdeDerive)) {
        const start = pendingStart ?? index;
        transportLines.fill(true, start, index + 1);
        const depth = braceDelta(line);
        if (depth > 0) transportDepth = depth;
      }
      pendingSerdeDerive = false;
      pendingStart = null;
      continue;
    }
    pendingSerdeDerive = false;
    pendingStart = null;
  }
  return transportLines;
}

/** Checks whether a default expression has an adjacent rationale. */
export function hasDefaultRationale(lines, index) {
  if (contextHas(lines, index, "DEFAULT-JUSTIFICATION:", 4)) return true;
  const start = Math.max(0, index - 4);
  const end = Math.min(lines.length, index + 3);
  return /\/\/[/!]?[\s\S]{0,240}\b(?:omitt(?:ed|ing)?|missing|absent|optional|default(?:s|ed)?|empty|zero)\b/iu.test(lines.slice(start, end).join("\n"));
}
