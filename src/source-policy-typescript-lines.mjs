import { normalizeRel } from './path-utils.mjs';
import { readLines } from './source-policy-helpers.mjs';
import { scanTypeScriptSourceRules } from './source-policy-typescript-source.mjs';
import { scanTypeScriptTestRules } from './source-policy-typescript-tests.mjs';

export function scanTypeScriptLineRules(root, filePath, rel, lines) {
  const resolvedRel = rel ?? normalizeRel(root, filePath);
  const sourceLines = lines ?? readLines(filePath);
  const violations = [];
  violations.push(...scanTypeScriptSourceRules(root, filePath, resolvedRel, sourceLines));
  violations.push(...scanTypeScriptTestRules(root, filePath, resolvedRel, sourceLines));
  return violations;
}
