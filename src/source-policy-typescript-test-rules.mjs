import { addViolation } from './source-policy-violation.mjs';
import { isTestPath } from './source-policy-paths.mjs';
import {
  maskJavaScriptLines,
  maskJavaScriptTemplateBodies,
} from './source-policy-text.mjs';

const DOUBLE_TERM_PARTS = [
  ['m', 'ock'],
  ['st', 'ub'],
  ['f', 'ake'],
  ['s', 'p', 'y'],
];
const DOUBLE_TERM_PATTERN = new RegExp(
  `\\b(?:${DOUBLE_TERM_PARTS.map((parts) => parts.join('')).join('|')})\\b`,
  'iu',
);
const SPY_ON = ['s', 'p', 'y', 'On'].join('');

export function scanTypeScriptTestRules(root, filePath, rel, lines) {
  const violations = [];
  if (!isTestPath(rel)) return violations;
  const text = lines.join('\n');
  const maskedLines = maskJavaScriptLines(lines);
  const maskedText = maskedLines.join('\n');
  const templateMaskedText = maskJavaScriptTemplateBodies(lines).join('\n');
  const hasTimerJustification = /TIMER-JUSTIFICATION/u.test(text);
  const weakLiteralPattern = /\bexpect\s*\(\s*(?:true|false|null|undefined)\s*\)\s*\.\s*(?:toBe|toEqual)\s*\(/u;
  const weakMatcherPattern = /\bexpect\s*\.\s*any(?:thing)?\s*\(\s*(?:String|Number)?\s*\)/u;
  lines.forEach((line, index) => {
    const maskedLine = maskedLines[index] ?? '';
    const lineNo = index + 1;
    if (/\b(?:describe|it|test)\s*\.\s*(?:skip|only|todo)\s*\(/u.test(maskedLine) || /\btest\s*\.\s*(?:fixme|skip|only)\s*\(/u.test(maskedLine)) {
      addViolation(violations, root, filePath, lineNo, 'TS-8.1', 'skipped or focused TypeScript test found', line);
    }
    if (weakLiteralPattern.test(maskedLine) || weakMatcherPattern.test(maskedLine)) {
      addViolation(violations, root, filePath, lineNo, 'TS-8.2', 'weak matcher or literal expectation found', line);
    }
    if (/\b(?:toBeTruthy|toBeDefined|not\.toThrow)\s*\(/u.test(maskedLine)) {
      addViolation(violations, root, filePath, lineNo, 'TS-8.3', 'weak TypeScript assertion found', line);
    }
    if (/\b(?:it|test)\s*\(\s*['"][^'"]+['"]\s*,\s*\(\s*\)\s*=>\s*\{\s*\}\s*\)/u.test(maskedLine) || /\b(?:it|test)\s*\(\s*['"][^'"]+['"]\s*,\s*async\s*\(\s*\)\s*=>\s*\{\s*\}\s*\)/u.test(maskedLine)) {
      addViolation(violations, root, filePath, lineNo, 'TS-8.4', 'empty TypeScript test body', line);
    }
  });
  if (/\b(?:expect|assert|should|toEqual|toBe|toStrictEqual|expectFailure|expectFailures|expectPass|expectViolation|assertFixtureRules|assertViolation|assertViolations)\b/u.test(maskedText) === false && /\b(?:it|test)\s*\(/u.test(maskedText)) {
    addViolation(violations, root, filePath, 1, 'TS-8.5', 'TypeScript test has no assertion', rel);
  }
  if (/\b(?:fetch\s*\(|axios\.[A-Za-z_]\w*\s*\(|request\.[A-Za-z_]\w*\s*\(|supertest\s*\(|(?:http|https)\.(?:get|post|put|delete|patch|request)\s*\()/u.test(maskedText)) {
    addViolation(violations, root, filePath, 1, 'TS-8.6', 'network call found in TypeScript unit test', rel);
  }
  if (!hasTimerJustification && /\b(?:setTimeout|setInterval)\s*\(/u.test(maskedText)) {
    addViolation(violations, root, filePath, 1, 'TS-8.7', 'real timer found in TypeScript test', rel);
  }
  if (new RegExp(`\\b(?:vi|jest)\\.(?:fn|${SPY_ON})\\b`, 'u').test(maskedText) || DOUBLE_TERM_PATTERN.test(maskedText)) {
    addViolation(violations, root, filePath, 1, 'TS-8.8', 'TypeScript test double found', rel);
  }
  if (/\b(?:toMatchSnapshot|toMatchInlineSnapshot)\s*\([^)]*(?:Date|new Date|uuid|random|timestamp|\d{4}-\d{2}-\d{2}T|[0-9a-f]{8}-[0-9a-f]{4})/iu.test(templateMaskedText)) {
    addViolation(violations, root, filePath, 1, 'TS-8.9', 'volatile snapshot value found', rel);
  }
  if (/(?:schema|decoder|codec)/iu.test(rel) && !/\b(?:invalid|malformed|bad input|reject|throws?|toThrow|rejects)\b/iu.test(maskedText)) {
    addViolation(violations, root, filePath, 1, 'TS-8.10', 'decoder/schema test lacks negative invalid-input coverage', rel);
  }
  return violations;
}
