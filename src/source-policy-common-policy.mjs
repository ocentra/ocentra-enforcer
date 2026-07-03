import { addViolation } from './source-policy-violation.mjs';
import { readLines } from './source-policy-helpers.mjs';

export function scanCommonPolicyFiles(root, filePath, rel, text) {
  const violations = [];
  if (/^(?:output|test-results|playwright-report)\//iu.test(rel)) {
    addViolation(violations, root, filePath, 1, 'GEN-1.2', 'generated output path is in source scope', rel);
  }
  if (/\.github\/workflows\/.*\.ya?ml$/iu.test(rel) || rel.startsWith('scripts/')) {
    text.split('\n').forEach((line, index) => {
      if (/\bnpm\s+install\b/iu.test(line) && !/\bnpm\s+ci\b/iu.test(line)) {
        addViolation(violations, root, filePath, index + 1, 'TS-7.12', 'CI/script uses npm install instead of npm ci', line);
      }
    });
  }
  if (/(?:^|\/)(?:eslint\.config\.[cm]?js|\.eslintrc(?:\.[cm]?js|\.json)?)$/iu.test(rel)) {
    const requiredRules = [
      '@typescript-eslint/no-floating-promises',
      '@typescript-eslint/no-explicit-any',
      '@typescript-eslint/no-unsafe-assignment',
    ];
    const missing = requiredRules.filter((ruleName) => !text.includes(ruleName));
    if (missing.length > 0) {
      addViolation(violations, root, filePath, 1, 'TS-7.13', `ESLint config misses strict TypeScript rules: ${missing.join(', ')}`, rel);
    }
  }
  return violations;
}
