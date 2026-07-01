import { mkdir, readdir, readFile, stat, writeFile } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import { join, relative } from 'node:path';

const root = process.cwd();
const outputDirectory = join(root, 'output', 'browser-plan-proof', '23-e2e-and-manual-proof-artifacts');
const proofDirectory = join(root, 'test-results', 'browser-plan-e2e-manual-proof-artifacts');

await main();

async function main() {
  await mkdir(outputDirectory, { recursive: true });
  await mkdir(proofDirectory, { recursive: true });

  const evidence = {
    profileMatrix: await latestTopLevelJson('test-results/managed-browser-profile-matrix'),
    intervention: await latestTopLevelJson('test-results/managed-browser-intervention-proof'),
    serviceProof: await latestTopLevelJson('test-results/managed-browser-service-proof'),
    windowsEnforcement: await latestTopLevelJson('test-results/windows-managed-unmanaged-browser-enforcement-proof'),
    domainAdapter: await jsonFile('test-results/v0-8-browser-domain-adapter-proof/proof.json'),
    performanceHealth: await jsonFile('test-results/browser-performance-health-proof/proof.json'),
    profileScreenshots: await filesWithExtension('test-results/managed-browser-profile-matrix', '.png'),
    interventionScreenshots: await filesWithExtension('test-results/managed-browser-intervention-proof', '.png'),
  };

  const manifest = {
    schemaVersion: 1,
    proofMode: 'browser-plan-e2e-manual-proof-artifacts',
    generatedAt: new Date().toISOString(),
    sourceDirectories: [
      'test-results/managed-browser-profile-matrix',
      'test-results/managed-browser-service-proof',
      'test-results/managed-browser-intervention-proof',
      'test-results/windows-managed-unmanaged-browser-enforcement-proof',
      'test-results/v0-8-browser-domain-adapter-proof',
      'test-results/browser-performance-health-proof',
    ],
    rows: artifactRows(evidence),
  };
  manifest.summary = summarizeRows(manifest.rows);

  const proofPath = join(proofDirectory, 'proof.json');
  const markdownPath = join(outputDirectory, 'artifact-manifest.md');
  await writeFile(proofPath, `${JSON.stringify(manifest, null, 2)}\n`);
  await writeFile(markdownPath, `${manifestMarkdown(manifest)}\n`);

  console.log('browser-plan-e2e-manual-proof-artifacts-ok=true');
  console.log(`proof=${relativePath(proofPath)}`);
  console.log(`manifest=${relativePath(markdownPath)}`);
  console.log(
    `present=${manifest.summary.artifactPresentRows} partial=${manifest.summary.partialManualRequiredRows} manualRequired=${manifest.summary.manualRequiredRows}`
  );
}

