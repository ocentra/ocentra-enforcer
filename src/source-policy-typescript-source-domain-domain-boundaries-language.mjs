import { addViolation } from './source-policy-violation.mjs';
import {
  consolePattern,
  declareGlobalPattern,
  defaultExportPattern,
  exportedArrowNoReturnPattern,
  exportedFunctionNoReturnPattern,
  exportedObjectLiteralPattern,
  letInitializerPattern,
  processEnvPattern,
} from './source-policy-scanner-shared.mjs';
import {
  isConfigBoundaryPath,
  isDecoderBoundaryPath,
  isDomainLikeTypeScriptPath,
  isTypeOwnerPath,
  isTypeScriptTypedPath,
} from './source-policy-paths.mjs';
import { maskJavaScriptLine } from './source-policy-text.mjs';

const BOUNDARY_LANGUAGE_RULES = [
  {
    ruleId: 'TS-6.13',
    label: 'TypeScript default export found',
    pattern: defaultExportPattern,
    skipWhen: (rel) => !isTypeScriptTypedPath(rel),
  },
  { ruleId: 'TS-6.17', label: 'ambient declare global outside type owner', pattern: declareGlobalPattern, skipWhen: isTypeOwnerPath },
  { ruleId: 'TS-6.18', label: 'process.env outside config boundary', pattern: processEnvPattern, skipWhen: isConfigBoundaryPath },
  { ruleId: 'TS-6.19', label: 'JSON.parse outside decoder boundary', pattern: /\bJSON\.parse\s*\(/u, skipWhen: isDecoderBoundaryPath },
  { ruleId: 'TS-6.24', label: 'console logging found in source', pattern: consolePattern },
  {
    ruleId: 'TS-6.37',
    label: 'exported function missing explicit return type',
    pattern: exportedFunctionNoReturnPattern,
    skipWhen: (rel) => !isTypeScriptTypedPath(rel),
  },
  {
    ruleId: 'TS-6.37',
    label: 'exported function missing explicit return type',
    pattern: exportedArrowNoReturnPattern,
    skipWhen: (rel) => !isTypeScriptTypedPath(rel),
  },
  {
    ruleId: 'TS-6.38',
    label: 'exported object literal API found',
    pattern: exportedObjectLiteralPattern,
    skipWhen: (rel) => !isTypeScriptTypedPath(rel),
  },
  { ruleId: 'TS-6.39', label: 'single-assignment let found', pattern: letInitializerPattern, skipWhen: (rel) => !isDomainLikeTypeScriptPath(rel) },
];

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
