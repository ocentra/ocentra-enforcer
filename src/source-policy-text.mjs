import { maskMultilineTemplates } from './source-policy-template-mask.mjs';

export function maskJavaScriptLine(line) {
  return line
    .replace(/\/\/.*$/u, '')
    .replace(/'(?:[^'\\]|\\.)*'/gu, "''")
    .replace(/"(?:[^"\\]|\\.)*"/gu, '""')
    .replace(/`(?:[^`\\]|\\.)*`/gu, '``');
}

export function maskJavaScriptLines(lines) {
  return maskMultilineTemplates(lines, maskJavaScriptLine);
}

export function maskJavaScriptTemplateBodies(lines) {
  return maskMultilineTemplates(lines, (segment) => segment);
}