function artifactRows(evidence) {
  const profileFamilies = browserFamilies(evidence.profileMatrix?.data?.supportedBrowsers);
  const interventionFamilies = browserFamilies(evidence.intervention?.data?.browsers);
  const profileScreenshots = evidence.profileScreenshots;
  const interventionScreenshots = evidence.interventionScreenshots;
  const domainAdapterPresent = evidence.domainAdapter !== undefined;
  const performancePresent = evidence.performanceHealth !== undefined;

  return [
    claimRow({
      id: 'managed-edge-evidence',
      label: 'Managed Edge evidence',
      present: profileFamilies.has('edge'),
      artifacts: [
        jsonArtifact('profile-matrix-json', evidence.profileMatrix),
        countedArtifact(
          'profile-matrix-screenshots',
          profileScreenshots.filter((item) => item.includes('edge'))
        ),
      ],
      manualRequiredReason:
        'Run managed-browser profile matrix on a Windows host with Edge installed before release evidence claims.',
      claimsProved: ['Edge managed profile artifact is indexed when profile-matrix proof exists.'],
      claimsNotProved: ['Release support for every Edge channel requires fresh manual platform proof.'],
    }),
    claimRow({
      id: 'managed-chrome-evidence',
      label: 'Managed Chrome or Chrome for Testing evidence',
      present: profileFamilies.has('chrome'),
      artifacts: [
        jsonArtifact('profile-matrix-json', evidence.profileMatrix),
        countedArtifact(
          'profile-matrix-screenshots',
          profileScreenshots.filter((item) => item.includes('chrome'))
        ),
      ],
      manualRequiredReason:
        'Run managed-browser profile matrix on a Windows host with Chrome or Chrome for Testing installed before release evidence claims.',
      claimsProved: ['Chrome managed profile artifact is indexed when profile-matrix proof exists.'],
      claimsNotProved: [
        'Chrome for Testing channel coverage requires separate manual platform proof when not in the run.',
      ],
    }),
    claimRow({
      id: 'unmanaged-chrome-bypass',
      label: 'Unmanaged Chrome bypass',
      present: evidence.windowsEnforcement !== undefined,
      artifacts: [jsonArtifact('windows-managed-unmanaged-proof-json', evidence.windowsEnforcement)],
      manualRequiredReason:
        'Run windows-managed-unmanaged-browser-enforcement proof on a Windows host before unmanaged bypass release claims.',
      claimsProved: ['Windows managed/unmanaged proof JSON is indexed when present.'],
      claimsNotProved: ['Host network/domain blocking and unmanaged exact URL evidence remain unclaimed.'],
    }),
    claimRow({
      id: 'bridge-disconnect-stale',
      label: 'Bridge disconnect stale state',
      present: evidence.serviceProof !== undefined,
      artifacts: [jsonArtifact('managed-browser-service-proof-json', evidence.serviceProof)],
      manualRequiredReason:
        'Run managed-browser service proof with bridge disconnect/stale scenario before claiming live stale-state evidence.',
      claimsProved: ['Managed service proof JSON is indexed when present.'],
      claimsNotProved: ['Fresh disconnect timing is not claimed without a current service proof artifact.'],
    }),
    claimRow({
      id: 'policy-dry-run',
      label: 'Policy dry-run and adapter proof',
      present: domainAdapterPresent,
      artifacts: [jsonArtifact('v0-8-browser-domain-adapter-proof-json', evidence.domainAdapter)],
      manualRequiredReason: 'Run v0-8 browser domain adapter proof before dry-run claims.',
      claimsProved: ['Domain-adapter proof JSON is indexed when present.'],
      claimsNotProved: ['Dry-run proof is not active enforcement authority.'],
    }),
    claimRow({
      id: 'managed-block-page',
      label: 'Managed block page',
      present: evidence.intervention !== undefined && interventionScreenshots.length > 0,
      artifacts: [
        jsonArtifact('managed-browser-intervention-proof-json', evidence.intervention),
        countedArtifact('intervention-screenshots', interventionScreenshots),
      ],
      manualRequiredReason:
        'Run managed-browser intervention proof on a host with supported browsers before block-page screenshot claims.',
      claimsProved: ['Managed block/warn/hold screenshot artifacts are indexed when intervention proof exists.'],
      claimsNotProved: ['C-owned final child UX polish is not claimed by this artifact index.'],
    }),
    partialRow({
      id: 'url-video-intelligence',
      label: 'URL/video intelligence classification',
      artifacts: [
        jsonArtifact('managed-browser-intervention-proof-json', evidence.intervention),
        countedArtifact(
          'video-route-screenshots',
          interventionScreenshots.filter((item) => item.includes('youtube') || item.includes('video'))
        ),
      ],
      manualRequiredReason:
        'Model/provider classification, policy decision, action, and degraded-state artifacts are manual-required until URL/video intelligence exists.',
      partialPresent: evidence.intervention !== undefined,
      claimsProved: ['Route-level intervention artifacts can be indexed when present.'],
      claimsNotProved: ['Semantic video classification quality and provider/model evidence are not claimed.'],
    }),
    partialRow({
      id: 'social-signup-account-feed',
      label: 'Social signup/account/feed proof',
      artifacts: [
        jsonArtifact('managed-browser-intervention-proof-json', evidence.intervention),
        countedArtifact(
          'social-route-screenshots',
          interventionScreenshots.filter((item) => item.includes('social') || item.includes('signup'))
        ),
      ],
      manualRequiredReason:
        'Approval workflow, parent decision, policy audit, degraded states, and platform-specific social proof are manual-required.',
      partialPresent: evidence.intervention !== undefined,
      claimsProved: ['Managed intervention route artifacts can be indexed when present.'],
      claimsNotProved: ['Social account ownership, feed identity, and final parent/child approval UX are not claimed.'],
    }),
    partialRow({
      id: 'browser-game-cloud-gaming',
      label: 'Browser-game and cloud-gaming proof',
      artifacts: [
        jsonArtifact('managed-browser-intervention-proof-json', evidence.intervention),
        jsonArtifact('performance-health-proof-json', evidence.performanceHealth),
        countedArtifact(
          'game-cloud-screenshots',
          interventionScreenshots.filter((item) => item.includes('game') || item.includes('cloud'))
        ),
      ],
      manualRequiredReason:
        'Runtime signals, metadata, AI result, policy decision, time-budget action, and cloud-gaming heuristics are manual-required.',
      partialPresent: evidence.intervention !== undefined || performancePresent,
      claimsProved: ['Hold-page and manual-required performance rows can be indexed when artifacts exist.'],
      claimsNotProved: [
        'Canvas/WebGL/gamepad/fullscreen signals and cloud bandwidth/session heuristics are not claimed.',
      ],
    }),
    claimRow({
      id: 'unsupported-firefox-later-adapter',
      label: 'Unsupported Firefox or later-adapter state',
      present: unsupportedBrowserCount(evidence.profileMatrix?.data) > 0,
      artifacts: [jsonArtifact('profile-matrix-json', evidence.profileMatrix)],
      manualRequiredReason:
        'Unsupported/later-adapter proof requires a profile-matrix run that observes an installed unsupported browser state.',
      claimsProved: ['Unsupported browser states are indexed when profile-matrix proof includes them.'],
      claimsNotProved: ['Later adapter support is not claimed until a supported adapter exists.'],
    }),
    claimRow({
      id: 'windows-manual-proof-matrix',
      label: 'Windows manual proof matrix',
      present: evidence.windowsEnforcement !== undefined,
      artifacts: [jsonArtifact('windows-managed-unmanaged-proof-json', evidence.windowsEnforcement)],
      manualRequiredReason: 'Run the Windows managed/unmanaged proof before Windows release evidence claims.',
      claimsProved: ['Windows proof JSON is indexed when present.'],
      claimsNotProved: ['AppLocker/WDAC release enforcement remains separate from this artifact index.'],
    }),
    manualRow({
      id: 'mac-linux-android-ios-manual-matrices',
      label: 'macOS/Linux/Android/iOS manual matrices',
      manualRequiredReason:
        'Cross-platform browser proof matrices are manual-required until platform-specific browser/mobile work starts.',
      claimsNotProved: [
        'macOS browser support',
        'Linux browser support',
        'Android browser behavior',
        'iOS browser behavior',
      ],
    }),
  ];
}

