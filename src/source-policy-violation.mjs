import { normalizeRel } from './path-utils.mjs';
import { SOURCE_POLICY_RULES } from './source-policy-rule-registry.mjs';

export function addViolation(violations, root, filePath, line, ruleId, detail, sourceLine = null) {
  const rule = SOURCE_POLICY_RULES[ruleId] ?? { title: 'Unknown rule', snippet: '' };
  violations.push({
    ruleId,
    title: rule.title,
    detail,
    file: normalizeRel(root, filePath),
    line,
    snippet: rule.snippet,
    source: sourceLine?.trim() ?? null,
  });
}
