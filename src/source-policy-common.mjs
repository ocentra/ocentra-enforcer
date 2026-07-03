import path from 'node:path';
import { normalizeRel } from './path-utils.mjs';
import { scanPackageManifestForTypeScriptPolicy } from './source-policy-typescript-manifest.mjs';
import { scanCommonPolicyFiles } from './source-policy-common-policy.mjs';
import { scanCommonSecurityAndDoubles } from './source-policy-common-security.mjs';
import { isForbiddenSensitivePath } from './source-policy-helpers.mjs';
import { addViolation } from './source-policy-violation.mjs';

export function scanAdditionalCommonFile(root, filePath, lines) {
  const rel = normalizeRel(root, filePath);
  const text = lines.join('\n');
  const violations = [];

  if (isForbiddenSensitivePath(rel)) {
    addViolation(violations, root, filePath, 1, 'SEC-1.2', 'forbidden sensitive file path', rel);
  }

  if (path.basename(rel) === 'package.json') {
    violations.push(...scanPackageManifestForTypeScriptPolicy(root, filePath));
    violations.push(...scanCommonPolicyFiles(root, filePath, rel, text));
  }

  if (path.basename(rel) !== 'package.json') {
    violations.push(...scanCommonPolicyFiles(root, filePath, rel, text));
  }
  violations.push(...scanCommonSecurityAndDoubles(root, filePath, rel, lines, text));

  return violations;
}
