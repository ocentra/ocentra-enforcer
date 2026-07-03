import { scanBoundaryOperationRules } from './source-policy-typescript-source-domain-domain-boundaries-ops.mjs';
import { scanBoundaryLanguageRules } from './source-policy-typescript-source-domain-domain-boundaries-language.mjs';

export function scanTypeScriptDomainBoundaryRules(root, filePath, rel, lines) {
  return [
    ...scanBoundaryLanguageRules(root, filePath, rel, lines),
    ...scanBoundaryOperationRules(root, filePath, rel, lines),
  ];
}
