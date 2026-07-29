const WRAPPER_TYPES = new Set(["Arc", "Box", "Option", "Promise", "Rc"]);

/** Splits a generic boundary result type into its top-level arguments. */
export function splitBoundaryTypeArguments(typeText) {
  const open = typeText.indexOf("<");
  const close = typeText.lastIndexOf(">");
  if (open < 0 || close <= open) return [];
  const args = [];
  let depth = 0;
  let start = open + 1;
  for (let index = open + 1; index < close; index += 1) {
    const character = typeText[index];
    if (character === "<" || character === "[") depth += 1;
    if (character === ">" || character === "]") depth -= 1;
    if (character === "," && depth === 0) {
      args.push(typeText.slice(start, index).trim());
      start = index + 1;
    }
  }
  args.push(typeText.slice(start, close).trim());
  return args;
}

function outerTypeName(typeText) {
  return typeText.trim().match(/^(?:[A-Za-z_]\w*::)*([A-Za-z_]\w*)/u)?.[1] ?? "";
}

/** Extracts the success type from a boundary result signature. */
export function boundarySuccessType(returnType) {
  let current = returnType.trim().replace(/:$/u, "");
  for (let depth = 0; depth < 5; depth += 1) {
    const outer = outerTypeName(current);
    const args = splitBoundaryTypeArguments(current);
    if (outer === "Result" && args.length >= 1) current = args[0];
    else if (outer === "Either" && args.length >= 2) current = args[1];
    else if (WRAPPER_TYPES.has(outer) && args.length >= 1) current = args[0];
    else break;
  }
  return current.trim();
}

/** Extracts the error type from a boundary result signature. */
export function boundaryErrorType(returnType) {
  const outer = outerTypeName(returnType);
  const args = splitBoundaryTypeArguments(returnType);
  if (outer === "Result" && args.length >= 2) return args[1];
  if (outer === "Either" && args.length >= 2) return args[0];
  return null;
}