function claimRow({ id, label, present, artifacts, manualRequiredReason, claimsProved, claimsNotProved }) {
  return {
    id,
    label,
    state: present ? 'artifact-present' : 'manual-required',
    artifacts,
    manualRequiredReason: present ? null : manualRequiredReason,
    claimsProved: present ? claimsProved : [],
    claimsNotProved: present ? claimsNotProved : [manualRequiredReason, ...claimsNotProved],
  };
}

function partialRow({ id, label, artifacts, manualRequiredReason, partialPresent, claimsProved, claimsNotProved }) {
  return {
    id,
    label,
    state: partialPresent ? 'partial-manual-required' : 'manual-required',
    artifacts,
    manualRequiredReason,
    claimsProved: partialPresent ? claimsProved : [],
    claimsNotProved: [manualRequiredReason, ...claimsNotProved],
  };
}

function manualRow({ id, label, manualRequiredReason, claimsNotProved }) {
  return {
    id,
    label,
    state: 'manual-required',
    artifacts: [],
    manualRequiredReason,
    claimsProved: [],
    claimsNotProved: [manualRequiredReason, ...claimsNotProved],
  };
}

function jsonArtifact(kind, item) {
  if (item === null) {
    return { kind, state: 'manual-required', path: null };
  }
  return { kind, state: 'present', path: relativePath(item.path) };
}

