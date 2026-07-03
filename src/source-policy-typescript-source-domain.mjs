import { scanTypeScriptSourceRules as scanTypeScriptSourceRulesImpl } from './source-policy-typescript-source-domain-rules.mjs';

export function scanTypeScriptSourceRules(root, filePath, rel, lines) {
  return scanTypeScriptSourceRulesImpl(root, filePath, rel, lines);
}
