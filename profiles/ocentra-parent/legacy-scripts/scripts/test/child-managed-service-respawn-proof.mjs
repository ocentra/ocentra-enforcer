import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';
import { spawn } from 'node:child_process';
import { tsImport } from 'tsx/esm/api';

const repoRoot = process.cwd();
const proofMode = 'child-managed-service-respawn-proof';
const outputDir = join(repoRoot, 'test-results', proofMode);
const proofPath = join(outputDir, 'proof.json');
const commands = [];
const proofLabels = [];

await main();

async function main() {
  await mkdir(outputDir, { recursive: true });

  await runNpm([
    'exec',
    '--workspace',
    '@ocentra-parent/schema-domain',
    '--',
    'vitest',
    'run',
    'tests/proof/child-managed-service-respawn-proof.test.ts',
  ]);

  const sourceProof = {
    windows: await assertWindowsSourceProof(),
    macos: await assertMacosSourceProof(),
    linux: await assertLinuxSourceProof(),
    android: await assertAndroidManualState(),
    ios: await assertIosUnsupportedState(),
  };
  const runtimeReadModel = await parseRuntimeReadModel(buildRuntimeReadModel());
  const scriptWiring = await assertScriptWiring();

  const proof = {
    schemaVersion: 1,
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    proofMode,
    commands,
    proofLabels,
    evidence: {
      sourceProof,
      contract: 'packages/schema-domain/src/child-managed-service-respawn-proof.ts',
      contractTest: 'packages/schema-domain/tests/proof/child-managed-service-respawn-proof.test.ts',
      output: relativePath(proofPath),
    },
    runtimeReadModel,
    scriptWiring,
    supportedPlatformRespawn: {
      windows:
        'ci-mechanical-proof: WinSW service XML, WiX service control, and lifecycle cleanup keep kill/reboot/service-manager restart explicit while deliberate stop stays manual-required.',
      macos:
        'ci-mechanical-proof: launchd KeepAlive and RunAtLoad plus bootstrap and bootout package scripts keep restart support and teardown boundaries explicit.',
      linux:
        'ci-mechanical-proof: systemd Restart=always, enable/restart package hooks, and prerm stop/disable keep respawn support and teardown boundaries explicit.',
    },
    manualOrUnsupportedPlatformStates: {
      android:
        'manual-required: foreground service, kill recovery, reboot recovery, stop path, and uninstall behavior still need emulator or physical-device lifecycle artifacts.',
      ios:
        'unsupported: the child iOS package is capability-only and does not claim a managed background service respawn surface.',
    },
    nonClaims: [
      'live post-install runtime health on Windows, macOS, or Linux hosts',
      'desktop stop requests silently auto-respawning without operator action',
      'Android foreground-service runtime parity or reboot survival',
      'iOS persistent background daemon, managed service, or desktop-style restart behavior',
      'parent client update, rollback, or installer proof closing child respawn claims',
    ],
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(`child-managed-service-respawn-proof-ok:${proofLabels.join(',')}`);
  console.log(`evidence=${proofPath}`);
}

async function assertWindowsSourceProof() {
  const serviceXml = await readRepoFile('scripts/release/windows/OcentraParentAgentService.xml');
  const wix = await readRepoFile('scripts/release/windows/OcentraParentAgent.wxs');
  const lifecycleHost = await readRepoFile('scripts/release/windows/package-lifecycle-host.mjs');

  assertIncludes(serviceXml, '<startmode>Automatic</startmode>', 'Windows service auto start');
  assertIncludes(serviceXml, '<onfailure action="restart" delay="10 sec" />', 'Windows service first restart');
  assertIncludes(serviceXml, '<onfailure action="restart" delay="30 sec" />', 'Windows service second restart');
  assertIncludes(wix, 'FirstFailureActionType="restart"', 'Windows WiX first failure restart');
  assertIncludes(wix, 'SecondFailureActionType="restart"', 'Windows WiX second failure restart');
  assertIncludes(wix, 'ThirdFailureActionType="restart"', 'Windows WiX third failure restart');
  assertIncludes(wix, 'Stop="both"', 'Windows explicit stop path');
  assertIncludes(wix, 'Remove="uninstall"', 'Windows explicit remove path');
  assertIncludes(lifecycleHost, "lifecycle.uninstall.status = 'uninstalled'", 'Windows uninstall lifecycle status');
  assertIncludes(lifecycleHost, 'assertCleanup(lifecycle);', 'Windows uninstall cleanup assertion');
  assertIncludes(
    lifecycleHost,
    'service-remained-after-uninstall',
    'Windows service teardown failure detection'
  );
  assertIncludes(
    lifecycleHost,
    'process-remained-after-uninstall',
    'Windows process teardown failure detection'
  );

  proofLabels.push('windows.child-respawn-source-proof');
  return {
    serviceXml: 'scripts/release/windows/OcentraParentAgentService.xml',
    wix: 'scripts/release/windows/OcentraParentAgent.wxs',
    lifecycleHost: 'scripts/release/windows/package-lifecycle-host.mjs',
  };
}

async function assertMacosSourceProof() {
  const buildScript = await readRepoFile('scripts/release/macos/build-agent-package.sh');
  const plist = await readRepoFile('scripts/release/macos/ca.ocentra.parent.agent.plist');

  assertIncludes(plist, '<key>RunAtLoad</key>', 'macOS launchd RunAtLoad key');
  assertIncludes(plist, '<key>KeepAlive</key>', 'macOS launchd KeepAlive key');
  assertIncludes(buildScript, 'launchctl bootstrap system /Library/LaunchDaemons/ca.ocentra.parent.agent.plist', 'macOS bootstrap');
  assertIncludes(buildScript, 'launchctl enable system/ca.ocentra.parent.agent', 'macOS enable');
  assertIncludes(buildScript, 'launchctl bootout system /Library/LaunchDaemons/ca.ocentra.parent.agent.plist', 'macOS bootout');
  assertIncludes(buildScript, "cat > \"$scripts_root/preinstall\"", 'macOS explicit preinstall stop path');
  assertIncludes(buildScript, "cat > \"$scripts_root/postinstall\"", 'macOS explicit postinstall restart path');

  proofLabels.push('macos.child-respawn-source-proof');
  return {
    buildScript: 'scripts/release/macos/build-agent-package.sh',
    plist: 'scripts/release/macos/ca.ocentra.parent.agent.plist',
  };
}

async function assertLinuxSourceProof() {
  const buildScript = await readRepoFile('scripts/release/linux/build-agent-package.sh');
  const serviceUnit = await readRepoFile('scripts/release/linux/ocentra-parent-agent.service');

  assertIncludes(serviceUnit, 'Restart=always', 'Linux systemd restart policy');
  assertIncludes(serviceUnit, 'RestartSec=10', 'Linux restart delay');
  assertIncludes(serviceUnit, 'WantedBy=multi-user.target', 'Linux boot enable target');
  assertIncludes(buildScript, 'systemctl enable ocentra-parent-agent.service', 'Linux enable path');
  assertIncludes(buildScript, 'systemctl restart ocentra-parent-agent.service', 'Linux restart path');
  assertIncludes(buildScript, 'systemctl stop ocentra-parent-agent.service', 'Linux stop path');
  assertIncludes(buildScript, 'systemctl disable ocentra-parent-agent.service', 'Linux disable path');
  assertIncludes(buildScript, 'systemctl daemon-reload', 'Linux daemon reload teardown');

  proofLabels.push('linux.child-respawn-source-proof');
  return {
    buildScript: 'scripts/release/linux/build-agent-package.sh',
    serviceUnit: 'scripts/release/linux/ocentra-parent-agent.service',
  };
}

async function assertAndroidManualState() {
  const androidReadme = await readRepoFile('platforms/android/README.md');
  const androidLifecycleProof = await readRepoFile('packages/schema-domain/src/child-android-lifecycle-proof.ts');

  assertIncludes(
    androidReadme,
    'Current aggregate state is scaffold/manual-required/not-implemented',
    'Android aggregate manual state'
  );
  assertIncludes(androidReadme, 'Child-agent runtime parity is manual-required.', 'Android runtime parity manual state');
  assertIncludes(androidLifecycleProof, "'reboot-recovery'", 'Android reboot recovery row');
  assertIncludes(androidLifecycleProof, 'RequiredManualPackagePhases', 'Android manual package phases');
  assertIncludes(androidLifecycleProof, "proof.deviceOwnerAuthorityState === 'manual-required'", 'Android device owner manual state');
  assertIncludes(androidLifecycleProof, "proof.removalStateBoundary.includes('manual-required')", 'Android removal manual state');

  proofLabels.push('android.child-respawn-manual-boundary');
  return {
    readme: 'platforms/android/README.md',
    lifecycleContract: 'packages/schema-domain/src/child-android-lifecycle-proof.ts',
  };
}

async function assertIosUnsupportedState() {
  const iosReadme = await readRepoFile('platforms/ios/README.md');
  const statusView = await readRepoFile('platforms/ios/OcentraParentAgent/AgentStatusViewController.swift');
  const iosContract = await readRepoFile('packages/schema-domain/src/child-ios-entitlement-capability-proof.ts');

  assertIncludes(iosReadme, 'Current aggregate state is simulator/manual-required/planned', 'iOS aggregate manual state');
  assertIncludes(iosReadme, 'Entitlement and real-device proof are manual-required.', 'iOS manual device proof');
  assertIncludes(statusView, 'service-mode=capability-only', 'iOS capability-only status surface');
  assertIncludes(statusView, 'background-execution=manual-required', 'iOS background execution manual state');
  assertIncludes(
    iosContract,
    'no hidden daemon or persistent background service is claimed',
    'iOS no daemon boundary'
  );
  assertIncludes(iosContract, 'background execution remains manual-required', 'iOS background execution boundary');

  proofLabels.push('ios.child-respawn-unsupported-boundary');
  return {
    readme: 'platforms/ios/README.md',
    statusView: 'platforms/ios/OcentraParentAgent/AgentStatusViewController.swift',
    entitlementContract: 'packages/schema-domain/src/child-ios-entitlement-capability-proof.ts',
  };
}

function buildRuntimeReadModel() {
  return {
    schemaVersion: proofMode,
    platformProofs: [
      platformProof(
        'windows',
        'winsw-service',
        'ci-mechanical-proof',
        'windows-service-installer',
        'proved',
        'proved',
        'proved',
        'manual-required',
        'proved',
        'proved',
        'proved',
        'WinSW service XML, WiX service install, and Windows lifecycle cleanup prove auto-start, failure restart, and uninstall stop/remove boundaries for the child agent service.',
        'Windows MSI uninstall stops and removes the child agent service, then lifecycle cleanup asserts no child agent service processes remain.',
        'Windows respawn proof is service-manager configuration only; it does not claim live post-install runtime health beyond the named package lifecycle surfaces.',
        [
          'scripts/release/windows/OcentraParentAgentService.xml',
          'scripts/release/windows/OcentraParentAgent.wxs',
          'scripts/release/windows/package-lifecycle-host.mjs',
        ]
      ),
      platformProof(
        'macos',
        'launchd-daemon',
        'ci-mechanical-proof',
        'macos-launchd-package',
        'proved',
        'proved',
        'proved',
        'manual-required',
        'proved',
        'proved',
        'proved',
        'launchd plist and package install scripts prove RunAtLoad, KeepAlive, bootstrap, and enable behavior for the child agent daemon.',
        'macOS package preinstall and postinstall bootout commands make the stop path explicit instead of hiding teardown behind respawn language.',
        'macOS respawn proof is launchd/package configuration only; it does not claim notarization, live host install success, or non-launchd runtime health.',
        ['scripts/release/macos/build-agent-package.sh', 'scripts/release/macos/ca.ocentra.parent.agent.plist']
      ),
      platformProof(
        'linux',
        'systemd-service',
        'ci-mechanical-proof',
        'linux-systemd-package',
        'proved',
        'proved',
        'proved',
        'manual-required',
        'proved',
        'proved',
        'proved',
        'systemd unit and Debian package scripts prove Restart=always, boot enablement, and restart wiring for the child agent service.',
        'Linux package prerm and postrm scripts stop, disable, and reload systemd so the stop path remains explicit and testable.',
        'Linux respawn proof is systemd/package configuration only; it does not claim non-systemd hosts, baseline portability, or live runtime health beyond this slice.',
        ['scripts/release/linux/build-agent-package.sh', 'scripts/release/linux/ocentra-parent-agent.service']
      ),
      platformProof(
        'android',
        'android-foreground-service',
        'manual-required',
        'android-device-proof',
        'manual-required',
        'manual-required',
        'manual-required',
        'manual-required',
        'manual-required',
        'unsupported',
        'manual-required',
        'Android child-agent package proof keeps foreground service and reboot recovery manual-required until emulator or physical-device artifacts exist.',
        'Android stop, reboot, uninstall, and restart survival remain manual-required because this slice has no real device lifecycle artifacts.',
        'Android does not reuse desktop service-manager proof; foreground-service runtime parity stays manual-required until device proof exists.',
        ['platforms/android/README.md', 'packages/schema-domain/src/child-android-lifecycle-proof.ts']
      ),
      platformProof(
        'ios',
        'ios-capability-surface',
        'unsupported',
        'ios-capability-package',
        'unsupported',
        'unsupported',
        'unsupported',
        'unsupported',
        'unsupported',
        'unsupported',
        'unsupported',
        'iOS child-agent proof is capability-only; no persistent background daemon or managed service respawn surface is claimed.',
        'iOS capability packaging does not expose a managed service stop or respawn path in this slice.',
        'iOS cannot reuse desktop service-manager or Android foreground-service proof; managed service respawn is unsupported here.',
        [
          'platforms/ios/README.md',
          'platforms/ios/OcentraParentAgent/AgentStatusViewController.swift',
          'packages/schema-domain/src/child-ios-entitlement-capability-proof.ts',
        ]
      ),
    ],
    claimBoundaries: {
      desktopServiceManagers:
        'Only Windows WinSW, macOS launchd, and Linux systemd rows claim managed respawn in this slice.',
      stopPathNegativeCases:
        'Supported desktop rows keep deliberate stop and teardown paths explicit instead of treating them as silent respawn.',
      mobileNoReuse:
        'Android stays manual-required and iOS stays unsupported; mobile rows do not inherit desktop respawn support.',
      parentProofSeparation:
        'Parent client update, rollback, installer, or release proofs do not close child managed service respawn claims.',
      runtimeHealthSeparation:
        'Service-manager configuration proof does not claim live post-install runtime health beyond the named package and lifecycle surfaces.',
    },
    updatedAt: new Date().toISOString(),
  };
}

async function parseRuntimeReadModel(readModel) {
  const module = await importTsModule('packages/schema-domain/src/child-managed-service-respawn-proof.ts');
  const parsed = module.ChildManagedServiceRespawnReadModelSchema.parse(readModel);
  proofLabels.push('schema-domain.child-managed-service-respawn-proof-parse');
  return parsed;
}

async function assertScriptWiring() {
  const packageJson = JSON.parse(await readRepoFile('package.json'));
  const schemaDomainPackage = JSON.parse(await readRepoFile('packages/schema-domain/package.json'));
  const script = packageJson.scripts['test:child-managed-service-respawn-proof'];
  if (script !== `node scripts/test/${proofMode}.mjs`) {
    throw new Error('Missing root test:child-managed-service-respawn-proof script.');
  }
  if (!schemaDomainPackage.exports['./child-managed-service-respawn-proof']) {
    throw new Error('Missing schema-domain export for ./child-managed-service-respawn-proof.');
  }
  proofLabels.push('package-scripts.child-managed-service-respawn-proof');
  return {
    rootScript: 'test:child-managed-service-respawn-proof',
    schemaDomainExport: './child-managed-service-respawn-proof',
    sourceContract: 'packages/schema-domain/src/child-managed-service-respawn-proof.ts',
  };
}

function platformProof(
  platform,
  supervisor,
  proofState,
  runtimeOwner,
  respawnState,
  restartSurvivalState,
  killRecoveryState,
  stopRecoveryState,
  rebootRecoveryState,
  serviceManagerRestartState,
  teardownState,
  proofRequirement,
  teardownRequirement,
  claimBoundary,
  sourceRefs
) {
  return {
    platform,
    supervisor,
    proofState,
    runtimeOwner,
    respawnState,
    restartSurvivalState,
    killRecoveryState,
    stopRecoveryState,
    rebootRecoveryState,
    serviceManagerRestartState,
    teardownState,
    proofRequirement,
    teardownRequirement,
    claimBoundary,
    sourceRefs,
  };
}

async function readRepoFile(path) {
  return readFile(join(repoRoot, path), 'utf8');
}

async function runNpm(args) {
  await runCommand(...npmCommand([...args]));
}

async function runCommand(commandName, args) {
  commands.push([commandName, ...args].join(' '));
  await new Promise((resolve, reject) => {
    const child = spawn(commandName, args, { cwd: repoRoot, stdio: 'inherit', windowsHide: true });
    child.once('exit', (code) =>
      code === 0 ? resolve() : reject(new Error(`${commandName} ${args.join(' ')} exited with ${code}`))
    );
    child.once('error', reject);
  });
}

async function importTsModule(relativePath) {
  return tsImport(pathToFileURL(join(repoRoot, relativePath)).href, import.meta.url);
}

async function gitHead() {
  const chunks = [];
  await new Promise((resolve, reject) => {
    const child = spawn('git', ['rev-parse', 'HEAD'], { cwd: repoRoot, stdio: ['ignore', 'pipe', 'pipe'] });
    child.stdout.on('data', (chunk) => chunks.push(String(chunk)));
    child.once('exit', (code) => (code === 0 ? resolve() : reject(new Error('git rev-parse HEAD failed'))));
    child.once('error', reject);
  });
  return chunks.join('').trim();
}

function relativePath(path) {
  return relative(repoRoot, path).replaceAll('\\', '/');
}

function assertIncludes(value, expected, label) {
  if (!value.includes(expected)) {
    throw new Error(`${label}: missing ${expected}`);
  }
}

function npmCommand(args) {
  const command = process.platform === 'win32' ? 'cmd' : 'npm';
  const commandArgs = process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
  return [command, commandArgs];
}
