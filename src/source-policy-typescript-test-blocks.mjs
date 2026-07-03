import { addViolation } from './source-policy-violation.mjs';
import { maskJavaScriptLine } from './source-policy-text.mjs';

export function scanTypeScriptTestBlocks(root, filePath, lines) {
  const violations = [];
  let current = null;
  let braceDepth = 0;
  const flush = () => {
    if (!current) return;
    const body = current.lines.join('\n').trim();
    if (body === '' || body === '{}' || /(?:=>|function\s*\([^)]*\))\s*\{\s*\}\s*\)\s*;?$/u.test(body)) {
      addViolation(violations, root, filePath, current.line, 'TS-8.4', 'empty TypeScript test body', current.header);
    } else if (!/\b(?:expect|assert|should|toEqual|toBe|toStrictEqual|expectFailure|expectPass|expectViolation|assertFixtureRules|assertViolation|assertViolations)\b/u.test(body)) {
      addViolation(violations, root, filePath, current.line, 'TS-8.5', 'TypeScript test has no assertion', current.header);
    }
    current = null;
    braceDepth = 0;
  };
  lines.forEach((line, index) => {
    if (!current && /^\s*(?:it|test)\s*\(/u.test(line)) {
      flush();
      current = { line: index + 1, header: line, lines: [line] };
      braceDepth = braceDelta(maskJavaScriptLine(line));
      if (braceDepth <= 0 && /\)\s*;?\s*$/u.test(line)) flush();
      return;
    }
    if (current) {
      current.lines.push(line);
      braceDepth += braceDelta(maskJavaScriptLine(line));
      if (braceDepth <= 0 && /^\s*\}\s*\)\s*;?\s*$/u.test(line)) flush();
    }
  });
  flush();
  return violations;
}

function braceDelta(line) {
  let delta = 0;
  for (const char of line) {
    if (char === '{') delta += 1;
    if (char === '}') delta -= 1;
  }
  return delta;
}
