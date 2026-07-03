import { readLines } from './source-policy-helpers.mjs';
import { scanPackageManifestDependencySection } from './source-policy-typescript-package-manifest-dependencies-section.mjs';

const SECTIONS = ['dependencies', 'devDependencies', 'peerDependencies', 'optionalDependencies'];
const SCHEMA_NAMES = new Set(['zod', 'zod-to-json-schema', 'zod-validation-error']);

export function scanDirectSchemaPackageManifestDependencies(root, filePath) {
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
      (name) => SCHEMA_NAMES.has(name),
      'TS-7.14',
      (name, _version, sectionName) => `direct ${name} dependency in ${sectionName}`,
    ),
  );
}
