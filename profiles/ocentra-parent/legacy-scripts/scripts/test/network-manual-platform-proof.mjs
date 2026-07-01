import { spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'network-plan-proof', '11-manual-platform-proof');
const testRoot = join('test-results', 'network-manual-platform-proof');
const sourceBranch = runText('git', ['branch', '--show-current']).trim();
const sourceCommit = runText('git', ['rev-parse', 'HEAD']).trim();
const sourceStatusShort = runText('git', ['status', '--short']);

mkdirSync(proofRoot, { recursive: true });
mkdirSync(testRoot, { recursive: true });

const platformMatrix = [
  platformRecord({
    platform: 'Windows',
    capability: 'Npcap live capture observation',
    proofRows: ['13'],
    state:
      'manual-required until host driver, permission, bounded capture, stop, quota, retention, custody, and private-traffic refs are supplied',
    permission: 'Administrator or equivalent Npcap capture permission on a named child device interface',
    manualSteps: [
      'Identify child Windows host, Ocentra agent build, network interface, and Npcap installation version.',
      'Attach driver, interface, permission, bounded-capture, clean-stop, quota, retention/delete/export, custody, and private-traffic-exclusion refs.',
      'Run the live-capture proof gate and retain command log plus host/device proof evidence before claiming capture readiness.',
    ],
  }),
  platformRecord({
    platform: 'Windows',
    capability: 'DNS proxy/block/redirect and Windows Firewall adapter proof boundaries',
    proofRows: ['37', '38'],
    state:
      'manual-required/unavailable unless supported capability, policy, apply/result, rollback, and audit refs are present',
    permission: 'Host DNS configuration or Windows Firewall administrative permission, depending on adapter kind',
    manualSteps: [
      'Name child Windows host, adapter kind, parent rule ref, evidence ref, and policy decision ref.',
      'Attach target/rule, supported capability, adapter authorization, apply/result, rollback, and audit refs.',
      'Keep dry-run/manual/unavailable states non-executable until artifact refs are present and validated.',
    ],
  }),
  platformRecord({
    platform: 'Windows',
    capability: 'WFP signed/permissioned lab proof gate',
    proofRows: ['39'],
    state:
      'manual-required unless signed driver/package, admin permission, provider registration, layer matrix, rollback, lab result, and audit refs are present',
    permission: 'Administrator permission plus signed Windows Filtering Platform driver/package proof',
    manualSteps: [
      'Name child Windows host, WFP target/provider/layer refs, and signed package version.',
      'Attach administrator permission, driver signing/package, provider-registration, layer-capability, rollback, lab-result, and audit refs.',
      'Keep research-only/manual/unavailable states non-executable and do not claim packet blocking without the lab proof pack.',
    ],
  }),
  platformRecord({
    platform: 'Android',
    capability: 'VpnService physical-device proof gate',
    proofRows: ['40'],
    state:
      'manual-required until physical device, VpnService declaration, consent, package identity, interface, traffic observation, rollback, and audit refs are present',
    permission: 'Android VpnService user consent, with Device Owner proof only when that authority is claimed',
    manualSteps: [
      'Name physical child Android device, OS version, package identity, service declaration, and VpnService consent artifact.',
      'Attach virtual-interface, traffic-observation, rollback, audit, and physical-device proof refs.',
      'Attach Device Owner proof only if the product claim uses Device Owner authority.',
    ],
  }),
  platformRecord({
    platform: 'Apple',
    capability: 'Network Extension entitlement/device proof gate',
    proofRows: ['41'],
    state:
      'manual-required until entitlement, provisioning, signing, device/TestFlight, extension configuration, rollback, and audit refs are present',
    permission:
      'Approved Apple Network Extension entitlement and device/TestFlight proof; supervision/MDM proof only when claimed',
    manualSteps: [
      'Name Apple device, OS version, developer team, entitlement approval, provisioning profile, signing, and bundle/extension refs.',
      'Attach device/TestFlight, extension declaration/configuration, rollback, and audit refs.',
      'Attach supervision or MDM proof only if the product claim relies on managed-device authority.',
    ],
  }),
  platformRecord({
    platform: 'Linux',
    capability: 'nftables/eBPF/TUN distro proof gate',
    proofRows: ['42'],
    state:
      'manual-required until distro/kernel, permission, adapter API, service-manager, rollback, lab result, and audit refs are present',
    permission: 'Distro-specific privileged network adapter permission for nftables, eBPF, or TUN',
    manualSteps: [
      'Name distro, kernel, service manager, selected adapter kind, and child host proof.',
      'Attach permission, adapter API capability, adapter plan, service-manager scope, rollback, lab-result, and audit refs.',
      'Keep generic Linux support unavailable until the selected distro/kernel proof pack exists.',
    ],
  }),
];

const commands = [
  cargoTest('network-live-capture-platform-tests', 'live_capture'),
  cargoTest('network-dns-adapter-tests', 'dns_adapter'),
  cargoTest('network-windows-firewall-tests', 'windows_firewall_adapter'),
  cargoTest('network-windows-wfp-tests', 'windows_wfp_gate'),
  cargoTest('network-android-vpnservice-tests', 'android_vpn_service_gate'),
  cargoTest('network-apple-network-extension-tests', 'apple_network_extension_gate'),
  cargoTest('network-linux-adapter-tests', 'linux_adapter_gate'),
  {
    name: 'network-evidence-clippy',
    command: 'cargo',
    args: ['clippy', '-p', 'ocentra-network-evidence', '--all-targets', '--', '-D', 'warnings'],
    log: join(proofRoot, 'clippy.log'),
  },
  {
    name: 'source-shape',
    command: 'node',
    args: ['scripts/check-source-shape.mjs'],
    log: join(proofRoot, 'source-shape.log'),
  },
];
const commandResults = commands.map(runCommand);

