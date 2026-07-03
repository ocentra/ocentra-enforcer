import { scanCommonSecurityManifest } from './source-policy-common-security-manifest.mjs';
import { scanCommonSecurityTestDoubles } from './source-policy-common-security-test-doubles.mjs';
import { scanSensitivePathPolicy } from './source-policy-common-security-sensitive.mjs';

export function scanCommonSecurityAndDoubles(root, filePath, rel, lines, text) {
  return [
    ...scanSensitivePathPolicy(root, filePath, rel),
    ...scanCommonSecurityManifest(root, filePath, rel),
    ...scanCommonSecurityTestDoubles(root, filePath, rel, lines, text),
  ];
}
