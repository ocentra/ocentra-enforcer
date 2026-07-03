import { addViolation } from './source-policy-violation.mjs';
import { isForbiddenSensitivePath } from './source-policy-helpers.mjs';

export function scanCommonSecurityManifest(root, filePath, rel) {
  const violations = [];
  if (isForbiddenSensitivePath(rel)) {
    addViolation(violations, root, filePath, 1, 'SEC-1.2', 'forbidden sensitive file path', rel);
  }
  return violations;
}