writeFileSync(join(proofRoot, 'platform-claim-matrix.json'), `${JSON.stringify(platformMatrix, null, 2)}\n`);

const proof = {
  proof: 'network-manual-platform-proof',
  checkedAt: new Date().toISOString(),
  branch: sourceBranch,
  commit: sourceCommit,
  statusShort: sourceStatusShort,
  proofRoot,
  testRoot,
  commands: commandResults,
  artifacts: {
    platformClaimMatrix: join(proofRoot, 'platform-claim-matrix.json'),
    manualPlatformProof: join(proofRoot, '11-manual-platform-proof.md'),
    proofSummary: join(proofRoot, 'proof-summary.json'),
    testProof: join(testRoot, 'proof.json'),
  },
  coveredRows: [
    '13 Live pcap/Npcap/libpcap capture adapter',
    '37 DNS proxy/block/redirect adapter',
    '38 Windows Firewall adapter',
    '39 Windows WFP research/proof gate',
    '40 Android VpnService adapter/proof gate',
    '41 Apple Network Extension adapter/proof gate',
    '42 Linux nftables/eBPF/TUN adapter/proof gate',
  ],
  platformRecords: platformMatrix,
  provenBoundaries: [
    'platform-specific manual-required and unavailable labels are explicit',
    'OS/device/permission proof refs are named before any platform claim upgrade',
    'required apply/result, rollback, lab/device, and audit refs are enumerated by platform',
    'manual-required, unavailable, research-only, weak-evidence, and missing-artifact paths remain non-executable',
  ],
  notClaimed: [
    'live packet capture driver invocation',
    'host DNS mutation, proxy installation, Windows Firewall mutation, WFP driver install, or packet blocking',
    'Android VPN tunnel/filtering or Device Owner behavior without physical-device proof',
    'Apple Network Extension behavior, supervision, MDM, or app-level control without entitlement/device proof',
    'Linux adapter install, packet filtering, kernel hook load, TUN mutation, or service-manager install',
    'exact URL, page content, private message, search query, or decrypted payload availability',
    'policy authority, adapter action authority, or enforcement command publication',
  ],
  screenshotPolicy:
    'Screenshots are not attached because this slice is a non-UI contract/proof harness; live host or device proof must attach screenshots/logs before platform capability claims are upgraded.',
};

writeFileSync(join(proofRoot, 'proof-summary.json'), `${JSON.stringify(proof, null, 2)}\n`);
writeFileSync(join(testRoot, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
writeFileSync(join(proofRoot, '11-manual-platform-proof.md'), `${renderMarkdownProof(proof)}\n`);

console.log('network-manual-platform-proof-ok:platform-gate-tests,clippy,source-shape');
console.log(`proof=${join(proofRoot, 'proof-summary.json')}`);

function platformRecord({ platform, capability, proofRows, state, permission, manualSteps }) {
  return {
    platform,
    capability,
    proofRows,
    manualRequiredLabel: state,
    requiredPermission: permission,
    exactManualSteps: manualSteps,
    logEvidence: 'command log captured by network-manual-platform-proof harness',
    screenshotEvidence: 'not applicable until a live UI/host/device proof claim is made',
  };
}

function cargoTest(name, filter) {
  return {
    name,
    command: 'cargo',
    args: ['test', '-p', 'ocentra-network-evidence', filter],
    log: join(proofRoot, `${filter.replaceAll('_', '-')}.log`),
  };
}

function runCommand(entry) {
  const result = spawnSync(entry.command, entry.args, { encoding: 'utf8', shell: false });
  writeFileSync(entry.log, `${result.stdout ?? ''}${result.stderr ?? ''}`);
  if (result.status !== 0) {
    throw new Error(`${entry.name} failed with exit ${result.status}`);
  }
  return {
    name: entry.name,
    command: [entry.command, ...entry.args].join(' '),
    status: result.status,
    log: entry.log,
  };
}

function renderMarkdownProof(proof) {
  const rows = proof.platformRecords.flatMap((record) => [
    `## ${record.platform} - ${record.capability}`,
    `Proof rows: ${record.proofRows.join(', ')}`,
    `Manual-required label: ${record.manualRequiredLabel}`,
    `Required permission: ${record.requiredPermission}`,
    'Exact manual steps:',
    ...record.exactManualSteps.map((step, index) => `${index + 1}. ${step}`),
    `Log evidence: ${record.logEvidence}`,
    `Screenshot evidence: ${record.screenshotEvidence}`,
    '',
  ]);
  return [
    '# Network Manual Platform Proof',
    '',
    `Branch: ${proof.branch}`,
    `Source commit: ${proof.commit}`,
    `Source status: ${proof.statusShort.length === 0 ? 'clean' : proof.statusShort}`,
    '',
    'This proof aggregates the existing platform-specific Rust proof gates into the required network-plan row 11 manual/platform proof pack.',
    'It names the OS/device/permission evidence needed before platform claims can move beyond manual-required, unavailable, research-only, or proof-gated state.',
    '',
    ...rows,
    '## Not Claimed',
    ...proof.notClaimed.map((claim) => `- ${claim}`),
    '',
    `Screenshot policy: ${proof.screenshotPolicy}`,
  ].join('\n');
}

function runText(command, args) {
  const result = spawnSync(command, args, { encoding: 'utf8', shell: false });
  if (result.status !== 0) {
    throw new Error(`${command} ${args.join(' ')} failed with exit ${result.status}`);
  }
  return `${result.stdout ?? ''}${result.stderr ?? ''}`;
}
