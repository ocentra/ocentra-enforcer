/** Determines whether a path belongs to a TypeScript domain layer. */
export function isDomainLikeTypeScriptPath(rel) {
  return (
    /(^|\/)src\/(?:domain(?:\/|\.|$)|domains\/|model(?:s)?\/|state(?:\/|\.|$)|value-objects?\/|entities?\/|aggregates?\/)/u.test(rel)
    || /(^|\/)(?:domain|state|model)\.[jt]sx?$/u.test(rel)
  );
}

/** Determines whether a path is typed TypeScript source. */
export function isTypeScriptTypedPath(rel) {
  return /\.(?:cts|mts|ts|tsx)$/iu.test(rel);
}

/** Determines whether a path is a tooling boundary module. */
export function isToolingBoundaryPath(rel) {
  return isTestPath(rel) ||
    /^(?:scripts|mcp|eslint-rules|adapters|tests|schemas)\//u.test(rel) ||
    /^crates\/enforcer-literal-scan\/integration\//u.test(rel) ||
    /^(?:Tools|tools)\/ocentra-literal-scan\/integration\//u.test(rel) ||
    /^src\/(?:checks|check-[^/]+|cli(?:-[^/]+)?|codex-install|documentation-hints|generic-[^/]+|harness(?:-[^/]+)?|literal-risk(?:-[^/]+)?|path-utils|policy|proof(?:-[^/]+)?|routing|rule-[^/]+|source-policy-[^/]+)\.mjs$/u.test(rel) ||
    /^src\/coordination\//u.test(rel);
}

/** Determines whether a path is a configuration boundary module. */
export function isConfigBoundaryPath(rel) {
  return isToolingBoundaryPath(rel) || /(?:^|\/)(?:config|configs|configuration|env|environment)(?:\/|\.|-)|(?:^|\/)[^/]*(?:config|env)[^/]*\.(?:ts|tsx|js|mjs|cjs)$/iu.test(rel);
}

/** Determines whether a path is a decoding boundary module. */
export function isDecoderBoundaryPath(rel) {
  return isToolingBoundaryPath(rel) || /(?:^|\/)(?:schema|schemas|decoder|decoders|codec|codecs|boundary|boundaries|adapter|adapters|transport|serde)(?:\/|\.|-)/iu.test(rel);
}

/** Determines whether a path names an index module. */
export function isIndexModule(rel) {
  return /(?:^|\/)index\.[cm]?[jt]sx?$/iu.test(rel);
}

/** Determines whether a path is inside a test surface. */
export function isTestPath(rel) {
  return /(?:^|\/)(?:tests?|__tests__|spec)(?:\/|$)|\.(?:test|spec)\.[cm]?[jt]sx?$/iu.test(rel);
}

/** Determines whether a path owns domain type declarations. */
export function isTypeOwnerPath(rel) {
  return /(?:^|\/)(?:types?|globals?|ambient|declarations)(?:\/|\.|-)|\.d\.ts$/iu.test(rel);
}
