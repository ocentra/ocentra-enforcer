import { scanCommonSecurityAndDoubles as scanCommonSecurityAndDoublesImpl } from './source-policy-common-security-rules.mjs';

export function scanCommonSecurityAndDoubles(root, filePath, rel, lines, text) {
  return scanCommonSecurityAndDoublesImpl(root, filePath, rel, lines, text);
}
