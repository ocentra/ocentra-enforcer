import { addViolation } from './source-policy-violation.mjs';
import {
  anySpreadPattern,
  anyTypePattern,
  dateDomainPattern,
  doubleAssertionPattern,
  definiteAssignmentPattern,
  dynamicCodePattern,
  dynamicImportPattern,
  emptyCatchPattern,
  enumPattern,
  mapStringDomainPattern,
  nakedDomainAliasPattern,
  namespacePattern,
  nonNullAssertionPattern,
  optionalFieldPattern,
  partialPattern,
  promiseAnyUnknownPattern,
  rawBooleanDomainPattern,
  rawDtoSpreadPattern,
  rawNumberDomainPattern,
  recordStringDomainPattern,
  recordUnknownPayloadPattern,
  returnNullPattern,
  sharedMutationPattern,
  stringArrayDomainPattern,
  throwStringPattern,
  typeAssertionPattern,
  undefinedStatePattern,
  unknownEscapePattern,
} from './source-policy-scanner-shared.mjs';
import { isDomainLikeTypeScriptPath } from './source-policy-paths.mjs';
import { maskJavaScriptLine } from './source-policy-text.mjs';

const CORE_RULES = [
  { ruleId: 'TS-6.1', label: 'TypeScript any is forbidden', pattern: anyTypePattern },
  { ruleId: 'TS-6.2', label: 'TypeScript unknown cannot escape boundaries', pattern: unknownEscapePattern },
  { ruleId: 'TS-6.3', label: 'TypeScript type assertion found', pattern: typeAssertionPattern },
  { ruleId: 'TS-6.4', label: 'TypeScript double assertion found', pattern: doubleAssertionPattern },
  { ruleId: 'TS-6.5', label: 'TypeScript non-null assertion found', pattern: nonNullAssertionPattern },
  { ruleId: 'TS-6.6', label: 'TypeScript definite assignment assertion found', pattern: definiteAssignmentPattern },
  { ruleId: 'TS-6.7', label: 'raw string domain alias found', pattern: nakedDomainAliasPattern },
  { ruleId: 'TS-6.8', label: 'raw number domain value found', pattern: rawNumberDomainPattern },
  { ruleId: 'TS-6.9', label: 'raw boolean domain parameter found', pattern: rawBooleanDomainPattern },
  { ruleId: 'TS-6.10', label: 'Record<string, domain> API found', pattern: recordStringDomainPattern },
  { ruleId: 'TS-6.11', label: 'Map<string, domain> API found', pattern: mapStringDomainPattern },
  { ruleId: 'TS-6.12', label: 'string[] domain API found', pattern: stringArrayDomainPattern },
  { ruleId: 'TS-6.15', label: 'TypeScript namespace declaration found', pattern: namespacePattern },
  { ruleId: 'TS-6.16', label: 'TypeScript enum found', pattern: enumPattern },
  { ruleId: 'TS-6.20', label: 'Date found in domain API', pattern: dateDomainPattern },
  { ruleId: 'TS-6.21', label: 'Promise<any|unknown> found', pattern: promiseAnyUnknownPattern },
  { ruleId: 'TS-6.22', label: 'floating promise found', pattern: /^\s*(?!await\b|return\b|void\b)(?:[A-Za-z_$][\w$]*\.)?(?:[A-Za-z_$][\w$]*Async|fetch[A-Za-z0-9_$]*)\s*\([^;]*\)\s*;/u },
  { ruleId: 'TS-6.23', label: 'swallowed promise catch found', pattern: /\.catch\s*\(\s*(?:\(\s*\)|[A-Za-z_$][\w$]*)\s*=>\s*\{\s*\}\s*\)/u },
  { ruleId: 'TS-6.24', label: 'console logging found in source', pattern: /\bconsole\.(?:log|debug|info|warn|error|trace)\s*\(/u },
  { ruleId: 'TS-6.25', label: 'string error throw found', pattern: throwStringPattern, useRawLine: true },
  { ruleId: 'TS-6.26', label: 'return null found in domain API', pattern: returnNullPattern },
  { ruleId: 'TS-6.27', label: 'undefined domain state found', pattern: undefinedStatePattern },
  {
    ruleId: 'TS-6.28',
    label: 'optional domain field found',
    pattern: optionalFieldPattern,
    skipWhen: (rel) => !isDomainLikeTypeScriptPath(rel),
  },
  { ruleId: 'TS-6.29', label: 'Partial<T> domain logic found', pattern: partialPattern },
  { ruleId: 'TS-6.30', label: 'Record<string, unknown> payload found', pattern: recordUnknownPayloadPattern },
  { ruleId: 'TS-6.32', label: 'dynamic import found in domain code', pattern: dynamicImportPattern },
  { ruleId: 'TS-6.34', label: 'dynamic code execution found', pattern: dynamicCodePattern },
  { ruleId: 'TS-6.35', label: 'raw DTO spread found', pattern: rawDtoSpreadPattern },
  { ruleId: 'TS-6.36', label: 'any spread into domain object found', pattern: anySpreadPattern },
  { ruleId: 'TS-6.40', label: 'mutating shared object found', pattern: sharedMutationPattern },
];

function scanRuleSet(root, filePath, lines, rules) {
  const violations = [];
  const rel = filePath ? filePath.replaceAll('\\', '/') : '';
  for (const [index, line] of lines.entries()) {
    const masked = maskJavaScriptLine(line);
    for (const rule of rules) {
      if (rule.skipWhen && rule.skipWhen(rel)) continue;
      const candidate = rule.useRawLine ? line : masked;
      if (rule.pattern.test(candidate)) {
        addViolation(violations, root, filePath, index + 1, rule.ruleId, rule.label, line);
      }
    }
  }
  return violations;
}

export function scanTypeScriptDomainCoreRules(root, filePath, rel, lines) {
  void rel;
  return scanRuleSet(root, filePath, lines, CORE_RULES);
}
