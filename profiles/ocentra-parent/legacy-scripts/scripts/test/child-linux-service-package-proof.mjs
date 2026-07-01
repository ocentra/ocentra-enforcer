import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';
import { spawn } from 'node:child_process';
import { tsImport } from 'tsx/esm/api';

const repoRoot = process.cwd();
const proofMode = 'child-linux-service-package-proof';
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
    'tests/proof/child-linux-service-package-proof.test.ts',
  ]);

  const sourceProof = await assertLinuxSourceProof();
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
      contract: 'packages/schema-domain/src/child-linux-service-package-proof.ts',
      contractTest: 'packages/schema-domain/tests/proof/child-linux-service-package-proof.test.ts',
      smokeScript: 'scripts/smoke/linux-deb-smoke.sh',
      output: relativePath(proofPath),
    },
    runtimeReadModel,
    scriptWiring,
    childLinuxServicePackageProved: {
      distributionMode:
        'direct-deb-package: the child Linux distribution path is an amd64 Debian package with sha256 sidecars, not a repository channel',
      artifactState:
        'deb-script-defined: the release script stages the child agent binary and systemd unit into a Debian package layout',
      serviceManagerBoundaryState:
        'systemd-boundary-scripted: maintainer scripts reload systemd, enable the service, restart it on install, and stop/disable it on remove',
      checksumState:
        'sha256-sidecar-scripted: the release script writes package checksums and the smoke script verifies them before install',
      distroSupportState:
        'ubuntu-22.04-amd64-glibc-2.35: baseline metadata is explicit and does not generalize to every Linux distribution',
      packageSigningState:
        'unsigned: no debsig, dpkg-sig, GPG, or repository signature step is present in the child Linux package flow',
    },
    childLinuxServiceStillManual: [
      'package install on a real Linux host from the intended package-manager path',
      'systemd service start and health on a real Linux host',
      'restart or crash recovery on a real Linux host beyond the scripted Restart=always policy',
      'signed package or repository distribution artifacts',
      'apt repository promotion or distro feed publication',
      'non-systemd distro support or broader Linux portability beyond the Ubuntu 22.04 amd64 / glibc 2.35 baseline',
      'live uninstall, purge, and daemon cleanup artifacts from a Linux host',
      'parent-client parity or cross-platform readiness claims',
    ],
    nonClaims: [
      'generic Linux support across distros or init systems',
      'signed package distribution or apt repository readiness',
      'live Linux install success, runtime launch success, or crash-free service health',
      'Restart=always as host-proved crash recovery',
      'uninstall, purge, or daemon cleanup success on a Linux host',
      'Windows, macOS, Android, iOS, or parent-client readiness',
    ],
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(`child-linux-service-package-proof-ok:${proofLabels.join(',')}`);
  console.log(`evidence=${proofPath}`);
}

