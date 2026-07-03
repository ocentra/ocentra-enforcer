import { scanAdditionalCommonFile as scanAdditionalCommonFileImpl } from './source-policy-common.mjs';
import { scanAdditionalTypeScriptFile as scanAdditionalTypeScriptFileImpl } from './source-policy-typescript.mjs';

export function scanAdditionalTypeScriptFile(root, filePath) {
  return scanAdditionalTypeScriptFileImpl(root, filePath);
}

export function scanAdditionalCommonFile(root, filePath, lines) {
  return scanAdditionalCommonFileImpl(root, filePath, lines);
}
