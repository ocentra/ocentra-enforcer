export function normalizedNameTokens(name) {
  return name
    .replace(/([a-z0-9])([A-Z])/gu, "$1_$2")
    .toLowerCase()
    .split(/[^a-z0-9]+/u)
    .filter(Boolean);
}

export function isSuspiciousSerializedFieldName(name) {
  const tokens = normalizedNameTokens(name);
  const lastToken = tokens.at(-1);
  const secondToLastToken = tokens.at(-2);
  return (
    lastToken === "id" ||
    lastToken === "ids" ||
    lastToken === "ref" ||
    lastToken === "refs" ||
    (secondToLastToken === "event" && lastToken === "type") ||
    (secondToLastToken === "command" && lastToken === "type")
  );
}

export function braceDelta(line) {
  return (line.match(/\{/gu) ?? []).length - (line.match(/\}/gu) ?? []).length;
}

export function hasStringLiteral(line) {
  return /"(?:[^"\\]|\\.)*"/u.test(line);
}
