import { scanBoundaryCommandRules } from './source-policy-typescript-source-domain-domain-boundaries-ops-commands.mjs';
import { scanBoundaryTimerRules } from './source-policy-typescript-source-domain-domain-boundaries-ops-timer.mjs';

export function scanBoundaryOperationRules(root, filePath, rel, lines) {
  return [
    ...scanBoundaryTimerRules(root, filePath, rel, lines),
    ...scanBoundaryCommandRules(root, filePath, rel, lines),
  ];
}