async function assertLinuxSourceProof() {
  const buildScript = await readRepoFile('scripts/release/linux/build-agent-package.sh');
  const serviceUnit = await readRepoFile('scripts/release/linux/ocentra-parent-agent.service');
  const smokeScript = await readRepoFile('scripts/smoke/linux-deb-smoke.sh');

  assertIncludes(buildScript, 'dpkg-deb --build', 'Linux Debian package build command');
  assertIncludes(buildScript, 'cargo build --release -p ocentra-parent-agent-service', 'Linux child binary build');
  assertIncludes(
    buildScript,
    '/opt/ocentra/ocentra-parent-agent/bin/ocentra-parent-agent-service',
    'Linux child binary install path'
  );
  assertIncludes(
    buildScript,
    '/lib/systemd/system/ocentra-parent-agent.service',
    'Linux systemd unit install path'
  );
  assertIncludes(buildScript, 'X-Ocentra-Linux-Baseline', 'Linux baseline metadata');
  assertIncludes(buildScript, 'X-Ocentra-Min-GLIBC', 'Linux minimum glibc metadata');
  assertIncludes(buildScript, 'sha256sum', 'Linux checksum sidecar generation');
  assertIncludes(buildScript, 'systemctl daemon-reload', 'Linux daemon reload path');
  assertIncludes(buildScript, 'systemctl enable ocentra-parent-agent.service', 'Linux enable path');
  assertIncludes(buildScript, 'systemctl restart ocentra-parent-agent.service', 'Linux restart path');
  assertIncludes(buildScript, 'systemctl stop ocentra-parent-agent.service', 'Linux stop path');
  assertIncludes(buildScript, 'systemctl disable ocentra-parent-agent.service', 'Linux disable path');
  assertNotIncludes(buildScript, 'dpkg-sig', 'Linux package signing');
  assertNotIncludes(buildScript, 'debsigs', 'Linux package signing');
  assertNotIncludes(buildScript, 'gpg --detach-sign', 'Linux repository signing');
  assertNotIncludes(buildScript, 'reprepro', 'Linux repository promotion');
  assertNotIncludes(buildScript, 'aptly', 'Linux repository promotion');

  assertIncludes(serviceUnit, 'ExecStart=/opt/ocentra/ocentra-parent-agent/bin/ocentra-parent-agent-service', 'Linux systemd ExecStart');
  assertIncludes(serviceUnit, 'Restart=always', 'Linux systemd restart policy');
  assertIncludes(serviceUnit, 'RestartSec=10', 'Linux systemd restart delay');
  assertIncludes(serviceUnit, 'WantedBy=multi-user.target', 'Linux systemd target');

  assertIncludes(smokeScript, 'sha256sum --check', 'Linux smoke checksum verification');
  assertIncludes(smokeScript, 'sudo dpkg -i', 'Linux smoke install path');
  assertIncludes(smokeScript, 'sudo dpkg -r', 'Linux smoke remove path');
  assertIncludes(smokeScript, 'sudo dpkg -P', 'Linux smoke purge path');
  assertIncludes(smokeScript, 'Skipping install/remove smoke because passwordless sudo is unavailable.', 'Linux smoke manual host fallback');
  assertIncludes(smokeScript, 'Agent executable remained after remove.', 'Linux smoke uninstall cleanup guard');

  proofLabels.push('linux-service-package.source-proof');
  return {
    buildScript: 'scripts/release/linux/build-agent-package.sh',
    serviceUnit: 'scripts/release/linux/ocentra-parent-agent.service',
    smokeScript: 'scripts/smoke/linux-deb-smoke.sh',
  };
}

