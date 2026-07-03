import { scanTypeScriptTestBlocks as scanTypeScriptTestBlocksImpl } from './source-policy-typescript-test-blocks.mjs';
import { scanTypeScriptTestRules as scanTypeScriptTestRulesImpl } from './source-policy-typescript-test-rules.mjs';

export function scanTypeScriptTestBlocks(root, filePath, lines) {
  return scanTypeScriptTestBlocksImpl(root, filePath, lines);
}

export function scanTypeScriptTestRules(root, filePath, rel, lines) {
  return scanTypeScriptTestRulesImpl(root, filePath, rel, lines);
}
