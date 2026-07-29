const DTO_RETURN = /->\s*(?:(?:[A-Za-z_][A-Za-z0-9_]*::)*(?:Result|Option)\s*<\s*)?(?:[A-Za-z_][A-Za-z0-9_]*::)*(?<target>[A-Z][A-Za-z0-9_]*(?:Dto|DTO|Request|Response|Envelope))\b/u;

export function returnedDtoTarget(tail) {
  return DTO_RETURN.exec(tail)?.groups?.target ?? null;
}

export function descriptorFromMatch(match) {
  const targetName = returnedDtoTarget(match.groups?.tail ?? "");
  return targetName ? { name: match.groups?.name ?? "", targetName } : null;
}
