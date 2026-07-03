import { scanLoosePackageManifestVersions } from './source-policy-typescript-package-manifest-dependencies-loose.mjs';
import { scanDirectSchemaPackageManifestDependencies } from './source-policy-typescript-package-manifest-dependencies-schema.mjs';

export function scanPackageManifestDependencies(root, filePath) {
  return [
    ...scanLoosePackageManifestVersions(root, filePath),
    ...scanDirectSchemaPackageManifestDependencies(root, filePath),
  ];
}