function countedArtifact(kind, paths) {
  return {
    kind,
    state: paths.length > 0 ? 'present' : 'manual-required',
    count: paths.length,
    paths: paths.slice(0, 20),
  };
}

function summarizeRows(rows) {
  return {
    totalRows: rows.length,
    artifactPresentRows: rows.filter((row) => row.state === 'artifact-present').length,
    partialManualRequiredRows: rows.filter((row) => row.state === 'partial-manual-required').length,
    manualRequiredRows: rows.filter((row) => row.state === 'manual-required').length,
  };
}

function manifestMarkdown(manifest) {
  const lines = [
    '# WP23 Artifact Manifest',
    '',
    `Generated: ${manifest.generatedAt}`,
    '',
    '| Claim | State | Evidence | Manual-required reason |',
    '| --- | --- | --- | --- |',
  ];
  for (const row of manifest.rows) {
    const evidence = row.artifacts
      .map((artifact) => {
        if (artifact.path !== undefined && artifact.path !== undefined) {
          return artifact.path;
        }
        if (artifact.count !== undefined) {
          return `${artifact.kind}: ${artifact.count}`;
        }
        return `${artifact.kind}: manual-required`;
      })
      .join('<br>');
    lines.push(
      `| ${row.label} | ${row.state} | ${evidence.length > 0 ? evidence : 'none'} | ${
        row.manualRequiredReason ?? 'none'
      } |`
    );
  }
  lines.push('', '## Summary', '');
  lines.push(`- Total rows: ${manifest.summary.totalRows}`);
  lines.push(`- Artifact-present rows: ${manifest.summary.artifactPresentRows}`);
  lines.push(`- Partial manual-required rows: ${manifest.summary.partialManualRequiredRows}`);
  lines.push(`- Manual-required rows: ${manifest.summary.manualRequiredRows}`);
  return lines.join('\n');
}

function browserFamilies(values) {
  const families = new Set();
  if (!Array.isArray(values)) {
    return families;
  }
  for (const item of values) {
    const family = item?.browser?.family;
    if (typeof family === 'string') {
      families.add(family);
    }
  }
  return families;
}

function unsupportedBrowserCount(data) {
  const count = data?.summary?.unsupportedInstalledBrowserCount;
  return Number.isInteger(count) ? count : 0;
}

async function latestTopLevelJson(relativeDirectory) {
  const directory = join(root, relativeDirectory);
  if (!existsSync(directory)) {
    return null;
  }
  const entries = await readdir(directory, { withFileTypes: true });
  const files = [];
  for (const entry of entries) {
    if (!entry.isFile() || !entry.name.endsWith('.json')) {
      continue;
    }
    const path = join(directory, entry.name);
    const stats = await stat(path);
    files.push({ path, mtimeMs: stats.mtimeMs });
  }
  files.sort((left, right) => right.mtimeMs - left.mtimeMs);
  if (files.length === 0) {
    return null;
  }
  return readJson(files[0].path);
}

async function jsonFile(relativeFile) {
  const path = join(root, relativeFile);
  if (!existsSync(path)) {
    return null;
  }
  return readJson(path);
}

async function readJson(path) {
  const content = await readFile(path, 'utf8');
  return { path, data: JSON.parse(content) };
}

async function filesWithExtension(relativeDirectory, extension) {
  const directory = join(root, relativeDirectory);
  if (!existsSync(directory)) {
    return [];
  }
  const matches = [];
  await collectFiles(directory, extension, matches);
  return matches.map(relativePath).sort();
}

async function collectFiles(directory, extension, matches) {
  const entries = await readdir(directory, { withFileTypes: true });
  for (const entry of entries) {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) {
      await collectFiles(path, extension, matches);
      continue;
    }
    if (entry.isFile() && entry.name.endsWith(extension)) {
      matches.push(path);
    }
  }
}

function relativePath(path) {
  return relative(root, path).replaceAll('\\', '/');
}
