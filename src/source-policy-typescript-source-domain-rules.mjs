import { scanTypeScriptDomainRules } from './source-policy-typescript-source-domain-domain.mjs';
import { scanTypeScriptSourceRulesOnly } from './source-policy-typescript-source-domain-source.mjs';

export function scanTypeScriptSourceRules(root, filePath, rel, lines) {
  return [
    ...scanTypeScriptSourceRulesOnly(root, filePath, rel, lines),
    ...scanTypeScriptDomainRules(root, filePath, rel, lines),
  ];
}
