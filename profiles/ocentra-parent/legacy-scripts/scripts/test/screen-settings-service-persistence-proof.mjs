import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';

const repoRoot = process.cwd();
const outputRoot = resolve(repoRoot, 'output', 'screen-plan-proof', 'screen-settings-service-persistence');
const proofPath = join(outputRoot, 'proof-summary.json');

const commands = [
  {
    label: 'agent-protocol screen settings contract tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-parent-agent-protocol', 'screen_settings'],
  },
  {
    label: 'agent-service screen settings persistence tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-parent-agent-service', 'screen_settings'],
  },
];

const results = commands.map(runCommand);
const failed = results.filter((result) => result.status !== 0);
assert(
  failed.length === 0,
  `screen settings persistence validation failed: ${failed.map((result) => result.label).join(', ')}`
);

const protocolPath = join(repoRoot, 'crates', 'agent-protocol', 'src', 'screen_settings.rs');
const runtimePath = join(repoRoot, 'crates', 'agent-service', 'src', 'screen_settings_runtime.rs');
const storePath = join(repoRoot, 'crates', 'agent-service', 'src', 'screen_settings_store.rs');
const runtimeTestsPath = join(repoRoot, 'crates', 'agent-service', 'src', 'screen_settings_runtime_tests.rs');

for (const path of [protocolPath, runtimePath, storePath, runtimeTestsPath]) {
  assert(existsSync(path), `missing expected source file: ${path}`);
}

const runtimeTests = readFileSync(runtimeTestsPath, 'utf8');
assert(
  runtimeTests.includes('screen_settings_runtime_persists_parent_opt_in_across_reload'),
  'persistence reload test must remain explicit'
);
assert(
  runtimeTests.includes('screen_settings_runtime_rejects_raw_image_retention'),
  'raw image retention rejection test must remain explicit'
);
assert(
  runtimeTests.includes('ScreenSettingsRejectionReason::PolicyModeInconsistent'),
  'observe-only policy-use rejection must remain explicit'
);

const summary = {
  proof: 'screen-settings-service-persistence',
  generatedAt: new Date().toISOString(),
  branchScope: 'codex/screen-ai-full-scope-b',
  sourceFiles: [
    relativePath(protocolPath),
    relativePath(runtimePath),
    relativePath(storePath),
    relativePath(runtimeTestsPath),
  ],
  validation: results.map((result) => ({
    label: result.label,
    command: [result.command, ...result.args].join(' '),
    status: result.status,
    stdoutTail: tail(result.stdout),
    stderrTail: tail(result.stderr),
  })),
  proves: [
    'Rust protocol mirrors parent screen settings fields used by the TypeScript ScreenAnalysisParentSettingSchema boundary.',
    'The service runtime reports a disabled default without silently enabling capture, trigger capture, policy use, or raw image retention.',
    'A parent opt-in strict dry-run setting is persisted to a local JSON service store and survives a reload through a new runtime instance.',
    'The service rejects raw image retention, observe-only policy use, stale base setting versions, and inconsistent unsafe settings before persistence.',
  ],
  custody: {
    store: 'local child-device JSON service store',
    rawImageRetainedDefault: false,
    deleteAfterSuccessRequired: true,
    deleteAfterExpiryRequired: true,
    ocentraHostedDefaultStore: false,
  },
  nonClaims: [
    'This proof does not wire the parent portal Settings route to the service command path.',
    'This proof does not enable raw screenshot retention, live view, raw remote upload, or cloud AI.',
    'This proof does not claim production retention-control UI, privacy/legal approval, or live capture behavior changes.',
  ],
};

mkdirSync(outputRoot, { recursive: true });
writeFileSync(proofPath, `${JSON.stringify(summary, null, 2)}\n`);
console.log(`screen-settings-service-persistence-proof-ok:${proofPath}`);

function runCommand(commandSpec) {
  const result = spawnSync(commandSpec.command, commandSpec.args, {
    cwd: repoRoot,
    encoding: 'utf8',
    shell: false,
  });
  return {
    ...commandSpec,
    status: result.status ?? 1,
    stdout: result.stdout ?? '',
    stderr: result.stderr ?? '',
  };
}

function tail(text) {
  return text.split(/\r?\n/u).filter(Boolean).slice(-12);
}

function relativePath(path) {
  return path.replace(`${repoRoot}\\`, '').replaceAll('\\', '/');
}

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}
