import { scanTypeScriptDomainBoundaryRules } from './source-policy-typescript-source-domain-domain-boundaries.mjs';
import { scanTypeScriptDomainCoreRules } from './source-policy-typescript-source-domain-domain-core.mjs';
import { isToolingBoundaryPath } from './source-policy-paths.mjs';

export function scanTypeScriptDomainRules(root, filePath, rel, lines) {
  if (isToolingBoundaryPath(rel)) return [];
  return [
    ...scanTypeScriptDomainCoreRules(root, filePath, rel, lines),
    ...scanTypeScriptDomainBoundaryRules(root, filePath, rel, lines),
  ];
}
