import { addViolation } from './source-policy-violation.mjs';
import { timerPattern } from './source-policy-scanner-shared.mjs';
import { hasNearbyTimerJustification } from './source-policy-helpers.mjs';
import { maskJavaScriptLine } from './source-policy-text.mjs';

export function scanBoundaryTimerRules(root, filePath, rel, lines) {
  void rel;
  const violations = [];
  for (const [index, line] of lines.entries()) {
    const masked = maskJavaScriptLine(line);
    if (timerPattern.test(masked) && !hasNearbyTimerJustification(lines, index)) {
      addViolation(violations, root, filePath, index + 1, 'TS-6.31', 'timer sleep found without justification', line);
    }
  }
  return violations;
}
