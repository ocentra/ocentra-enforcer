import { spawnSync } from 'node:child_process';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = dirname(dirname(dirname(fileURLToPath(import.meta.url))));

const proofScripts = [
  'app-game-source-gated-policy-preview-read-model-proof.mjs',
  'app-game-source-gated-policy-preview-timer-handoff-proof.mjs',
  'app-game-source-gated-policy-preview-timer-status-proof.mjs',
  'app-game-source-gated-policy-preview-timer-runtime-readiness-proof.mjs',
  'app-game-source-gated-policy-preview-timer-scheduler-persistence-proof.mjs',
  'app-game-source-gated-policy-preview-timer-audit-rollback-handoff-proof.mjs',
  'app-game-source-gated-policy-preview-timer-audit-rollback-read-model-proof.mjs',
  'app-game-source-gated-policy-preview-timer-audit-rollback-parent-surface-intent-proof.mjs',
  'app-game-source-gated-policy-preview-timer-service-readiness-handoff-proof.mjs',
  'app-game-source-gated-policy-preview-timer-service-readiness-read-model-proof.mjs',
  'app-game-source-gated-policy-preview-timer-service-readiness-protocol-handoff-proof.mjs',
  'app-game-source-gated-policy-preview-timer-service-readiness-protocol-read-model-proof.mjs',
  'app-game-source-gated-policy-preview-timer-service-readiness-protocol-command-handoff-proof.mjs',
  'app-game-source-gated-policy-preview-timer-service-readiness-service-handler-handoff-proof.mjs',
  'app-game-source-gated-policy-preview-timer-service-readiness-read-api-handoff-proof.mjs',
  'app-game-source-gated-policy-preview-timer-service-readiness-read-api-response-handoff-proof.mjs',
  'app-game-source-gated-policy-preview-timer-service-readiness-read-api-response-consumer-handoff-proof.mjs',
  'app-game-source-gated-policy-preview-timer-service-readiness-response-consumer-parent-surface-handoff-proof.mjs',
  'app-game-source-gated-policy-preview-timer-service-readiness-response-consumer-parent-surface-read-model-handoff-proof.mjs',
  'app-game-source-gated-policy-preview-timer-service-readiness-response-consumer-parent-surface-status-handoff-proof.mjs',
  'app-game-source-gated-policy-preview-timer-service-readiness-response-consumer-parent-surface-status-read-model-handoff-proof.mjs',
  'app-game-source-gated-policy-preview-timer-service-readiness-response-consumer-parent-surface-status-read-model-parent-surface-handoff-proof.mjs',
  'app-game-source-gated-policy-preview-timer-service-readiness-response-consumer-parent-surface-status-read-model-parent-surface-read-model-handoff-proof.mjs',
  'app-game-source-gated-policy-preview-timer-service-readiness-response-consumer-parent-surface-status-read-model-parent-surface-read-model-proof.mjs',
  'app-game-source-gated-policy-preview-timer-service-readiness-response-consumer-parent-surface-status-read-model-parent-surface-read-model-service-handoff-proof.mjs',
  'app-game-source-gated-policy-preview-timer-service-readiness-response-consumer-parent-surface-status-read-model-parent-surface-read-model-service-read-model-handoff-proof.mjs',
  'app-game-timer-service-event-handoff-proof.mjs',
  'app-game-timer-service-read-api-handoff-proof.mjs',
  'app-game-timer-service-read-api-response-handoff-proof.mjs',
  'app-game-timer-service-read-api-response-consumer-handoff-proof.mjs',
  'app-game-timer-service-read-api-response-consumer-parent-surface-handoff-proof.mjs',
];

for (const proofScript of proofScripts) {
  const result = spawnSync(process.execPath, [join(repoRoot, 'scripts', 'test', proofScript)], {
    cwd: repoRoot,
    stdio: 'inherit',
    windowsHide: true,
  });

  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }
}
