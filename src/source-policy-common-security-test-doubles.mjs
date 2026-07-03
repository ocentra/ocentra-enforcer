import { addViolation } from './source-policy-violation.mjs';
import { isSourceLikeForTestDoubles, hasNearbyWindowsGuard } from './source-policy-helpers.mjs';
import { isTestPath } from './source-policy-paths.mjs';
import { testDoublePatterns, windowsOnlyCommandPatterns } from './source-policy-scanner-shared.mjs';

export function scanCommonSecurityTestDoubles(root, filePath, rel, lines, text) {
  const violations = [];
  if (isTestPath(rel) && /(?:schema|decoder|codec)/iu.test(rel) && !/\b(?:invalid|malformed|bad input|reject|throws?|toThrow|rejects)\b/iu.test(text)) {
    addViolation(violations, root, filePath, 1, 'TS-8.10', 'decoder/schema test lacks negative invalid-input coverage', rel);
  }
  if (isSourceLikeForTestDoubles(rel)) {
    for (const [index, line] of lines.entries()) {
      const match = testDoublePatterns.find((rule) => rule.pattern.test(line));
      if (match) {
        addViolation(violations, root, filePath, index + 1, 'TEST-1.1', match.label, line);
      }
    }
  }
  if (rel.startsWith('scripts/') && rel.endsWith('.mjs')) {
    for (const [index, line] of lines.entries()) {
      const match = windowsOnlyCommandPatterns.find((rule) => rule.pattern.test(line));
      if (match && !hasNearbyWindowsGuard(lines, index)) {
        addViolation(violations, root, filePath, index + 1, 'PORT-1.1', match.label, line);
      }
    }
  }
  return violations;
}
