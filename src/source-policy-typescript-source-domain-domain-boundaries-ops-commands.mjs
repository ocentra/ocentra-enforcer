import { addViolation } from './source-policy-violation.mjs';
import {
  childProcessPattern,
  dynamicCodePattern,
  dynamicImportPattern,
} from './source-policy-scanner-shared.mjs';
import { isToolingBoundaryPath } from './source-policy-paths.mjs';
import { maskJavaScriptLine } from './source-policy-text.mjs';

const RULES = [
  { ruleId: 'TS-6.32', label: 'dynamic import found in domain code', pattern: dynamicImportPattern },
  {
    ruleId: 'TS-6.33',
    label: 'child_process usage outside script boundary',
    pattern: childProcessPattern,
    skipWhen: isToolingBoundaryPath,
    useRawLine: true,
  },
  { ruleId: 'TS-6.34', label: 'dynamic code execution found', pattern: dynamicCodePattern },
];

export function scanBoundaryCommandRules(root, filePath, rel, lines) {
  const violations = [];
  for (const [index, line] of lines.entries()) {
    const masked = maskJavaScriptLine(line);
    for (const rule of RULES) {
      if (rule.skipWhen && rule.skipWhen(rel)) continue;
      const candidate = rule.useRawLine ? line : masked;
      if (rule.pattern.test(candidate)) {
        addViolation(violations, root, filePath, index + 1, rule.ruleId, rule.label, line);
      }
    }
  }
  return violations;
}
