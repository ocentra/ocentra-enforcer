import fs from 'node:fs';
import path from 'node:path';
import { addViolation } from './source-policy-violation.mjs';
import { readLines } from './source-policy-helpers.mjs';

export function scanPackageManifestLockfiles(root, filePath) {
  let parsed;
  try {
    parsed = JSON.parse(readLines(filePath).join('\n'));
  } catch {
    return [];
  }
  void parsed;
  const dir = path.dirname(filePath);
  const lockfiles = ['package-lock.json', 'npm-shrinkwrap.json', 'pnpm-lock.yaml', 'yarn.lock'];
  const presentLocks = lockfiles.filter((name) => fs.existsSync(path.join(dir, name)));
  const violations = [];
  if (presentLocks.length === 0) {
    addViolation(violations, root, filePath, 1, 'TS-7.10', 'package manager lockfile missing', path.basename(filePath));
  }
  if (presentLocks.length > 1) {
    addViolation(violations, root, filePath, 1, 'TS-7.15', `multiple lockfiles found: ${presentLocks.join(', ')}`, path.basename(filePath));
  }
  return violations;
}
