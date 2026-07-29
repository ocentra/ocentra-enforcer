import { addViolation } from './source-policy-violation.mjs';
import {
  consolePattern,
  declareGlobalPattern,
  letInitializerPattern,
  processEnvPattern,
} from './source-policy-scanner-shared.mjs';
import {
  isConfigBoundaryPath,
  isDecoderBoundaryPath,
  isDomainLikeTypeScriptPath,
  isTypeOwnerPath,
} from './source-policy-paths.mjs';
import { maskJavaScriptLine } from './source-policy-text.mjs';

const BOUNDARY_LANGUAGE_RULES = [
  { ruleId: 'TS-6.17', label: 'ambient declare global outside type owner', pattern: declareGlobalPattern, skipWhen: isTypeOwnerPath },
  { ruleId: 'TS-6.18', label: 'process.env outside config boundary', pattern: processEnvPattern, skipWhen: isConfigBoundaryPath },
  { ruleId: 'TS-6.19', label: 'JSON.parse outside decoder boundary', pattern: /\bJSON\.parse\s*\(/u, skipWhen: isDecoderBoundaryPath },
  { ruleId: 'TS-6.24', label: 'console logging found in source', pattern: consolePattern },
  { ruleId: 'TS-6.39', label: 'single-assignment let found', pattern: letInitializerPattern, skipWhen: (rel) => !isDomainLikeTypeScriptPath(rel) },
];

/** Scans a boundary module for language-level policy violations. */
export function scanBoundaryLanguageRules(root, filePath, rel, lines) {
  const violations = [];
  for (const [index, line] of lines.entries()) {
    const masked = maskJavaScriptLine(line);
    for (const rule of BOUNDARY_LANGUAGE_RULES) {
      if (rule.skipWhen && rule.skipWhen(rel)) continue;
      if (rule.pattern.test(masked)) {
        addViolation(violations, root, filePath, index + 1, rule.ruleId, rule.label, line);
      }
    }
  }
  return violations;
}
