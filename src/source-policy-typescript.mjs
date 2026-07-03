import path from 'node:path';
import { normalizeRel } from './path-utils.mjs';
import { readLines } from './source-policy-helpers.mjs';
import {
  scanPackageManifestForTypeScriptPolicy,
  scanTsConfigStrictness,
} from './source-policy-typescript-manifest.mjs';
import { scanTypeScriptTestBlocks } from './source-policy-typescript-tests.mjs';
import { scanTypeScriptLineRules } from './source-policy-typescript-lines.mjs';

export function scanAdditionalTypeScriptFile(root, filePath) {
  const rel = normalizeRel(root, filePath);
  const lines = readLines(filePath);
  const violations = [];
  violations.push(...scanTypeScriptLineRules(root, filePath, rel, lines));
  if (path.basename(rel) === 'package.json') violations.push(...scanPackageManifestForTypeScriptPolicy(root, filePath));
  if (/^tsconfig(?:\.[^.]+)?\.json$/iu.test(path.basename(rel))) violations.push(...scanTsConfigStrictness(root, filePath));
  if (/^(?:test|tests|spec|__tests__)$/iu.test(path.dirname(rel).split(/[\\/]/u).pop() ?? '')) violations.push(...scanTypeScriptTestBlocks(root, filePath, lines));
  return violations;
}