function buildRuntimeReadModel() {
  return {
    schemaVersion: proofMode,
    packageName: 'ocentra-parent-agent',
    serviceName: 'ocentra-parent-agent.service',
    unitPath: '/lib/systemd/system/ocentra-parent-agent.service',
    binaryPath: '/opt/ocentra/ocentra-parent-agent/bin/ocentra-parent-agent-service',
    distributionMode: 'direct-deb-package',
    artifactState: 'deb-script-defined',
    serviceManagerBoundaryState: 'systemd-boundary-scripted',
    installState: 'dpkg-install-scripted-manual-host-proof',
    runtimeState: 'systemd-start-scripted-manual-host-proof',
    restartState: 'restart-policy-scripted-manual-host-proof',
    checksumState: 'sha256-sidecar-scripted',
    packageSigningState: 'unsigned',
    repositoryState: 'direct-deb-only',
    distroSupportState: 'ubuntu-22.04-amd64-glibc-2.35',
    uninstallState: 'dpkg-remove-scripted-manual-host-proof',
    cleanupState: 'daemon-reload-scripted-manual-host-proof',
    serviceManagerProof: {
      packageName: 'ocentra-parent-agent',
      serviceName: 'ocentra-parent-agent.service',
      unitPath: '/lib/systemd/system/ocentra-parent-agent.service',
      binaryPath: '/opt/ocentra/ocentra-parent-agent/bin/ocentra-parent-agent-service',
      commands: [
        'child.linux.service.package.proof.get',
        'child.linux.service.lifecycle.proof.get',
        'child.linux.service.manual-proof.get',
      ],
      events: [
        'child.linux.service.package.proof.reported',
        'child.linux.service.lifecycle.proof.reported',
        'child.linux.service.manual-proof.reported',
      ],
      runtimeOwner: 'linux-dpkg-maintainer-scripts',
      proofRequirement:
        'Linux child package proof names the direct Debian package path, systemd service boundary, checksum sidecars, and manual-host-only runtime gaps',
      claimBoundary:
        'systemd unit, maintainer scripts, and smoke script prove only the Linux Debian package and service-manager boundary; they do not prove signed distribution, non-systemd hosts, or live runtime health',
    },
    surfaceProofs: surfaceProofs(),
    lifecycleProofs: lifecycleProofs(),
    claimBoundaries: {
      packageArtifact:
        'Debian package script and staged payload prove only the child Linux amd64 artifact layout and maintainer-script boundary',
      distributionBoundary:
        'the child Linux distribution path is a direct .deb artifact with sha256 sidecars; no apt repository, package feed, or production release channel is attached',
      distroBoundary:
        'Linux package proof is limited to Ubuntu 22.04 amd64 with glibc 2.35 baseline metadata and does not imply generic distro-wide or non-systemd support',
      serviceManagerBoundary:
        'systemd unit and maintainer scripts prove only the Linux systemd service-manager boundary for the child agent package',
      runtimeBoundary:
        'source and smoke scripts do not prove installed runtime health, crash-free behavior, or non-systemd host behavior in this proof surface',
      restartBoundary:
        'Restart=always and post-install restart wiring do not prove crash recovery on a real Linux host without service-manager artifacts',
      checksumBoundary:
        'sha256 sidecars are scripted and smoke-verified, but checksum proof does not imply package signing or repository promotion',
      signingBoundary:
        'the child Linux package is unsigned in this proof surface because no debsig, dpkg-sig, GPG, or repository signature artifact is attached',
      uninstallBoundary:
        'prerm stop/disable hooks and smoke remove checks make uninstall expectations explicit, but host uninstall proof remains manual-required without Linux package-manager artifacts',
      cleanupBoundary:
        'postrm daemon-reload and smoke purge checks make daemon cleanup expectations explicit, but live host cleanup proof remains manual-required without Linux artifacts',
      parentParityBoundary:
        'child Linux package proof does not imply parent-client distribution, Windows or macOS readiness, or hidden cross-platform parity claims',
    },
    updatedAt: new Date().toISOString(),
  };
}

async function parseRuntimeReadModel(readModel) {
  const module = await importTsModule('packages/schema-domain/src/child-linux-service-package-proof.ts');
  const parsed = module.ChildLinuxServicePackageProofReadModelSchema.parse(readModel);
  proofLabels.push('schema-domain.child-linux-service-package-proof-parse');
  return parsed;
}

async function assertScriptWiring() {
  const packageJson = JSON.parse(await readRepoFile('package.json'));
  const schemaDomainPackage = JSON.parse(await readRepoFile('packages/schema-domain/package.json'));
  const script = packageJson.scripts['test:child-linux-service-package-proof'];
  if (script !== `node scripts/test/${proofMode}.mjs`) {
    throw new Error('Missing root test:child-linux-service-package-proof script.');
  }
  if (!schemaDomainPackage.exports['./child-linux-service-package-proof']) {
    throw new Error('Missing schema-domain export for ./child-linux-service-package-proof.');
  }
  proofLabels.push('package-scripts.child-linux-service-package-proof');
  return {
    rootScript: 'test:child-linux-service-package-proof',
    schemaDomainExport: './child-linux-service-package-proof',
    sourceContract: 'packages/schema-domain/src/child-linux-service-package-proof.ts',
  };
}

