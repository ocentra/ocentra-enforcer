import { readLines } from './source-policy-helpers.mjs';
import { scanPackageManifestDependencySection } from './source-policy-typescript-package-manifest-dependencies-section.mjs';

const SECTIONS = ['dependencies', 'devDependencies', 'peerDependencies', 'optionalDependencies'];

export function scanLoosePackageManifestVersions(root, filePath) {
  let parsed;
  try {
    parsed = JSON.parse(readLines(filePath).join('\n'));
  } catch {
    return [];
  }
  return SECTIONS.flatMap((section) =>
    scanPackageManifestDependencySection(
      root,
      filePath,
      parsed,
      section,
      (_name, version) => /^(?:\^|~|\*|latest\b|git\+|github:|file:|link:|workspace:\*)/iu.test(version),
      'TS-7.11',
      (name, version) => `loose dependency version ${name}@${version}`,
    ),
  );
}
