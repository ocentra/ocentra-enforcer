import { balancedBodyAt } from "./rust-rules-source-test-evidence-ranges-balanced.mjs";

function balancedEndAt(source, openingBrace) {
  const body = balancedBodyAt(source, openingBrace);
  return body ? openingBrace + body.length : source.length;
}

/** Collects source ranges guarded for test-only compilation. */
export function testOnlyRanges(masked) {
  const ranges = [];
  for (const match of masked.matchAll(/^\s*#\[cfg\(test\)\]\s*$/gmu)) {
    const moduleStart = masked.indexOf("mod ", match.index + match[0].length);
    if (moduleStart < 0) continue;
    const openingBrace = masked.indexOf("{", moduleStart);
    if (openingBrace >= 0) ranges.push([match.index, balancedEndAt(masked, openingBrace)]);
  }
  return ranges;
}

/** Determines whether an index falls within any supplied source range. */
export function isInsideRanges(index, ranges) {
  return ranges.some(([start, end]) => index >= start && index < end);
}