function surfaceProofs() {
  return [
    surfaceProof('deb-build-script', 'package-lifecycle', 'manual-required', 'ci-mechanical-proof', 'linux-deb-build-script'),
    surfaceProof(
      'direct-deb-distribution',
      'package-lifecycle',
      'manual-required',
      'ci-mechanical-proof',
      'linux-deb-build-script'
    ),
    surfaceProof(
      'service-binary-path',
      'headless-agent-service',
      'manual-required',
      'ci-mechanical-proof',
      'linux-release-binary'
    ),
    surfaceProof('systemd-unit', 'headless-agent-service', 'manual-required', 'ci-mechanical-proof', 'linux-systemd-unit'),
    surfaceProof('dpkg-install-path', 'package-lifecycle', 'manual-required', 'ci-mechanical-proof', 'linux-smoke-script'),
    surfaceProof('systemctl-enable', 'headless-agent-service', 'manual-required', 'ci-mechanical-proof', 'linux-dpkg-maintainer-scripts'),
    surfaceProof('systemctl-restart', 'headless-agent-service', 'manual-required', 'ci-mechanical-proof', 'linux-dpkg-maintainer-scripts'),
    surfaceProof('systemctl-stop', 'headless-agent-service', 'manual-required', 'ci-mechanical-proof', 'linux-dpkg-maintainer-scripts'),
    surfaceProof('systemctl-disable', 'headless-agent-service', 'manual-required', 'ci-mechanical-proof', 'linux-dpkg-maintainer-scripts'),
    surfaceProof('daemon-reload-hook', 'headless-agent-service', 'manual-required', 'ci-mechanical-proof', 'linux-dpkg-maintainer-scripts'),
    surfaceProof('checksum-sidecar', 'package-lifecycle', 'manual-required', 'ci-mechanical-proof', 'linux-sha256-sidecar'),
    surfaceProof('signing-review', 'store-distribution', 'manual-required', 'unsigned', 'linux-package-signing'),
    surfaceProof('distro-baseline-review', 'package-lifecycle', 'manual-required', 'ci-mechanical-proof', 'linux-deb-build-script'),
    surfaceProof('uninstall-cleanup-review', 'package-lifecycle', 'manual-required', 'ci-mechanical-proof', 'linux-smoke-script'),
  ];
}

function lifecycleProofs() {
  return [
    lifecycleProof('release-script', 'ci-mechanical-proof', 'linux-deb-build-script'),
    lifecycleProof('binary-stage', 'ci-mechanical-proof', 'linux-release-binary'),
    lifecycleProof('systemd-unit', 'ci-mechanical-proof', 'linux-systemd-unit'),
    lifecycleProof('package-build', 'ci-mechanical-proof', 'linux-deb-build-script'),
    lifecycleProof('checksum-write', 'ci-mechanical-proof', 'linux-sha256-sidecar'),
    lifecycleProof('install-path', 'ci-mechanical-proof', 'linux-smoke-script'),
    lifecycleProof('service-enable', 'ci-mechanical-proof', 'linux-dpkg-maintainer-scripts'),
    lifecycleProof('service-restart', 'ci-mechanical-proof', 'linux-dpkg-maintainer-scripts'),
    lifecycleProof('service-stop', 'ci-mechanical-proof', 'linux-dpkg-maintainer-scripts'),
    lifecycleProof('service-disable', 'ci-mechanical-proof', 'linux-dpkg-maintainer-scripts'),
    lifecycleProof('daemon-reload', 'ci-mechanical-proof', 'linux-dpkg-maintainer-scripts'),
    lifecycleProof('signing-review', 'unsigned', 'linux-package-signing'),
    lifecycleProof('repository-review', 'manual-required', 'linux-manual-proof'),
    lifecycleProof('cleanup-review', 'ci-mechanical-proof', 'linux-smoke-script'),
  ];
}

function surfaceProof(surface, parentCapability, parentCapabilityStatus, proofState, runtimeOwner) {
  const proofRequirement = `${surface} remains ${proofState} until real Linux package artifacts change it`;
  return {
    surface,
    parentCapability,
    parentCapabilityStatus,
    proofState,
    runtimeOwner,
    proofRequirement,
    claimBoundary: proofRequirement,
  };
}

function lifecycleProof(phase, proofState, runtimeOwner) {
  return {
    phase,
    proofState,
    runtimeOwner,
    proofRequirement: `${phase} proof state is ${proofState}`,
    claimBoundary: `${phase} does not upgrade Linux package claims without platform artifacts`,
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

function assertNotIncludes(value, expected, label) {
  if (value.includes(expected)) {
    throw new Error(`${label}: unexpectedly contains ${expected}`);
  }
}

function npmCommand(args) {
  const command = process.platform === 'win32' ? 'cmd' : 'npm';
  const commandArgs = process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
  return [command, commandArgs];
}
