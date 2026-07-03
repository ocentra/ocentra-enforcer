export function functionName(signatureText) {
  return signatureText.match(/\bfn\s+([A-Za-z_][A-Za-z0-9_]*)\b/u)?.[1] ?? "";
}

export function functionParams(signatureText) {
  const open = signatureText.indexOf("(");
  if (open < 0) return "";
  let depth = 0;
  for (let i = open; i < signatureText.length; i += 1) {
    const ch = signatureText[i];
    if (ch === "(") depth += 1;
    if (ch === ")") {
      depth -= 1;
      if (depth === 0) return signatureText.slice(open + 1, i);
    }
  }
  return "";
}
