import { addViolation } from './source-policy-violation.mjs';

export function scanPackageManifestDependencySection(root, filePath, parsed, section, shouldFlag, ruleId, buildDetail) {
  const dependencies = parsed[section];
  if (dependencies == null || typeof dependencies !== 'object') return [];
  const violations = [];
  for (const [name, version] of Object.entries(dependencies)) {
    if (typeof version !== 'string') continue;
    if (!shouldFlag(name, version)) continue;
    addViolation(violations, root, filePath, 1, ruleId, buildDetail(name, version, section), `${name}: ${version}`);
  }
  return violations;
}
