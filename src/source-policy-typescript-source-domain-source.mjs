import { addViolation } from './source-policy-violation.mjs';
import {
  barrelReexportPattern,
  defaultExportPattern,
  exportedArrowNoReturnPattern,
  exportedFunctionNoReturnPattern,
  exportedObjectLiteralPattern,
  manualBrandPattern,
  nakedDomainAliasPattern,
  zodSourcePatterns,
} from './source-policy-scanner-shared.mjs';
import { isTypeScriptTypedPath } from './source-policy-paths.mjs';
import { maskJavaScriptLine } from './source-policy-text.mjs';

const SOURCE_RULES = [
  { ruleId: 'TS-1.2', label: 'Direct Zod source usage is forbidden', patterns: zodSourcePatterns.map((entry) => entry.pattern) },
  { ruleId: 'TS-1.3', label: 'naked domain string alias or manual brand found', patterns: [manualBrandPattern, nakedDomainAliasPattern] },
  {
    ruleId: 'TS-6.13',
    label: 'TypeScript default export found',
    pattern: defaultExportPattern,
    skipWhen: (rel) => !isTypeScriptTypedPath(rel),
  },
  { ruleId: 'TS-6.14', label: 'Index barrel re-export found', pattern: barrelReexportPattern },
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
];

function scanRuleSet(root, filePath, rel, lines, rules) {
  const violations = [];
  for (const line of lines) {
    const masked = maskJavaScriptLine(line);
    for (const rule of rules) {
      if (rule.skipWhen && rule.skipWhen(rel)) continue;
      const patterns = rule.patterns ?? [rule.pattern];
      const matched = patterns.some((pattern) =>
        rule.ruleId === "TS-1.2" && pattern.source.includes("zod")
          ? pattern.test(line)
          : pattern.test(masked),
      );
      if (matched) {
        addViolation(violations, root, filePath, 1, rule.ruleId, rule.label, line);
      }
    }
  }
  return violations;
}

export function scanTypeScriptSourceRulesOnly(root, filePath, rel, lines) {
  return scanRuleSet(root, filePath, rel, lines, SOURCE_RULES);
}
